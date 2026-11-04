import { useMemo, useState, type ReactNode } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { openPath, openUrl } from "@tauri-apps/plugin-opener";
import {
  AlertTriangle,
  ArrowRight,
  Check,
  Code2,
  Copy,
  Database,
  ExternalLink,
  FolderOpen,
  FolderPlus,
  GitBranch,
  Globe,
  Lock,
  LockOpen,
  Package,
  RefreshCw,
  Search,
  ShieldCheck,
  SquareTerminal,
  X,
  type LucideIcon,
} from "lucide-react";
import clsx from "clsx";
import { api, runtimeLabels, serviceLabels, type AppStatus, type Language, type ProjectFacts, type Server, type Settings, type Site, type SitesInfo } from "../lib/api";
import { useAppStore } from "../lib/store";
import ProjectDomains from "../components/ProjectDomains";
import ProjectProcesses from "../components/ProjectProcesses";
import { toolUrl } from "../lib/tools";
import { formatAgo, useNow } from "../lib/time";
import { Button, Card, PageHeader, Switch, inputClass } from "../components/ui";
import { ShareButton, TunnelRow, useProjectTunnel } from "../components/Tunnels";
import { AppControls, AppState, CommandsPanel, OutputPanel, Row, ServeSettings, appCommand, isActive, useAppActions, useAppInfo, useAppStatuses } from "../components/ProjectRun";

/** Avatar colors per framework; anything else gets the accent. */
const frameworkStyle: Record<string, string> = {
  Laravel: "bg-red-500/12 text-red-600 dark:text-red-400",
  WordPress: "bg-sky-600/12 text-sky-700 dark:text-sky-400",
  Symfony: "bg-zinc-500/15 text-fg",
  Drupal: "bg-blue-500/12 text-blue-600 dark:text-blue-400",
  CodeIgniter: "bg-orange-500/12 text-orange-600 dark:text-orange-400",
  CakePHP: "bg-rose-500/12 text-rose-600 dark:text-rose-400",
  Yii: "bg-emerald-500/12 text-emerald-700 dark:text-emerald-400",
  Slim: "bg-teal-500/12 text-teal-700 dark:text-teal-400",
  Statamic: "bg-pink-500/12 text-pink-600 dark:text-pink-400",
  Magento: "bg-orange-600/12 text-orange-700 dark:text-orange-400",
  PHP: "bg-indigo-500/12 text-indigo-600 dark:text-indigo-400",
  Vite: "bg-violet-500/12 text-violet-600 dark:text-violet-400",
  "Next.js": "bg-zinc-500/15 text-fg",
  Nuxt: "bg-emerald-500/12 text-emerald-700 dark:text-emerald-400",
  Astro: "bg-orange-500/12 text-orange-600 dark:text-orange-400",
  SvelteKit: "bg-orange-600/12 text-orange-700 dark:text-orange-400",
  NestJS: "bg-rose-500/12 text-rose-600 dark:text-rose-400",
  Remix: "bg-zinc-500/15 text-fg",
  Express: "bg-zinc-500/15 text-fg",
  Node: "bg-green-600/12 text-green-700 dark:text-green-400",
  Static: "bg-surface-2 text-muted",
};

const languageLabel: Record<Language, string> = { php: "PHP", node: "Node.js", java: "Java", python: "Python", go: "Go", rust: "Rust", static: "Static" };

type Sort = "recent" | "name";
type Tab = "overview" | "commands" | "domains" | "processes" | "settings";

export default function Sites() {
  const queryClient = useQueryClient();
  const notify = useAppStore((s) => s.notify);
  const openNewProject = useAppStore((s) => s.setNewProjectOpen);
  const onError = (e: unknown) => notify("error", String(e));
  const info = useQuery({ queryKey: ["sites"], queryFn: api.listSites, refetchInterval: 10_000 });
  const paths = useQuery({ queryKey: ["paths"], queryFn: api.envPaths });
  const statuses = useAppStatuses();
  const refresh = () => queryClient.invalidateQueries({ queryKey: ["sites"] });
  const [query, setQuery] = useState("");
  const [language, setLanguage] = useState<Language | null>(null);
  const [sort, setSort] = useState<Sort>("recent");
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [tab, setTab] = useState<Tab>("overview");

  const apply = useMutation({ mutationFn: api.applySites, onSuccess: refresh, onError });
  const trust = useMutation({
    mutationFn: api.trustCa,
    onSuccess: (note) => {
      notify(note ? "error" : "info", note ?? "Certificate trusted. Restart your browser if it still shows a warning.");
      refresh();
    },
    onError,
  });

  const data = info.data;
  const os = paths.data?.os;
  const all = useMemo(() => data?.sites ?? [], [data]);
  const languages = useMemo(() => (["php", "node", "static"] as Language[]).filter((l) => all.some((s) => s.language === l)), [all]);
  const q = query.trim().toLowerCase();
  const shown = all
    .filter((s) => !language || s.language === language)
    .filter((s) => !q || [s.name, s.domain, s.framework ?? "", s.gitBranch ?? ""].some((v) => v.toLowerCase().includes(q)))
    .sort((a, b) => (sort === "name" ? a.name.localeCompare(b.name) : b.modified - a.modified));
  const selectedSite = shown.find((site) => site.path === selectedPath) ?? shown[0];
  const running = all.filter((s) => isActive(statuses[s.name])).length;

  return (
    <>
      <PageHeader
        title="Sites"
        subtitle={
          <>
            Every folder in <span className="font-mono">www</span> is served at{" "}
            <span className="font-mono">&lt;folder&gt;.{data?.tld ?? "test"}</span>, sub-domains included.
          </>
        }
        actions={
          <>
            <Button onClick={() => paths.data && openPath(paths.data.www).catch(onError)}>
              <FolderOpen size={16} /> Open www
            </Button>
            <Button variant="primary" onClick={() => openNewProject(true)} title="New project (N)">
              <FolderPlus size={16} /> New project
            </Button>
            <Button onClick={() => info.refetch()} disabled={info.isFetching} aria-label="Refresh sites">
              <RefreshCw size={15} className={clsx(info.isFetching && "animate-spin")} />
            </Button>
          </>
        }
      />

      <div className="mb-5 space-y-3 empty:mb-0">
        {data?.needsApply && (
          <Banner
            icon={RefreshCw}
            tone="info"
            text="Your sites changed since the web server started. Apply to update domains, certificates and the hosts file."
            action={
              <Button variant="primary" onClick={() => apply.mutate()} disabled={apply.isPending}>
                {apply.isPending ? "Applying…" : "Apply changes"}
              </Button>
            }
          />
        )}
        {data && !data.hostsOk && !data.needsApply && data.sites.length > 0 && (
          <Banner
            icon={AlertTriangle}
            tone="warn"
            text={
              data.manageHosts
                ? "Some domains are not in the hosts file yet. This happens automatically when the web server starts, or update it now (needs administrator approval)."
                : "Some domains are not in the hosts file, and automatic updates are off in Settings."
            }
            action={
              <Button onClick={() => apply.mutate()} disabled={apply.isPending}>
                Update hosts file
              </Button>
            }
          />
        )}
        {data && data.unserved.length > 0 && (
          <Banner
            icon={AlertTriangle}
            tone="warn"
            text={
              <>
                {data.unserved.length === 1 ? "One folder in www isn't served:" : `${data.unserved.length} folders in www aren't served:`}
                <ul className="mt-1 space-y-0.5">
                  {data.unserved.map((u) => (
                    <li key={u.name}>
                      <span className="font-mono font-medium">{u.name}</span> <span className="text-muted">{u.reason}</span>
                    </li>
                  ))}
                </ul>
              </>
            }
          />
        )}
        {data && data.shadowedTools.length > 0 && (
          <Banner
            icon={AlertTriangle}
            tone="warn"
            text={`A folder in www uses ${data.shadowedTools.join(" and ")}, so that address shows your folder instead of the built-in tool. Rename the folder to get the tool back.`}
          />
        )}
        {data?.caTrusted === false && (
          <Banner
            icon={ShieldCheck}
            tone="warn"
            text={
              <>
                Browsers will warn on HTTPS sites until you trust LampRabit's local certificate authority.
                {os === "windows" && " Windows asks you to confirm once."}
                {os === "linux" && " Linux asks for your password once; Chrome and Firefox are covered too."}{" "}
                {os !== "linux" && <span className="text-muted">Firefox uses its own certificate list; enable "security.enterprise_roots.enabled" there.</span>}
              </>
            }
            action={
              <Button variant="primary" onClick={() => trust.mutate()} disabled={trust.isPending}>
                <ShieldCheck size={16} /> Trust certificate
              </Button>
            }
          />
        )}
      </div>

      {all.length === 0 ? (
        data && (
          <Card className="flex flex-col items-center gap-3 px-6 py-14 text-center">
            <div className="flex size-14 items-center justify-center rounded-full bg-accent-soft text-accent-text">
              <Globe size={26} />
            </div>
            <div className="text-lg font-semibold">No sites yet</div>
            <p className="max-w-md text-muted">
              Create a folder inside <span className="font-mono">www</span> with an <span className="font-mono">index.php</span> or a package.json, or start from a
              template: Laravel, WordPress, Next.js, Vite, Nuxt and more.
            </p>
            <Button variant="primary" onClick={() => openNewProject(true)}>
              <FolderPlus size={16} /> New project
            </Button>
          </Card>
        )
      ) : (
        <div className="grid items-start gap-5 lg:grid-cols-[minmax(240px,290px)_minmax(0,1fr)]">
          <Card className="min-w-0 overflow-hidden lg:sticky lg:top-4">
            <div className="space-y-2.5 border-b border-line p-3">
              <div className="flex items-center justify-between px-1">
                <h2 className="text-sm font-bold">
                  Projects <span className="font-normal text-muted tabular-nums">{shown.length === all.length ? all.length : `${shown.length} of ${all.length}`}</span>
                </h2>
                <span className="flex items-center gap-2 text-xs text-muted">
                  {running > 0 && (
                    <span className="flex items-center gap-1" title="Projects with a running dev server">
                      <span className="size-1.5 rounded-full bg-ok" aria-hidden /> {running} running
                    </span>
                  )}
                  <select value={sort} onChange={(e) => setSort(e.target.value as Sort)} aria-label="Sort projects" className="rounded-md bg-transparent py-0.5 text-xs text-muted hover:text-fg">
                    <option value="recent">Recent</option>
                    <option value="name">A–Z</option>
                  </select>
                </span>
              </div>
              <label className="flex h-9 items-center gap-2 rounded-lg border border-line bg-surface-2/60 px-2.5 focus-within:border-accent">
                <Search size={14} className="shrink-0 text-muted" aria-hidden />
                <input value={query} onChange={(e) => setQuery(e.target.value)} placeholder="Search name, framework, branch" aria-label="Search projects" className="w-full min-w-0 bg-transparent text-sm outline-none placeholder:text-muted" />
                {query && (
                  <button type="button" onClick={() => setQuery("")} aria-label="Clear search" className="text-muted hover:text-fg">
                    <X size={13} />
                  </button>
                )}
              </label>
              {languages.length > 1 && (
                <div className="flex flex-wrap gap-1" role="radiogroup" aria-label="Language">
                  {[null, ...languages].map((l) => (
                    <button
                      key={l ?? "all"}
                      role="radio"
                      aria-checked={language === l}
                      onClick={() => setLanguage(l)}
                      className={clsx("rounded-md px-2 py-0.5 text-xs font-medium transition-colors", language === l ? "bg-accent-soft text-accent-text" : "text-muted hover:bg-surface-2 hover:text-fg")}
                    >
                      {l ? languageLabel[l] : "All"}
                    </button>
                  ))}
                </div>
              )}
            </div>
            {shown.length === 0 ? (
              <p className="px-4 py-8 text-center text-sm text-muted">No projects match.</p>
            ) : (
              <ul className="scrollbar-thin space-y-0.5 p-1.5 lg:max-h-[calc(100dvh-16rem)] lg:overflow-y-auto lg:overscroll-contain" aria-label="Projects">
                {shown.map((site) => (
                  <li key={site.path}>
                    <ProjectRow site={site} status={statuses[site.name]} selected={selectedSite?.path === site.path} onSelect={() => setSelectedPath(site.path)} />
                  </li>
                ))}
              </ul>
            )}
          </Card>
          <div className="min-w-0">{selectedSite ? <ProjectDetail key={selectedSite.path} site={selectedSite} info={data} status={statuses[selectedSite.name]} tab={tab} setTab={setTab} /> : null}</div>
        </div>
      )}
    </>
  );
}

function Banner({ icon: Icon, tone, text, action }: { icon: LucideIcon; tone: "info" | "warn"; text: ReactNode; action?: ReactNode }) {
  return (
    <div className={clsx("flex flex-wrap items-center gap-4 rounded-2xl border px-5 py-3.5", tone === "info" ? "border-accent/25 bg-accent-soft" : "border-warn/30 bg-warn/10")}>
      <Icon size={20} className={tone === "info" ? "text-accent-text" : "text-warn"} />
      <div className="min-w-60 flex-1 text-sm">{text}</div>
      {action}
    </div>
  );
}

function Avatar({ site, status, size = "md" }: { site: Site; status?: AppStatus; size?: "md" | "lg" }) {
  return (
    <span className="relative shrink-0">
      <span
        className={clsx(
          "flex items-center justify-center font-bold",
          size === "lg" ? "size-12 rounded-2xl text-lg" : "size-9 rounded-xl text-sm",
          frameworkStyle[site.framework ?? ""] ?? "bg-accent-soft text-accent-text",
        )}
        aria-hidden
      >
        {(site.framework && site.framework !== "PHP" && site.framework !== "Static" ? site.framework : site.name).charAt(0).toUpperCase()}
      </span>
      {(isActive(status) || site.route) && (
        <span
          className={clsx("absolute -right-0.5 -bottom-0.5 rounded-full border-2 border-surface", size === "lg" ? "size-3.5" : "size-3", site.route || status?.state === "running" ? "bg-ok" : "animate-pulse bg-warn")}
          title={site.route ? `Served from port ${site.route.port}` : "Starting"}
        />
      )}
    </span>
  );
}

function ProjectRow({ site, status, selected, onSelect }: { site: Site; status?: AppStatus; selected: boolean; onSelect: () => void }) {
  return (
    <button
      type="button"
      aria-current={selected ? "true" : undefined}
      onClick={onSelect}
      className={clsx("flex w-full items-center gap-3 rounded-xl px-2.5 py-2 text-left transition-colors", selected ? "bg-accent-soft" : "hover:bg-surface-2")}
    >
      <Avatar site={site} status={status} />
      <span className="min-w-0 flex-1">
        <span className={clsx("block truncate text-sm font-semibold", selected && "text-accent-text")}>{site.name}</span>
        <span className="block truncate text-xs text-muted">
          {site.framework && site.framework !== "Static" ? site.framework : languageLabel[site.language]}
          {site.gitBranch && (
            <>
              <span aria-hidden> · </span>
              <GitBranch size={10} className="inline" aria-hidden /> {site.gitBranch}
            </>
          )}
        </span>
      </span>
      {site.route ? (
        <span className="shrink-0 rounded-md bg-ok/12 px-1.5 py-0.5 font-mono text-[11px] text-accent-text" title={`Served from port ${site.route.port}`}>
          :{site.route.port}
        </span>
      ) : (
        status?.state === "crashed" && <span className="size-2 shrink-0 rounded-full bg-danger" title="Crashed" />
      )}
    </button>
  );
}

function ProjectDetail({ site, info, status, tab, setTab }: { site: Site; info?: SitesInfo; status?: AppStatus; tab: Tab; setTab: (t: Tab) => void }) {
  const notify = useAppStore((s) => s.notify);
  const onError = (e: unknown) => notify("error", String(e));
  const settings = useQuery({ queryKey: ["settings"], queryFn: api.getSettings });
  const app = useAppInfo(site);
  const facts = useQuery({ queryKey: ["project-facts", site.name], queryFn: () => api.projectFacts(site.name), refetchInterval: 30_000 });
  const tunnel = useProjectTunnel(site);
  const nodeStatus = useQuery({ queryKey: ["node"], queryFn: api.nodeStatus, enabled: site.nodeProject });
  const dynamic = site.serveMode === "dynamic";
  // Running from your own terminal (or anything else LampRabit didn't start).
  const external = !!site.route && !isActive(status);
  // PHP projects with a package.json have an asset dev server (Vite) next to the site.
  const devServer = !dynamic && site.nodeProject && !!app.data && !!appCommand(app.data.config);
  const php = site.php ?? settings.data?.versions.php ?? null;
  const tabs: { id: Tab; label: string }[] = [
    { id: "overview", label: "Overview" },
    { id: "commands", label: "Commands" },
    { id: "domains", label: "Domains" },
    { id: "processes", label: "Processes" },
    { id: "settings", label: "Settings" },
  ];

  return (
    <div className="space-y-4">
      <Card className="p-5">
        <div className="flex flex-wrap items-center gap-4">
          <Avatar site={site} status={status} size="lg" />
          <div className="min-w-0 flex-1">
            <div className="flex min-w-0 flex-wrap items-center gap-x-2.5 gap-y-1">
              <h2 className="truncate text-2xl leading-tight font-bold tracking-tight">{site.name}</h2>
              {site.framework && site.framework !== "Static" && site.framework !== "PHP" && (
                <Chip className={frameworkStyle[site.framework]}>
                  {site.framework}
                  {facts.data?.frameworkVersion && <span className="font-mono opacity-80"> {facts.data.frameworkVersion}</span>}
                </Chip>
              )}
              {external ? <LivePill site={site} /> : (dynamic || devServer) && <AppState status={status} />}
            </div>
            <button onClick={() => openUrl(site.url).catch(onError)} className="mt-1 flex max-w-full items-center gap-1.5 font-mono text-sm text-muted hover:text-accent-text" title={`Open ${site.url}`}>
              {site.ssl ? <Lock size={12} className="shrink-0" aria-hidden /> : <LockOpen size={12} className="shrink-0" aria-hidden />}
              <span className="truncate">{site.url.replace(/\/$/, "")}</span>
            </button>
          </div>
          <div className="flex shrink-0 flex-wrap items-center gap-2">
            {dynamic && !external && <AppControls name={site.name} status={status} />}
            <Button variant={dynamic && !isActive(status) && !external ? "secondary" : "primary"} onClick={() => openUrl(site.url).catch(onError)}>
              <ExternalLink size={15} /> Open site
            </Button>
          </div>
        </div>

        {/* What it runs on, as one quiet line: the details are on the Overview tab. */}
        <p className="mt-3 flex flex-wrap items-center gap-x-2 gap-y-1 text-sm text-muted">
          {stackParts(site, {
            php: site.server === "frankenphp" ? "FrankenPHP's built-in PHP" : php ? `PHP ${php}` : "PHP (none installed)",
            java: languageLine("Java", site),
            python: languageLine("Python", site),
            go: languageLine("Go", site),
            rust: languageLine("Rust", site),
            node: site.language === "node" || devServer ? `Node.js ${status?.node ?? app.data?.config.node ?? site.node ?? nodeStatus.data?.current ?? ""}`.trim() : null,
            pm: site.nodeProject && app.data ? (app.data.config.packageManager ?? app.data.detectedPackageManager) : null,
          }).map((part, i) => (
            <span key={part} className="flex items-center gap-2">
              {i > 0 && <span aria-hidden className="text-line">•</span>}
              {part}
            </span>
          ))}
        </p>

        <ServingLine site={site} status={status} command={app.data ? appCommand(app.data.config) : null} php={php} />

        <div className="mt-3 flex flex-wrap items-center gap-1 border-t border-line pt-3">
          <Button size="sm" variant="ghost" onClick={() => api.openInEditor(site.path).catch(onError)}>
            <Code2 size={14} /> Editor
          </Button>
          <Button size="sm" variant="ghost" onClick={() => api.openTerminal(site.path).catch(onError)}>
            <SquareTerminal size={14} /> Terminal
          </Button>
          <Button size="sm" variant="ghost" onClick={() => openPath(site.path).catch(onError)}>
            <FolderOpen size={14} /> Folder
          </Button>
          {!tunnel && <ShareButton site={site} />}
          <span className="flex-1" />
          {devServer && (
            <span className="flex items-center gap-2 text-sm text-muted">
              Dev server <AppControls name={site.name} status={status} size="sm" />
            </span>
          )}
        </div>
        {tunnel && (
          <div className="mt-3">
            <TunnelRow tunnel={tunnel} />
          </div>
        )}
      </Card>

      <div role="tablist" aria-label={`${site.name} sections`} className="flex gap-1 border-b border-line">
        {tabs.map((t) => (
          <button
            key={t.id}
            role="tab"
            aria-selected={tab === t.id}
            onClick={() => setTab(t.id)}
            className={clsx("-mb-px border-b-2 px-3.5 py-2 text-sm font-medium transition-colors", tab === t.id ? "border-accent text-fg" : "border-transparent text-muted hover:text-fg")}
          >
            {t.label}
          </button>
        ))}
      </div>

      {tab === "overview" && <Overview site={site} info={info} settings={settings.data} facts={facts.data} status={status} php={php} onCommands={() => setTab("commands")} />}
      {tab === "commands" && (
        <div className="space-y-4">
          <Card className="p-5">
            <CommandsPanel site={site} status={status} />
          </Card>
          <OutputPanel name={site.name} />
        </div>
      )}
      {tab === "domains" && <ProjectDomains site={site} settings={settings.data} />}
      {tab === "processes" && <ProjectProcesses site={site} settings={settings.data} />}
      {tab === "settings" && <SettingsTab site={site} settings={settings.data} status={status} />}
    </div>
  );
}

/** "Python 3.12.9 (.python-version)": the version a project runs on, and why. */
function languageLine(label: string, site: Site): string {
  if (!site.languageVersion) return site.versionRequest ? `${label} ${site.versionRequest.version} (not installed)` : `${label} (not installed)`;
  return site.versionRequest ? `${label} ${site.languageVersion} (${site.versionRequest.file})` : `${label} ${site.languageVersion}`;
}

/** The project's language, runtime versions and package manager, for the header line. */
function stackParts(site: Site, v: { php: string; java: string; python: string; go: string; rust: string; node: string | null; pm: string | null }): string[] {
  const parts: string[] = [];
  if (site.language === "php") parts.push(v.php);
  if (site.language === "java") parts.push(v.java);
  if (site.language === "python") parts.push(v.python);
  if (site.language === "go") parts.push(v.go);
  if (site.language === "rust") parts.push(v.rust);
  if (site.language === "static") parts.push("Static HTML");
  if (v.node) parts.push(v.node);
  if (v.pm) parts.push(v.pm);
  return parts;
}

function Chip({ className, children }: { className?: string; children: ReactNode }) {
  return <span className={clsx("inline-flex items-center gap-1 rounded-md px-2 py-0.5 text-xs font-medium", className ?? "bg-surface-2 text-muted")}>{children}</span>;
}

/** "Running · :5173 · your terminal": a dev server LampRabit found rather than started. */
function LivePill({ site }: { site: Site }) {
  const port = site.ports.find((p) => p.port === site.route?.port);
  return (
    <span className="inline-flex items-center gap-1.5 rounded-full bg-surface-2 px-2.5 py-1 text-xs font-medium text-accent-text" title={port?.command}>
      <span className="size-1.5 rounded-full bg-ok" aria-hidden />
      Running <span className="font-mono text-muted">:{site.route?.port}</span>
      <span className="text-muted">· started outside LampRabit</span>
    </span>
  );
}

/**
 * Where the domain's requests end up, live: the port the project listens on (with a
 * choice when there are several), or its files. PHP projects can be pointed at a port
 * explicitly, e.g. `php artisan serve`.
 */
function ServingLine({ site, status, command, php }: { site: Site; status?: AppStatus; command: string | null; php: string | null }) {
  const queryClient = useQueryClient();
  const notify = useAppStore((s) => s.notify);
  const pick = useMutation({
    mutationFn: (port: number | null) => api.setSiteRoute(site.name, port),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["sites"] }),
    onError: (e) => notify("error", String(e)),
  });
  const folder = relative(site.path, site.docroot);
  const routed = site.route ? site.ports.find((p) => p.port === site.route!.port) : undefined;
  const LampRabits = isActive(status);
  const unrouted = site.ports.filter((p) => p.port !== site.route?.port);
  let target: ReactNode;
  if (site.route) {
    target = (
      <>
        Dev server on <span className="font-mono">{site.route.host}:{site.route.port}</span>
        <span className="text-muted">
          {" "}
          · {routed?.process ?? "process"}
          {routed ? ` (PID ${routed.pid})` : ""} · {LampRabits ? "started here" : "started outside LampRabit"}
        </span>
        {LampRabits && command && <code className="ml-2 rounded bg-surface px-1.5 py-0.5 font-mono text-xs">{command.split("{port}").join(String(site.route.port))}</code>}
      </>
    );
  } else if (site.proxy && site.language === "java") {
    target = <>Tomcat, which serves the project's web app</>;
  } else if (site.language === "php") {
    target = (
      <>
        <span className="font-mono">{folder}</span>{" "}
        {site.server === "frankenphp" ? "with FrankenPHP's built-in PHP" : php ? `with PHP ${php}` : "(no PHP installed)"}
      </>
    );
  } else {
    target = (
      <>
        <span className="font-mono">{folder}</span> as static files
      </>
    );
  }
  return (
    <div className="mt-4 space-y-2">
      <div className="flex min-w-0 items-center gap-2 rounded-xl bg-surface-2/70 px-3 py-2 text-sm">
        <Globe size={14} className="shrink-0 text-muted" aria-hidden />
        <span className="shrink-0 font-mono text-muted">{site.domain}</span>
        <ArrowRight size={13} className="shrink-0 text-muted" aria-hidden />
        <span className="min-w-0 flex-1 truncate">{target}</span>
        <span className="shrink-0 text-xs text-muted">by {serviceLabels[site.server]}</span>
        {site.route && site.ports.length > 1 && (
          <select
            value={site.routePort ?? ""}
            onChange={(e) => pick.mutate(e.target.value ? Number(e.target.value) : null)}
            aria-label="Port the domain goes to"
            className="shrink-0 rounded-md border border-line bg-surface px-2 py-1 text-xs text-fg"
          >
            <option value="">Automatic (:{Math.min(...site.ports.map((p) => p.port))})</option>
            {site.ports.map((p) => (
              <option key={p.port} value={p.port}>
                :{p.port} · {p.process}
              </option>
            ))}
          </select>
        )}
        {site.language === "php" && site.route && (
          <Button size="sm" variant="ghost" onClick={() => pick.mutate(null)}>
            Back to PHP
          </Button>
        )}
      </div>
      {!site.route && site.serveMode === "dynamic" && (
        <p className="px-1 text-xs text-muted">
          No dev server is running. Start it here, or run it yourself (e.g. <code className="font-mono">npm run dev</code> in the project folder): the domain switches to it within a few
          seconds, and back to the files when it stops.
        </p>
      )}
      {site.language === "php" && !site.route && unrouted.length > 0 && (
        <div className="flex flex-wrap items-center gap-2 px-1 text-xs text-muted">
          {site.framework === "Laravel" && (
            <span className="basis-full">
              With <code className="font-mono">composer dev</code> or <code className="font-mono">npm run dev</code>, keep the site on PHP: Vite's hot reload, queues and logs work alongside it.
            </span>
          )}
          <span>Also listening:</span>
          {unrouted.map((p) => (
            <span key={p.port} className="flex items-center gap-1.5 rounded-lg border border-line px-2 py-0.5" title={p.command}>
              <span className="font-mono text-fg">:{p.port}</span> {p.process}
              <button type="button" onClick={() => pick.mutate(p.port)} className="font-medium text-accent-text hover:underline">
                Route {site.domain} here
              </button>
            </span>
          ))}
        </div>
      )}
    </div>
  );
}

/** A folder inside the project, as "public/"; "/" for the project itself. */
function relative(root: string, dir: string) {
  const rest = dir.slice(root.length).replace(/^[\\/]+/, "");
  return rest ? `${rest.replace(/\\/g, "/")}/` : "project folder";
}

function Overview({
  site,
  info,
  settings,
  facts,
  status,
  php,
  onCommands,
}: {
  site: Site;
  info?: SitesInfo;
  settings?: Settings;
  facts?: ProjectFacts;
  status?: AppStatus;
  php: string | null;
  onCommands: () => void;
}) {
  const notify = useAppStore((s) => s.notify);
  const now = useNow();
  const onError = (e: unknown) => notify("error", String(e));
  const node = useQuery({ queryKey: ["node"], queryFn: api.nodeStatus, enabled: site.nodeProject });
  const install = useMutation({
    mutationFn: (manager: string) => (manager === "Composer" ? api.runProjectCommand(site.name, "composer install", "composer install") : api.appInstall(site.name)),
    onSuccess: onCommands,
    onError,
  });
  const dbUrl = site.database ? databaseUrl(site.database, info, settings) : null;
  const nodeVersion = status?.node ?? site.app?.node ?? site.node ?? node.data?.current ?? null;
  const nodeMissing = !!site.node && !!node.data?.manager && !node.data.installed.some((v) => v === site.node || v.startsWith(`${site.node}.`));
  const appUrl = facts?.env.find(([k]) => k === "APP_URL")?.[1];
  const urlMismatch = !!appUrl && appUrl.replace(/\/$/, "") !== site.url.replace(/\/$/, "");
  const git = facts?.git;
  const remote = git?.remote ? webRemote(git.remote) : null;

  return (
    <div className="space-y-4">
      {(isActive(status) || status?.task) && <OutputPanel name={site.name} height="max-h-56 min-h-24" />}

      <div className="grid gap-4 xl:grid-cols-2">
        <Section title="Project">
          {facts?.package && (
            <Fact label="Package">
              <span className="font-mono">{facts.package}</span>
              {facts.version && <span className="text-muted"> {facts.version}</span>}
            </Fact>
          )}
          {facts?.description && <Fact label="Description">{facts.description}</Fact>}
          <Fact label="Framework">
            {site.framework && site.framework !== "Static" && site.framework !== "PHP" ? site.framework : "None"}
            {facts?.frameworkVersion && <span className="font-mono text-muted"> {facts.frameworkVersion}</span>}
          </Fact>
          <Fact label="Language">{languageLabel[site.language]}</Fact>
          {facts?.template && <Fact label="Created from">{facts.template} template</Fact>}
          <Fact label="Folder">
            <CopyText text={site.path} />
          </Fact>
          {site.modified > 0 && <Fact label="Changed">{formatAgo(site.modified, now)}</Fact>}
        </Section>

        <Section title="Runtime and serving">
          <Fact label="URL">
            <button onClick={() => openUrl(site.url).catch(onError)} className="font-mono hover:text-accent-text hover:underline">
              {site.url.replace(/\/$/, "")}
            </button>
          </Fact>
          <Fact label="Served">
            {site.route
              ? `By the dev server on port ${site.route.port}, behind `
              : site.language === "java" && site.proxy
                ? "By Tomcat, behind "
                : site.language === "php"
                  ? site.server === "frankenphp" ? "With its built-in PHP by " : "With PHP by "
                  : "As files by "}
            {serviceLabels[site.server]}
            {!site.route && !site.proxy && <span className="text-muted"> from {relative(site.path, site.docroot)}</span>}
          </Fact>
          {site.ports.length > 0 && (
            <Fact label="Listening">
              <ul className="space-y-0.5">
                {site.ports.map((p) => (
                  <li key={p.port} title={p.command} className="truncate">
                    <span className="font-mono">{p.host}:{p.port}</span> <span className="text-muted">· {p.process} (PID {p.pid})</span>
                    {site.route?.port === p.port && <span className="text-accent-text"> · routed</span>}
                  </li>
                ))}
              </ul>
            </Fact>
          )}
          {site.serveMode === "dynamic" && !site.route && (
            <Fact label="Port">{site.app?.port ? `${site.app.port} when started here` : <span className="text-muted">picked when it starts</span>}</Fact>
          )}
          <Fact label="HTTPS">{site.ssl ? "On, with a local certificate" : "Off"}</Fact>
          {site.language === "php" && site.server === "frankenphp" && (
            <Fact label="PHP">
              FrankenPHP's built-in PHP
              {facts?.phpRequirement && <span className="text-muted"> · needs {facts.phpRequirement}</span>}
            </Fact>
          )}
          {site.language === "php" && site.server !== "frankenphp" && (
            <Fact label="PHP">
              {php ?? "none installed"}
              {!site.php && <span className="text-muted"> (default)</span>}
              {facts?.phpRequirement && <span className="text-muted"> · needs {facts.phpRequirement}</span>}
            </Fact>
          )}
          {site.nodeProject && (
            <Fact label="Node.js">
              {nodeVersion ?? "NVM's default"}
              {site.node && <span className="text-muted"> · .nvmrc asks for {site.node}</span>}
              {nodeMissing && <span className="text-warn"> (not installed)</span>}
              {facts?.nodeRequirement && <span className="text-muted"> · engines {facts.nodeRequirement}</span>}
            </Fact>
          )}
        </Section>

        {facts && facts.dependencies.length > 0 && (
          <Section title="Dependencies">
            {facts.dependencies.map((d) => (
              <Fact key={d.manager} label={d.manager === "Composer" ? "Composer" : `Node (${d.manager})`}>
                <span className="flex flex-wrap items-center gap-2">
                  <span>
                    {d.count} {d.count === 1 ? "package" : "packages"}
                    {d.devCount > 0 && <span className="text-muted"> + {d.devCount} dev</span>}
                  </span>
                  {d.installed ? (
                    <span className="flex items-center gap-1 text-xs text-accent-text">
                      <Check size={12} /> Installed
                    </span>
                  ) : (
                    <>
                      <span className="text-xs text-warn">Not installed</span>
                      <Button size="sm" variant="soft" onClick={() => install.mutate(d.manager)} disabled={install.isPending || !!status?.task}>
                        <Package size={13} /> Install
                      </Button>
                    </>
                  )}
                </span>
              </Fact>
            ))}
          </Section>
        )}

        {(git || site.gitBranch) && (
          <Section title="Git">
            {site.gitBranch && (
              <Fact label="Branch">
                <span className="flex items-center gap-1.5 font-mono">
                  <GitBranch size={13} aria-hidden /> {site.gitBranch}
                </span>
              </Fact>
            )}
            {git?.changes != null && <Fact label="Changes">{git.changes === 0 ? "Clean" : <span className="text-warn">{git.changes} uncommitted {git.changes === 1 ? "file" : "files"}</span>}</Fact>}
            {git?.commit && (
              <Fact label="Last commit">
                <span className="font-mono text-muted">{git.commit[0]}</span> {git.commit[1]} <span className="text-muted">· {formatAgo(git.commit[2] * 1000, now)}</span>
              </Fact>
            )}
            {git?.remote && (
              <Fact label="Remote">
                {remote ? (
                  <button onClick={() => openUrl(remote).catch(onError)} className="font-mono break-all hover:text-accent-text hover:underline">
                    {git.remote}
                  </button>
                ) : (
                  <span className="font-mono break-all">{git.remote}</span>
                )}
              </Fact>
            )}
          </Section>
        )}

        {(facts?.envFile || site.database) && (
          <Section title={facts?.envFile ? `Environment (${facts.envFile})` : "Database"}>
            {site.database && (
              <Fact label="Database">
                <span className="flex items-center gap-1.5">
                  <Database size={13} aria-hidden />
                  {dbUrl ? (
                    <button onClick={() => openUrl(dbUrl).catch(onError)} className="font-mono hover:text-accent-text hover:underline" title="Open in phpMyAdmin">
                      {site.database}
                    </button>
                  ) : (
                    <span className="font-mono">{site.database}</span>
                  )}
                </span>
              </Fact>
            )}
            {facts?.env.map(([key, value]) => (
              <Fact key={key} label={key} mono>
                <span className="font-mono break-all">{value}</span>
                {key === "APP_URL" && urlMismatch && <span className="block text-xs text-warn">Differs from {site.url.replace(/\/$/, "")}; links and assets may point elsewhere.</span>}
              </Fact>
            ))}
            {facts?.envFile && facts.env.length === 0 && <p className="text-sm text-muted">No common settings found. Keys and passwords are never shown here.</p>}
          </Section>
        )}
      </div>

      <Section
        title="Quick commands"
        action={
          <button type="button" onClick={onCommands} className="text-xs font-medium text-accent-text hover:underline">
            All commands
          </button>
        }
      >
        <CommandsPanel site={site} status={status} limit={6} onMore={onCommands} />
      </Section>
    </div>
  );
}

/** https URL for a git remote (GitHub-style ssh remotes included), to open in the browser. */
function webRemote(remote: string) {
  const ssh = /^git@([^:]+):(.+?)(\.git)?$/.exec(remote);
  if (ssh) return `https://${ssh[1]}/${ssh[2]}`;
  return /^https?:\/\//.test(remote) ? remote.replace(/\.git$/, "") : null;
}

function Section({ title, action, children }: { title: string; action?: ReactNode; children: ReactNode }) {
  return (
    <Card className="p-5">
      <div className="mb-3 flex items-center justify-between gap-3">
        <h3 className="text-xs font-bold tracking-wider text-muted uppercase">{title}</h3>
        {action}
      </div>
      <dl className="space-y-2.5">{children}</dl>
    </Card>
  );
}

function Fact({ label, mono, children }: { label: string; mono?: boolean; children: ReactNode }) {
  return (
    <div className="grid grid-cols-[8.5rem_minmax(0,1fr)] gap-3 text-sm">
      <dt className={clsx("truncate text-muted", mono && "font-mono text-xs leading-5")}>{label}</dt>
      <dd className="min-w-0">{children}</dd>
    </div>
  );
}

function CopyText({ text }: { text: string }) {
  const [copied, setCopied] = useState(false);
  return (
    <button
      type="button"
      onClick={() =>
        navigator.clipboard.writeText(text).then(() => {
          setCopied(true);
          setTimeout(() => setCopied(false), 1200);
        })
      }
      className="group flex max-w-full items-center gap-1.5 text-left font-mono text-xs hover:text-accent-text"
      title="Copy"
    >
      <span className="selectable truncate">{text}</span>
      {copied ? <Check size={12} className="shrink-0 text-accent-text" /> : <Copy size={12} className="shrink-0 text-muted group-hover:text-accent-text" />}
    </button>
  );
}

function SettingsTab({ site, settings, status }: { site: Site; settings?: Settings; status?: AppStatus }) {
  const queryClient = useQueryClient();
  const notify = useAppStore((s) => s.notify);
  const onError = (e: unknown) => notify("error", String(e));
  const installed = useQuery({ queryKey: ["installed"], queryFn: api.installedRuntimes });
  const app = useAppInfo(site);
  const { restart } = useAppActions(site.name);
  const phpVersions = installed.data?.find((i) => i.runtime === "php")?.versions ?? [];
  const onSuccess = () => {
    for (const key of ["sites", "settings", "services", "php-status"]) queryClient.invalidateQueries({ queryKey: [key] });
  };
  const toggleSsl = useMutation({ mutationFn: (ssl: boolean) => api.setSiteSsl(site.name, ssl), onSuccess, onError });
  const setPhp = useMutation({ mutationFn: (v: string | null) => api.setSitePhp(site.name, v), onSuccess, onError });
  const setServer = useMutation({ mutationFn: (v: Server | null) => api.setSiteServer(site.name, v), onSuccess, onError });
  const setLanguage = useMutation({ mutationFn: (v: string | null) => api.setLanguageVersion(site.name, v), onSuccess, onError });
  const setWorker = useMutation({ mutationFn: (v: boolean) => api.setSiteWorker(site.name, v), onSuccess, onError });
  const restartWorkers = useMutation({
    mutationFn: api.restartFrankenphpWorkers,
    onSuccess: () => notify("info", "FrankenPHP's workers reloaded the code"),
    onError,
  });
  const worker = !!settings?.sites[site.name]?.worker;
  const languageRuntime = (["python", "java", "go", "rust"] as const).find((l) => l === site.language);
  const languageVersions = languageRuntime ? (installed.data?.find((i) => i.runtime === languageRuntime)?.versions ?? []) : [];
  const busy = toggleSsl.isPending || setPhp.isPending || setServer.isPending || setWorker.isPending;
  const select = clsx(inputClass.replace("w-full", ""), "w-52");

  return (
    <div className="space-y-4">
      <Section title="Web server">
        <div className={clsx("space-y-4 text-sm", busy && "opacity-60")}>
          {(languageRuntime === "python" || languageRuntime === "java") && (
            <Row
              title={`${runtimeLabels[languageRuntime]} version`}
              hint={`Saved in the project's ${languageRuntime === "python" ? ".python-version" : ".java-version"}, so other tools (pyenv, jenv) read it too. python/java in the project's folder use it.`}
            >
              <select value={site.versionRequest?.version ?? ""} disabled={setLanguage.isPending} onChange={(e) => setLanguage.mutate(e.target.value || null)} className={select}>
                <option value="">Default · {settings?.versions[languageRuntime] ?? "none"}</option>
                {site.versionRequest && !languageVersions.includes(site.versionRequest.version) && (
                  <option value={site.versionRequest.version}>{site.versionRequest.version} (from {site.versionRequest.file})</option>
                )}
                {languageVersions.map((v) => (
                  <option key={v} value={v}>
                    {v}
                  </option>
                ))}
              </select>
            </Row>
          )}
          {(languageRuntime === "go" || languageRuntime === "rust") && (
            <Row
              title={`${runtimeLabels[languageRuntime]} version`}
              hint={languageRuntime === "go" ? "Set by the go and toolchain lines in go.mod." : "Set by rust-toolchain.toml; rustup installs that toolchain on first use."}
            >
              <span className="text-muted">
                {site.versionRequest ? `${site.versionRequest.version} (${site.versionRequest.file})` : `Default · ${settings?.versions[languageRuntime] ?? "none"}`}
              </span>
            </Row>
          )}
          {site.language === "php" && site.server === "frankenphp" && (
            <Row title="PHP version" hint="FrankenPHP runs its own built-in PHP, so the PHP versions and extensions installed in LampRabit don't apply here.">
              <span className="text-muted">FrankenPHP's PHP</span>
            </Row>
          )}
          {site.language === "php" && site.server !== "frankenphp" && (
            <Row title="PHP version" hint="Each site can use its own; the others keep theirs.">
              <select value={site.php ?? ""} disabled={busy} onChange={(e) => setPhp.mutate(e.target.value || null)} className={select}>
                <option value="">Default · {settings?.versions.php ?? "none"}</option>
                {phpVersions.map((v) => (
                  <option key={v} value={v}>
                    {v}
                  </option>
                ))}
              </select>
            </Row>
          )}
          {settings && settings.webServers.length > 1 && (
            <Row
              title="Web server"
              hint={`${runtimeLabels[settings.webServers[0]]}, the primary server, uses ports ${settings.ports.http}/${settings.ports.https}; the others their own. The other servers redirect this domain to the one you pick.`}
            >
              <select value={site.server} disabled={busy} onChange={(e) => setServer.mutate(e.target.value as Server)} className={select}>
                {settings.webServers.map((s) => (
                  <option key={s} value={s}>
                    {runtimeLabels[s]}
                  </option>
                ))}
              </select>
            </Row>
          )}
          {site.server === "frankenphp" && site.language === "php" && (
            <Row
              title="Worker mode"
              hint="FrankenPHP keeps the app in memory between requests, which is much faster. The app has to support it (Laravel Octane, Symfony Runtime…); after changing code, restart the workers."
            >
              <div className="flex items-center gap-2">
                {worker && (
                  <Button size="sm" variant="ghost" disabled={restartWorkers.isPending} onClick={() => restartWorkers.mutate()}>
                    {restartWorkers.isPending ? "Restarting…" : "Restart workers"}
                  </Button>
                )}
                <Switch checked={worker} onChange={(v) => setWorker.mutate(v)} label={`Worker mode for ${site.domain}`} />
              </div>
            </Row>
          )}
          <Row title="HTTPS" hint={`A certificate for ${site.domain} from LampRabit's local authority.`}>
            <Switch checked={site.ssl} onChange={(v) => toggleSsl.mutate(v)} label={`HTTPS for ${site.domain}`} />
          </Row>
        </div>
      </Section>

      {site.nodeProject && app.data && (
        <Section
          title={site.language === "node" ? "Dev server" : "Asset dev server"}
          action={
            isActive(status) ? (
              <Button size="sm" variant="ghost" onClick={() => restart.mutate()}>
                Restart to apply
              </Button>
            ) : undefined
          }
        >
          <ServeSettings site={site} info={app.data} status={status} />
        </Section>
      )}
    </div>
  );
}

/** phpMyAdmin opened on the project's database, on the default database server. */
function databaseUrl(db: string, info?: SitesInfo, settings?: Settings) {
  const base = toolUrl("phpmyadmin", info, settings?.ports.mailUi);
  return base ? `${base}?server=1&db=${encodeURIComponent(db)}` : null;
}
