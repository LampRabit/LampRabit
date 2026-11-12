import { useEffect, useMemo, useState, type ReactNode } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ArrowDown, ArrowLeftRight, ArrowUp, Check, ChevronLeft, Copy, GitBranch, GitCommitHorizontal, GitMerge, KeyRound, Link2, Minus, Pencil, Plus, RefreshCw, RotateCcw, Trash2, Undo2, UserRound, X } from "lucide-react";
import clsx from "clsx";
import { api, type GitBranch as Branch, type GitFile, type GitRepoSettings, type GitStatus, type Site } from "../lib/api";
import { convertUrl, isSsh, looksSensitive, nextStep, parseDiff, progressPercent, useGitProgress } from "../lib/git";
import { usePageActive } from "../lib/pageActive";
import { useAppStore } from "../lib/store";
import { formatAgo } from "../lib/time";
import Dialog from "./Dialog";
import { Button, inputClass } from "./ui";

/**
 * A project's Git: changes with diffs, staging and commits, history, branches, remotes and
 * the repository's own settings. Everything goes through the git command, so the user's
 * credential helpers and SSH keys work; fetch, pull and push never prompt on a terminal.
 *
 * The view leads with what the repository needs next (commit, push, publish, pull, add a
 * remote, finish a merge): that action is the one highlighted, the others stay quiet.
 */

type View = "changes" | "history" | "branches" | "remotes";
type SyncAction = "fetch" | "pull" | "push";

export default function ProjectGit({ site }: { site: Site }) {
  const queryClient = useQueryClient();
  const notify = useAppStore((s) => s.notify);
  const onError = (e: unknown) => notify("error", String(e));
  const navigate = useAppStore((s) => s.navigate);
  const version = useQuery({ queryKey: ["git-version"], queryFn: api.gitVersion, staleTime: Infinity });
  const active = usePageActive();
  const status = useQuery({ queryKey: ["git-status", site.name], queryFn: () => api.gitStatus(site.name), refetchInterval: active ? 5000 : false, enabled: !!version.data });
  const [view, setView] = useState<View>("changes");
  // Whatever changed the repository refreshes everything that shows it.
  const refresh = () => {
    for (const key of ["git-status", "git-branches", "git-log", "git-commit", "project-facts", "git-repo-settings", "git-stashes", "git-tags"]) queryClient.invalidateQueries({ queryKey: [key, site.name] });
    queryClient.invalidateQueries({ queryKey: ["sites"] });
  };
  const init = useMutation({ mutationFn: () => api.gitInit(site.name), onSuccess: refresh, onError });
  // Each run gets its own id, so its progress and Cancel don't mix with another's.
  const [op, setOp] = useState<string | null>(null);
  const progress = useGitProgress(op);
  const sync = useMutation({
    mutationFn: (action: SyncAction) => {
      const id = `${site.name}:${action}:${Date.now()}`;
      setOp(id);
      return api.gitSync(site.name, action, id);
    },
    onSettled: () => setOp(null),
    onSuccess: (out, action) => {
      notify("info", action === "fetch" ? "Fetched" : action === "pull" ? "Pulled" : "Pushed", out.trim() ? out.trim().split("\n").slice(-6) : undefined);
      refresh();
    },
    onError: (e) => {
      if (String(e) !== "Cancelled.") onError(e);
      refresh();
    },
  });
  const abort = useMutation({ mutationFn: () => api.gitAbort(site.name), onSuccess: refresh, onError });
  const carryOn = useMutation({ mutationFn: () => api.gitContinue(site.name), onSuccess: refresh, onError });

  if (version.isLoading) return <p className="text-sm text-muted">Looking for Git…</p>;
  if (!version.data) {
    return (
      <div className="text-sm">
        <p className="font-semibold text-fg">Git isn't installed</p>
        <p className="mt-1 text-muted">
          LampRabit uses the Git on your computer, with your credentials and SSH keys. On Windows it can install one for you (MinGit); on Linux use your package manager (sudo apt install git), on
          macOS run xcode-select --install.
        </p>
        <Button className="mt-3" variant="primary" onClick={() => navigate("services")}>
          <GitBranch size={14} /> Install Git from Services
        </Button>
      </div>
    );
  }
  const s = status.data;
  if (!s) return <p className="text-sm text-muted">{status.isError ? String(status.error) : "Reading the repository…"}</p>;
  if (!s.repo) {
    return (
      <div className="flex flex-wrap items-center justify-between gap-3">
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

  const step = nextStep(s);
  const conflicts = s.files.filter((f) => f.conflicted).length;
  const views: [View, string, ReactNode?][] = [
    ["changes", "Changes", s.files.length ? <Count n={s.files.length} on={view === "changes"} /> : null],
    ["history", "History"],
    ["branches", "Branches"],
    ["remotes", "Remotes", step === "remote" ? <span className="ml-1.5 inline-block size-1.5 rounded-full bg-accent align-middle" aria-label="Needs a remote" /> : null],
  ];
  // Sync buttons: the one the repository needs is filled; the others are quiet; ones that
  // can't work yet aren't shown at all.
  const syncButton = (action: SyncAction, icon: ReactNode, label: string, title: string, primary: boolean) => (
    <button
      type="button"
      onClick={() => sync.mutate(action)}
      disabled={sync.isPending}
      title={title}
      className={clsx(
        "flex items-center gap-1 rounded-md px-2 py-1 text-xs font-medium disabled:opacity-50",
        primary ? "bg-accent text-accent-fg hover:bg-accent-hover" : "text-muted hover:bg-surface-2 hover:text-fg",
      )}
    >
      {sync.isPending && sync.variables === action ? <RefreshCw size={12} className="animate-spin" /> : icon} {label}
    </button>
  );

  return (
    <div className="space-y-3">
      <div className="flex flex-wrap items-center gap-1">
        <div role="tablist" aria-label="Git" className="flex flex-wrap gap-0.5">
          {views.map(([id, label, extra]) => (
            <button key={id} role="tab" aria-selected={view === id} onClick={() => setView(id)} className={clsx("rounded-md px-2.5 py-1 text-sm", view === id ? "bg-surface font-semibold text-fg shadow-card" : "text-muted hover:text-fg")}>
              {label}
              {extra}
            </button>
          ))}
        </div>
        <span className="flex-1" />
        {s.remotes.length === 0 ? (
          s.head && (
            <button type="button" onClick={() => setView("remotes")} className="flex items-center gap-1 rounded-md bg-accent px-2 py-1 text-xs font-medium text-accent-fg hover:bg-accent-hover" title="Add a remote (GitHub, GitLab…) to push this repository">
              <Link2 size={12} /> Add a remote
            </button>
          )
        ) : (
          <>
            {s.upstream && (
              <span className="mr-1 text-xs text-muted" title={`Tracks ${s.upstream}`}>
                {s.ahead === 0 && s.behind === 0 ? "Up to date" : [s.ahead && `${s.ahead} to push`, s.behind && `${s.behind} to pull`].filter(Boolean).join(" · ")}
              </span>
            )}
            {syncButton("fetch", <RefreshCw size={12} />, "Fetch", "Get the remotes' new commits and branches without changing your files", false)}
            {s.upstream && syncButton("pull", <ArrowDown size={12} />, s.behind ? `Pull ${s.behind}` : "Pull", `Bring ${s.upstream}'s commits into ${s.branch}`, step === "pull")}
            {s.branch && s.head && (s.upstream ? syncButton("push", <ArrowUp size={12} />, s.ahead ? `Push ${s.ahead}` : "Push", `Send your commits to ${s.upstream}`, step === "push") : syncButton("push", <ArrowUp size={12} />, "Publish branch", `Push ${s.branch} and track it on the remote`, step === "publish"))}
          </>
        )}
      </div>

      {sync.isPending && op && <ProgressBar line={progress} label={`${sync.variables === "fetch" ? "Fetching" : sync.variables === "pull" ? "Pulling" : "Pushing"}…`} onCancel={() => void api.gitCancel(op)} />}

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

      {view === "changes" && <Changes site={site} status={s} onChanged={refresh} onIdentity={() => setView("remotes")} onRemotes={() => setView("remotes")} onPush={() => sync.mutate("push")} pushing={sync.isPending && sync.variables === "push"} />}
      {view === "history" && <History site={site} />}
      {view === "branches" && <Branches site={site} current={s.branch} upstream={s.upstream} onChanged={refresh} />}
      {view === "remotes" && <RemotesAndSettings site={site} status={s} onChanged={refresh} />}
    </div>
  );
}

/** Git's progress for a network operation, with a bar when it gives a percentage, and Cancel. */
function ProgressBar({ line, label, onCancel }: { line: string | null; label: string; onCancel: () => void }) {
  const percent = progressPercent(line);
  return (
    <div className="flex items-center gap-3 rounded-lg border border-line bg-surface px-3 py-2 text-xs" role="status" aria-live="polite">
      <RefreshCw size={13} className="shrink-0 animate-spin text-accent-text" aria-hidden />
      <span className="min-w-0 flex-1">
        <span className="block truncate text-fg">{line ?? label}</span>
        {percent !== null && (
          <span className="mt-1 block h-1 overflow-hidden rounded-full bg-surface-2">
            <span className="block h-full bg-accent transition-[width]" style={{ width: `${percent}%` }} />
          </span>
        )}
      </span>
      <Button size="sm" variant="ghost" onClick={onCancel}>
        <X size={13} /> Cancel
      </Button>
    </div>
  );
}

function Count({ n, on }: { n: number; on: boolean }) {
  return <span className={clsx("ml-1.5 rounded-full px-1.5 text-[11px] tabular-nums", on ? "bg-accent-soft text-accent-text" : "bg-surface-2")}>{n}</span>;
}

const letters: Record<string, string> = { M: "Modified", A: "Added", D: "Deleted", R: "Renamed", C: "Copied", T: "Type changed", "!": "Conflict", "?": "New, not tracked yet" };
const letterColor = (code: string) => (code === "D" ? "text-danger-text" : code === "?" || code === "A" ? "text-ok" : code === "!" ? "text-warn" : "text-accent-text");

/** The two panes (list, diff) share one height, so the diff gets the room and the page doesn't grow. */
const panes = "grid gap-3 lg:h-[min(66vh,42rem)] lg:grid-cols-[minmax(16rem,22rem)_minmax(0,1fr)]";

/** A file's name first, its folder after in grey: the name is what people scan for. */
function FileName({ path }: { path: string }) {
  const slash = path.lastIndexOf("/");
  return (
    <>
      <span className="truncate text-[13px] text-fg">{path.slice(slash + 1)}</span>
      {slash > 0 && <span className="min-w-0 truncate text-[11px] text-muted">{path.slice(0, slash)}</span>}
    </>
  );
}

function Changes({
  site,
  status,
  onChanged,
  onIdentity,
  onRemotes,
  onPush,
  pushing,
}: {
  site: Site;
  status: GitStatus;
  onChanged: () => void;
  onIdentity: () => void;
  onRemotes: () => void;
  onPush: () => void;
  pushing: boolean;
}) {
  const notify = useAppStore((s) => s.notify);
  const onError = (e: unknown) => notify("error", String(e));
  const files = status.files;
  const firstCommit = !status.head;
  const identityMissing = !status.identity.name || !status.identity.email;
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
  // Nothing staged but changes there: the commit takes all of them (git add --all first),
  // like VS Code's "commit all" when nothing is staged.
  const commitAll = staged.length === 0 && unstaged.length > 0 && !amend;
  const commit = useMutation({
    mutationFn: () => api.gitCommit(site.name, message, amend, commitAll),
    onSuccess: () => {
      notify("info", amend ? "Amended the last commit" : "Committed");
      setMessage("");
      setAmend(false);
      onChanged();
    },
    onError,
  });
  const stashes = useQuery({ queryKey: ["git-stashes", site.name], queryFn: () => api.gitStashes(site.name) });
  const stash = useMutation({
    mutationFn: (action: Parameters<typeof api.gitStash>[1]) => api.gitStash(site.name, action),
    onSuccess: (_, action) => {
      if (action.kind === "save") notify("info", "Changes stashed: the working tree is clean. Bring them back from Stashes.");
      stashes.refetch();
      onChanged();
    },
    onError: (e) => {
      onError(e);
      stashes.refetch();
      onChanged();
    },
  });
  const stashList = (stashes.data ?? []).length > 0 && (
    <section>
      <div className="sticky top-0 z-[1] border-b border-line bg-surface px-3 py-1.5 text-[11px] font-semibold tracking-wide text-muted uppercase">
        Stashes <span className="tabular-nums">{stashes.data!.length}</span>
      </div>
      <ul>
        {stashes.data!.map((st) => (
          <li key={st.index} className="group flex items-center gap-2 px-3 py-1 hover:bg-surface-2">
            <span className="min-w-0 flex-1">
              <span className="block truncate text-[13px] text-fg">{st.message.replace(/^On [^:]+: /, "")}</span>
              <span className="block text-[11px] text-muted">{formatAgo(st.time * 1000)}</span>
            </span>
            <button type="button" onClick={() => stash.mutate({ kind: "pop", index: st.index })} disabled={stash.isPending} className="text-[11px] font-medium text-accent-text hover:underline" title="Bring the changes back and remove the stash">
              Restore
            </button>
            <button type="button" onClick={() => stash.mutate({ kind: "apply", index: st.index })} disabled={stash.isPending} className="text-[11px] font-medium text-muted opacity-0 group-hover:opacity-100 hover:underline focus-visible:opacity-100" title="Bring the changes back and keep the stash">
              Apply
            </button>
            <button type="button" onClick={() => stash.mutate({ kind: "drop", index: st.index })} disabled={stash.isPending} className="rounded p-0.5 text-muted opacity-0 group-hover:opacity-100 hover:text-danger-text focus-visible:opacity-100" aria-label="Delete this stash" title="Delete this stash">
              <Trash2 size={12} />
            </button>
          </li>
        ))}
      </ul>
    </section>
  );
  const hasWork = staged.length > 0 || unstaged.length > 0;
  const newFiles = unstaged.filter((f) => f.staged === "?").length;
  const sensitive = (commitAll ? unstaged : staged).filter((f) => looksSensitive(f.path)).map((f) => f.path);
  const canCommit = conflicted.length === 0 && (amend || (hasWork && !!message.trim()));
  const commitLabel = amend ? "Amend" : commitAll ? `Commit all ${unstaged.length}` : `Commit ${staged.length}`;

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
            const code = kind === "conflict" ? "!" : isStaged ? f.staged : f.staged === "?" ? "?" : f.unstaged;
            const active = selected?.path === f.path && selected.staged === isStaged;
            return (
              <li key={`${kind}:${f.path}`} className={clsx("group flex items-center gap-1 pr-1.5 pl-3", active ? "bg-accent-soft/50" : "hover:bg-surface-2")}>
                <button type="button" onClick={() => setSelected({ path: f.path, staged: isStaged })} className="flex min-w-0 flex-1 items-baseline gap-2 py-1 text-left" title={f.from ? `${f.from} → ${f.path}` : f.path}>
                  <span className={clsx("w-3 shrink-0 font-mono text-[11px] font-bold", letterColor(code))} title={letters[code] ?? code} aria-label={letters[code] ?? code}>
                    {code === "?" ? "U" : code}
                  </span>
                  <FileName path={f.path} />
                </button>
                {kind !== "staged" && (
                  <button type="button" onClick={() => setDiscarding([f.path])} className="rounded p-1 text-muted opacity-0 group-hover:opacity-100 hover:text-danger-text focus-visible:opacity-100" title="Discard these changes" aria-label={`Discard changes to ${f.path}`}>
                    <Undo2 size={13} />
                  </button>
                )}
                <button
                  type="button"
                  onClick={() => stage.mutate({ paths: [f.path], on: !isStaged })}
                  disabled={stage.isPending}
                  className="rounded p-1 text-muted opacity-0 group-hover:opacity-100 hover:text-fg focus-visible:opacity-100"
                  title={isStaged ? "Unstage: leave it out of the commit" : kind === "conflict" ? "Mark as resolved" : "Stage: commit only staged files"}
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
  // Under the message: commit when there's something to commit, else what comes next.
  const footer =
    hasWork || amend ? (
      <Button size="sm" variant={canCommit ? "primary" : "secondary"} onClick={() => commit.mutate()} disabled={!canCommit || commit.isPending} title={commitAll ? "Stages every change, then commits (Ctrl+Enter)" : "Commit the staged files (Ctrl+Enter)"}>
        <GitCommitHorizontal size={13} /> {commit.isPending ? "Committing…" : commitLabel}
      </Button>
    ) : status.remotes.length === 0 ? (
      status.head && (
        <Button size="sm" variant="primary" onClick={onRemotes}>
          <Link2 size={13} /> Add a remote to push
        </Button>
      )
    ) : status.branch && status.head && (!status.upstream || status.ahead > 0) ? (
      <Button size="sm" variant="primary" onClick={onPush} disabled={pushing}>
        <ArrowUp size={13} /> {pushing ? "Pushing…" : !status.upstream ? "Publish branch" : `Push ${status.ahead} ${status.ahead === 1 ? "commit" : "commits"}`}
      </Button>
    ) : null;

  if (files.length === 0 && !amend) {
    return (
      <div className="flex flex-wrap items-center gap-3 rounded-lg border border-line bg-surface px-4 py-3">
        <Check size={16} className="shrink-0 text-ok" aria-hidden />
        <p className="min-w-0 flex-1 text-sm text-fg">
          {status.remotes.length === 0
            ? "Everything is committed. Add a remote to back it up."
            : status.ahead > 0 || (!status.upstream && status.head)
              ? "Everything is committed; your commits aren't on the remote yet."
              : "Everything is committed and pushed."}
        </p>
        {!firstCommit && (
          <label className="flex items-center gap-1.5 text-xs text-muted" title="Change the last commit's message">
            <input type="checkbox" checked={amend} onChange={(e) => setAmend(e.target.checked)} className="size-3.5 accent-[var(--accent)]" /> Amend last commit
          </label>
        )}
        {footer}
        {stashList && <div className="basis-full overflow-hidden rounded-lg border border-line">{stashList}</div>}
      </div>
    );
  }

  return (
    <div className={panes}>
      <div className="flex min-h-64 flex-col overflow-hidden rounded-lg border border-line bg-surface">
        <div className="space-y-2 border-b border-line p-2.5">
          {identityMissing && (
            <p className="text-[11px] text-warn">
              Git doesn't know your name and email yet.{" "}
              <button type="button" onClick={onIdentity} className="font-medium underline">
                Set them
              </button>
            </p>
          )}
          {firstCommit && staged.length > 0 && <p className="text-[11px] text-muted">No commits yet: the project's files are staged for the first one. Unstage any you want to leave out.</p>}
          {commitAll && newFiles > 0 && sensitive.length === 0 && <p className="text-[11px] text-muted">Commit all includes {newFiles} new {newFiles === 1 ? "file" : "files"}. Stage files with + to commit only those.</p>}
          {sensitive.length > 0 && (
            <p className="text-[11px] text-warn" role="alert">
              {sensitive.slice(0, 2).join(", ")}
              {sensitive.length > 2 && ` and ${sensitive.length - 2} more`} {sensitive.length === 1 ? "looks" : "look"} like secrets or local settings. Add {sensitive.length === 1 ? "it" : "them"} to .gitignore, or stage only the files you mean to commit.
            </p>
          )}
          {hasWork || amend ? (
            <textarea
              value={message}
              onChange={(e) => setMessage(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter" && (e.ctrlKey || e.metaKey) && canCommit) commit.mutate();
              }}
              rows={2}
              placeholder={amend ? "New message (empty keeps the last one)" : firstCommit ? "Initial commit" : `Message (Ctrl+Enter to commit${commitAll ? " all" : ""})`}
              aria-label="Commit message"
              className={clsx(inputClass, "resize-none py-1.5 font-sans")}
            />
          ) : (
            <p className="py-1 text-xs text-muted">
              {status.remotes.length === 0
                ? "Everything is committed. Add a remote to back it up."
                : status.ahead > 0 || (!status.upstream && status.head)
                  ? "Everything is committed; your commits aren't on the remote yet."
                  : "Everything is committed and pushed."}
            </p>
          )}
          <div className="flex items-center gap-2">
            {!firstCommit && hasWork && (
              <label className="flex items-center gap-1.5 text-[11px] text-muted" title="Add the changes to the last commit instead of making a new one">
                <input type="checkbox" checked={amend} onChange={(e) => setAmend(e.target.checked)} className="size-3.5 accent-[var(--accent)]" /> Amend
              </label>
            )}
            <span className="flex-1" />
            {footer}
          </div>
        </div>
        <div className="min-h-0 flex-1 overflow-y-auto">
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
            "Changes",
            unstaged,
            "unstaged",
            <span className="flex gap-3">
              <button type="button" onClick={() => stash.mutate({ kind: "save", message: message.trim(), untracked: true })} disabled={stash.isPending} className={clsx(textButton, "text-muted")} title="Set all changes aside (new files too) and get a clean working tree; bring them back from Stashes">
                Stash
              </button>
              <button type="button" onClick={() => setDiscarding(unstaged.map((f) => f.path))} className={clsx(textButton, "text-danger-text")}>
                Discard all
              </button>
              <button type="button" onClick={() => stage.mutate({ paths: unstaged.map((f) => f.path), on: true })} className={clsx(textButton, "text-accent-text")}>
                Stage all
              </button>
            </span>,
          )}
          {stashList}
        </div>
      </div>
      <div className="flex min-h-64 min-w-0 flex-col overflow-hidden rounded-lg border border-line bg-surface">
        {selected && current ? (
          <FileDiff site={site} file={current} staged={selected.staged} onChanged={onChanged} />
        ) : (
          <div className="flex flex-1 items-center justify-center p-4 text-sm text-muted">{files.length ? "Select a file to see its changes." : "No changes."}</div>
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

function FileDiff({ site, file, staged, onChanged }: { site: Site; file: GitFile; staged: boolean; onChanged: () => void }) {
  const notify = useAppStore((s) => s.notify);
  const onError = (e: unknown) => notify("error", String(e));
  const untracked = file.staged === "?";
  const diff = useQuery({
    queryKey: ["git-diff", site.name, file.path, staged, file.staged, file.unstaged],
    queryFn: () => api.gitDiff(site.name, file.path, staged, untracked),
    enabled: !file.conflicted,
  });
  // Stage or unstage one part of the file (git add -p for one hunk).
  const hunk = useMutation({
    mutationFn: (index: number) => api.gitStageHunk(site.name, file.path, index, staged),
    onSuccess: () => {
      diff.refetch();
      onChanged();
    },
    onError,
  });
  const resolve = useMutation({ mutationFn: (theirs: boolean) => api.gitResolve(site.name, file.path, theirs), onSuccess: onChanged, onError });
  if (file.conflicted) {
    return (
      <>
        <DiffTitle path={file.path} note="Conflict" />
        <div className="space-y-3 p-4 text-sm">
          <p className="text-fg">Both sides changed this file. Keep one side's version, or merge them by hand in your editor (look for the &lt;&lt;&lt;&lt;&lt;&lt;&lt; markers), then mark it resolved.</p>
          <div className="flex flex-wrap gap-2">
            <Button size="sm" onClick={() => resolve.mutate(false)} disabled={resolve.isPending} title="git checkout --ours, then stage">
              Keep my version
            </Button>
            <Button size="sm" onClick={() => resolve.mutate(true)} disabled={resolve.isPending} title="git checkout --theirs, then stage">
              Take the incoming version
            </Button>
            <Button size="sm" variant="ghost" onClick={() => api.openInEditor(`${site.path}/${file.path}`).catch(onError)}>
              Open in editor
            </Button>
          </div>
          <p className="text-xs text-muted">Merged it by hand? Use the ✓ next to the file to mark it resolved.</p>
        </div>
      </>
    );
  }
  return (
    <>
      <DiffTitle path={file.path} note={staged ? "Staged" : untracked ? "New file" : "Changes"} from={file.from} />
      <DiffView
        text={diff.data}
        loading={diff.isLoading}
        error={diff.isError ? String(diff.error) : null}
        hunkAction={untracked ? undefined : { label: staged ? "Unstage this part" : "Stage this part", run: (i) => hunk.mutate(i), busy: hunk.isPending }}
      />
    </>
  );
}

function DiffTitle({ path, note, from, extra }: { path: string; note?: string; from?: string | null; extra?: ReactNode }) {
  return (
    <div className="flex shrink-0 items-center gap-2 border-b border-line px-3 py-1.5 text-xs">
      <span className="flex min-w-0 items-baseline gap-2">
        <FileName path={path} />
      </span>
      {from && <span className="truncate text-muted">from {from}</span>}
      <span className="flex-1" />
      {extra}
      {note && <span className="shrink-0 rounded bg-surface-2 px-1.5 py-0.5 text-[11px] text-muted">{note}</span>}
    </div>
  );
}

/** Line numbers in a gutter, added and removed lines tinted, no +/− marks in the text. */
function DiffView({ text, loading, error, hunkAction }: { text?: string; loading?: boolean; error?: string | null; hunkAction?: { label: string; run: (hunk: number) => void; busy?: boolean } }) {
  const rows = useMemo(() => parseDiff(text ?? ""), [text]);
  let hunkIndex = -1;
  if (loading) return <p className="p-3 text-xs text-muted">Loading…</p>;
  if (error) return <p className="p-3 text-xs text-danger-text">{error}</p>;
  if (rows.length === 0) return <p className="p-3 text-xs text-muted">No text changes (a binary file, or only its mode changed).</p>;
  const gutter = "w-10 shrink-0 select-none pr-2 text-right text-muted/70 tabular-nums";
  return (
    <div className="selectable min-h-0 flex-1 overflow-auto font-mono text-[12px] leading-5" aria-label="Changes">
      <div className="min-w-max">
        {rows.map((row, i) =>
          row.kind === "hunk" ? (
            ((index) => (
              <div key={i} className="group/hunk flex items-center border-y border-line/60 bg-surface-2/70 py-0.5 text-[11px] text-muted">
                <span className={gutter}>⋯</span>
                <span className={gutter} />
                <span className="flex-1 px-3">{row.text}</span>
                {hunkAction && (
                  // Sticky on the right, so it stays in reach when long lines scroll sideways.
                  <button
                    type="button"
                    onClick={() => hunkAction.run(index)}
                    disabled={hunkAction.busy}
                    className="sticky right-2 mr-2 rounded border border-line bg-surface px-1.5 font-sans text-[11px] text-fg opacity-70 group-hover/hunk:opacity-100 hover:border-accent focus-visible:opacity-100"
                  >
                    {hunkAction.label}
                  </button>
                )}
              </div>
            ))(++hunkIndex)
          ) : row.kind === "note" ? (
            <div key={i} className="px-3 py-0.5 text-[11px] text-muted italic">
              {row.text}
            </div>
          ) : (
            <div key={i} className={clsx("flex", row.kind === "add" && "bg-ok/12", row.kind === "del" && "bg-danger/12")}>
              <span className={clsx(gutter, row.kind === "del" && "bg-danger/10")}>{row.old ?? ""}</span>
              <span className={clsx(gutter, row.kind === "add" && "bg-ok/10")}>{row.new ?? ""}</span>
              <span className="sr-only">{row.kind === "add" ? "added" : row.kind === "del" ? "removed" : ""}</span>
              <span className="pr-4 pl-3 whitespace-pre text-fg">{row.text || " "}</span>
            </div>
          ),
        )}
      </div>
    </div>
  );
}

function History({ site }: { site: Site }) {
  const [pages, setPages] = useState(1);
  const log = useQuery({ queryKey: ["git-log", site.name, pages], queryFn: () => api.gitLog(site.name, 0, pages * 50) });
  const commits = log.data ?? [];
  const [open, setOpen] = useState<string | null>(null);
  // The newest commit opens by default, so the right side never starts empty.
  const shown = open ?? commits[0]?.hash ?? null;
  return (
    <div className={panes}>
      <div className="min-h-64 overflow-y-auto rounded-lg border border-line bg-surface">
        {log.isLoading && <p className="p-4 text-sm text-muted">Loading history…</p>}
        {!log.isLoading && commits.length === 0 && <p className="p-6 text-center text-sm text-muted">No commits yet.</p>}
        <ol>
          {commits.map((c, i) => (
            <li key={c.hash} className="relative">
              {/* A thin line through the dots reads as a timeline. */}
              {i < commits.length - 1 && <span className="absolute top-5 bottom-0 left-[1.1rem] w-px bg-line" aria-hidden />}
              <button
                type="button"
                onClick={() => setOpen(c.hash)}
                aria-current={c.hash === shown ? "true" : undefined}
                className={clsx("relative flex w-full gap-3 px-3 py-2 text-left", c.hash === shown ? "bg-accent-soft/50" : "hover:bg-surface-2")}
              >
                <span className={clsx("mt-1.5 size-2.5 shrink-0 rounded-full border-2", i === 0 ? "border-accent bg-accent" : "border-muted bg-surface")} aria-hidden />
                <span className="min-w-0 flex-1">
                  <span className="block truncate text-[13px] font-medium text-fg">{c.subject}</span>
                  <span className="mt-0.5 flex items-center gap-1.5 text-[11px] text-muted">
                    <span className="truncate" title={c.email}>
                      {c.author}
                    </span>
                    <span className="shrink-0" title={new Date(c.time * 1000).toLocaleString()}>
                      · {formatAgo(c.time * 1000)}
                    </span>
                    <span className="ml-auto shrink-0 font-mono">{c.short}</span>
                  </span>
                  {c.refs && <Refs refs={c.refs} />}
                </span>
              </button>
            </li>
          ))}
        </ol>
        {commits.length === pages * 50 && (
          <button type="button" className="w-full py-2 text-xs font-medium text-accent-text hover:underline" onClick={() => setPages((p) => p + 1)}>
            Show older commits
          </button>
        )}
      </div>
      <div className="flex min-h-64 min-w-0 flex-col overflow-hidden rounded-lg border border-line bg-surface">
        {shown ? <CommitView key={shown} site={site} hash={shown} onOpen={setOpen} /> : <div className="flex flex-1 items-center justify-center p-4 text-sm text-muted">No commits yet.</div>}
      </div>
    </div>
  );
}

/** Branch and tag names on a commit, as small badges. */
function Refs({ refs }: { refs: string }) {
  const names = refs
    .split(",")
    .map((r) => r.trim().replace(/^HEAD -> /, ""))
    .filter((r) => r && r !== "HEAD");
  return (
    <span className="mt-1 flex flex-wrap gap-1">
      {names.map((r) => (
        <span key={r} className={clsx("rounded px-1.5 font-mono text-[10px]", r.startsWith("tag: ") ? "bg-warn/15 text-warn" : r.includes("/") ? "bg-surface-2 text-muted" : "bg-accent-soft text-accent-text")}>
          {r.replace(/^tag: /, "tag ")}
        </span>
      ))}
    </span>
  );
}

/** One commit: who and when, its full message, the files it changed with counts, and a file's diff. */
function CommitView({ site, hash, onOpen }: { site: Site; hash: string; onOpen: (hash: string) => void }) {
  const notify = useAppStore((s) => s.notify);
  const queryClient = useQueryClient();
  const details = useQuery({ queryKey: ["git-commit", site.name, hash], queryFn: () => api.gitCommitDetails(site.name, hash) });
  const [file, setFile] = useState<string | null>(null);
  const [confirm, setConfirm] = useState<"revert" | "restore" | null>(null);
  const changed = () => {
    for (const key of ["git-status", "git-log", "project-facts"]) queryClient.invalidateQueries({ queryKey: [key, site.name] });
  };
  const revert = useMutation({
    mutationFn: () => api.gitRevert(site.name, hash),
    onSuccess: () => {
      notify("info", "Reverted: a new commit undoes this one.");
      setConfirm(null);
      changed();
    },
    onError: (e) => notify("error", String(e)),
  });
  const restore = useMutation({
    mutationFn: (path: string) => api.gitRestoreFile(site.name, hash, path),
    onSuccess: (_, path) => {
      notify("info", `${path} is back to this commit's version; the change is staged under Changes.`);
      setConfirm(null);
      changed();
    },
    onError: (e) => notify("error", String(e)),
  });
  const d = details.data;
  if (details.isLoading) return <p className="p-4 text-sm text-muted">Loading the commit…</p>;
  if (!d) return <p className="p-4 text-sm text-danger-text">{String(details.error)}</p>;
  const copy = (text: string, what: string) =>
    navigator.clipboard.writeText(text).then(
      () => notify("info", `Copied ${what}`),
      () => notify("error", "Couldn't copy"),
    );
  if (file) {
    const f = d.files.find((x) => x.path === file);
    return (
      <>
        <div className="flex shrink-0 items-center gap-2 border-b border-line px-2 py-1.5">
          <button type="button" onClick={() => setFile(null)} className="flex min-w-0 items-center gap-1 rounded px-1.5 py-0.5 text-xs font-medium text-accent-text hover:bg-surface-2">
            <ChevronLeft size={13} className="shrink-0" /> <span className="truncate">{d.subject}</span>
          </button>
          <span className="flex-1" />
          {f?.status !== "D" &&
            (confirm === "restore" ? (
              <>
                <span className="text-xs text-muted">Replace your copy of {file.split("/").pop()}?</span>
                <Button size="sm" variant="primary" onClick={() => restore.mutate(file)} disabled={restore.isPending}>
                  Restore
                </Button>
                <Button size="sm" variant="ghost" onClick={() => setConfirm(null)}>
                  Cancel
                </Button>
              </>
            ) : (
              <Button size="sm" variant="ghost" onClick={() => setConfirm("restore")} title="Put this file back the way it was in this commit">
                <Undo2 size={13} /> Restore this version
              </Button>
            ))}
        </div>
        <DiffTitle path={file} from={f?.from} note={f ? letters[f.status] : undefined} extra={f && <LineCounts add={f.additions} del={f.deletions} binary={f.binary} />} />
        <CommitFileDiff site={site} hash={hash} path={file} />
      </>
    );
  }
  const totals = d.files.reduce((t, f) => ({ add: t.add + f.additions, del: t.del + f.deletions }), { add: 0, del: 0 });
  const sameCommitter = d.committer === d.author && d.committerEmail === d.authorEmail;
  const when = (t: number) => `${new Date(t * 1000).toLocaleString()} (${formatAgo(t * 1000)})`;
  return (
    <div className="min-h-0 flex-1 overflow-y-auto">
      <div className="space-y-3 border-b border-line p-4">
        <div className="flex items-start gap-3">
          <h4 className="min-w-0 flex-1 text-base font-semibold text-fg">{d.subject}</h4>
          {confirm === "revert" ? (
            <span className="flex shrink-0 items-center gap-2">
              <span className="text-xs text-muted">Add a commit that undoes it?</span>
              <Button size="sm" variant="primary" onClick={() => revert.mutate()} disabled={revert.isPending}>
                Revert
              </Button>
              <Button size="sm" variant="ghost" onClick={() => setConfirm(null)}>
                Cancel
              </Button>
            </span>
          ) : (
            <Button size="sm" variant="ghost" onClick={() => setConfirm("revert")} title="Undo this commit's changes with a new commit; history stays as it is">
              <RotateCcw size={13} /> Revert
            </Button>
          )}
        </div>
        <div>
          {d.body && <p className="selectable mt-1.5 text-sm whitespace-pre-wrap text-muted">{d.body}</p>}
          {d.refs && <Refs refs={d.refs} />}
        </div>
        <dl className="grid grid-cols-[6.5rem_minmax(0,1fr)] gap-x-3 gap-y-1 text-xs">
          <dt className="text-muted">Author</dt>
          <dd className="truncate text-fg">
            {d.author} <span className="text-muted">&lt;{d.authorEmail}&gt;</span>
          </dd>
          <dt className="text-muted">Date</dt>
          <dd className="text-fg">{when(d.authorTime)}</dd>
          {!sameCommitter && (
            <>
              <dt className="text-muted">Committed by</dt>
              <dd className="truncate text-fg">
                {d.committer} <span className="text-muted">&lt;{d.committerEmail}&gt;</span>, {formatAgo(d.committerTime * 1000)}
              </dd>
            </>
          )}
          <dt className="text-muted">Commit</dt>
          <dd className="flex min-w-0 items-center gap-1.5 font-mono text-fg">
            <span className="selectable truncate">{d.hash}</span>
            <button type="button" onClick={() => copy(d.hash, "the commit hash")} className="shrink-0 rounded p-0.5 text-muted hover:text-fg" aria-label="Copy the commit hash" title="Copy">
              <Copy size={12} />
            </button>
          </dd>
          <dt className="text-muted">{d.parents.length > 1 ? "Parents" : "Parent"}</dt>
          <dd className="flex flex-wrap gap-2 font-mono">
            {d.parents.length === 0 ? (
              <span className="font-sans text-muted">None: the first commit</span>
            ) : (
              d.parents.map((p) => (
                <button key={p} type="button" onClick={() => onOpen(p)} className="text-accent-text hover:underline" title="Open this commit">
                  {p.slice(0, 8)}
                </button>
              ))
            )}
          </dd>
        </dl>
      </div>
      <div className="flex items-center gap-3 px-4 pt-3 pb-1 text-xs text-muted">
        <span className="font-semibold text-fg">
          {d.files.length} {d.files.length === 1 ? "file" : "files"} changed
        </span>
        <LineCounts add={totals.add} del={totals.del} />
        {d.parents.length > 1 && <span>(compared with the first parent)</span>}
      </div>
      <ul className="pb-2">
        {d.files.map((f) => (
          <li key={f.path}>
            <button type="button" onClick={() => setFile(f.path)} className="flex w-full items-baseline gap-2 px-4 py-1 text-left hover:bg-surface-2" title={f.from ? `${f.from} → ${f.path}: show the changes` : `${f.path}: show the changes`}>
              <span className={clsx("w-3 shrink-0 font-mono text-[11px] font-bold", letterColor(f.status))} title={letters[f.status]}>
                {f.status}
              </span>
              <span className="flex min-w-0 flex-1 items-baseline gap-2">
                <FileName path={f.path} />
              </span>
              <LineCounts add={f.additions} del={f.deletions} binary={f.binary} />
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
}

function CommitFileDiff({ site, hash, path }: { site: Site; hash: string; path: string }) {
  const diff = useQuery({ queryKey: ["git-commit", site.name, hash, path], queryFn: () => api.gitCommitFileDiff(site.name, hash, path) });
  return <DiffView text={diff.data} loading={diff.isLoading} error={diff.isError ? String(diff.error) : null} />;
}

/** "+12 −3" with a five-block bar, as code hosts show a change's size. */
function LineCounts({ add, del, binary }: { add: number; del: number; binary?: boolean }) {
  if (binary) return <span className="shrink-0 text-[11px] text-muted">binary</span>;
  const total = add + del;
  const green = total ? Math.round((add / total) * 5) : 0;
  return (
    <span className="flex shrink-0 items-center gap-1.5 font-mono text-[11px] tabular-nums">
      <span className="text-ok">+{add}</span>
      <span className="text-danger-text">−{del}</span>
      <span className="flex gap-px" aria-hidden>
        {[0, 1, 2, 3, 4].map((i) => (
          <span key={i} className={clsx("size-1.5 rounded-[1px]", !total ? "bg-line" : i < green ? "bg-ok" : "bg-danger")} />
        ))}
      </span>
    </span>
  );
}

function Branches({ site, current, upstream, onChanged }: { site: Site; current: string | null; upstream: string | null; onChanged: () => void }) {
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
      if (action.kind === "setUpstream") notify("info", `${current} now tracks ${action.upstream}`);
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
  const remoteAll = (list.data ?? []).filter((b) => b.remote);
  const remote = remoteAll.filter((b) => !local.some((l) => l.upstream === b.name));

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
            <span className={clsx("flex items-center gap-1.5 truncate font-mono text-sm text-fg", b.current && "font-bold")}>
              {b.current && <Check size={12} className="text-ok" aria-label="Current branch" />}
              {b.name}
              {b.upstream && <span className="truncate font-sans text-[11px] font-normal text-muted">→ {b.upstream}</span>}
              {b.track && <span className="font-sans text-[11px] font-normal text-warn">{b.track.replace(/[[\]]/g, "")}</span>}
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
      <div className="flex flex-wrap items-end gap-3">
        <form
          className="flex min-w-64 flex-1 gap-2"
          onSubmit={(e) => {
            e.preventDefault();
            act.mutate({ kind: "create", branch: name.trim(), from: null, switch: true });
          }}
        >
          <input value={name} onChange={(e) => setName(e.target.value)} placeholder="new-branch-name" aria-label="New branch name" className={clsx(inputClass, "font-mono")} />
          <Button type="submit" variant="primary" disabled={!name.trim() || act.isPending} title={`Create it from ${current ?? "HEAD"} and switch to it`}>
            <Plus size={14} /> Create branch
          </Button>
        </form>
        {current && remoteAll.length > 0 && (
          <label className="flex items-center gap-2 text-xs text-muted">
            {current} tracks
            <select
              value={upstream ?? ""}
              onChange={(e) => e.target.value && act.mutate({ kind: "setUpstream", upstream: e.target.value })}
              className={clsx(inputClass.replace("w-full", ""), "w-48 py-1.5 font-mono text-xs")}
              aria-label={`Remote branch ${current} tracks`}
            >
              {!upstream && <option value="">nothing yet</option>}
              {remoteAll.map((b) => (
                <option key={b.name} value={b.name}>
                  {b.name}
                </option>
              ))}
            </select>
          </label>
        )}
      </div>
      {list.isLoading && <p className="text-sm text-muted">Loading branches…</p>}
      {local.length > 0 && (
        <section>
          <h4 className="px-1 pb-1 text-[11px] font-bold tracking-wider text-muted uppercase">Local</h4>
          <ul className="overflow-hidden rounded-lg border border-line bg-surface">{local.map(row)}</ul>
        </section>
      )}
      {remote.length > 0 && (
        <section>
          <h4 className="px-1 pb-1 text-[11px] font-bold tracking-wider text-muted uppercase">Only on the remote</h4>
          <ul className="overflow-hidden rounded-lg border border-line bg-surface">{remote.map(row)}</ul>
        </section>
      )}
      <Tags site={site} onChanged={onChanged} />
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

/** Tags (release names): create one on the current commit, push it, delete it. */
function Tags({ site, onChanged }: { site: Site; onChanged: () => void }) {
  const notify = useAppStore((s) => s.notify);
  const queryClient = useQueryClient();
  const tags = useQuery({ queryKey: ["git-tags", site.name], queryFn: () => api.gitTags(site.name) });
  const status = queryClient.getQueryData<GitStatus>(["git-status", site.name]);
  const remote = status?.remotes.find((r) => r.name === "origin")?.name ?? status?.remotes[0]?.name;
  const [name, setName] = useState("");
  const [message, setMessage] = useState("");
  const act = useMutation({
    mutationFn: (action: Parameters<typeof api.gitTag>[1]) => api.gitTag(site.name, action),
    onSuccess: (_, action) => {
      if (action.kind === "create") {
        setName("");
        setMessage("");
      }
      if (action.kind === "push") notify("info", `Pushed ${action.name} to ${action.remote}`);
      tags.refetch();
      onChanged();
    },
    onError: (e) => notify("error", String(e)),
  });
  return (
    <section className="space-y-2">
      <h4 className="px-1 text-[11px] font-bold tracking-wider text-muted uppercase">Tags</h4>
      <form
        className="flex flex-wrap gap-2"
        onSubmit={(e) => {
          e.preventDefault();
          act.mutate({ kind: "create", name: name.trim(), message, commit: null });
        }}
      >
        <input value={name} onChange={(e) => setName(e.target.value)} placeholder="v1.0.0" aria-label="Tag name" className={clsx(inputClass.replace("w-full", ""), "w-32 font-mono")} />
        <input value={message} onChange={(e) => setMessage(e.target.value)} placeholder="Release notes (optional)" aria-label="Tag message" className={clsx(inputClass.replace("w-full", ""), "min-w-48 flex-1")} />
        <Button type="submit" disabled={!name.trim() || act.isPending} title="Tag the current commit">
          <Plus size={14} /> Tag
        </Button>
      </form>
      {(tags.data ?? []).length > 0 && (
        <ul className="overflow-hidden rounded-lg border border-line bg-surface">
          {tags.data!.map((t) => (
            <li key={t.name} className="flex items-center gap-2 border-b border-line/60 px-3 py-1.5 last:border-0">
              <span className="font-mono text-sm text-fg">{t.name}</span>
              <span className="min-w-0 flex-1 truncate text-[11px] text-muted">
                {t.subject} · {t.commit} · {formatAgo(t.time * 1000)}
              </span>
              {remote && (
                <Button size="sm" variant="ghost" onClick={() => act.mutate({ kind: "push", name: t.name, remote })} disabled={act.isPending} title={`Push the tag to ${remote}`}>
                  <ArrowUp size={13} /> Push
                </Button>
              )}
              <Button size="sm" variant="ghost" onClick={() => act.mutate({ kind: "delete", name: t.name })} disabled={act.isPending} aria-label={`Delete ${t.name}`} title="Delete the tag here (not on the remote)">
                <Trash2 size={13} />
              </Button>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

/** Hosts offered when adding a remote by owner/repository. */
const hosts = [
  { id: "github.com", label: "GitHub" },
  { id: "gitlab.com", label: "GitLab" },
  { id: "bitbucket.org", label: "Bitbucket" },
];

function RemotesAndSettings({ site, status, onChanged }: { site: Site; status: GitStatus; onChanged: () => void }) {
  const notify = useAppStore((s) => s.notify);
  const onError = (e: unknown) => notify("error", String(e));
  const remotes = status.remotes;
  const identity = status.identity;
  const remote = useMutation({
    mutationFn: (action: Parameters<typeof api.gitRemote>[1]) => api.gitRemote(site.name, action),
    onSuccess: onChanged,
    onError,
  });
  const [who, setWho] = useState({ name: identity.local ? (identity.name ?? "") : "", email: identity.local ? (identity.email ?? "") : "" });
  const saveWho = useMutation({
    mutationFn: (v: { name: string; email: string }) => api.gitSetIdentity(site.name, v.name, v.email),
    onSuccess: (_, v) => {
      notify("info", v.name || v.email ? "This repository now commits as you set" : "This repository uses your global Git identity again");
      onChanged();
    },
    onError,
  });

  return (
    <div className="grid gap-4 xl:grid-cols-2">
      <section className="space-y-3 rounded-lg border border-line bg-surface p-4">
        <h4 className="text-sm font-semibold text-fg">Remotes</h4>
        {remotes.length === 0 && <p className="text-xs text-muted">No remote yet. Add one to push this project to GitHub, GitLab, Bitbucket or your own server.</p>}
        <ul className="space-y-2">
          {remotes.map((r) => (
            <RemoteRow key={r.name} remote={r} busy={remote.isPending} onAction={(a) => remote.mutateAsync(a).catch(() => {})} />
          ))}
        </ul>
        <AddRemote taken={remotes.map((r) => r.name)} busy={remote.isPending} onAdd={(name, url) => remote.mutate({ kind: "add", name, url })} />
        <p className="text-[11px] text-muted">HTTPS uses your Git's credential helper (it may ask you to sign in once); SSH uses your SSH key. LampRabit never stores passwords or tokens.</p>
      </section>

      <div className="space-y-4">
        <RepoSettingsCard site={site} onChanged={onChanged} />
        <section className="space-y-3 rounded-lg border border-line bg-surface p-4">
          <h4 className="flex items-center gap-2 text-sm font-semibold text-fg">
            <UserRound size={15} /> Identity for this repository
          </h4>
          <p className="text-xs text-muted">
            {identity.local
              ? "This repository commits as the name and email below."
              : identity.name || identity.email
                ? `Commits are by ${identity.name ?? "?"} <${identity.email ?? "?"}> from your global Git settings. Set another for this repository only:`
                : "Git has no name or email yet, so commits will fail. Set them for this repository:"}
          </p>
          <form
            className="grid gap-2 sm:grid-cols-2"
            onSubmit={(e) => {
              e.preventDefault();
              saveWho.mutate(who);
            }}
          >
            <input value={who.name} onChange={(e) => setWho({ ...who, name: e.target.value })} placeholder={identity.name ?? "Your Name"} aria-label="Name" className={inputClass} />
            <input value={who.email} onChange={(e) => setWho({ ...who, email: e.target.value })} placeholder={identity.email ?? "you@example.com"} type="email" aria-label="Email" className={inputClass} />
            <div className="flex gap-2 sm:col-span-2">
              <Button type="submit" size="sm" variant="primary" disabled={saveWho.isPending}>
                Save
              </Button>
              {identity.local && (
                <Button
                  type="button"
                  size="sm"
                  variant="ghost"
                  onClick={() => {
                    setWho({ name: "", email: "" });
                    saveWho.mutate({ name: "", email: "" });
                  }}
                >
                  <RotateCcw size={13} /> Use the global identity
                </Button>
              )}
            </div>
          </form>
          <p className="text-[11px] text-muted">Only this repository's config changes; your global Git settings are left alone.</p>
        </section>
      </div>
    </div>
  );
}

function RemoteRow({ remote: r, busy, onAction }: { remote: GitStatus["remotes"][number]; busy: boolean; onAction: (a: Parameters<typeof api.gitRemote>[1]) => Promise<unknown> }) {
  const [editing, setEditing] = useState<{ name: string; url: string; pushUrl: string; separate: boolean } | null>(null);
  const [removing, setRemoving] = useState(false);
  const convert = convertUrl(r.url);
  if (editing) {
    // Rename first, then the URLs under the new name; each step waits for the one before.
    const save = async () => {
      const name = editing.name.trim();
      if (name !== r.name) await onAction({ kind: "rename", from: r.name, to: name });
      if (editing.url.trim() !== r.url) await onAction({ kind: "setUrl", name, url: editing.url.trim() });
      const push = editing.separate ? editing.pushUrl.trim() : "";
      if (push !== (r.pushUrl ?? "")) await onAction({ kind: "setPushUrl", name, url: push || null });
      setEditing(null);
    };
    return (
      <li className="space-y-2 rounded-lg border border-accent/40 p-3">
        <label className="block text-xs text-muted">
          Name
          <input value={editing.name} onChange={(e) => setEditing({ ...editing, name: e.target.value })} className={clsx(inputClass, "mt-1 font-mono")} />
        </label>
        <label className="block text-xs text-muted">
          {editing.separate ? "Fetch URL" : "URL"}
          <input value={editing.url} onChange={(e) => setEditing({ ...editing, url: e.target.value })} className={clsx(inputClass, "mt-1 font-mono")} />
        </label>
        <label className="flex items-center gap-2 text-xs text-muted">
          <input
            type="checkbox"
            checked={editing.separate}
            onChange={(e) => setEditing({ ...editing, separate: e.target.checked, pushUrl: editing.pushUrl || convertUrl(editing.url)?.url || editing.url })}
            className="size-3.5 accent-[var(--accent)]"
          />
          Push to a different URL (for example fetch over HTTPS, push over SSH)
        </label>
        {editing.separate && (
          <label className="block text-xs text-muted">
            Push URL
            <input value={editing.pushUrl} onChange={(e) => setEditing({ ...editing, pushUrl: e.target.value })} className={clsx(inputClass, "mt-1 font-mono")} />
          </label>
        )}
        <div className="flex gap-2">
          <Button size="sm" variant="primary" onClick={() => void save()} disabled={busy || !editing.name.trim() || !editing.url.trim()}>
            Save
          </Button>
          <Button size="sm" variant="ghost" onClick={() => setEditing(null)}>
            Cancel
          </Button>
        </div>
      </li>
    );
  }
  return (
    <li className="rounded-lg border border-line p-3">
      <div className="flex items-start gap-1">
        <span className="min-w-0 flex-1">
          <span className="flex items-center gap-2 text-sm font-semibold text-fg">
            {r.name}
            <span className="rounded bg-surface-2 px-1.5 text-[10px] font-medium text-muted">{isSsh(r.url) ? "SSH" : "HTTPS"}</span>
          </span>
          <span className="block truncate font-mono text-xs text-muted" title={r.url}>
            {r.url}
          </span>
          {r.pushUrl && (
            <span className="block truncate font-mono text-xs text-muted" title={r.pushUrl}>
              push: {r.pushUrl}
            </span>
          )}
        </span>
        {convert && (
          <Button size="sm" variant="ghost" onClick={() => void onAction({ kind: "setUrl", name: r.name, url: convert.url })} disabled={busy} title={`Change to ${convert.url}`}>
            {convert.to === "SSH" ? <KeyRound size={13} /> : <ArrowLeftRight size={13} />} Use {convert.to}
          </Button>
        )}
        <Button size="sm" variant="ghost" onClick={() => setEditing({ name: r.name, url: r.url, pushUrl: r.pushUrl ?? "", separate: !!r.pushUrl })} aria-label={`Edit ${r.name}`} title="Rename, change URLs">
          <Pencil size={13} />
        </Button>
        {removing ? (
          <>
            <Button size="sm" variant="danger" onClick={() => void onAction({ kind: "remove", name: r.name })} disabled={busy}>
              Remove
            </Button>
            <Button size="sm" variant="ghost" onClick={() => setRemoving(false)} aria-label="Keep it">
              <X size={13} />
            </Button>
          </>
        ) : (
          <Button size="sm" variant="ghost" onClick={() => setRemoving(true)} aria-label={`Remove ${r.name}`} title="Remove (the repository on the server isn't touched)">
            <Trash2 size={13} />
          </Button>
        )}
      </div>
    </li>
  );
}

/** Add a remote by host and owner/repository over HTTPS or SSH, or by pasting any URL. */
function AddRemote({ taken, busy, onAdd }: { taken: string[]; busy: boolean; onAdd: (name: string, url: string) => void }) {
  const [name, setName] = useState(taken.includes("origin") ? "" : "origin");
  const [protocol, setProtocol] = useState<"https" | "ssh">("https");
  const [host, setHost] = useState("github.com");
  const [repo, setRepo] = useState("");
  const [custom, setCustom] = useState("");
  // A pasted full URL in the owner/repo box is taken as it is.
  const pasted = /^(https?:\/\/|ssh:\/\/|[\w.-]+@[\w.-]+:)/.test(repo.trim());
  const path = repo.trim().replace(/^\/+|\/+$/g, "").replace(/\.git$/, "");
  const url = host === "custom" ? custom.trim() : pasted ? repo.trim() : path ? (protocol === "ssh" ? `git@${host}:${path}.git` : `https://${host}/${path}.git`) : "";
  const nameTaken = taken.includes(name.trim());
  const ok = !!name.trim() && !nameTaken && !!url && (host === "custom" || pasted || /^[\w.-]+\/[\w./-]+$/.test(path));
  return (
    <form
      className="space-y-2 rounded-lg bg-surface-2/50 p-3"
      onSubmit={(e) => {
        e.preventDefault();
        if (!ok) return;
        onAdd(name.trim(), url);
        setRepo("");
        setCustom("");
        setName("");
      }}
    >
      <div className="flex flex-wrap items-center gap-2">
        <span className="text-xs font-semibold text-fg">Add a remote</span>
        <span className="flex-1" />
        <div className="flex rounded-md border border-line bg-surface p-0.5 text-xs" role="radiogroup" aria-label="Protocol">
          {(["https", "ssh"] as const).map((p) => (
            <button
              key={p}
              type="button"
              role="radio"
              aria-checked={protocol === p}
              onClick={() => setProtocol(p)}
              disabled={host === "custom" || pasted}
              className={clsx("rounded px-2 py-0.5 font-medium disabled:opacity-50", protocol === p && host !== "custom" && !pasted ? "bg-accent text-accent-fg" : "text-muted hover:text-fg")}
              title={p === "https" ? "Signs in through your Git credential helper" : "Uses your SSH key"}
            >
              {p.toUpperCase()}
            </button>
          ))}
        </div>
      </div>
      <div className="grid grid-cols-[7rem_minmax(0,1fr)] gap-2">
        <input value={name} onChange={(e) => setName(e.target.value)} placeholder="origin" aria-label="Remote name" className={clsx(inputClass, "font-mono")} />
        <select value={host} onChange={(e) => setHost(e.target.value)} aria-label="Host" className={inputClass}>
          {hosts.map((h) => (
            <option key={h.id} value={h.id}>
              {h.label}
            </option>
          ))}
          <option value="custom">Other URL…</option>
        </select>
        {host === "custom" ? (
          <input value={custom} onChange={(e) => setCustom(e.target.value)} placeholder="https://git.example.com/team/repo.git or git@host:team/repo.git" aria-label="Remote URL" className={clsx(inputClass, "col-span-2 font-mono")} />
        ) : (
          <input value={repo} onChange={(e) => setRepo(e.target.value)} placeholder="owner/repository, or paste a URL" aria-label="Owner and repository" className={clsx(inputClass, "col-span-2 font-mono")} />
        )}
      </div>
      <div className="flex items-center gap-2">
        <span className={clsx("min-w-0 flex-1 truncate font-mono text-[11px]", nameTaken ? "text-warn" : "text-muted")} title={url}>
          {nameTaken ? `A remote called ${name.trim()} exists already` : url}
        </span>
        <Button type="submit" size="sm" disabled={!ok || busy}>
          <Plus size={13} /> Add
        </Button>
      </div>
    </form>
  );
}

/** Pull mode, pruning and first-push tracking for this repository only. */
function RepoSettingsCard({ site, onChanged }: { site: Site; onChanged: () => void }) {
  const notify = useAppStore((s) => s.notify);
  const settings = useQuery({ queryKey: ["git-repo-settings", site.name], queryFn: () => api.gitRepoSettings(site.name) });
  const save = useMutation({
    mutationFn: (next: GitRepoSettings) => api.gitSetRepoSettings(site.name, next),
    onSuccess: () => {
      settings.refetch();
      onChanged();
    },
    onError: (e) => notify("error", String(e)),
  });
  const s = settings.data;
  return (
    <section className="space-y-3 rounded-lg border border-line bg-surface p-4">
      <h4 className="text-sm font-semibold text-fg">Sync settings</h4>
      {!s ? (
        <p className="text-xs text-muted">Loading…</p>
      ) : (
        <div className="space-y-2.5 text-sm">
          <label className="flex flex-wrap items-center justify-between gap-2">
            <span>
              <span className="block text-fg">When pulling</span>
              <span className="block text-[11px] text-muted">How your commits and the remote's are combined.</span>
            </span>
            <select value={s.pull ?? ""} onChange={(e) => save.mutate({ ...s, pull: (e.target.value || null) as GitRepoSettings["pull"] })} className={clsx(inputClass.replace("w-full", ""), "w-56 py-1.5")} disabled={save.isPending}>
              <option value="">Your global Git setting</option>
              <option value="merge">Merge (a merge commit when needed)</option>
              <option value="rebase">Rebase (your commits on top)</option>
              <option value="ff-only">Fast-forward only (stop if they differ)</option>
            </select>
          </label>
          <label className="flex items-start gap-2">
            <input type="checkbox" checked={s.prune} onChange={(e) => save.mutate({ ...s, prune: e.target.checked })} className="mt-1 size-3.5 accent-[var(--accent)]" disabled={save.isPending} />
            <span>
              <span className="block text-fg">Clean up deleted remote branches on fetch</span>
              <span className="block text-[11px] text-muted">Branches deleted on the server disappear here too (fetch.prune).</span>
            </span>
          </label>
          <label className="flex items-start gap-2">
            <input type="checkbox" checked={s.autoSetupRemote} onChange={(e) => save.mutate({ ...s, autoSetupRemote: e.target.checked })} className="mt-1 size-3.5 accent-[var(--accent)]" disabled={save.isPending} />
            <span>
              <span className="block text-fg">Track new branches on their first push</span>
              <span className="block text-[11px] text-muted">A plain git push from the terminal publishes a new branch too (push.autoSetupRemote).</span>
            </span>
          </label>
        </div>
      )}
    </section>
  );
}

/** Clones a repository into www/; the folder then shows as a project like any other. */
export function CloneDialog({ onClose, onCloned }: { onClose: () => void; onCloned: (folder: string) => void }) {
  const notify = useAppStore((s) => s.notify);
  const [url, setUrl] = useState("");
  const [folder, setFolder] = useState("");
  const [op, setOp] = useState<string | null>(null);
  const progress = useGitProgress(op);
  const guess = url.trim().replace(/\/+$/, "").split(/[/:]/).pop()?.replace(/\.git$/, "").toLowerCase().replace(/[^a-z0-9_-]+/g, "-").replace(/^-+|-+$/g, "") ?? "";
  const clone = useMutation({
    mutationFn: () => {
      const id = `clone:${Date.now()}`;
      setOp(id);
      return api.gitClone(url.trim(), folder.trim() || undefined, id);
    },
    onSettled: () => setOp(null),
    onSuccess: (name) => {
      notify("info", `Cloned into www/${name}`);
      onCloned(name);
      onClose();
    },
    onError: (e) => String(e) !== "Cancelled." && notify("error", String(e)),
  });
  return (
    <Dialog
      open
      onClose={() => {
        // Closing while it runs stops the clone; the partial folder is removed.
        if (op) void api.gitCancel(op);
        onClose();
      }}
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
        {clone.isPending && op && <ProgressBar line={progress} label="Connecting…" onCancel={() => void api.gitCancel(op)} />}
        <button type="submit" hidden />
      </form>
    </Dialog>
  );
}
