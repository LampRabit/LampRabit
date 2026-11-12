import type { GitStatus } from "./api";

/** Pure Git helpers for the Git tab, kept apart from React so they're tested on their own. */

/** What the repository needs next, in order of urgency. */
export function nextStep(s: GitStatus): "resolve" | "commit" | "pull" | "push" | "publish" | "remote" | null {
  if (s.operation) return "resolve";
  if (s.files.length > 0) return "commit";
  if (s.behind > 0) return "pull";
  if (s.remotes.length === 0) return s.head ? "remote" : null;
  if (s.branch && !s.upstream && s.head) return "publish";
  if (s.ahead > 0) return "push";
  return null;
}

export type DiffRow = { kind: "hunk"; text: string } | { kind: "add" | "del" | "ctx"; old: number | null; new: number | null; text: string } | { kind: "note"; text: string };

/** A unified diff as rows with old and new line numbers, the way editors show it. */
export function parseDiff(text: string): DiffRow[] {
  const rows: DiffRow[] = [];
  let oldNo = 0;
  let newNo = 0;
  let inHunk = false;
  for (const line of text.split("\n")) {
    const hunk = /^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@ ?(.*)$/.exec(line);
    if (hunk) {
      oldNo = Number(hunk[1]);
      newNo = Number(hunk[2]);
      inHunk = true;
      rows.push({ kind: "hunk", text: hunk[3] });
      continue;
    }
    if (!inHunk) {
      if (/^Binary files/.test(line)) rows.push({ kind: "note", text: "Binary file: no text to show." });
      continue;
    }
    if (line.startsWith("+")) rows.push({ kind: "add", old: null, new: newNo++, text: line.slice(1) });
    else if (line.startsWith("-")) rows.push({ kind: "del", old: oldNo++, new: null, text: line.slice(1) });
    else if (line.startsWith("\\")) rows.push({ kind: "note", text: line.slice(2) });
    else if (line.startsWith(" ")) rows.push({ kind: "ctx", old: oldNo++, new: newNo++, text: line.slice(1) });
    else if (line.startsWith("diff ")) inHunk = false;
  }
  return rows;
}

/** https://host/owner/repo.git ⇄ git@host:owner/repo.git; null when the URL is neither. */
export function convertUrl(url: string): { url: string; to: "SSH" | "HTTPS" } | null {
  const https = /^https:\/\/(?:[^@/]+@)?([^/]+)\/(.+?)(?:\.git)?\/?$/.exec(url);
  if (https) return { url: `git@${https[1]}:${https[2]}.git`, to: "SSH" };
  const ssh = /^(?:ssh:\/\/)?git@([^:/]+)[:/](.+?)(?:\.git)?$/.exec(url);
  if (ssh) return { url: `https://${ssh[1]}/${ssh[2]}.git`, to: "HTTPS" };
  return null;
}

export const isSsh = (url: string) => /^(ssh:\/\/|[\w.-]+@[\w.-]+:)/.test(url);
