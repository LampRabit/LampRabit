"""Export the user's own prompts from Claude Code session transcripts into .ai/sessions/.

Usage (from the repository root):
  python .ai/tools/export-prompts.py <transcripts_dir> .ai/sessions [session_id_to_skip]

<transcripts_dir> is Claude Code's project folder, e.g. ~/.claude/projects/<project-slug>/ .
Writes one file per session: <date>-<short id>.md. Existing files are overwritten, so a
re-run refreshes them. Check the output for secrets before committing: the repo is public.
"""
import json, os, re, sys

src, out = sys.argv[1], sys.argv[2]
skip = sys.argv[3] if len(sys.argv) > 3 else None
os.makedirs(out, exist_ok=True)

NOISE = re.compile(r"<(system-reminder|ide_opened_file|ide_selection|command-message|command-name|command-args|local-command-stdout|local-command-caveat)>.*?</\1>", re.S)

def text_of(content):
    if isinstance(content, str):
        return content
    parts = []
    for c in content or []:
        if isinstance(c, dict) and c.get("type") == "text":
            parts.append(c.get("text", ""))
        elif isinstance(c, dict) and c.get("type") == "tool_result":
            return None  # tool output, not the user
    return "\n".join(parts)

index = []
for name in sorted(os.listdir(src)):
    if not name.endswith(".jsonl"):
        continue
    sid = name[:-6]
    if sid == skip:
        continue
    prompts, first_ts, summary = [], None, None
    with open(os.path.join(src, name), encoding="utf-8") as f:
        for line in f:
            try:
                d = json.loads(line)
            except Exception:
                continue
            if d.get("type") == "summary" and not summary:
                summary = d.get("summary")
            if d.get("type") != "user" or d.get("isSidechain") or d.get("isMeta"):
                continue
            if d.get("isCompactSummary"):
                continue
            msg = d.get("message", {})
            t = text_of(msg.get("content"))
            if not t:
                continue
            t = NOISE.sub("", t).strip()
            if not t or t.startswith("[Request interrupted") or t.startswith("This session is being continued"):
                continue
            if t.startswith("Caveat:") or t.startswith("<task-notification>"):
                continue
            ts = d.get("timestamp", "")
            first_ts = first_ts or ts
            prompts.append((ts, t))
    if not prompts:
        continue
    date = first_ts[:10]
    fname = f"{date}-{sid[:8]}.md"
    with open(os.path.join(out, fname), "w", encoding="utf-8", newline="\n") as w:
        w.write(f"# Session {sid[:8]} ({date})\n\n")
        w.write(f"Full id: `{sid}`. {len(prompts)} prompts from the user, verbatim, in order.\n")
        if summary:
            w.write(f"Claude Code's summary: {summary}\n")
        w.write("\n")
        for i, (ts, t) in enumerate(prompts, 1):
            w.write(f"## {i}. {ts[:16].replace('T', ' ')} UTC\n\n{t}\n\n")
    first = prompts[0][1].splitlines()[0][:120]
    index.append((date, fname, len(prompts), first))

for date, fname, n, first in index:
    print(f"{date}\t{fname}\t{n}\t{first}")
