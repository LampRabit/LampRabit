import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { useMutation, useQueries, useQuery, useQueryClient } from "@tanstack/react-query";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import {
  AlertCircle,
  ArrowLeft,
  ArrowRight,
  Bot,
  Braces,
  Check,
  CheckCircle2,
  ChevronDown,
  Circle,
  Coffee,
  Database,
  FlaskConical,
  FolderInput,
  FolderOpen,
  Globe,
  HardDrive,
  Layers,
  Loader2,
  Newspaper,
  Play,
  Plus,
  RotateCw,
  Sparkles,
  Sun,
  Terminal,
  X,
} from "lucide-react";
import clsx from "clsx";
import { api, serviceLabels, type ImportSource, type ImportedProject, type NodeRelease, type Release, type ServiceKind, type Settings, type StarterItem } from "../lib/api";
import { labelFor, useCatalog } from "../lib/catalog";
import { installKey, useAppStore } from "../lib/store";
import { formatBytes } from "../lib/usage";
import Dialog from "./Dialog";
import Logo from "./Logo";
import MoveEnvironmentDialog from "./MoveEnvironmentDialog";
import { Button } from "./ui";

/**
 * The setup guide: a full-window onboarding with a step rail. On the first run it welcomes
 * the developer, asks where the LampRabit folder goes (before anything is downloaded into
 * it), then what they build: each usage brings its recommended components, listed by the
 * Services page's categories with sizes and platform notes, and any other can be added.
 * They install one by one with progress and a retry each; then projects can be copied over
 * from other tools (Laragon, XAMPP, Herd…), the developer gives their name for Home's
 * greeting, and a first project is offered. Settings → General opens it again (without the
 * welcome and folder steps), or only its import step.
 */

type UsageId = "php" | "laravel" | "wordpress" | "javascript" | "python" | "java" | "go" | "ai";

export const usages: { id: UsageId; title: string; blurb: string; icon: typeof Globe; runtimes: string[] }[] = [
  { id: "php", title: "PHP websites", blurb: "Plain PHP, Symfony, CodeIgniter…", icon: Globe, runtimes: ["apache", "php", "composer", "mariadb", "mailpit", "phpmyadmin", "git"] },
  { id: "laravel", title: "Laravel", blurb: "With Vite, queues and cache", icon: Layers, runtimes: ["apache", "php", "composer", "mariadb", "redis", "mailpit", "phpmyadmin", "node", "git"] },
  { id: "wordpress", title: "WordPress", blurb: "Themes, plugins and sites", icon: Newspaper, runtimes: ["apache", "php", "mariadb", "mailpit", "phpmyadmin", "git"] },
  { id: "javascript", title: "JavaScript", blurb: "Node.js, Next.js, Vite, Express…", icon: Braces, runtimes: ["apache", "node", "git"] },
  { id: "python", title: "Python", blurb: "Django, Flask, FastAPI", icon: FlaskConical, runtimes: ["apache", "python", "postgres", "redis", "git"] },
  { id: "java", title: "Java", blurb: "Spring Boot, Tomcat web apps", icon: Coffee, runtimes: ["apache", "java", "maven", "tomcat", "postgres", "git"] },
  { id: "go", title: "Go and Rust", blurb: "APIs and services", icon: Terminal, runtimes: ["apache", "go", "rust", "postgres", "redis", "git"] },
  { id: "ai", title: "AI and automation", blurb: "Local models and workflows", icon: Bot, runtimes: ["ollama", "n8n"] },
];

/** Services-page entries that aren't components to add here. */
const notOffered = new Set(["tunnels", "sqlite"]);

type Step = "welcome" | "folder" | "stack" | "install" | "import" | "name" | "done";
const stepLabels: Record<Step, string> = { welcome: "Welcome", folder: "Storage folder", stack: "Your stack", install: "Install", import: "Your projects", name: "About you", done: "Ready" };

export default function FirstRunDialog() {
  const queryClient = useQueryClient();
  const first = useQuery({ queryKey: ["first-run"], queryFn: api.firstRun, staleTime: Infinity });
  const reopened = useAppStore((s) => s.setupGuideOpen);
  const setReopened = useAppStore((s) => s.setSetupGuideOpen);
  const settings = useQuery({ queryKey: ["settings"], queryFn: api.getSettings });
  if (!settings.data) return null;
  if (first.data) return <Flow settings={settings.data} firstRun onDone={() => queryClient.setQueryData(["first-run"], false)} />;
  if (reopened) return <Flow key={reopened} settings={settings.data} only={reopened === "import" ? "import" : undefined} onDone={() => setReopened(false)} />;
  return null;
}

function Flow({ settings, firstRun = false, only, onDone }: { settings: Settings; firstRun?: boolean; only?: "import"; onDone: () => void }) {
  const queryClient = useQueryClient();
  const notify = useAppStore((s) => s.notify);
  const catalog = useCatalog();
  // Decided once, so saving the name doesn't take its step away mid-flow.
  const [steps] = useState<Step[]>(() => {
    if (only) return ["import"];
    const askName = firstRun || !settings.developerName?.trim();
    return [...(firstRun ? (["welcome", "folder"] as Step[]) : []), "stack", "install", "import", ...(askName ? (["name"] as Step[]) : []), "done"];
  });
  const [step, setStep] = useState<Step>(steps[0]);
  const [picked, setPicked] = useState<UsageId[]>(["php"]);
  const [extra, setExtra] = useState<string[]>([]);
  const [skipped, setSkipped] = useState<Set<string>>(new Set());
  const [installed, setInstalled] = useState(0);
  const [imported, setImported] = useState(0);

  // In the Services page's order, so each category's components stay together.
  const order = useMemo(() => (catalog.data?.groups ?? []).flatMap((g) => g.items), [catalog.data]);
  const chosen = useMemo(() => {
    const ids = [...new Set([...usages.filter((u) => picked.includes(u.id)).flatMap((u) => u.runtimes), ...extra])];
    const at = (id: string) => (order.indexOf(id) < 0 ? order.length : order.indexOf(id));
    return ids.sort((a, b) => at(a) - at(b));
  }, [picked, extra, order]);
  const wanted = chosen.filter((r) => !skipped.has(r));

  const finishFolder = useMutation({
    mutationFn: api.finishFirstRun,
    onSuccess: () => setStep("stack"),
    onError: (e) => notify("error", String(e)),
  });
  // Leaving at any step still settles the folder, so later runs don't ask again.
  const close = () => {
    if (firstRun) api.finishFirstRun().catch(() => {});
    for (const key of ["installed", "settings", "services", "node", "sites"]) queryClient.invalidateQueries({ queryKey: [key] });
    onDone();
  };
  const next = (from: Step) => {
    const i = steps.indexOf(from);
    if (i < 0 || i === steps.length - 1) close();
    else setStep(steps[i + 1]);
  };
  const nav: Nav = { steps, current: step, onSkip: close, skipLabel: only ? "Close" : firstRun ? "Skip setup" : "Close the guide" };

  switch (step) {
    case "welcome":
      return <WelcomeStep nav={nav} onNext={() => next("welcome")} />;
    case "folder":
      return <FolderStep nav={nav} root={settings.rootDir} busy={finishFolder.isPending} onBack={() => setStep("welcome")} onNext={() => finishFolder.mutate()} />;
    case "stack":
      return (
        <StackStep
          nav={nav}
          picked={picked}
          setPicked={setPicked}
          extra={extra}
          setExtra={setExtra}
          chosen={chosen}
          skipped={skipped}
          setSkipped={setSkipped}
          onBack={firstRun ? () => setStep("folder") : undefined}
          onInstall={() => setStep("install")}
          onSkip={() => setStep("import")}
        />
      );
    case "install":
      return (
        <InstallPane
          nav={nav}
          runtimes={wanted}
          labels={Object.fromEntries(wanted.map((id) => [id, labelFor(catalog.data, id)]))}
          onNext={(count) => {
            setInstalled(count);
            next("install");
          }}
        />
      );
    case "import":
      return <ImportPane nav={nav} nextLabel={only ? "Done" : "Continue"} onImported={setImported} onNext={() => next("import")} />;
    case "name":
      return <NameStep nav={nav} initial={settings.developerName ?? ""} onBack={() => setStep("import")} onNext={() => next("name")} />;
    default:
      return <DoneStep nav={nav} wanted={wanted} picked={picked} installed={installed} imported={imported} onClose={close} />;
  }
}

type Nav = { steps: Step[]; current: Step; onSkip: () => void; skipLabel: string };

/**
 * The onboarding's frame: under the title bar (which keeps the window controls and drag
 * area), a rail of steps and the current step with its actions at the bottom. It's a modal
 * dialog to assistive tech and to the app's shortcuts, which pause while one is open.
 */
function Shell({
  nav,
  title,
  lead,
  children,
  footer,
  wide = false,
}: {
  nav: Nav;
  title: string;
  lead?: ReactNode;
  children: ReactNode;
  footer?: ReactNode;
  wide?: boolean;
}) {
  const body = useRef<HTMLDivElement>(null);
  const index = nav.steps.indexOf(nav.current);
  const single = nav.steps.length === 1;
  // Each step starts at the top with its main control focused.
  useEffect(() => {
    if (body.current) body.current.scrollTop = 0;
    const target = body.current?.querySelector<HTMLElement>("[data-autofocus]") ?? body.current?.querySelector<HTMLElement>("h1");
    target?.focus({ preventScroll: true });
  }, [nav.current]);
  // Only the import on its own (opened from Settings) closes with Escape: on the first run
  // it would throw away the setup by accident.
  const skip = useRef(nav.onSkip);
  skip.current = nav.onSkip;
  useEffect(() => {
    if (!single) return;
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && skip.current();
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [single]);

  return createPortal(
    <div role="dialog" aria-modal="true" aria-label={title} className="motion-backdrop fixed inset-x-0 top-12 bottom-0 z-40 flex bg-bg">
      {!single && (
        <aside className="hidden w-64 shrink-0 flex-col border-r border-line bg-surface px-5 py-6 md:flex">
          <div className="flex items-center gap-2.5">
            <Logo className="size-9" />
            <div className="leading-tight">
              <div className="font-bold tracking-tight">
                Lamp<span className="text-accent-text">Rabit</span>
              </div>
              <div className="text-xs text-muted">Setup</div>
            </div>
          </div>
          <ol className="mt-8 space-y-1" aria-label="Setup steps">
            {nav.steps.map((s, i) => {
              const state = i < index ? "done" : i === index ? "current" : "next";
              return (
                <li key={s} aria-current={state === "current" ? "step" : undefined} className={clsx("flex items-center gap-3 rounded-lg px-2 py-1.5 text-sm", state === "current" && "bg-accent-soft/60")}>
                  <span
                    className={clsx(
                      "flex size-6 shrink-0 items-center justify-center rounded-full text-xs font-semibold",
                      state === "done" && "bg-accent text-accent-fg",
                      state === "current" && "border-2 border-accent text-accent-text",
                      state === "next" && "border border-line text-muted",
                    )}
                  >
                    {state === "done" ? <Check size={13} strokeWidth={3} /> : i + 1}
                  </span>
                  <span className={clsx(state === "current" ? "font-semibold text-fg" : state === "done" ? "text-fg" : "text-muted")}>{stepLabels[s]}</span>
                </li>
              );
            })}
          </ol>
          <button type="button" onClick={nav.onSkip} className="mt-auto self-start text-xs text-muted hover:text-fg hover:underline">
            {nav.skipLabel}
          </button>
        </aside>
      )}
      <div className="flex min-w-0 flex-1 flex-col">
        {/* Narrow windows have no rail: a bar shows how far along the setup is. */}
        {!single && (
          <div className="flex items-center gap-3 border-b border-line bg-surface px-5 py-2.5 md:hidden">
            <span className="text-xs font-medium text-muted">
              Step {index + 1} of {nav.steps.length}
            </span>
            <div className="h-1.5 flex-1 overflow-hidden rounded-full bg-surface-2">
              <div className="h-full rounded-full bg-accent transition-[width]" style={{ width: `${((index + 1) / nav.steps.length) * 100}%` }} />
            </div>
            <button type="button" onClick={nav.onSkip} className="text-xs text-muted hover:text-fg">
              {nav.skipLabel}
            </button>
          </div>
        )}
        <div ref={body} className="min-h-0 flex-1 overflow-y-auto">
          <div key={nav.current} className={clsx("motion-page mx-auto px-6 py-8 sm:px-10 sm:py-10", wide ? "max-w-6xl" : "max-w-3xl")}>
            {single && (
              <div className="mb-2 flex justify-end">
                <button type="button" onClick={nav.onSkip} aria-label="Close" className="flex size-8 items-center justify-center rounded-lg text-muted hover:bg-surface-2 hover:text-fg">
                  <X size={17} />
                </button>
              </div>
            )}
            {!single && (
              <p className="text-xs font-semibold tracking-wider text-accent-text uppercase">
                Step {index + 1} of {nav.steps.length}
              </p>
            )}
            <h1 tabIndex={-1} className="mt-1 text-3xl font-extrabold tracking-tight outline-none sm:text-4xl">
              {title}
            </h1>
            {lead && <div className="mt-3 max-w-2xl text-[15px] leading-relaxed text-muted">{lead}</div>}
            <div className="mt-8">{children}</div>
          </div>
        </div>
        {footer && (
          <div className="border-t border-line bg-surface/90 backdrop-blur">
            <div className={clsx("mx-auto flex flex-wrap items-center gap-3 px-6 py-4 sm:px-10", wide ? "max-w-6xl" : "max-w-3xl")}>{footer}</div>
          </div>
        )}
      </div>
    </div>,
    document.body,
  );
}

function BackButton({ onClick }: { onClick?: () => void }) {
  if (!onClick) return null;
  return (
    <Button variant="ghost" size="lg" onClick={onClick}>
      <ArrowLeft size={16} /> Back
    </Button>
  );
}

function WelcomeStep({ nav, onNext }: { nav: Nav; onNext: () => void }) {
  const points = [
    { icon: Globe, title: "Every folder is a site", text: "Put a project in www and it's served at https://<folder>.test, with a trusted certificate." },
    { icon: Layers, title: "Versions side by side", text: "Several PHP, Node.js, Python or database versions; each project picks its own." },
    { icon: FolderInput, title: "Bring your projects", text: "Copy them over from Laragon, XAMPP, Herd, MAMP and others. The originals stay put." },
  ];
  return (
    <Shell
      nav={nav}
      title="Welcome to LampRabit"
      lead="Your local web development stack: web servers, languages, databases and tools, all in one app. Setting it up takes a few minutes."
      footer={
        <>
          <span className="mr-auto text-xs text-muted">Nothing is downloaded until you choose.</span>
          <Button variant="primary" size="lg" onClick={onNext} data-autofocus>
            Get started <ArrowRight size={16} />
          </Button>
        </>
      }
    >
      <div className="motion-stagger grid gap-3 sm:grid-cols-3">
        {points.map((p) => (
          <div key={p.title} className="rounded-2xl border border-line bg-surface p-5 shadow-card">
            <span className="flex size-10 items-center justify-center rounded-xl bg-accent-soft text-accent-text">
              <p.icon size={20} aria-hidden />
            </span>
            <h2 className="mt-4 font-semibold">{p.title}</h2>
            <p className="mt-1 text-sm text-muted">{p.text}</p>
          </div>
        ))}
      </div>
    </Shell>
  );
}

function FolderStep({ nav, root, busy, onBack, onNext }: { nav: Nav; root: string; busy: boolean; onBack: () => void; onNext: () => void }) {
  const [moving, setMoving] = useState(false);
  const queryClient = useQueryClient();
  const inside = [
    { name: "bin", text: "The runtimes LampRabit downloads" },
    { name: "www", text: "Your projects, one folder each" },
    { name: "data", text: "Database files" },
  ];
  return (
    <Shell
      nav={nav}
      title="Where should LampRabit keep its files?"
      lead="Everything goes in one folder. It can grow to several gigabytes, so pick a drive with room. You can move it later in Settings → General."
      footer={
        <>
          <BackButton onClick={onBack} />
          <span className="mr-auto" />
          <Button size="lg" onClick={() => setMoving(true)}>
            <FolderInput size={16} /> Choose another folder…
          </Button>
          <Button variant="primary" size="lg" onClick={onNext} disabled={busy} data-autofocus>
            Use this folder <ArrowRight size={16} />
          </Button>
        </>
      }
    >
      {moving && (
        <MoveEnvironmentDialog
          current={root}
          onClose={() => {
            setMoving(false);
            // A finished move changed the folder; show where it is now.
            queryClient.invalidateQueries({ queryKey: ["settings"] });
          }}
        />
      )}
      <div className="rounded-2xl border border-line bg-surface p-5 shadow-card">
        <div className="flex items-center gap-3">
          <span className="flex size-10 shrink-0 items-center justify-center rounded-xl bg-accent-soft text-accent-text">
            <HardDrive size={20} aria-hidden />
          </span>
          <p className="min-w-0 font-mono text-[15px] break-all text-fg">{root}</p>
        </div>
        <ul className="mt-5 grid gap-2 sm:grid-cols-3">
          {inside.map((f) => (
            <li key={f.name} className="rounded-xl bg-surface-2/60 px-3 py-2.5">
              <span className="flex items-center gap-1.5 font-mono text-sm font-semibold text-fg">
                <FolderOpen size={14} className="text-muted" aria-hidden /> {f.name}
              </span>
              <span className="mt-0.5 block text-xs text-muted">{f.text}</span>
            </li>
          ))}
        </ul>
      </div>
    </Shell>
  );
}

function StackStep({
  nav,
  picked,
  setPicked,
  extra,
  setExtra,
  chosen,
  skipped,
  setSkipped,
  onBack,
  onInstall,
  onSkip,
}: {
  nav: Nav;
  picked: UsageId[];
  setPicked: (p: UsageId[]) => void;
  extra: string[];
  setExtra: (e: string[]) => void;
  chosen: string[];
  skipped: Set<string>;
  setSkipped: (s: Set<string>) => void;
  onBack?: () => void;
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
  const summary = loading ? "Checking downloads…" : toInstall.length === 0 ? "Everything is installed" : `${toInstall.length} to install${total ? ` · about ${formatBytes(total)}` : ""}`;

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
    <Shell
      nav={nav}
      wide
      title="What do you build?"
      lead="Pick everything you work on. Each brings the services it needs; you can untick any of them, or add others."
      footer={
        <>
          <BackButton onClick={onBack} />
          <Button variant="ghost" size="lg" onClick={onSkip}>
            Skip, I'll use Services
          </Button>
          <span className="mr-auto" />
          <span className="text-sm text-muted" aria-live="polite">
            {summary}
          </span>
          <Button variant="primary" size="lg" onClick={onInstall} disabled={loading || chosen.every((id) => skipped.has(id))}>
            {toInstall.length === 0 ? "Continue" : "Install"} <ArrowRight size={16} />
          </Button>
        </>
      }
    >
      <div className="grid gap-8 lg:grid-cols-[minmax(0,1fr)_22rem]">
        <div className="grid content-start gap-3 sm:grid-cols-2" role="group" aria-label="What you build">
          {usages.map((u, i) => {
            const on = picked.includes(u.id);
            return (
              <button
                key={u.id}
                type="button"
                aria-pressed={on}
                data-autofocus={i === 0 ? true : undefined}
                onClick={() => setPicked(on ? picked.filter((p) => p !== u.id) : [...picked, u.id])}
                className={clsx(
                  "motion-press relative flex items-start gap-3.5 rounded-2xl border bg-surface p-4 text-left shadow-card transition-colors",
                  on ? "border-accent ring-2 ring-accent/25" : "border-line hover:border-accent/50",
                )}
              >
                <span className={clsx("flex size-11 shrink-0 items-center justify-center rounded-xl transition-colors", on ? "bg-accent text-accent-fg" : "bg-accent-soft text-accent-text")}>
                  <u.icon size={21} aria-hidden />
                </span>
                <span className="min-w-0 flex-1 pr-6">
                  <span className="block font-semibold text-fg">{u.title}</span>
                  <span className="mt-0.5 block text-sm text-muted">{u.blurb}</span>
                </span>
                <span
                  className={clsx("absolute top-3.5 right-3.5 flex size-5 items-center justify-center rounded-full border transition-colors", on ? "border-accent bg-accent text-accent-fg" : "border-line")}
                  aria-hidden
                >
                  {on && <Check size={12} strokeWidth={3} />}
                </span>
              </button>
            );
          })}
        </div>

        <aside className="min-w-0 self-start rounded-2xl border border-line bg-surface shadow-card lg:sticky lg:top-0">
          <div className="flex items-baseline justify-between border-b border-line px-4 py-3">
            <h2 className="font-semibold">Your stack</h2>
            <span className="text-xs text-muted">{chosen.length - [...skipped].filter((s) => chosen.includes(s)).length} components</span>
          </div>
          <div className="space-y-4 px-4 py-3">
            {failed && <p className="text-xs text-danger-text">Couldn't check some downloads: {String(failed.error)}</p>}
            {chosen.length === 0 && <p className="py-4 text-center text-sm text-muted">Pick what you build, or add components below.</p>}
            {sections
              .filter((s) => s.rows.length > 0)
              .map((s) => (
                <section key={s.id} aria-label={s.title}>
                  <h3 className="mb-1 text-[11px] font-semibold tracking-wider text-muted uppercase">{s.title}</h3>
                  <ul className="space-y-0.5">
                    {s.rows.map((id) => {
                      const off = skipped.has(id);
                      const item = byId.get(id);
                      const { note, tone } = describe(id, item, node.data, id !== "node" && !item && loading);
                      return (
                        <li key={id} className={clsx("group flex items-center gap-2.5 rounded-lg px-1.5 py-1 hover:bg-surface-2/60", off && "opacity-50")}>
                          <input type="checkbox" checked={!off} onChange={() => toggleSkip(id)} aria-label={`Include ${label(id)}`} className="size-4 shrink-0 accent-[var(--accent)]" />
                          <div className="min-w-0 flex-1">
                            <div className="text-sm font-medium text-fg">{label(id)}</div>
                            <div className={clsx("truncate text-xs", tone)} title={note}>
                              {note}
                            </div>
                          </div>
                          {extra.includes(id) && (
                            <button type="button" aria-label={`Remove ${label(id)}`} className="text-muted opacity-0 group-hover:opacity-100 hover:text-fg focus:opacity-100" onClick={() => setExtra(extra.filter((e) => e !== id))}>
                              <X size={14} />
                            </button>
                          )}
                        </li>
                      );
                    })}
                  </ul>
                </section>
              ))}
            {blocked && <p className="text-xs text-warn">Components that can't be installed on this computer are skipped; their note says why.</p>}
          </div>
          {sections.some((s) => s.more.length > 0) && (
            <details className="group border-t border-line">
              <summary className="flex cursor-pointer list-none items-center gap-2 px-4 py-3 text-sm font-medium text-accent-text [&::-webkit-details-marker]:hidden">
                <Plus size={15} aria-hidden /> Add more services
                <ChevronDown size={15} className="ml-auto transition-transform group-open:rotate-180" aria-hidden />
              </summary>
              <section aria-label="Add more" className="space-y-2.5 px-4 pb-4">
                {sections
                  .filter((s) => s.more.length > 0)
                  .map((s) => (
                    <div key={s.id}>
                      <div className="mb-1 text-[11px] font-semibold tracking-wider text-muted uppercase">{s.title}</div>
                      <div className="flex flex-wrap gap-1.5">
                        {s.more.map((id) => (
                          <button
                            key={id}
                            type="button"
                            onClick={() => {
                              setExtra([...extra, id]);
                              if (skipped.has(id)) toggleSkip(id);
                            }}
                            className="inline-flex items-center gap-1 rounded-full border border-line px-2.5 py-1 text-xs text-muted hover:border-accent hover:text-fg"
                          >
                            <Plus size={11} aria-hidden /> {label(id)}
                          </button>
                        ))}
                      </div>
                    </div>
                  ))}
              </section>
            </details>
          )}
        </aside>
      </div>
    </Shell>
  );
}

function InstallPane({ nav, runtimes, labels, onNext }: { nav: Nav; runtimes: string[]; labels: Record<string, string>; onNext: (installed: number) => void }) {
  const installer = useInstaller(runtimes, labels);
  const { finished, failed, done, total } = installer;
  return (
    <Shell
      nav={nav}
      title={!finished ? "Installing your stack" : failed ? "Almost there" : "Your stack is installed"}
      lead={
        !finished
          ? "Each component downloads, is checked and unpacked, one at a time. You can hide this and keep using LampRabit; the installs carry on."
          : failed
            ? "Some components didn't install. Retry them, or carry on and install them later from Services."
            : "Everything you picked is ready to use."
      }
      footer={
        <>
          <span className="mr-auto text-sm text-muted">
            {done} of {total} done
          </span>
          {!finished && (
            <Button variant="ghost" size="lg" onClick={nav.onSkip}>
              Hide
            </Button>
          )}
          <Button variant="primary" size="lg" onClick={() => onNext(done)} disabled={!finished} data-autofocus>
            Continue <ArrowRight size={16} />
          </Button>
        </>
      }
    >
      <div className="mb-4 h-2 overflow-hidden rounded-full bg-surface-2" role="progressbar" aria-valuemin={0} aria-valuemax={total} aria-valuenow={done} aria-label="Installed">
        <div className="h-full rounded-full bg-accent transition-[width] duration-500" style={{ width: `${total ? (done / total) * 100 : 100}%` }} />
      </div>
      <InstallList installer={installer} className="rounded-2xl border border-line bg-surface shadow-card" />
    </Shell>
  );
}

function NameStep({ nav, initial, onBack, onNext }: { nav: Nav; initial: string; onBack: () => void; onNext: () => void }) {
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
  const hour = new Date().getHours();
  const greeting = hour < 5 || hour >= 17 ? "Good evening" : hour < 12 ? "Good morning" : "Good afternoon";
  return (
    <Shell
      nav={nav}
      title="Last thing: what should we call you?"
      lead="Home greets you by name. It stays on this computer, and you can change it in Settings → General."
      footer={
        <>
          <BackButton onClick={onBack} />
          <span className="mr-auto" />
          <Button variant="ghost" size="lg" onClick={onNext}>
            Skip
          </Button>
          <Button variant="primary" size="lg" onClick={() => save.mutate()} disabled={save.isPending || !name.trim()}>
            Continue <ArrowRight size={16} />
          </Button>
        </>
      }
    >
      <form
        className="max-w-md"
        onSubmit={(e) => {
          e.preventDefault();
          if (name.trim()) save.mutate();
        }}
      >
        <label htmlFor="developer-name" className="text-sm font-medium text-fg">
          Your name
        </label>
        <input
          id="developer-name"
          data-autofocus
          value={name}
          maxLength={40}
          autoComplete="given-name"
          placeholder="Your name"
          onChange={(e) => setName(e.target.value)}
          className="mt-1.5 w-full rounded-xl border border-line bg-surface px-4 py-3 text-lg text-fg shadow-card focus:border-accent focus:outline-none"
        />
      </form>
      <div className="mt-8 rounded-2xl border border-dashed border-line p-6" aria-hidden>
        <div className="text-[11px] font-semibold tracking-wider text-muted uppercase">On Home</div>
        <div className="mt-2 flex items-center gap-3 text-3xl font-extrabold tracking-tight">
          {greeting}, {name.trim() || "developer"}
          <Sun size={26} strokeWidth={1.75} className="text-accent-text" />
        </div>
      </div>
    </Shell>
  );
}

function DoneStep({ nav, wanted, picked, installed, imported, onClose }: { nav: Nav; wanted: string[]; picked: UsageId[]; installed: number; imported: number; onClose: () => void }) {
  const queryClient = useQueryClient();
  const notify = useAppStore((s) => s.notify);
  const setNewProjectOpen = useAppStore((s) => s.setNewProjectOpen);
  const settings = useQuery({ queryKey: ["settings"], queryFn: api.getSettings });
  const name = settings.data?.developerName?.trim();
  // The components that run as services (not PHP, Composer or Node), to start in one go.
  const startable = wanted.filter((r) => r in serviceLabels) as ServiceKind[];
  const startNow = useMutation({
    mutationFn: () => api.applyServices(startable, []),
    onSuccess: (problems) => {
      for (const p of problems) notify("error", p);
      queryClient.invalidateQueries({ queryKey: ["services"] });
    },
    onError: (e) => notify("error", String(e)),
  });
  const examples = picked.includes("wordpress") ? "WordPress" : picked.includes("laravel") ? "Laravel" : picked.includes("javascript") ? "Next.js, Vite, Express" : "Laravel, Symfony, WordPress";
  const facts = [
    installed > 0 && { icon: Database, text: `${installed} ${installed === 1 ? "component" : "components"} installed` },
    imported > 0 && { icon: FolderInput, text: `${imported} ${imported === 1 ? "project" : "projects"} imported` },
    { icon: Globe, text: "Sites at https://<folder>.test" },
  ].filter(Boolean) as { icon: typeof Globe; text: string }[];
  return (
    <Shell
      nav={nav}
      title={name ? `You're all set, ${name}` : "You're all set"}
      lead={`Create a project from a template (${examples}…), with its database made for you, or open Home to start and stop services.`}
      footer={
        <>
          <span className="mr-auto text-xs text-muted">This guide is in Settings → General whenever you want more services or to import projects.</span>
          <Button size="lg" onClick={onClose}>
            Go to Home
          </Button>
          <Button
            variant="primary"
            size="lg"
            data-autofocus
            onClick={() => {
              onClose();
              setNewProjectOpen(true);
            }}
          >
            <Sparkles size={16} /> Create a project
          </Button>
        </>
      }
    >
      <div className="flex flex-wrap gap-2">
        {facts.map((f) => (
          <span key={f.text} className="inline-flex items-center gap-2 rounded-full border border-line bg-surface px-3 py-1.5 text-sm text-fg shadow-card">
            <f.icon size={15} className="text-accent-text" aria-hidden /> {f.text}
          </span>
        ))}
      </div>
      {startable.length > 0 && (
        <div className="mt-6 flex flex-wrap items-center gap-4 rounded-2xl border border-line bg-surface p-5 shadow-card">
          <span className="flex size-10 shrink-0 items-center justify-center rounded-xl bg-accent-soft text-accent-text">
            {startNow.isSuccess ? <CheckCircle2 size={20} /> : <Play size={18} />}
          </span>
          <div className="min-w-0 flex-1">
            <div className="font-semibold">{startNow.isSuccess ? "Started" : "Start your services now?"}</div>
            <div className="text-sm text-muted">{startNow.isSuccess ? "Their status is on Home." : startable.map((k) => serviceLabels[k]).join(", ")}</div>
          </div>
          {!startNow.isSuccess && (
            <Button variant="primary" onClick={() => startNow.mutate()} disabled={startNow.isPending}>
              <Play size={13} className="fill-current" /> {startNow.isPending ? "Starting…" : "Start them"}
            </Button>
          )}
        </div>
      )}
    </Shell>
  );
}

type ImportState = { status: "working" | "done" | "error"; files?: number; error?: string; copied?: ImportedProject };
const validName = /^[A-Za-z0-9][A-Za-z0-9_-]{0,62}$/;

/**
 * Projects from other local environments, copied into www/ (the originals stay, so the other
 * tool keeps working). Tools LampRabit doesn't look for are covered by "Choose a folder".
 */
function ImportPane({ nav, nextLabel, onImported, onNext }: { nav: Nav; nextLabel: string; onImported: (count: number) => void; onNext: () => void }) {
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
    let count = done.length;
    for (const p of chosen) {
      setStates((s) => ({ ...s, [p.path]: { status: "working" } }));
      try {
        const copied = await api.importProject(p.path, nameOf(p.path));
        setStates((s) => ({ ...s, [p.path]: { status: "done", copied } }));
        count += 1;
      } catch (e) {
        setStates((s) => ({ ...s, [p.path]: { status: "error", error: String(e) } }));
      }
    }
    setRunning(false);
    onImported(count);
    queryClient.invalidateQueries({ queryKey: ["sites"] });
  };

  const skippedDeps = done.some((s) => s.copied?.skipped.some((k) => k === "vendor" || k === "node_modules"));
  return (
    <Shell
      nav={nav}
      title="Bring your projects over"
      lead={
        <>
          Projects from Laragon, XAMPP, AMPPS, MAMP, Herd, Valet, FlyEnv, EnvKit, WampServer, ServBay or Local are copied into <span className="font-mono text-fg">www</span>, where each gets
          its .test domain. The originals stay where they are, so the other tool keeps working.
        </>
      }
      footer={
        <>
          <Button size="lg" onClick={chooseFolder} disabled={running}>
            <FolderOpen size={16} /> Choose a folder…
          </Button>
          <span className="mr-auto" />
          {chosen.length > 0 ? (
            <Button variant="primary" size="lg" onClick={run} disabled={running || invalid}>
              {running ? "Copying…" : `Copy ${chosen.length} into www`}
            </Button>
          ) : (
            <Button variant="primary" size="lg" onClick={onNext} disabled={running}>
              {done.length ? nextLabel : nav.steps.length === 1 ? "Close" : "Skip"} <ArrowRight size={16} />
            </Button>
          )}
        </>
      }
    >
      <div className="space-y-4 text-sm">
        {found.isLoading && (
          <p className="flex items-center gap-2 text-muted">
            <Loader2 size={14} className="animate-spin" /> Looking for other tools…
          </p>
        )}
        {found.isError && <p className="text-xs text-danger-text">Couldn't look for other tools: {String(found.error)}</p>}
        {found.isSuccess && sources.length === 0 && (
          <div className="rounded-2xl border border-dashed border-line px-6 py-10 text-center">
            <FolderInput size={28} className="mx-auto text-muted" aria-hidden />
            <p className="mt-3 font-medium text-fg">No other tool's projects were found on this computer</p>
            <p className="mt-1 text-muted">If yours are somewhere else (EnvKit, a Sites folder…), choose the folder that holds them. Or skip this; you can import later from Settings.</p>
          </div>
        )}
        {sources.map((s) => {
          const all = s.projects.every((p) => selected.has(p.path) || states[p.path]?.status === "done");
          return (
            <section key={`${s.id}:${s.folders[0] ?? ""}`} aria-label={s.name} className="overflow-hidden rounded-2xl border border-line bg-surface shadow-card">
              <header className="flex flex-wrap items-center gap-2 border-b border-line px-4 py-3">
                <span className="font-semibold text-fg">{s.name}</span>
                <span className="min-w-0 flex-1 truncate font-mono text-xs text-muted" title={s.folders.join("\n")}>
                  {s.folders.join(" · ")}
                </span>
                {s.projects.length > 0 && (
                  <button
                    type="button"
                    className="text-xs font-medium text-accent-text hover:underline"
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
                    <li key={p.path} className="flex flex-wrap items-center gap-x-3 gap-y-1 px-4 py-2.5">
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
                <p className="border-t border-line bg-surface-2/40 px-4 py-2.5 text-xs text-muted">
                  Its databases stay in {s.name}: <span className="text-fg">{s.databases.join(", ")}</span>. Export them there (phpMyAdmin → Export → SQL), then import the file with LampRabit's phpMyAdmin
                  or Adminer.
                </p>
              )}
            </section>
          );
        })}
        {done.length > 0 && (
          <p className="rounded-xl bg-accent-soft/50 px-4 py-3 text-sm text-fg">
            Copied {done.length} {done.length === 1 ? "project" : "projects"} into www.
            {skippedDeps && <span className="text-muted"> Dependency folders (vendor, node_modules) weren't copied: each project's Overview offers to install them.</span>}
          </p>
        )}
      </div>
    </Shell>
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

type ItemState = { status: "waiting" | "working" | "done" | "error"; error?: string };
type Installer = ReturnType<typeof useInstaller>;

/**
 * Installs the components one by one; a failed one can be retried. Downloads share the
 * connection, and later ones (Composer, phpMyAdmin) are better off once PHP is there. Runs
 * once for the list it started with; installs carry on in the backend if it's hidden.
 */
function useInstaller(given: string[], knownLabels?: Record<string, string>, chosen?: Record<string, Release>) {
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
  return {
    runtimes,
    states,
    labels,
    current,
    progress: current?.key ? installs[current.key] : undefined,
    finished: values.every((s) => s.status === "done" || s.status === "error"),
    failed: values.filter((s) => s.status === "error").length,
    done: values.filter((s) => s.status === "done").length,
    total: values.length,
    retry: installOne,
  };
}

function InstallList({ installer, className }: { installer: Installer; className?: string }) {
  const { runtimes, states, labels, current, progress, finished, retry } = installer;
  return (
    <ul className={clsx("divide-y divide-line", className ?? "rounded-xl border border-line")}>
      {runtimes.map((id) => {
        const st = states[id];
        const mine = st.status === "working" && current?.id === id;
        return (
          <li key={id} className="px-4 py-2.5">
            <div className="flex items-center gap-3">
              {st.status === "done" ? (
                <CheckCircle2 size={17} className="shrink-0 text-ok" />
              ) : st.status === "error" ? (
                <AlertCircle size={17} className="shrink-0 text-danger-text" />
              ) : st.status === "working" ? (
                <Loader2 size={17} className="shrink-0 animate-spin text-accent-text" />
              ) : (
                <Circle size={17} className="shrink-0 text-muted" />
              )}
              <span className="flex-1 text-sm font-medium text-fg">{labels[id] ?? id}</span>
              {mine && progress && (
                <span className="text-xs text-muted tabular-nums">
                  {progress.stage === "downloading" ? `${formatBytes(progress.downloaded)}${progress.total ? ` of ${formatBytes(progress.total)}` : ""}` : progress.stage}
                </span>
              )}
              {st.status === "error" && finished && (
                <Button size="sm" onClick={() => retry(id)}>
                  <RotateCw size={12} /> Retry
                </Button>
              )}
            </div>
            {mine && progress?.total ? (
              <div className="mt-1.5 ml-[29px] h-1 overflow-hidden rounded-full bg-surface-2">
                <div className="h-full bg-accent transition-[width]" style={{ width: `${Math.min(100, (progress.downloaded / progress.total) * 100)}%` }} />
              </div>
            ) : null}
            {st.error && <p className="mt-1 ml-[29px] text-xs break-words text-danger-text">{st.error}</p>}
          </li>
        );
      })}
    </ul>
  );
}

/**
 * The installs as a dialog, for a profile import's services (a service's id is its
 * runtime's), with exact releases when the profile carried versions.
 */
export function InstallStep({
  runtimes,
  onClose,
  onNext,
  title = "Installing",
  nextLabel = "Continue",
  labels,
  releases,
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
  const installer = useInstaller(runtimes, labels, releases);
  const { finished, failed } = installer;
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
      <InstallList installer={installer} />
    </Dialog>
  );
}

/** A ref that always holds the latest value, for effects that must run once. */
function useRefLatest<T>(value: T) {
  const [ref] = useState(() => ({ current: value }));
  ref.current = value;
  return ref;
}
