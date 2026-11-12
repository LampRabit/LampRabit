import { useEffect, useMemo, useState, type ReactNode } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ArrowDown, ArrowUp, Check, GitBranch, GitCommitHorizontal, GitMerge, Minus, Pencil, Plus, RefreshCw, RotateCcw, Trash2, Undo2, UserRound, X } from "lucide-react";
import clsx from "clsx";
import { api, type GitBranch as Branch, type GitFile, type Site } from "../lib/api";
import { useAppStore } from "../lib/store";
import { formatAgo } from "../lib/time";
import Dialog from "./Dialog";
import { Button, inputClass } from "./ui";

/**
 * A project's Git: changes with diffs, staging and commits, history, branches, remotes and
 * the repository's own identity. Everything goes through the git command, so the user's
 * credential helpers and SSH keys work; fetch, pull and push never prompt on a terminal.
 */

type View = "changes" | "history" | "branches" | "remotes";

export default function ProjectGit({ site }: { site: Site }) {
  const queryClient = useQueryClient();
  const notify = useAppStore((s) => s.notify);
  const onError = (e: unknown) => notify("error", String(e));
  const version = useQuery({ queryKey: ["git-version"], queryFn: api.gitVersion, staleTime: Infinity });
  const status = useQuery({ queryKey: ["git-status", site.name], queryFn: () => api.gitStatus(site.name), refetchInterval: 5000, enabled: !!version.data });
  const [view, setView] = useState<View>("changes");
  // Whatever changed the repository refreshes everything that shows it.
  const refresh = () => {
    for (const key of ["git-status", "git-branches", "git-log", "project-facts"]) queryClient.invalidateQueries({ queryKey: [key, site.name] });
    queryClient.invalidateQueries({ queryKey: ["sites"] });
  };
  const init = useMutation({ mutationFn: () => api.gitInit(site.name), onSuccess: refresh, onError });
  const sync = useMutation({
    mutationFn: (action: "fetch" | "pull" | "push") => api.gitSync(site.name, action),
    onSuccess: (out, action) => {
      notify("info", action === "fetch" ? "Fetched" : action === "pull" ? "Pulled" : "Pushed", out.trim() ? out.trim().split("\n").slice(-6) : undefined);
      refresh();
    },
    onError: (e) => {
      onError(e);
      refresh();
    },
  });
  const abort = useMutation({ mutationFn: () => api.gitAbort(site.name), onSuccess: refresh, onError });
  const carryOn = useMutation({ mutationFn: () => api.gitContinue(site.name), onSuccess: refresh, onError });

  if (version.isLoading) return <p className="p-5 text-sm text-muted">Looking for Git…</p>;
  if (!version.data) {
    return (
      <div className="p-5 text-sm">
        <p className="font-semibold text-fg">Git isn't installed</p>
        <p className="mt-1 text-muted">Install Git from git-scm.com (or your package manager), then reopen this tab. LampRabit uses your own Git, with your credentials and SSH keys.</p>
      </div>
    );
  }
  const s = status.data;
  if (!s) return <p className="p-5 text-sm text-muted">{status.isError ? String(status.error) : "Reading the repository…"}</p>;
  if (!s.repo) {
    return (
      <div className="flex flex-wrap items-center justify-between gap-3 p-5">
        <div className="text-sm">
          <p className="font-semibold text-fg">This project isn't a Git repository</p>
          <p className="text-muted">Start one to track changes, branch and push to GitHub, GitLab or any remote.</p>
        </div>
        <Button variant="primary" onClick={() => init.mutate()} disabled={init.isPending}>
          <GitBranch size={14} /> Initialize a repository
        </Button>
      </div>
    );
  }

  const conflicts = s.files.filter((f) => f.conflicted).length;
  // Where the branch stands against its remote, in a few characters (the branch itself is in the card's header).
  const where = s.remotes.length === 0 ? "No remote" : !s.branch ? `Detached at ${s.head ?? "?"}` : !s.upstream ? "Not pushed yet" : s.ahead || s.behind ? null : "Up to date";
  const views: [View, string, number?][] = [
    ["changes", "Changes", s.files.length],
    ["history", "History"],
    ["branches", "Branches"],
    ["remotes", "Remotes"],
  ];
  const syncButton = (action: "fetch" | "pull" | "push", icon: ReactNode, label: string, disabled: boolean, title: string) => (
    <button
      type="button"
      onClick={() => sync.mutate(action)}
      disabled={sync.isPending || disabled}
      title={title}
      className="flex items-center gap-1 rounded-md px-2 py-1 text-xs font-medium text-muted hover:bg-surface-2 hover:text-fg disabled:opacity-40 disabled:hover:bg-transparent"
    >
      {sync.isPending && sync.variables === action ? <RefreshCw size={12} className="animate-spin" /> : icon} {label}
    </button>
  );

  return (
    <div className="space-y-3 p-4">
      <div className="flex flex-wrap items-center gap-1 border-b border-line pb-2">
        <div role="tablist" aria-label="Git" className="flex flex-wrap gap-0.5">
          {views.map(([id, label, count]) => (
            <button
              key={id}
              role="tab"
              aria-selected={view === id}
              onClick={() => setView(id)}
              className={clsx("rounded-md px-2.5 py-1 text-sm", view === id ? "bg-surface-2 font-semibold text-fg" : "text-muted hover:text-fg")}
            >
              {label}
              {!!count && <span className={clsx("ml-1.5 rounded-full px-1.5 text-[11px] tabular-nums", view === id ? "bg-accent-soft text-accent-text" : "bg-surface-2")}>{count}</span>}
            </button>
          ))}
        </div>
        <span className="flex-1" />
        <span className="mr-1 text-xs text-muted" title={s.upstream ? `Tracks ${s.upstream}` : undefined}>
          {where ?? (
            <>
              {s.ahead > 0 && <span className="text-accent-text">{s.ahead} to push</span>}
              {s.ahead > 0 && s.behind > 0 && " · "}
              {s.behind > 0 && <span className="text-warn">{s.behind} to pull</span>}
            </>
          )}
        </span>
        {syncButton("fetch", <RefreshCw size={12} />, "Fetch", s.remotes.length === 0, "Get the remotes' new commits and branches without changing your files")}
        {syncButton("pull", <ArrowDown size={12} />, "Pull", !s.upstream, s.upstream ? `Merge ${s.upstream} into ${s.branch}` : "The branch doesn't track a remote branch yet")}
        {syncButton("push", <ArrowUp size={12} />, "Push", s.remotes.length === 0 || !s.branch, s.remotes.length === 0 ? "Add a remote first (Remotes)" : s.upstream ? `Send your commits to ${s.upstream}` : "Push this branch and track it on the remote")}
      </div>

      {s.operation && (
        <div className="flex flex-wrap items-center gap-3 rounded-lg border border-warn/30 bg-warn/10 px-3 py-2 text-sm" role="status">
          <GitMerge size={15} className="text-warn" />
          <span className="flex-1 text-fg">
            A {s.operation} is in progress
            {conflicts > 0 ? `: ${conflicts} ${conflicts === 1 ? "file has" : "files have"} conflicts. Fix them in your editor, then mark them resolved.` : ". Everything is resolved."}
          </span>
          <Button size="sm" variant="primary" onClick={() => carryOn.mutate()} disabled={conflicts > 0 || carryOn.isPending}>
            Continue {s.operation}
          </Button>
          <Button size="sm" variant="ghost" onClick={() => abort.mutate()} disabled={abort.isPending}>
            Abort
          </Button>
        </div>
      )}

      {view === "changes" && <Changes site={site} files={s.files} firstCommit={!s.head} identityMissing={!s.identity.name || !s.identity.email} onChanged={refresh} onIdentity={() => setView("remotes")} />}
      {view === "history" && <History site={site} />}
      {view === "branches" && <Branches site={site} current={s.branch} onChanged={refresh} />}
      {view === "remotes" && <RemotesAndIdentity site={site} remotes={s.remotes} identity={s.identity} onChanged={refresh} />}
    </div>
  );
}

const letters: Record<string, string> = { M: "Modified", A: "Added", D: "Deleted", R: "Renamed", C: "Copied", T: "Type changed", U: "Conflict", "?": "New, not tracked yet" };

/** The two panes (list, diff) share one height, so the diff gets the room and the page doesn't grow. */
const panes = "grid gap-3 lg:h-[min(64vh,40rem)] lg:grid-cols-[minmax(16rem,22rem)_minmax(0,1fr)]";

function Changes({
  site,
  files,
  firstCommit,
  identityMissing,
  onChanged,
  onIdentity,
}: {
  site: Site;
  files: GitFile[];
  firstCommit: boolean;
  identityMissing: boolean;
  onChanged: () => void;
  onIdentity: () => void;
}) {
  const notify = useAppStore((s) => s.notify);
  const onError = (e: unknown) => notify("error", String(e));
  const staged = files.filter((f) => !f.conflicted && f.staged !== " " && f.staged !== "?");
  const unstaged = files.filter((f) => !f.conflicted && (f.unstaged !== " " || f.staged === "?"));
  const conflicted = files.filter((f) => f.conflicted);
  const [selected, setSelected] = useState<{ path: string; staged: boolean } | null>(null);
  const [message, setMessage] = useState("");
  const [amend, setAmend] = useState(false);
  const [discarding, setDiscarding] = useState<string[] | null>(null);

  // The selection clears when its file is committed or discarded.
  const current = selected && files.find((f) => f.path === selected.path);
  useEffect(() => {
    if (selected && !current) setSelected(null);
  }, [selected, current]);

  const stage = useMutation({ mutationFn: ({ paths, on }: { paths: string[]; on: boolean }) => api.gitStage(site.name, paths, on), onSuccess: onChanged, onError });
  const discard = useMutation({
    mutationFn: (paths: string[]) => api.gitDiscard(site.name, paths),
    onSuccess: () => {
      setDiscarding(null);
      onChanged();
    },
    onError,
  });
  const commit = useMutation({
    mutationFn: () => api.gitCommit(site.name, message, amend),
    onSuccess: () => {
      notify("info", amend ? "Amended the last commit" : "Committed");
      setMessage("");
      setAmend(false);
      onChanged();
    },
    onError,
  });
  const canCommit = (staged.length > 0 || amend) && (!!message.trim() || amend) && conflicted.length === 0;

  const group = (title: string, list: GitFile[], kind: "staged" | "unstaged" | "conflict", action?: ReactNode) =>
    list.length > 0 && (
      <section>
        <div className="sticky top-0 z-[1] flex items-center justify-between border-b border-line bg-surface px-3 py-1.5">
          <span className="text-[11px] font-semibold tracking-wide text-muted uppercase">
            {title} <span className="tabular-nums">{list.length}</span>
          </span>
          {action}
        </div>
        <ul>
          {list.map((f) => {
            const isStaged = kind === "staged";
            const code = kind === "conflict" ? "U" : isStaged ? f.staged : f.staged === "?" ? "?" : f.unstaged;
            const active = selected?.path === f.path && selected.staged === isStaged;
            const slash = f.path.lastIndexOf("/");
            const name = f.path.slice(slash + 1);
            const folder = slash > 0 ? f.path.slice(0, slash) : "";
            return (
              <li key={`${kind}:${f.path}`} className={clsx("group flex items-center gap-1 pr-1.5 pl-3", active ? "bg-accent-soft/50" : "hover:bg-surface-2")}>
                <button type="button" onClick={() => setSelected({ path: f.path, staged: isStaged })} className="flex min-w-0 flex-1 items-baseline gap-2 py-1 text-left" title={f.from ? `${f.from} → ${f.path}` : f.path}>
                  <span
                    className={clsx("w-3 shrink-0 font-mono text-[11px] font-bold", code === "D" ? "text-danger-text" : code === "?" || code === "A" ? "text-ok" : code === "U" ? "text-warn" : "text-accent-text")}
                    title={letters[code] ?? code}
                    aria-label={letters[code] ?? code}
                  >
                    {code === "?" ? "+" : code}
                  </span>
                  <span className="truncate text-[13px] text-fg">{name}</span>
                  {folder && <span className="min-w-0 truncate text-[11px] text-muted">{folder}</span>}
                </button>
                {kind !== "staged" && (
                  <button
                    type="button"
                    onClick={() => setDiscarding([f.path])}
                    className="rounded p-1 text-muted opacity-0 group-hover:opacity-100 hover:text-danger-text focus-visible:opacity-100"
                    title="Discard these changes"
                    aria-label={`Discard changes to ${f.path}`}
                  >
                    <Undo2 size={13} />
                  </button>
                )}
                <button
                  type="button"
                  onClick={() => stage.mutate({ paths: [f.path], on: !isStaged })}
                  disabled={stage.isPending}
                  className="rounded p-1 text-muted opacity-60 group-hover:opacity-100 hover:text-fg focus-visible:opacity-100"
                  title={isStaged ? "Unstage: leave it out of the commit" : kind === "conflict" ? "Mark as resolved" : "Stage: include it in the commit"}
                  aria-label={`${isStaged ? "Unstage" : "Stage"} ${f.path}`}
                >
                  {isStaged ? <Minus size={13} /> : kind === "conflict" ? <Check size={13} /> : <Plus size={13} />}
                </button>
              </li>
            );
          })}
        </ul>
      </section>
    );

  const textButton = "text-[11px] font-medium hover:underline";
  return (
    <div className={panes}>
      <div className="flex min-h-64 flex-col overflow-hidden rounded-lg border border-line">
        <div className="min-h-0 flex-1 overflow-y-auto">
          {files.length === 0 && <p className="p-6 text-center text-sm text-muted">No changes. Everything is committed.</p>}
          {group("Conflicts", conflicted, "conflict")}
          {group(
            "Staged",
            staged,
            "staged",
            <button type="button" onClick={() => stage.mutate({ paths: staged.map((f) => f.path), on: false })} className={clsx(textButton, "text-muted")}>
              Unstage all
            </button>,
          )}
          {group(
            "Not staged",
            unstaged,
            "unstaged",
            <span className="flex gap-3">
              <button type="button" onClick={() => setDiscarding(unstaged.map((f) => f.path))} className={clsx(textButton, "text-danger-text")}>
                Discard all
              </button>
              <button type="button" onClick={() => stage.mutate({ paths: unstaged.map((f) => f.path), on: true })} className={clsx(textButton, "text-accent-text")}>
                Stage all
              </button>
            </span>,
          )}
        </div>
        <div className="space-y-2 border-t border-line bg-surface-2/40 p-2.5">
          {/* A new project's template stages its files; say so before the first commit, so a big
              number on the button isn't a surprise. */}
          {firstCommit && staged.length > 0 && (
            <p className="text-[11px] text-muted">No commits yet. The project's files are staged for a first commit; unstage any you want to leave out.</p>
          )}
          {identityMissing && (
            <p className="text-[11px] text-warn">
              Git doesn't know your name and email yet.{" "}
              <button type="button" onClick={onIdentity} className="font-medium underline">
                Set them
              </button>
            </p>
          )}
          <textarea
            value={message}
            onChange={(e) => setMessage(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && (e.ctrlKey || e.metaKey) && canCommit) commit.mutate();
            }}
            rows={2}
            placeholder={amend ? "New message (empty keeps the last one)" : firstCommit ? "Initial commit" : "Commit message"}
            aria-label="Commit message"
            className={clsx(inputClass, "resize-none py-1.5 font-sans")}
          />
          <div className="flex items-center gap-2">
            {!firstCommit && (
              <label className="flex items-center gap-1.5 text-[11px] text-muted" title="Add the staged changes to the last commit instead of making a new one">
                <input type="checkbox" checked={amend} onChange={(e) => setAmend(e.target.checked)} className="size-3.5 accent-[var(--accent)]" /> Amend
              </label>
            )}
            <span className="flex-1 text-right text-[11px] text-muted">{staged.length > 0 ? `${staged.length} staged` : unstaged.length ? "Stage files to commit" : ""}</span>
            <Button size="sm" variant={canCommit ? "primary" : "secondary"} onClick={() => commit.mutate()} disabled={!canCommit || commit.isPending} title="Commit the staged files (Ctrl+Enter)">
              <GitCommitHorizontal size={13} /> {commit.isPending ? "Committing…" : "Commit"}
            </Button>
          </div>
        </div>
      </div>
      <div className="flex min-h-64 min-w-0 flex-col overflow-hidden rounded-lg border border-white/10 bg-terminal">
        {selected && current ? (
          <FileDiff site={site} file={current} staged={selected.staged} />
        ) : (
          <div className="flex flex-1 items-center justify-center p-4 text-sm text-slate-400">{files.length ? "Select a file to see its changes." : "Nothing to show."}</div>
        )}
      </div>
      {discarding && (
        <Dialog
          open
          onClose={() => setDiscarding(null)}
          title={discarding.length === 1 ? "Discard these changes?" : `Discard changes to ${discarding.length} files?`}
          icon={<Undo2 size={18} />}
          footer={
            <>
              <Button onClick={() => setDiscarding(null)} data-autofocus>
                Keep them
              </Button>
              <Button variant="danger" onClick={() => discard.mutate(discarding)} disabled={discard.isPending}>
                Discard
              </Button>
            </>
          }
        >
          <p className="text-sm text-muted">The files go back to their last committed state; new files are deleted. This can't be undone.</p>
          <ul className="mt-2 max-h-40 overflow-y-auto font-mono text-xs text-fg">
            {discarding.map((p) => (
              <li key={p} className="truncate">
                {p}
              </li>
            ))}
          </ul>
        </Dialog>
      )}
    </div>
  );
}

function FileDiff({ site, file, staged }: { site: Site; file: GitFile; staged: boolean }) {
  const untracked = file.staged === "?";
  const diff = useQuery({
    queryKey: ["git-diff", site.name, file.path, staged, file.staged, file.unstaged],
    queryFn: () => api.gitDiff(site.name, file.path, staged, untracked),
  });
  return (
    <>
      <div className="flex shrink-0 items-center gap-2 border-b border-white/10 bg-terminal-2 px-3 py-1.5 text-xs">
        <span className="truncate font-mono text-slate-200">{file.path}</span>
        <span className="shrink-0 text-slate-400">{staged ? "staged" : untracked ? "new file" : "not staged"}</span>
      </div>
      <DiffText text={diff.data} loading={diff.isLoading} error={diff.isError ? String(diff.error) : null} />
    </>
  );
}

/**
 * A unified diff, coloured by line, without git's header noise (diff --git, index, ---/+++
 * repeat what the pane's title says); it scrolls inside its pane.
 */
function DiffText({ text, loading, error }: { text?: string; loading?: boolean; error?: string | null }) {
  const lines = useMemo(() => (text ?? "").split("\n").filter((l) => !/^(diff --git |index [0-9a-f]+\.\.|--- |\+\+\+ |new file mode|deleted file mode|similarity index)/.test(l)), [text]);
  if (loading) return <p className="p-3 text-xs text-slate-400">Loading…</p>;
  if (error) return <p className="p-3 text-xs text-red-300">{error}</p>;
  if (!text?.trim()) return <p className="p-3 text-xs text-slate-400">No text changes (a binary file, or only its mode changed).</p>;
  return (
    <pre className="selectable min-h-0 flex-1 overflow-auto py-1.5 font-mono text-[12px] leading-5">
      {lines.map((line, i) => (
        <div
          key={i}
          className={clsx(
            "px-3 whitespace-pre",
            line.startsWith("+") && "bg-emerald-500/10 text-emerald-300",
            line.startsWith("-") && "bg-red-500/10 text-red-300",
            line.startsWith("@@") && "mt-1 text-sky-300/80",
            /^(commit |Author:|Date:)/.test(line) && "text-slate-400",
            !/^[+\-@]/.test(line) && "text-slate-300",
          )}
        >
          {line || " "}
        </div>
      ))}
    </pre>
  );
}

function History({ site }: { site: Site }) {
  const [pages, setPages] = useState(1);
  const log = useQuery({ queryKey: ["git-log", site.name, pages], queryFn: () => api.gitLog(site.name, 0, pages * 50) });
  const [open, setOpen] = useState<string | null>(null);
  const show = useQuery({ queryKey: ["git-show", site.name, open], queryFn: () => api.gitShow(site.name, open!), enabled: !!open });
  const commits = log.data ?? [];
  return (
    <div className={panes}>
      <div className="min-h-64 overflow-y-auto rounded-lg border border-line">
        {log.isLoading && <p className="p-4 text-sm text-muted">Loading history…</p>}
        {!log.isLoading && commits.length === 0 && <p className="p-6 text-center text-sm text-muted">No commits yet.</p>}
        <ul>
          {commits.map((c) => (
            <li key={c.hash}>
              <button
                type="button"
                onClick={() => setOpen(c.hash === open ? null : c.hash)}
                aria-pressed={c.hash === open}
                className={clsx("block w-full border-b border-line/60 px-3 py-1.5 text-left", c.hash === open ? "bg-accent-soft/50" : "hover:bg-surface-2")}
              >
                <span className="block truncate text-[13px] text-fg">{c.subject}</span>
                <span className="flex items-center gap-1.5 text-[11px] text-muted">
                  <span className="font-mono">{c.short}</span>·
                  <span className="truncate" title={c.email}>
                    {c.author}
                  </span>
                  <span className="shrink-0">· {formatAgo(c.time * 1000)}</span>
                  {c.refs && <span className="truncate font-mono text-accent-text">{c.refs.replace(/^HEAD -> /, "")}</span>}
                </span>
              </button>
            </li>
          ))}
        </ul>
        {commits.length === pages * 50 && (
          <button type="button" className="w-full py-2 text-xs font-medium text-accent-text hover:underline" onClick={() => setPages((p) => p + 1)}>
            Show older commits
          </button>
        )}
      </div>
      <div className="flex min-h-64 min-w-0 flex-col overflow-hidden rounded-lg border border-white/10 bg-terminal">
        {open ? (
          <DiffText text={show.data} loading={show.isLoading} error={show.isError ? String(show.error) : null} />
        ) : (
          <div className="flex flex-1 items-center justify-center p-4 text-sm text-slate-400">Select a commit to see what it changed.</div>
        )}
      </div>
    </div>
  );
}

function Branches({ site, current, onChanged }: { site: Site; current: string | null; onChanged: () => void }) {
  const notify = useAppStore((s) => s.notify);
  const onError = (e: unknown) => notify("error", String(e));
  const list = useQuery({ queryKey: ["git-branches", site.name], queryFn: () => api.gitBranches(site.name) });
  const [name, setName] = useState("");
  const [renaming, setRenaming] = useState<{ from: string; to: string } | null>(null);
  const [deleting, setDeleting] = useState<{ branch: string; force: boolean; why?: string } | null>(null);
  const act = useMutation({
    mutationFn: (action: Parameters<typeof api.gitBranch>[1]) => api.gitBranch(site.name, action),
    onSuccess: (_, action) => {
      if (action.kind === "create") setName("");
      if (action.kind === "rename") setRenaming(null);
      if (action.kind === "delete") setDeleting(null);
      if (action.kind === "merge") notify("info", `Merged ${action.branch} into ${current}`);
      onChanged();
      list.refetch();
    },
    onError: (e, action) => {
      // An unmerged branch asks again, with the reason, before deleting it anyway.
      if (action.kind === "delete" && !action.force && String(e).includes("aren't merged")) setDeleting({ branch: action.branch, force: true, why: String(e) });
      else onError(e);
      onChanged();
    },
  });
  const local = (list.data ?? []).filter((b) => !b.remote);
  const remote = (list.data ?? []).filter((b) => b.remote && !local.some((l) => l.upstream === b.name));

  const row = (b: Branch) => (
    <li key={b.name} className="flex flex-wrap items-center gap-2 border-b border-line/60 px-3 py-2 last:border-0">
      <span className="min-w-0 flex-1">
        {renaming?.from === b.name ? (
          <form
            className="flex gap-2"
            onSubmit={(e) => {
              e.preventDefault();
              act.mutate({ kind: "rename", from: b.name, to: renaming.to.trim() });
            }}
          >
            <input autoFocus value={renaming.to} onChange={(e) => setRenaming({ ...renaming, to: e.target.value })} className={clsx(inputClass, "py-1 font-mono")} aria-label="New branch name" />
            <Button size="sm" type="submit" disabled={!renaming.to.trim() || act.isPending}>
              Rename
            </Button>
            <Button size="sm" variant="ghost" type="button" onClick={() => setRenaming(null)} aria-label="Cancel">
              <X size={13} />
            </Button>
          </form>
        ) : (
          <>
            <span className={clsx("block truncate font-mono text-sm", b.current ? "font-bold text-fg" : "text-fg")}>
              {b.current && <Check size={12} className="mr-1 inline text-ok" />}
              {b.name}
              {b.track && <span className="ml-2 font-sans text-[11px] text-muted">{b.track}</span>}
            </span>
            <span className="block truncate text-[11px] text-muted">
              {b.subject} · {formatAgo(b.time * 1000)}
            </span>
          </>
        )}
      </span>
      {!b.current && renaming?.from !== b.name && (
        <>
          <Button size="sm" onClick={() => act.mutate({ kind: "switch", branch: b.name })} disabled={act.isPending} title={b.remote ? "Make a local branch tracking it, and switch to it" : "Switch to this branch"}>
            Switch
          </Button>
          {current && (
            <Button size="sm" variant="ghost" onClick={() => act.mutate({ kind: "merge", branch: b.name })} disabled={act.isPending} title={`Merge ${b.name} into ${current}`}>
              <GitMerge size={13} /> Merge
            </Button>
          )}
        </>
      )}
      {!b.remote && renaming?.from !== b.name && (
        <Button size="sm" variant="ghost" onClick={() => setRenaming({ from: b.name, to: b.name })} aria-label={`Rename ${b.name}`} title="Rename">
          <Pencil size={13} />
        </Button>
      )}
      {!b.remote && !b.current && (
        <Button size="sm" variant="ghost" onClick={() => setDeleting({ branch: b.name, force: false })} aria-label={`Delete ${b.name}`} title="Delete">
          <Trash2 size={13} />
        </Button>
      )}
    </li>
  );

  return (
    <div className="space-y-4">
      <form
        className="flex flex-wrap gap-2"
        onSubmit={(e) => {
          e.preventDefault();
          act.mutate({ kind: "create", branch: name.trim(), from: null, switch: true });
        }}
      >
        <input value={name} onChange={(e) => setName(e.target.value)} placeholder="new-branch-name" aria-label="New branch name" className={clsx(inputClass.replace("w-full", ""), "min-w-48 flex-1 font-mono")} />
        <Button type="submit" variant="primary" disabled={!name.trim() || act.isPending} title={`Create it from ${current ?? "HEAD"} and switch to it`}>
          <Plus size={14} /> Create branch
        </Button>
      </form>
      {list.isLoading && <p className="text-sm text-muted">Loading branches…</p>}
      {local.length > 0 && (
        <section>
          <h4 className="px-1 pb-1 text-[11px] font-bold tracking-wider text-muted uppercase">Local</h4>
          <ul className="overflow-hidden rounded-lg border border-line">{local.map(row)}</ul>
        </section>
      )}
      {remote.length > 0 && (
        <section>
          <h4 className="px-1 pb-1 text-[11px] font-bold tracking-wider text-muted uppercase">Remote only</h4>
          <ul className="overflow-hidden rounded-lg border border-line">{remote.map(row)}</ul>
        </section>
      )}
      {deleting && (
        <Dialog
          open
          onClose={() => setDeleting(null)}
          title={`Delete ${deleting.branch}?`}
          icon={<Trash2 size={18} />}
          footer={
            <>
              <Button onClick={() => setDeleting(null)} data-autofocus>
                Keep it
              </Button>
              <Button variant="danger" onClick={() => act.mutate({ kind: "delete", branch: deleting.branch, force: deleting.force })} disabled={act.isPending}>
                {deleting.force ? "Delete anyway" : "Delete"}
              </Button>
            </>
          }
        >
          <p className="text-sm text-muted">{deleting.why ?? "The local branch goes; the remote one (if any) stays."}</p>
        </Dialog>
      )}
    </div>
  );
}

function RemotesAndIdentity({
  site,
  remotes,
  identity,
  onChanged,
}: {
  site: Site;
  remotes: { name: string; url: string }[];
  identity: { name: string | null; email: string | null; local: boolean };
  onChanged: () => void;
}) {
  const notify = useAppStore((s) => s.notify);
  const onError = (e: unknown) => notify("error", String(e));
  const [adding, setAdding] = useState({ name: remotes.length ? "" : "origin", url: "" });
  const [editing, setEditing] = useState<{ name: string; url: string } | null>(null);
  const [who, setWho] = useState({ name: identity.local ? (identity.name ?? "") : "", email: identity.local ? (identity.email ?? "") : "" });
  const remote = useMutation({
    mutationFn: (action: Parameters<typeof api.gitRemote>[1]) => api.gitRemote(site.name, action),
    onSuccess: (_, action) => {
      if (action.kind === "add") setAdding({ name: "", url: "" });
      if (action.kind === "setUrl") setEditing(null);
      onChanged();
    },
    onError,
  });
  const saveWho = useMutation({
    mutationFn: () => api.gitSetIdentity(site.name, who.name, who.email),
    onSuccess: () => {
      notify("info", who.name || who.email ? "This repository now commits as you set" : "This repository uses your global Git identity again");
      onChanged();
    },
    onError,
  });

  return (
    <div className="grid gap-6 lg:grid-cols-2">
      <section className="space-y-3">
        <h4 className="text-sm font-semibold text-fg">Remotes</h4>
        {remotes.length === 0 && <p className="text-sm text-muted">No remote yet. Add one to push this project to GitHub, GitLab or a server.</p>}
        <ul className="space-y-2">
          {remotes.map((r) => (
            <li key={r.name} className="rounded-lg border border-line p-3">
              {editing?.name === r.name ? (
                <form
                  className="flex gap-2"
                  onSubmit={(e) => {
                    e.preventDefault();
                    remote.mutate({ kind: "setUrl", name: r.name, url: editing.url });
                  }}
                >
                  <input autoFocus value={editing.url} onChange={(e) => setEditing({ ...editing, url: e.target.value })} className={clsx(inputClass, "font-mono")} aria-label={`${r.name} URL`} />
                  <Button size="sm" type="submit" disabled={!editing.url.trim() || remote.isPending}>
                    Save
                  </Button>
                  <Button size="sm" variant="ghost" type="button" onClick={() => setEditing(null)} aria-label="Cancel">
                    <X size={13} />
                  </Button>
                </form>
              ) : (
                <div className="flex items-center gap-2">
                  <span className="min-w-0 flex-1">
                    <span className="block text-sm font-semibold text-fg">{r.name}</span>
                    <span className="block truncate font-mono text-xs text-muted" title={r.url}>
                      {r.url}
                    </span>
                  </span>
                  <Button size="sm" variant="ghost" onClick={() => setEditing({ name: r.name, url: r.url })} aria-label={`Change ${r.name}'s URL`} title="Change URL">
                    <Pencil size={13} />
                  </Button>
                  <Button size="sm" variant="ghost" onClick={() => remote.mutate({ kind: "remove", name: r.name })} aria-label={`Remove ${r.name}`} title="Remove (the remote repository itself isn't touched)">
                    <Trash2 size={13} />
                  </Button>
                </div>
              )}
            </li>
          ))}
        </ul>
        <form
          className="flex flex-wrap gap-2"
          onSubmit={(e) => {
            e.preventDefault();
            remote.mutate({ kind: "add", name: adding.name.trim(), url: adding.url.trim() });
          }}
        >
          <input value={adding.name} onChange={(e) => setAdding({ ...adding, name: e.target.value })} placeholder="origin" aria-label="Remote name" className={clsx(inputClass.replace("w-full", ""), "w-28 font-mono")} />
          <input value={adding.url} onChange={(e) => setAdding({ ...adding, url: e.target.value })} placeholder="git@github.com:you/repo.git" aria-label="Remote URL" className={clsx(inputClass.replace("w-full", ""), "min-w-48 flex-1 font-mono")} />
          <Button type="submit" disabled={!adding.name.trim() || !adding.url.trim() || remote.isPending}>
            <Plus size={14} /> Add
          </Button>
        </form>
        <p className="text-[11px] text-muted">Pushing uses your Git's credential helper or SSH key; LampRabit never stores passwords or tokens.</p>
      </section>

      <section className="space-y-3">
        <h4 className="flex items-center gap-2 text-sm font-semibold text-fg">
          <UserRound size={15} /> Identity for this repository
        </h4>
        <p className="text-xs text-muted">
          {identity.local ? "This repository commits as the name and email below." : identity.name || identity.email ? `It commits as ${identity.name ?? "?"} <${identity.email ?? "?"}> from your global Git settings. Set a different one for this repository only:` : "Git has no name or email yet, so commits will fail. Set them for this repository:"}
        </p>
        <form
          className="space-y-2"
          onSubmit={(e) => {
            e.preventDefault();
            saveWho.mutate();
          }}
        >
          <input value={who.name} onChange={(e) => setWho({ ...who, name: e.target.value })} placeholder={identity.name ?? "Your Name"} aria-label="Name" className={inputClass} />
          <input value={who.email} onChange={(e) => setWho({ ...who, email: e.target.value })} placeholder={identity.email ?? "you@example.com"} type="email" aria-label="Email" className={inputClass} />
          <div className="flex gap-2">
            <Button type="submit" variant="primary" disabled={saveWho.isPending}>
              Save
            </Button>
            {identity.local && (
              <Button
                type="button"
                variant="ghost"
                onClick={() => {
                  setWho({ name: "", email: "" });
                  api.gitSetIdentity(site.name, "", "").then(onChanged, onError);
                }}
              >
                <RotateCcw size={13} /> Use the global identity
              </Button>
            )}
          </div>
        </form>
        <p className="text-[11px] text-muted">Only this repository's config changes (git config --local); your global Git settings are left alone.</p>
      </section>
    </div>
  );
}

/** Clones a repository into www/; the folder then shows as a project like any other. */
export function CloneDialog({ onClose, onCloned }: { onClose: () => void; onCloned: (folder: string) => void }) {
  const notify = useAppStore((s) => s.notify);
  const [url, setUrl] = useState("");
  const [folder, setFolder] = useState("");
  const guess = url.trim().replace(/\/+$/, "").split(/[/:]/).pop()?.replace(/\.git$/, "").toLowerCase().replace(/[^a-z0-9_-]+/g, "-").replace(/^-+|-+$/g, "") ?? "";
  const clone = useMutation({
    mutationFn: () => api.gitClone(url.trim(), folder.trim() || undefined),
    onSuccess: (name) => {
      notify("info", `Cloned into www/${name}`);
      onCloned(name);
      onClose();
    },
    onError: (e) => notify("error", String(e)),
  });
  return (
    <Dialog
      open
      onClose={onClose}
      title="Clone a repository"
      icon={<GitBranch size={18} />}
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" onClick={() => clone.mutate()} disabled={!url.trim() || clone.isPending}>
            {clone.isPending ? "Cloning…" : "Clone"}
          </Button>
        </>
      }
    >
      <form
        className="space-y-3"
        onSubmit={(e) => {
          e.preventDefault();
          if (url.trim()) clone.mutate();
        }}
      >
        <label className="block text-sm">
          <span className="font-medium text-fg">Repository URL</span>
          <input data-autofocus value={url} onChange={(e) => setUrl(e.target.value)} placeholder="https://github.com/you/shop.git or git@github.com:you/shop.git" className={clsx(inputClass, "mt-1 font-mono")} />
        </label>
        <label className="block text-sm">
          <span className="font-medium text-fg">Folder in www</span>
          <input value={folder} onChange={(e) => setFolder(e.target.value)} placeholder={guess || "shop"} className={clsx(inputClass, "mt-1 font-mono")} />
          <span className="mt-1 block text-xs text-muted">The project is served as https://{folder.trim() || guess || "shop"}.test. Private repositories use your Git credentials or SSH key.</span>
        </label>
        <button type="submit" hidden />
      </form>
    </Dialog>
  );
}
