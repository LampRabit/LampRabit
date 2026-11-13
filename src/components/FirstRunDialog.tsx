import { useEffect, useMemo, useState } from "react";
import { useMutation, useQueries, useQuery, useQueryClient } from "@tanstack/react-query";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { AlertCircle, Bot, Braces, CheckCircle2, Circle, Coffee, FlaskConical, FolderInput, FolderOpen, Globe, Hand, Layers, Loader2, Newspaper, Play, Plus, RotateCw, Sparkles, Terminal } from "lucide-react";
import clsx from "clsx";
import { api, serviceLabels, type ImportSource, type ImportedProject, type NodeRelease, type Release, type ServiceKind, type StarterItem } from "../lib/api";
import { labelFor, useCatalog } from "../lib/catalog";
import { installKey, useAppStore } from "../lib/store";
import { formatBytes } from "../lib/usage";
import Dialog from "./Dialog";
import MoveEnvironmentDialog from "./MoveEnvironmentDialog";
import { Button } from "./ui";

/**
 * The setup guide. On the first run it asks where the LampRabit folder goes (before
 * anything is downloaded into it), then what the developer builds: each usage brings its
 * recommended components, shown by category with their download size and platform notes,
 * and any other component can be added. They install one by one with progress and a retry
 * each. Then projects can be copied over from other tools (Laragon, XAMPP, Herd…), the
 * developer says their name for Home's greeting, and a first project is offered. Settings →
 * General opens it again (without the folder step), or only its import step.
 */

type UsageId = "php" | "laravel" | "wordpress" | "javascript" | "python" | "java" | "go" | "ai";

export const usages: { id: UsageId; title: string; blurb: string; icon: typeof Globe; runtimes: string[] }[] = [
  { id: "php", title: "PHP websites", blurb: "Plain PHP, Symfony, CodeIgniter…", icon: Globe, runtimes: ["apache", "php", "composer", "mariadb", "mailpit", "phpmyadmin", "git"] },
  { id: "laravel", title: "Laravel", blurb: "With Vite, queues and cache", icon: Layers, runtimes: ["apache", "php", "composer", "mariadb", "redis", "mailpit", "phpmyadmin", "node", "git"] },
  { id: "wordpress", title: "WordPress", blurb: "Themes, plugins and sites", icon: Newspaper, runtimes: ["apache", "php", "mariadb", "mailpit", "phpmyadmin", "git"] },
  { id: "javascript", title: "JavaScript", blurb: "Node.js, Next.js, Vite, Express…", icon: Braces, runtimes: ["apache", "node", "git"] },
  { id: "python", title: "Python", blurb: "Django, Flask, FastAPI", icon: FlaskConical, runtimes: ["apache", "python", "postgres", "redis", "git"] },
  { id: "java", title: "Java", blurb: "Spring Boot and web apps on Tomcat", icon: Coffee, runtimes: ["apache", "java", "maven", "tomcat", "postgres", "git"] },
  { id: "go", title: "Go and Rust", blurb: "APIs and services", icon: Terminal, runtimes: ["apache", "go", "rust", "postgres", "redis", "git"] },
  { id: "ai", title: "AI and automation", blurb: "Local models and workflows", icon: Bot, runtimes: ["ollama", "n8n"] },
];

/** Services-page entries that aren't components to add here. */
const notOffered = new Set(["tunnels", "sqlite"]);

type ItemState = { status: "waiting" | "working" | "done" | "error"; error?: string };
type Step = "folder" | "starter" | "install" | "import" | "name" | "project";

export default function FirstRunDialog() {
  const queryClient = useQueryClient();
  const first = useQuery({ queryKey: ["first-run"], queryFn: api.firstRun, staleTime: Infinity });
  const reopened = useAppStore((s) => s.setupGuideOpen);
  const setReopened = useAppStore((s) => s.setSetupGuideOpen);
  if (first.data) return <Guide firstRun onDone={() => queryClient.setQueryData(["first-run"], false)} />;
  if (reopened) return <Guide key={reopened} only={reopened === "import" ? "import" : undefined} onDone={() => setReopened(false)} />;
  return null;
}

function Guide({ firstRun = false, only, onDone }: { firstRun?: boolean; only?: "import"; onDone: () => void }) {
  const queryClient = useQueryClient();
  const notify = useAppStore((s) => s.notify);
  const setNewProjectOpen = useAppStore((s) => s.setNewProjectOpen);
  const settings = useQuery({ queryKey: ["settings"], queryFn: api.getSettings });
  const catalog = useCatalog();
  const [step, setStep] = useState<Step>(only ?? (firstRun ? "folder" : "starter"));
  const [moving, setMoving] = useState(false);
  const [picked, setPicked] = useState<UsageId[]>(["php"]);
  const [extra, setExtra] = useState<string[]>([]);
  const [skipped, setSkipped] = useState<Set<string>>(new Set());

  // In the Services page's order, so each category's components stay together.
  const order = useMemo(() => (catalog.data?.groups ?? []).flatMap((g) => g.items), [catalog.data]);
  const chosen = useMemo(() => {
    const ids = [...new Set([...usages.filter((u) => picked.includes(u.id)).flatMap((u) => u.runtimes), ...extra])];
    const at = (id: string) => (order.indexOf(id) < 0 ? order.length : order.indexOf(id));
    return ids.sort((a, b) => at(a) - at(b));
  }, [picked, extra, order]);
  const wanted = chosen.filter((r) => !skipped.has(r));

  const startNow = useMutation({
    mutationFn: () => api.applyServices(wanted.filter((r) => r in serviceLabels) as ServiceKind[], []),
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
    for (const key of ["installed", "settings", "services", "node", "sites"]) queryClient.invalidateQueries({ queryKey: [key] });
    onDone();
  };

  if (!settings.data) return null;
  // The name is asked on the first run, and by a reopened guide while there's none yet.
  const askName = firstRun || !settings.data.developerName?.trim();
  const afterImport = () => (only ? close() : setStep(askName ? "name" : "project"));
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
        picked={picked}
        setPicked={setPicked}
        extra={extra}
        setExtra={setExtra}
        chosen={chosen}
        skipped={skipped}
        setSkipped={setSkipped}
        onClose={close}
        onInstall={() => setStep("install")}
        onSkip={() => setStep("import")}
      />
    );
  }
  if (step === "install") {
    const labels = Object.fromEntries(wanted.map((id) => [id, labelFor(catalog.data, id)]));
    return <InstallStep runtimes={wanted} labels={labels} onClose={close} onNext={() => setStep("import")} />;
  }
  if (step === "import") return <ImportStep onClose={close} onNext={afterImport} nextLabel={only ? "Done" : "Continue"} />;
  if (step === "name") return <NameStep initial={settings.data.developerName ?? ""} onClose={close} onNext={() => setStep("project")} />;

  // The components that run as services (not PHP, Composer or Node), to start in one go.
  const startable = wanted.filter((r) => r in serviceLabels) as ServiceKind[];
  const name = settings.data.developerName?.trim();
  const examples = picked.includes("wordpress") ? "WordPress" : picked.includes("laravel") ? "Laravel" : picked.includes("javascript") ? "Vite, Next.js, Express…" : "Laravel, Symfony, WordPress…";
  return (
    <Dialog
      open
      onClose={close}
      title={name ? `You're all set, ${name}` : "Your stack is ready"}
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
        <p className="text-xs">This guide is in Settings → General whenever you want more services or to import projects.</p>
      </div>
    </Dialog>
  );
}

function StarterStep({
  picked,
  setPicked,
  extra,
  setExtra,
  chosen,
  skipped,
  setSkipped,
  onClose,
  onInstall,
  onSkip,
}: {
  picked: UsageId[];
  setPicked: (p: UsageId[]) => void;
  extra: string[];
  setExtra: (e: string[]) => void;
  chosen: string[];
  skipped: Set<string>;
  setSkipped: (s: Set<string>) => void;
  onClose: () => void;
  onInstall: () => void;
  onSkip: () => void;
}) {
  const catalog = useCatalog();
  const node = useQuery({ queryKey: ["node"], queryFn: api.nodeStatus });
  // One plan per component, kept, so picking another usage only checks what's new.
  const planned = chosen.filter((id) => id !== "node");
  const plans = useQueries({
    queries: planned.map((id) => ({ queryKey: ["starter-item", id], queryFn: () => api.starterPlan([id]).then((items) => items[0] ?? null), staleTime: 5 * 60_000 })),
  });
  const byId = new Map<string, StarterItem>();
  planned.forEach((id, i) => {
    const item = plans[i]?.data;
    if (item) byId.set(id, item);
  });
  const loading = plans.some((p) => p.isLoading);
  const failed = plans.find((p) => p.isError);
  const toInstall = chosen.filter((id) => {
    if (skipped.has(id)) return false;
    if (id === "node") return !node.data?.current;
    const item = byId.get(id);
    return item && item.action.kind !== "ready" && item.action.kind !== "unavailable";
  });
  const total = toInstall.reduce((n, id) => n + (byId.get(id)?.size ?? 0), 0);
  const blocked = chosen.some((id) => !skipped.has(id) && byId.get(id)?.action.kind === "unavailable");
  const summary = loading ? "Checking downloads…" : toInstall.length === 0 ? "Everything is installed" : `${toInstall.length} to install${total ? `, about ${formatBytes(total)}` : ""}`;

  const groups = (catalog.data?.groups ?? []).map((g) => ({ ...g, items: g.items.filter((id) => !notOffered.has(id)) }));
  const grouped = new Set(groups.flatMap((g) => g.items));
  const sections = [
    ...groups.map((g) => ({ id: g.id, title: g.title, rows: g.items.filter((id) => chosen.includes(id)), more: g.items.filter((id) => !chosen.includes(id)) })),
    // Until the catalog loads, or for anything it doesn't group.
    { id: "rest", title: "Components", rows: chosen.filter((id) => !grouped.has(id)), more: [] as string[] },
  ];
  const label = (id: string) => (id === "node" ? "Node.js" : (byId.get(id)?.label ?? labelFor(catalog.data, id)));
  const toggleSkip = (id: string) => {
    const next = new Set(skipped);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    setSkipped(next);
  };

  return (
    <Dialog
      open
      onClose={onClose}
      title="What do you build?"
      icon={<Sparkles size={18} />}
      width="max-w-4xl"
      footer={
        <>
          <Button onClick={onSkip} className="mr-auto">
            Skip, I'll use Services
          </Button>
          <span className="text-xs text-muted">{summary}</span>
          <Button variant="primary" onClick={onInstall} disabled={loading || chosen.every((id) => skipped.has(id))} data-autofocus>
            {toInstall.length === 0 ? "Continue" : "Install"}
          </Button>
        </>
      }
    >
      <p className="mb-3 text-sm text-muted">Pick everything you work on. Each brings the services it needs; untick any you don't want, or add others from their category.</p>
      <div className="grid gap-4 md:grid-cols-[15rem_minmax(0,1fr)]">
        <div className="grid grid-cols-2 content-start gap-1.5 md:grid-cols-1" role="group" aria-label="What you build">
          {usages.map((u) => {
            const on = picked.includes(u.id);
            return (
              <button
                key={u.id}
                type="button"
                aria-pressed={on}
                onClick={() => setPicked(on ? picked.filter((p) => p !== u.id) : [...picked, u.id])}
                className={clsx("flex items-start gap-2.5 rounded-xl border px-3 py-2 text-left", on ? "border-accent bg-accent-soft/40" : "border-line hover:bg-surface-2")}
              >
                <u.icon size={16} className={clsx("mt-0.5 shrink-0", on ? "text-accent-text" : "text-muted")} aria-hidden />
                <span className="min-w-0">
                  <span className="block text-sm font-semibold text-fg">{u.title}</span>
                  <span className="block text-xs text-muted">{u.blurb}</span>
                </span>
              </button>
            );
          })}
        </div>
        <div className="min-w-0 space-y-3">
          {failed && <p className="text-xs text-danger-text">Couldn't check some downloads: {String(failed.error)}</p>}
          {chosen.length === 0 && <p className="rounded-xl border border-dashed border-line p-4 text-sm text-muted">Pick what you build, or add components below.</p>}
          {sections
            .filter((s) => s.rows.length > 0)
            .map((s) => (
              <section key={s.id} aria-label={s.title}>
                <h3 className="mb-1 text-xs font-semibold tracking-wide text-muted uppercase">{s.title}</h3>
                <ul className="divide-y divide-line rounded-xl border border-line">
                  {s.rows.map((id) => {
                    const off = skipped.has(id);
                    const item = byId.get(id);
                    const { note, tone } = describe(id, item, node.data, id !== "node" && !item && loading);
                    return (
                      <li key={id} className={clsx("flex items-center gap-3 px-3 py-2", off && "opacity-50")}>
                        <input type="checkbox" checked={!off} onChange={() => toggleSkip(id)} aria-label={`Include ${label(id)}`} className="size-4 accent-[var(--accent)]" />
                        <div className="min-w-0 flex-1">
                          <div className="text-sm font-medium text-fg">{label(id)}</div>
                          <div className={clsx("truncate text-xs", tone)} title={note}>
                            {note}
                          </div>
                        </div>
                        {extra.includes(id) && (
                          <button type="button" className="text-xs text-muted hover:text-fg" onClick={() => setExtra(extra.filter((e) => e !== id))}>
                            Remove
                          </button>
                        )}
                      </li>
                    );
                  })}
                </ul>
              </section>
            ))}
          {sections.some((s) => s.more.length > 0) && (
            <section aria-label="Add more">
              <h3 className="mb-1 text-xs font-semibold tracking-wide text-muted uppercase">Add more</h3>
              <div className="space-y-1.5 rounded-xl border border-line px-3 py-2">
                {sections
                  .filter((s) => s.more.length > 0)
                  .map((s) => (
                    <div key={s.id} className="flex flex-wrap items-center gap-1.5">
                      <span className="w-32 shrink-0 text-xs text-muted">{s.title}</span>
                      {s.more.map((id) => (
                        <button
                          key={id}
                          type="button"
                          onClick={() => {
                            setExtra([...extra, id]);
                            if (skipped.has(id)) toggleSkip(id);
                          }}
                          className="inline-flex items-center gap-1 rounded-full border border-line px-2 py-0.5 text-xs text-muted hover:border-accent hover:text-fg"
                        >
                          <Plus size={11} aria-hidden /> {label(id)}
                        </button>
                      ))}
                    </div>
                  ))}
              </div>
            </section>
          )}
          {blocked && <p className="text-xs text-warn">Components that can't be installed on this computer are skipped; their note says why.</p>}
        </div>
      </div>
    </Dialog>
  );
}

type ImportState = { status: "working" | "done" | "error"; files?: number; error?: string; copied?: ImportedProject };
const validName = /^[A-Za-z0-9][A-Za-z0-9_-]{0,62}$/;

/**
 * Projects from other local environments, copied into www/ (the originals stay, so the other
 * tool keeps working). Tools LampRabit doesn't look for are covered by "Choose a folder".
 */
export function ImportStep({ onClose, onNext, nextLabel = "Continue" }: { onClose: () => void; onNext: () => void; nextLabel?: string }) {
  const queryClient = useQueryClient();
  const notify = useAppStore((s) => s.notify);
  const found = useQuery({ queryKey: ["import-sources"], queryFn: api.importSources, staleTime: 0 });
  const [picked, setPickedFolders] = useState<ImportSource[]>([]);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [names, setNames] = useState<Record<string, string>>({});
  const [states, setStates] = useState<Record<string, ImportState>>({});
  const [running, setRunning] = useState(false);
  const sources = [...(found.data ?? []), ...picked];
  const projects = sources.flatMap((s) => s.projects);
  const nameOf = (path: string) => names[path] ?? projects.find((p) => p.path === path)?.target ?? "";
  const chosen = projects.filter((p) => selected.has(p.path) && states[p.path]?.status !== "done");
  const clash = (path: string) => chosen.some((p) => p.path !== path && nameOf(p.path).toLowerCase() === nameOf(path).toLowerCase());
  const invalid = chosen.some((p) => !validName.test(nameOf(p.path)) || clash(p.path));
  const done = Object.values(states).filter((s) => s.status === "done");

  // Copy progress for the project being copied (one at a time).
  useEffect(() => {
    const off = listen<{ name: string; files: number }>("import-progress", ({ payload }) =>
      setStates((s) => {
        const path = Object.keys(s).find((p) => s[p].status === "working");
        return path ? { ...s, [path]: { ...s[path], files: payload.files } } : s;
      }),
    );
    return () => {
      off.then((f) => f());
    };
  }, []);

  const chooseFolder = async () => {
    const dir = await open({ directory: true, title: "The folder that holds your projects" });
    if (typeof dir !== "string") return;
    try {
      const source = await api.importFolder(dir);
      setPickedFolders((list) => [...list.filter((s) => s.folders[0] !== source.folders[0]), source]);
    } catch (e) {
      notify("error", String(e));
    }
  };

  const run = async () => {
    setRunning(true);
    for (const p of chosen) {
      setStates((s) => ({ ...s, [p.path]: { status: "working" } }));
      try {
        const copied = await api.importProject(p.path, nameOf(p.path));
        setStates((s) => ({ ...s, [p.path]: { status: "done", copied } }));
      } catch (e) {
        setStates((s) => ({ ...s, [p.path]: { status: "error", error: String(e) } }));
      }
    }
    setRunning(false);
    queryClient.invalidateQueries({ queryKey: ["sites"] });
  };

  const skippedDeps = done.some((s) => s.copied?.skipped.some((k) => k === "vendor" || k === "node_modules"));
  return (
    <Dialog
      open
      onClose={onClose}
      title="Bring your projects over"
      icon={<FolderInput size={18} />}
      width="max-w-3xl"
      footer={
        <>
          <Button onClick={chooseFolder} className="mr-auto" disabled={running}>
            <FolderOpen size={15} /> Choose a folder…
          </Button>
          {chosen.length > 0 ? (
            <Button variant="primary" onClick={run} disabled={running || invalid} data-autofocus>
              {running ? "Copying…" : `Copy ${chosen.length} into www`}
            </Button>
          ) : (
            <Button variant="primary" onClick={onNext} disabled={running} data-autofocus>
              {done.length ? nextLabel : "Skip"}
            </Button>
          )}
        </>
      }
    >
      <div className="space-y-4 text-sm">
        <p className="text-muted">
          Projects from Laragon, XAMPP, AMPPS, MAMP, Herd, Valet, FlyEnv, EnvKit, WampServer, ServBay or Local are copied into <span className="font-mono">www</span>, where each gets its
          .test domain. The originals stay where they are, so the other tool keeps working.
        </p>
        {found.isLoading && (
          <p className="flex items-center gap-2 text-muted">
            <Loader2 size={14} className="animate-spin" /> Looking for other tools…
          </p>
        )}
        {found.isError && <p className="text-xs text-danger-text">Couldn't look for other tools: {String(found.error)}</p>}
        {found.isSuccess && sources.length === 0 && (
          <p className="rounded-xl border border-dashed border-line p-4 text-muted">No other tool's projects were found here. If yours are somewhere else (EnvKit, a Sites folder…), choose the folder that holds them.</p>
        )}
        {sources.map((s) => {
          const all = s.projects.every((p) => selected.has(p.path) || states[p.path]?.status === "done");
          return (
            <section key={`${s.id}:${s.folders[0] ?? ""}`} aria-label={s.name} className="rounded-xl border border-line">
              <header className="flex flex-wrap items-center gap-2 border-b border-line px-3 py-2">
                <span className="font-semibold text-fg">{s.name}</span>
                <span className="min-w-0 flex-1 truncate font-mono text-xs text-muted" title={s.folders.join("\n")}>
                  {s.folders.join(" · ")}
                </span>
                {s.projects.length > 0 && (
                  <button
                    type="button"
                    className="text-xs text-accent-text hover:underline"
                    disabled={running}
                    onClick={() => {
                      const next = new Set(selected);
                      for (const p of s.projects) {
                        if (all) next.delete(p.path);
                        else next.add(p.path);
                      }
                      setSelected(next);
                    }}
                  >
                    {all ? "Select none" : "Select all"}
                  </button>
                )}
              </header>
              <ul className="divide-y divide-line">
                {s.projects.map((p) => {
                  const st = states[p.path];
                  const on = selected.has(p.path);
                  const name = nameOf(p.path);
                  const bad = on && st?.status !== "done" && (!validName.test(name) || clash(p.path));
                  return (
                    <li key={p.path} className="flex flex-wrap items-center gap-x-3 gap-y-1 px-3 py-2">
                      {st?.status === "done" ? (
                        <CheckCircle2 size={16} className="shrink-0 text-ok" aria-label="Copied" />
                      ) : st?.status === "working" ? (
                        <Loader2 size={16} className="shrink-0 animate-spin text-accent-text" aria-label="Copying" />
                      ) : (
                        <input
                          type="checkbox"
                          checked={on}
                          disabled={running}
                          onChange={() => {
                            const next = new Set(selected);
                            if (on) next.delete(p.path);
                            else next.add(p.path);
                            setSelected(next);
                          }}
                          aria-label={`Import ${p.name}`}
                          className="size-4 accent-[var(--accent)]"
                        />
                      )}
                      <div className="min-w-0 flex-1">
                        <div className="flex flex-wrap items-center gap-2">
                          <span className="font-medium text-fg">{p.name}</span>
                          {p.framework && <span className="rounded-full bg-surface-2 px-2 py-0.5 text-[11px] text-muted">{p.framework}</span>}
                        </div>
                        <div className="truncate font-mono text-xs text-muted" title={p.path}>
                          {p.path}
                          {p.database && <span className="font-sans"> · database {p.database}</span>}
                        </div>
                      </div>
                      <label className="flex items-center gap-1 text-xs text-muted">
                        <span className="font-mono">www/</span>
                        <input
                          value={name}
                          disabled={!on || running || st?.status === "done"}
                          onChange={(e) => setNames({ ...names, [p.path]: e.target.value.trim() })}
                          aria-label={`Folder name for ${p.name}`}
                          aria-invalid={bad}
                          className={clsx("w-40 rounded-md border bg-surface px-2 py-1 font-mono text-xs text-fg disabled:opacity-60", bad ? "border-danger-text" : "border-line")}
                        />
                      </label>
                      {st?.status === "working" && st.files ? <span className="w-full pl-7 text-xs text-muted tabular-nums">{st.files} files copied…</span> : null}
                      {st?.status === "error" && <p className="w-full pl-7 text-xs break-words text-danger-text">{st.error}</p>}
                      {bad && <p className="w-full pl-7 text-xs text-danger-text">{clash(p.path) ? "Two projects would get this name." : "Use letters, numbers, dashes and underscores (it becomes the domain)."}</p>}
                    </li>
                  );
                })}
              </ul>
              {s.databases.length > 0 && (
                <p className="border-t border-line px-3 py-2 text-xs text-muted">
                  Its databases stay in {s.name}: <span className="text-fg">{s.databases.join(", ")}</span>. Export them there (phpMyAdmin → Export → SQL), then import the file with LampRabit's phpMyAdmin or
                  Adminer.
                </p>
              )}
            </section>
          );
        })}
        {done.length > 0 && (
          <p className="rounded-lg bg-surface-2 px-3 py-2 text-xs text-muted">
            Copied {done.length} {done.length === 1 ? "project" : "projects"} into www.
            {skippedDeps && " Dependency folders (vendor, node_modules) weren't copied: each project's Overview offers to install them."}
          </p>
        )}
      </div>
    </Dialog>
  );
}

/** The developer's name, for Home's greeting. */
function NameStep({ initial, onClose, onNext }: { initial: string; onClose: () => void; onNext: () => void }) {
  const queryClient = useQueryClient();
  const notify = useAppStore((s) => s.notify);
  const [name, setName] = useState(initial);
  const save = useMutation({
    mutationFn: async () => {
      const current = await api.getSettings();
      return api.saveSettings({ ...current, developerName: name.trim() });
    },
    onSuccess: (saved) => {
      queryClient.setQueryData(["settings"], saved);
      onNext();
    },
    onError: (e) => notify("error", String(e)),
  });
  return (
    <Dialog
      open
      onClose={onClose}
      title="Last thing: what should we call you?"
      icon={<Hand size={18} />}
      width="max-w-md"
      footer={
        <>
          <Button onClick={onNext} className="mr-auto">
            Skip
          </Button>
          <Button variant="primary" onClick={() => save.mutate()} disabled={save.isPending || !name.trim()}>
            Continue
          </Button>
        </>
      }
    >
      <form
        className="space-y-2 text-sm"
        onSubmit={(e) => {
          e.preventDefault();
          if (name.trim()) save.mutate();
        }}
      >
        <label className="block text-muted" htmlFor="developer-name">
          Home greets you by name. It stays on this computer; change it any time in Settings → General.
        </label>
        <input
          id="developer-name"
          data-autofocus
          value={name}
          maxLength={40}
          autoComplete="given-name"
          placeholder="Your name"
          onChange={(e) => setName(e.target.value)}
          className="w-full rounded-lg border border-line bg-surface px-3 py-2 text-fg"
        />
      </form>
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
