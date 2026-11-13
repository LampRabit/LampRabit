import { useEffect, useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { AlertCircle, CheckCircle2, Circle, FolderInput, FolderOpen, Loader2, Play, RotateCw, Sparkles } from "lucide-react";
import clsx from "clsx";
import { api, serviceLabels, type NodeRelease, type Release, type ServiceKind, type StarterItem } from "../lib/api";
import { installKey, useAppStore } from "../lib/store";
import { formatBytes } from "../lib/usage";
import Dialog from "./Dialog";
import MoveEnvironmentDialog from "./MoveEnvironmentDialog";
import { Button } from "./ui";

/**
 * The setup guide. On the first run it asks where the LampRabit folder goes (before
 * anything is downloaded into it), then offers a starter stack: each component with its
 * download size and platform notes, already-installed ones skipped, installed one by one
 * with progress and a retry per component, then optionally a first project. Settings →
 * General opens it again (without the folder step).
 */

type StarterId = "php" | "laravel" | "wordpress" | "node" | "full" | "custom";

const starters: { id: StarterId; title: string; blurb: string; runtimes: string[] }[] = [
  { id: "php", title: "PHP", blurb: "Apache, PHP, Composer, MariaDB and Mailpit: plain PHP sites and most frameworks.", runtimes: ["apache", "php", "composer", "mariadb", "mailpit", "phpmyadmin", "git"] },
  { id: "laravel", title: "Laravel", blurb: "The PHP stack plus Node.js for Vite and Redis for queues and cache.", runtimes: ["apache", "php", "composer", "mariadb", "redis", "mailpit", "phpmyadmin", "node", "git"] },
  { id: "wordpress", title: "WordPress", blurb: "Apache, PHP, MariaDB, phpMyAdmin and Mailpit to catch WordPress's mail.", runtimes: ["apache", "php", "mariadb", "mailpit", "phpmyadmin", "git"] },
  { id: "node", title: "Node.js", blurb: "Node.js through NVM, and Apache to give each project its .test domain.", runtimes: ["apache", "node", "mailpit", "git"] },
  {
    id: "full",
    title: "Full stack",
    blurb: "Both web servers, PHP, Node.js, MariaDB, MySQL, PostgreSQL, Redis, Mailpit and the database tools.",
    runtimes: ["apache", "nginx", "php", "composer", "node", "mariadb", "mysql", "postgres", "redis", "mailpit", "phpmyadmin", "adminer", "git"],
  },
  { id: "custom", title: "Custom", blurb: "Pick each component yourself.", runtimes: [] },
];

/** What Custom offers to pick from; everything else is on the Services page. */
const customChoices = ["apache", "nginx", "caddy", "php", "composer", "node", "git", "python", "java", "mariadb", "mysql", "postgres", "mongodb", "redis", "memcached", "mailpit", "phpmyadmin", "adminer"];

type ItemState = { status: "waiting" | "working" | "done" | "error"; error?: string };

export default function FirstRunDialog() {
  const queryClient = useQueryClient();
  const first = useQuery({ queryKey: ["first-run"], queryFn: api.firstRun, staleTime: Infinity });
  const reopened = useAppStore((s) => s.setupGuideOpen);
  const setReopened = useAppStore((s) => s.setSetupGuideOpen);
  if (first.data) return <Guide firstRun onDone={() => queryClient.setQueryData(["first-run"], false)} />;
  if (reopened) return <Guide onDone={() => setReopened(false)} />;
  return null;
}

function Guide({ firstRun = false, onDone }: { firstRun?: boolean; onDone: () => void }) {
  const queryClient = useQueryClient();
  const notify = useAppStore((s) => s.notify);
  const setNewProjectOpen = useAppStore((s) => s.setNewProjectOpen);
  const settings = useQuery({ queryKey: ["settings"], queryFn: api.getSettings });
  const [step, setStep] = useState<"folder" | "starter" | "install" | "project">(firstRun ? "folder" : "starter");
  const [moving, setMoving] = useState(false);
  const [starter, setStarter] = useState<StarterId>("php");
  const [custom, setCustom] = useState<string[]>(["apache", "php", "composer", "mariadb", "git"]);
  const [skipped, setSkipped] = useState<Set<string>>(new Set());
  const chosen = starter === "custom" ? custom : starters.find((s) => s.id === starter)!.runtimes;

  const startNow = useMutation({
    mutationFn: () => api.applyServices(chosen.filter((r) => !skipped.has(r) && r in serviceLabels) as ServiceKind[], []),
    onSuccess: (problems) => {
      for (const p of problems) notify("error", p);
      queryClient.invalidateQueries({ queryKey: ["services"] });
    },
    onError: (e) => notify("error", String(e)),
  });
  const finishFolder = useMutation({
    mutationFn: api.finishFirstRun,
    onSuccess: () => setStep("starter"),
    onError: (e) => notify("error", String(e)),
  });
  // Leaving at any step still settles the folder, so later runs don't ask again.
  const close = () => {
    if (firstRun) api.finishFirstRun().catch(() => {});
    for (const key of ["installed", "settings", "services", "node"]) queryClient.invalidateQueries({ queryKey: [key] });
    onDone();
  };

  if (!settings.data) return null;
  if (moving) {
    return (
      <MoveEnvironmentDialog
        current={settings.data.rootDir}
        onClose={() => {
          setMoving(false);
          // A finished move changed the folder; show where it is now.
          queryClient.invalidateQueries({ queryKey: ["settings"] });
        }}
      />
    );
  }

  if (step === "folder") {
    return (
      <Dialog
        open
        onClose={close}
        title="Where should LampRabit keep its files?"
        icon={<FolderOpen size={18} />}
        width="max-w-lg"
        footer={
          <>
            <Button onClick={() => setMoving(true)} className="mr-auto">
              <FolderInput size={15} /> Choose another folder…
            </Button>
            <Button variant="primary" onClick={() => finishFolder.mutate()} disabled={finishFolder.isPending} data-autofocus>
              Use this folder
            </Button>
          </>
        }
      >
        <div className="space-y-3 text-sm">
          <p className="text-muted">
            Everything goes in one folder: the runtimes LampRabit downloads, their configs, your projects (<span className="font-mono">www</span>) and databases (
            <span className="font-mono">data</span>). It can grow to several gigabytes, so pick a drive with room.
          </p>
          <p className="rounded-lg bg-surface-2 px-3 py-2 font-mono break-all text-fg">{settings.data.rootDir}</p>
          <p className="text-xs text-muted">You can move it later in Settings → General.</p>
        </div>
      </Dialog>
    );
  }

  if (step === "starter") {
    return (
      <StarterStep
        starter={starter}
        setStarter={setStarter}
        custom={custom}
        setCustom={setCustom}
        chosen={chosen}
        skipped={skipped}
        setSkipped={setSkipped}
        onClose={close}
        onInstall={() => setStep("install")}
      />
    );
  }
  if (step === "install") return <InstallStep runtimes={chosen.filter((r) => !skipped.has(r))} onClose={close} onNext={() => setStep("project")} />;

  // The components that run as services (not PHP, Composer or Node), to start in one go.
  const startable = chosen.filter((r) => !skipped.has(r) && r in serviceLabels) as ServiceKind[];
  const examples = starter === "wordpress" ? "WordPress" : starter === "laravel" ? "Laravel" : starter === "node" ? "Vite, Next.js, Express…" : "Laravel, Symfony, WordPress…";
  return (
    <Dialog
      open
      onClose={close}
      title="Your stack is ready"
      icon={<Sparkles size={18} />}
      width="max-w-lg"
      footer={
        <>
          <Button onClick={close} className="mr-auto">
            Not now
          </Button>
          <Button
            variant="primary"
            data-autofocus
            onClick={() => {
              close();
              setNewProjectOpen(true);
            }}
          >
            Create a first project
          </Button>
        </>
      }
    >
      <div className="space-y-3 text-sm text-muted">
        {startable.length > 0 && (
          <div className="flex flex-wrap items-center gap-3 rounded-lg border border-line bg-surface-2/50 px-3 py-2">
            <span className="min-w-0 flex-1 text-fg">
              {startNow.isSuccess ? "Started. Their status is on Home." : `Start ${startable.map((k) => serviceLabels[k]).join(", ")} now?`}
            </span>
            {!startNow.isSuccess && (
              <Button size="sm" variant="primary" onClick={() => startNow.mutate()} disabled={startNow.isPending}>
                <Play size={12} className="fill-current" /> {startNow.isPending ? "Starting…" : "Start them now"}
              </Button>
            )}
          </div>
        )}
        <p>
          Every folder in <span className="font-mono">www</span> is served as <span className="font-mono">https://&lt;folder&gt;.test</span> while the web server runs; Home starts and stops the services.
        </p>
        <p>A new project can come from a template ({examples}), with its database created for you.</p>
        <p className="text-xs">This guide is in Settings → General whenever you want another starter.</p>
      </div>
    </Dialog>
  );
}

function StarterStep({
  starter,
  setStarter,
  custom,
  setCustom,
  chosen,
  skipped,
  setSkipped,
  onClose,
  onInstall,
}: {
  starter: StarterId;
  setStarter: (s: StarterId) => void;
  custom: string[];
  setCustom: (c: string[]) => void;
  chosen: string[];
  skipped: Set<string>;
  setSkipped: (s: Set<string>) => void;
  onClose: () => void;
  onInstall: () => void;
}) {
  // The plan for every component offered, so switching starters doesn't wait again.
  const offered = useMemo(() => [...new Set([...starters.flatMap((s) => s.runtimes), ...customChoices])].filter((r) => r !== "node"), []);
  const plan = useQuery({ queryKey: ["starter-plan", offered.join(",")], queryFn: () => api.starterPlan(offered), staleTime: 5 * 60_000 });
  const node = useQuery({ queryKey: ["node"], queryFn: api.nodeStatus });
  const byId = new Map((plan.data ?? []).map((i) => [i.runtime as string, i]));
  const rows = chosen.map((id) => ({ id, item: byId.get(id) }));
  const toInstall = rows.filter(({ id, item }) => !skipped.has(id) && (id === "node" ? !node.data?.current : item && item.action.kind !== "ready" && item.action.kind !== "unavailable"));
  const total = toInstall.reduce((n, r) => n + (r.item?.size ?? 0), 0);
  const blocked = rows.some(({ id, item }) => !skipped.has(id) && item?.action.kind === "unavailable");
  const summary = plan.isLoading ? "Checking downloads…" : toInstall.length === 0 ? "Everything is installed" : `${toInstall.length} to install${total ? `, about ${formatBytes(total)}` : ""}`;

  return (
    <Dialog
      open
      onClose={onClose}
      title="Pick a starter"
      icon={<Sparkles size={18} />}
      width="max-w-3xl"
      footer={
        <>
          <Button onClick={onClose} className="mr-auto">
            Skip, I'll use Services
          </Button>
          <span className="text-xs text-muted">{summary}</span>
          <Button variant="primary" onClick={onInstall} disabled={plan.isLoading || chosen.length === 0} data-autofocus>
            {toInstall.length === 0 ? "Continue" : "Install"}
          </Button>
        </>
      }
    >
      <div className="grid gap-4 md:grid-cols-[14rem_minmax(0,1fr)]">
        <div className="space-y-1.5" role="radiogroup" aria-label="Starter">
          {starters.map((s) => (
            <button
              key={s.id}
              type="button"
              role="radio"
              aria-checked={starter === s.id}
              onClick={() => {
                setStarter(s.id);
                setSkipped(new Set());
              }}
              className={clsx("w-full rounded-xl border px-3 py-2 text-left", starter === s.id ? "border-accent bg-accent-soft/40" : "border-line hover:bg-surface-2")}
            >
              <span className="block text-sm font-semibold text-fg">{s.title}</span>
              <span className="block text-xs text-muted">{s.blurb}</span>
            </button>
          ))}
        </div>
        <div className="min-w-0">
          {starter === "custom" && (
            <div className="mb-3 flex flex-wrap gap-1.5">
              {customChoices.map((id) => {
                const on = custom.includes(id);
                const label = id === "node" ? "Node.js" : (byId.get(id)?.label ?? id);
                return (
                  <button
                    key={id}
                    type="button"
                    aria-pressed={on}
                    onClick={() => setCustom(on ? custom.filter((c) => c !== id) : [...custom, id])}
                    className={clsx("rounded-full border px-2.5 py-1 text-xs font-medium", on ? "border-accent bg-accent-soft text-accent-text" : "border-line text-muted hover:text-fg")}
                  >
                    {label}
                  </button>
                );
              })}
            </div>
          )}
          {plan.isError && <p className="mb-2 text-xs text-danger-text">Couldn't check the downloads: {String(plan.error)}</p>}
          <ul className="divide-y divide-line rounded-xl border border-line">
            {rows.length === 0 && <li className="p-3 text-sm text-muted">Pick components above.</li>}
            {rows.map(({ id, item }) => {
              const off = skipped.has(id);
              const { note, tone } = describe(id, item, node.data, plan.isLoading);
              return (
                <li key={id} className={clsx("flex items-center gap-3 px-3 py-2", off && "opacity-50")}>
                  <input
                    type="checkbox"
                    checked={!off}
                    onChange={() => {
                      const next = new Set(skipped);
                      if (off) next.delete(id);
                      else next.add(id);
                      setSkipped(next);
                    }}
                    aria-label={`Include ${item?.label ?? id}`}
                    className="size-4 accent-[var(--accent)]"
                  />
                  <div className="min-w-0 flex-1">
                    <div className="text-sm font-medium text-fg">{id === "node" ? "Node.js" : (item?.label ?? id)}</div>
                    <div className={clsx("truncate text-xs", tone)} title={note}>
                      {note}
                    </div>
                  </div>
                </li>
              );
            })}
          </ul>
          {blocked && <p className="mt-2 text-xs text-warn">Components that can't be installed on this computer are skipped; their note says why.</p>}
        </div>
      </div>
    </Dialog>
  );
}

/** One component's line: installed, what downloads (and its size), or why it can't. */
function describe(id: string, item: StarterItem | undefined, node: { manager: string | null; current: string | null } | undefined, loading: boolean): { note: string; tone: string } {
  if (id === "node") {
    if (node?.current) return { note: `Installed: ${node.current}`, tone: "text-ok" };
    return { note: node?.manager ? "The newest LTS, through NVM" : "NVM, then the newest LTS (Windows asks for administrator approval for NVM)", tone: "text-muted" };
  }
  if (!item) return { note: loading ? "Checking…" : "Not offered here", tone: "text-muted" };
  const a = item.action;
  switch (a.kind) {
    case "ready":
      return { note: `Installed: ${a.version}`, tone: "text-ok" };
    case "activate":
      return { note: `Installed: ${a.version} (becomes the version in use)`, tone: "text-muted" };
    case "install":
      return { note: `Downloads ${a.release.version}${item.size ? ` · ${formatBytes(item.size)}` : ""}`, tone: "text-muted" };
    case "unavailable":
      return { note: a.reason, tone: "text-warn" };
    default:
      return { note: "", tone: "text-muted" };
  }
}

/**
 * Installs the components one by one; a failed one can be retried. Also used by a profile
 * import, for the services the profiles need (a service's id is its runtime's).
 */
export function InstallStep({
  runtimes: given,
  onClose,
  onNext,
  title = "Installing your stack",
  nextLabel = "Continue",
  labels: knownLabels,
  releases: chosen,
}: {
  runtimes: string[];
  onClose: () => void;
  onNext: () => void;
  title?: string;
  nextLabel?: string;
  /** Names to show before each component's plan says its own. */
  labels?: Record<string, string>;
  /** Exact releases to install for some components (a profile import's versions). */
  releases?: Record<string, Release>;
}) {
  const queryClient = useQueryClient();
  // The list it opened with; the parent's array is new on every render.
  const [runtimes] = useState(given);
  const [started] = useState({ done: false });
  const installs = useAppStore((s) => s.installs);
  const [states, setStates] = useState<Record<string, ItemState>>(() => Object.fromEntries(runtimes.map((r) => [r, { status: "waiting" }])));
  const [current, setCurrent] = useState<{ id: string; key?: string } | null>(null);
  const [labels, setLabels] = useState<Record<string, string>>({ node: "Node.js", ...knownLabels });

  const installOne = async (id: string) => {
    setStates((s) => ({ ...s, [id]: { status: "working" } }));
    try {
      if (id === "node") {
        setCurrent({ id });
        let status = await api.nodeStatus();
        if (!status.manager) {
          await api.nvmInstall();
          status = await api.nodeStatus();
          if (!status.manager) throw new Error("NVM was installed, but LampRabit can't find it yet. Restart LampRabit, then install Node.js on the Services page.");
        }
        if (!status.current) {
          const releases: NodeRelease[] = await api.nodeAvailable();
          const pick = releases.find((r) => r.lts) ?? releases[0];
          if (!pick) throw new Error("No Node.js release found; try again from the Services page.");
          const version = pick.version.replace(/^v/, "");
          await api.nodeInstall(version);
          await api.nodeUse(version);
        }
      } else if (chosen?.[id]) {
        const release = chosen[id];
        setCurrent({ id, key: installKey(release.runtime, release.version) });
        await api.installRuntime(release);
        await api.setActiveVersion(release.runtime, release.version);
      } else {
        // Planned again right before installing: an earlier component may have changed things.
        const [item]: StarterItem[] = await api.starterPlan([id]);
        if (!item) throw new Error("This component isn't offered on this computer.");
        setLabels((l) => ({ ...l, [id]: item.label }));
        const a = item.action;
        if (a.kind === "unavailable") throw new Error(a.reason);
        if (a.kind === "activate") await api.setActiveVersion(item.runtime, a.version);
        if (a.kind === "install") {
          setCurrent({ id, key: installKey(a.release.runtime, a.release.version) });
          await api.installRuntime(a.release);
          await api.setActiveVersion(a.release.runtime, a.release.version);
        }
      }
      setStates((s) => ({ ...s, [id]: { status: "done" } }));
    } catch (e) {
      setStates((s) => ({ ...s, [id]: { status: "error", error: String(e) } }));
    } finally {
      setCurrent(null);
      for (const key of ["installed", "settings", "services", "node"]) queryClient.invalidateQueries({ queryKey: [key] });
    }
  };

  // One at a time, in order: downloads share the connection, and later ones (Composer,
  // phpMyAdmin) are better off once PHP is there. Runs once for the list it opened with;
  // installs carry on in the backend if the dialog is hidden.
  const installOneRef = useRefLatest(installOne);
  useEffect(() => {
    // Once only, even when React runs effects twice in development.
    if (started.done) return;
    started.done = true;
    (async () => {
      for (const id of runtimes) await installOneRef.current(id);
    })();
  }, [runtimes, installOneRef, started]);

  const values = Object.values(states);
  const finished = values.every((s) => s.status === "done" || s.status === "error");
  const failed = values.filter((s) => s.status === "error").length;
  const progress = current?.key ? installs[current.key] : undefined;

  return (
    <Dialog
      open
      onClose={onClose}
      title={finished ? (failed ? "Some components didn't install" : "Installed") : title}
      icon={finished ? <CheckCircle2 size={18} /> : <Loader2 size={18} className="animate-spin" />}
      width="max-w-xl"
      footer={
        <>
          <Button onClick={onClose} className="mr-auto">
            {finished ? "Close" : "Hide"}
          </Button>
          <Button variant="primary" onClick={onNext} disabled={!finished} data-autofocus>
            {nextLabel}
          </Button>
        </>
      }
    >
      <ul className="divide-y divide-line rounded-xl border border-line">
        {runtimes.map((id) => {
          const st = states[id];
          const mine = st.status === "working" && current?.id === id;
          return (
            <li key={id} className="px-3 py-2">
              <div className="flex items-center gap-3">
                {st.status === "done" ? (
                  <CheckCircle2 size={16} className="shrink-0 text-ok" />
                ) : st.status === "error" ? (
                  <AlertCircle size={16} className="shrink-0 text-danger-text" />
                ) : st.status === "working" ? (
                  <Loader2 size={16} className="shrink-0 animate-spin text-accent-text" />
                ) : (
                  <Circle size={16} className="shrink-0 text-muted" />
                )}
                <span className="flex-1 text-sm font-medium text-fg">{labels[id] ?? id}</span>
                {mine && progress && (
                  <span className="text-xs text-muted tabular-nums">
                    {progress.stage === "downloading" ? `${formatBytes(progress.downloaded)}${progress.total ? ` of ${formatBytes(progress.total)}` : ""}` : progress.stage}
                  </span>
                )}
                {st.status === "error" && finished && (
                  <Button size="sm" onClick={() => installOne(id)}>
                    <RotateCw size={12} /> Retry
                  </Button>
                )}
              </div>
              {mine && progress?.total ? (
                <div className="mt-1.5 h-1 overflow-hidden rounded-full bg-surface-2">
                  <div className="h-full bg-accent transition-[width]" style={{ width: `${Math.min(100, (progress.downloaded / progress.total) * 100)}%` }} />
                </div>
              ) : null}
              {st.error && <p className="mt-1 text-xs break-words text-danger-text">{st.error}</p>}
            </li>
          );
        })}
      </ul>
    </Dialog>
  );
}

/** A ref that always holds the latest value, for effects that must run once. */
function useRefLatest<T>(value: T) {
  const [ref] = useState(() => ({ current: value }));
  ref.current = value;
  return ref;
}
