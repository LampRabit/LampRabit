import { useEffect, useRef, useState, type ReactNode } from "react";
import { usePageActive } from "../lib/pageActive";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { openPath } from "@tauri-apps/plugin-opener";
import { openUrl } from "../lib/open";
import {
  Activity as Pulse,
  AlertTriangle,
  Check,
  ChevronDown,
  ChevronRight,
  Copy,
  Database,
  DatabaseZap,
  ExternalLink,
  Feather,
  Package,
  FileCog,
  FileText,
  FolderOpen,
  Keyboard,
  Layers,
  Layers3,
  LayoutGrid,
  LayoutList,
  Grid3x3,
  Save,
  Mail,
  Pencil,
  Play,
  RotateCcw,
  Plus,
  RotateCw,
  ScrollText,
  Server as ServerIcon,
  ShieldCheck,
  Gauge,
  Leaf,
  BarChart3,
  Network,
  Search,
  HardDrive,
  Bot,
  Workflow,
  Inbox,
  Flame,
  Cpu,
  Square,
  Table2,
  TerminalSquare,
  Trash2,
  Zap,
} from "lucide-react";
import clsx from "clsx";
import {
  api,
  runtimeLabels,
  serviceLabels,
  type Database as DatabaseKind,
  type PortStatus,
  type RuntimeInfo,
  type ServiceInfo,
  type ServiceKind,
  type Settings,
  type Site,
  type SitesInfo,
} from "../lib/api";
import { readPref, useAppStore, writePref, type Activity, notifyError } from "../lib/store";
import { dbTools, preferredTool, toolUrl } from "../lib/tools";
import { TILE, runtimeTiles, tileFor } from "../components/runtimes";
import { formatClock, formatUptime, useNow } from "../lib/time";
import { connection, copyText, envText } from "../lib/connections";
import { formatBytes, formatCpu, useUsage, useUsageSampler } from "../lib/usage";
import { isServer, primaryServer, serverEnabled, startAllHint, useStack } from "../lib/stack";
import { useHotkeys } from "../lib/shortcuts";
import { Button, Card, Pill, Segmented } from "../components/ui";
import Menu from "../components/Menu";
import ProblemPanel from "../components/ProblemPanel";
import { FreePort, NeedsAttention, StatusStrip, conflictFor, holder, useIssues } from "../components/HomeStatus";
import { useTunnels } from "../components/Tunnels";
import ProfilesDialog, { ProfilesTransfer } from "../components/ProfilesDialog";
import Sparkline from "../components/Sparkline";
import { Kbd } from "../components/Dialog";
import { infoFor, serviceLink, useCatalog } from "../lib/catalog";

const serviceMeta: Record<ServiceKind, { icon: typeof Feather; tile: string; blurb: string }> = {
  apache: { icon: Feather, tile: TILE, blurb: "Web server for your local sites" },
  nginx: { icon: ServerIcon, tile: TILE, blurb: "Fast web server for your local sites" },
  caddy: { icon: ShieldCheck, tile: TILE, blurb: "Web server with a short config file" },
  frankenphp: { icon: Flame, tile: TILE, blurb: "Web server with PHP built in" },
  tomcat: { icon: Cpu, tile: TILE, blurb: "Runs your Java web apps" },
  mariadb: { icon: Database, tile: TILE, blurb: "Database server for your local sites" },
  postgres: { icon: Database, tile: TILE, blurb: "PostgreSQL database server" },
  pgadmin: { icon: Gauge, tile: TILE, blurb: "PostgreSQL admin in the browser" },
  mongodb: { icon: Leaf, tile: TILE, blurb: "Document database" },
  clickhouse: { icon: BarChart3, tile: TILE, blurb: "Analytics database" },
  neo4j: { icon: Network, tile: TILE, blurb: "Graph database" },
  meilisearch: { icon: Search, tile: TILE, blurb: "Search engine" },
  typesense: { icon: Search, tile: TILE, blurb: "Search engine" },
  rustfs: { icon: HardDrive, tile: TILE, blurb: "S3 object storage" },
  ollama: { icon: Bot, tile: TILE, blurb: "Local AI models" },
  minio: { icon: HardDrive, tile: TILE, blurb: "S3 object storage (unmaintained upstream)" },
  elasticsearch: { icon: Search, tile: TILE, blurb: "Search engine" },
  opensearch: { icon: Search, tile: TILE, blurb: "Search engine" },
  n8n: { icon: Workflow, tile: TILE, blurb: "Workflow automation" },
  rabbitmq: { icon: Inbox, tile: TILE, blurb: "Message queue" },
  mysql: { icon: DatabaseZap, tile: TILE, blurb: "Oracle MySQL, next to MariaDB" },
  mailpit: { icon: Mail, tile: TILE, blurb: "Catches mail sent by your apps" },
  redis: { icon: Zap, tile: TILE, blurb: "Cache, sessions and queues" },
  memcached: { icon: Layers3, tile: TILE, blurb: "Memory cache for sessions and queries" },
};

/** Services listed only once installed; the core stack is always listed, with an Install button. */
const optional: ServiceKind[] = ["tomcat", "postgres", "pgadmin", "mongodb", "clickhouse", "neo4j", "meilisearch", "typesense", "rustfs", "ollama", "minio", "elasticsearch", "opensearch", "n8n", "rabbitmq", "mysql", "mailpit", "redis", "memcached"];

/**
 * Installed services, once there are any; before that, the core stack with Install buttons,
 * so a fresh setup has somewhere to start. Web servers only while enabled in Settings.
 */
const visible = (s: ServiceInfo, settings: Settings | undefined, anyInstalled: boolean) =>
  (s.installed || (!anyInstalled && !optional.includes(s.kind))) && serverEnabled(s.kind, settings?.webServers);

export default function Overview() {
  const now = useNow();
  const navigate = useAppStore((s) => s.navigate);
  const notify = useAppStore((s) => s.notify);
  const queryClient = useQueryClient();
  const connMode = useAppStore((s) => s.connMode);
  const setConnMode = useAppStore((s) => s.setConnMode);
  const openNewProject = useAppStore((s) => s.setNewProjectOpen);
  // Pages stay mounted when hidden; they only poll while on screen. Services poll with the
  // ports, so a conflict that ended (the other program quit) clears on its own.
  const active = usePageActive();
  const services = useQuery({ queryKey: ["services"], queryFn: api.listServices, refetchInterval: active ? 5000 : false });
  const sites = useQuery({ queryKey: ["sites"], queryFn: api.listSites, refetchInterval: active ? 15_000 : false });
  const settings = useQuery({ queryKey: ["settings"], queryFn: api.getSettings });
  const catalog = useCatalog();
  const ports = useQuery({ queryKey: ["port-status"], queryFn: api.portStatus, refetchInterval: active ? 5000 : false });
  const stack = useStack();

  const anyInstalled = (services.data ?? []).some((s) => s.installed);
  const list = (services.data ?? []).filter((s) => visible(s, settings.data, anyInstalled));
  const [selected, setSelected] = useState<Set<ServiceKind>>(new Set());
  const [focus, setFocus] = useState(0);
  const [profilesDialog, setProfilesDialog] = useState<{ initial?: ServiceKind[]; profile?: Settings["profiles"][number] } | null>(null);
  const [layout, setLayoutState] = useState<ServiceLayout>(() => (LAYOUTS.find((l) => l.id === readPref("home-services-layout"))?.id ?? "list"));
  const setLayout = (next: ServiceLayout) => {
    setLayoutState(next);
    writePref("home-services-layout", next);
  };
  const rows = useRef<(HTMLElement | null)[]>([]);
  useUsageSampler(stack.running.length > 0);

  const tunnels = useTunnels();
  // "Show fixes" in Needs attention opens that service's row and brings it into view.
  const [revealed, setRevealed] = useState<{ kind: ServiceKind; at: number } | null>(null);
  const shadows = useQuery({ queryKey: ["path-shadows"], queryFn: api.pathShadows, refetchInterval: active ? 60_000 : false });
  const issues = useIssues(list, ports.data, tunnels.data, shadows.data, (kind) => {
    setRevealed({ kind, at: Date.now() });
    rows.current[list.findIndex((s) => s.kind === kind)]?.scrollIntoView({ behavior: "smooth", block: "center" });
  });
  const installedCount = stack.usable.length;

  useEffect(() => {
    if (stack.profile) setSelected(new Set(stack.profile.services));
  }, [stack.profile?.name, stack.profile?.services.join("|")]);

  // Ticking services while a profile of your own is active edits that profile; with "All
  // services" it starts a new one.
  const updateProfile = useMutation({
    mutationFn: async ({ name, services }: { name: string; services: ServiceKind[] }) => {
      if (!settings.data) throw new Error("Settings are still loading");
      return api.saveSettings({ ...settings.data, profiles: settings.data.profiles.map((p) => (p.name === name ? { ...p, services } : p)) });
    },
    onSuccess: (saved, { name }) => {
      queryClient.setQueryData(["settings"], saved);
      notify("info", `Updated the ${name} profile`);
    },
    onError: notifyError,
  });

  const deleteProfile = useMutation({
    mutationFn: async (name: string) => {
      if (!settings.data) throw new Error("Settings are still loading");
      return api.saveSettings({ ...settings.data, profiles: settings.data.profiles.filter((p) => p.name !== name) });
    },
    onSuccess: (saved, name) => {
      queryClient.setQueryData(["settings"], saved);
      if (useAppStore.getState().activeProfile === name) useAppStore.getState().setActiveProfile(null);
      notify("info", `Deleted ${name} profile`);
    },
    onError: notifyError,
  });

  const selectable = list.filter((s) => s.installed);
  const chosen = selectable.filter((s) => selected.has(s.kind)).map((s) => s.kind);
  const allSelected = selectable.length > 0 && chosen.length === selectable.length;
  const toggle = (kind: ServiceKind) =>
    setSelected((cur) => {
      const next = new Set(cur);
      if (next.has(kind)) next.delete(kind);
      else next.add(kind);
      return next;
    });

  const focusRow = (step: number) => {
    // The first press lands on the current row; later ones move from it.
    const onRow = rows.current.includes(document.activeElement as HTMLElement);
    const clamped = Math.max(0, Math.min(list.length - 1, onRow ? focus + step : focus));
    setFocus(clamped);
    rows.current[clamped]?.focus();
  };
  const current = list[focus];
  const copy = (text: string, what: string) =>
    navigator.clipboard.writeText(text).then(
      () => notify("info", `Copied ${what}`),
      notifyError,
    );

  useHotkeys({
    arrowdown: () => focusRow(1),
    j: () => focusRow(1),
    arrowup: () => focusRow(-1),
    k: () => focusRow(-1),
    " ": () => current?.installed && toggle(current.kind),
    a: () => setSelected(allSelected ? new Set() : new Set(selectable.map((s) => s.kind))),
    enter: () => {
      if (!current?.installed) return;
      if (current.state === "running") stack.stop([current.kind]);
      else stack.start([current.kind]);
    },
    s: () => stack.start(chosen.length ? chosen : current?.installed ? [current.kind] : []),
    x: () => stack.stop(chosen.length ? chosen : current?.installed ? [current.kind] : []),
    r: () => current?.state === "running" && api.restartService(current.kind).catch(notifyError),
    o: () => current && openAction(current, sites.data, settings.data, navigate, catalog.data)?.run(),
    c: () => current?.installed && copy(copyText(connection(current), connMode), `${current.label} ${connMode === "env" ? ".env" : "URL"}`),
    e: () => setConnMode(connMode === "url" ? "env" : "url"),
    l: () => {
      if (current) useAppStore.getState().showLogs(current.kind);
    },
  });

  return (
    <>
      <div className="motion-stagger space-y-6">
        <header className="flex flex-wrap items-center justify-between gap-x-6 gap-y-4">
        <h1 className="text-3xl font-medium tracking-tight">Home</h1>
        <div className="flex flex-wrap items-center gap-2">
          {installedCount === 0 ? (
            <Button variant="primary" onClick={() => navigate("services")}>
              Set up your stack
            </Button>
          ) : chosen.length > 0 ? (
            // Ticked services: the buttons act on them, not on the whole stack.
            <>
              <Button variant="primary" onClick={() => stack.start(chosen)} disabled={stack.busy || stack.isUp(chosen)} title="S">
                <Play size={14} /> Start {chosen.length} selected
              </Button>
              <Button onClick={() => stack.stop(chosen)} disabled={stack.busy || !chosen.some((k) => stack.stoppable.some((s) => s.kind === k))} title="X">
                <Square size={12} className="fill-current" /> Stop {chosen.length} selected
              </Button>
            </>
          ) : (
            <>
              {stack.up ? (
                <Button variant="primary" onClick={stack.restartRunning} disabled={stack.busy} title="Restart the running services (Shift+R)">
                  <RotateCcw size={14} /> Restart
                </Button>
              ) : (
                <Button variant="primary" onClick={stack.startStack} disabled={stack.busy} title={stack.profile ? "Shift+S" : startAllHint}>
                  <Play size={14} /> {stack.profile ? `Start ${stack.profile.name}` : "Start all"}
                </Button>
              )}
              <Button onClick={stack.stopAll} disabled={stack.busy || stack.stoppable.length === 0} title={stack.stoppable.length === 0 ? "Nothing LampRabit runs is running" : "Shift+X"}>
                <Square size={12} className="fill-current" /> Stop all
              </Button>
            </>
          )}
        </div>
      </header>

      <div className="space-y-4" aria-live="polite">
        <StatusStrip services={services.data ?? []} sites={sites.data} tunnels={tunnels.data} issues={issues.length} />
        <NeedsAttention issues={issues} />
      </div>

      <VcRuntimeBanner />

      <div className="grid items-start gap-6 xl:grid-cols-[minmax(0,1fr)_340px]">
        <section aria-labelledby="services-heading">
          <div className="mb-3 empty:mb-0">
            {installedCount > 0 && (
            <ProfileTiles
              profiles={settings.data?.profiles ?? []}
              active={stack.profile?.name ?? null}
              onSelect={(name) => {
                useAppStore.getState().setActiveProfile(name);
                const profileServices = settings.data?.profiles.find((p) => p.name === name)?.services;
                setSelected(new Set(profileServices ?? selectable.map((s) => s.kind)));
              }}
              onManage={() => setProfilesDialog({})}
              onEdit={(profile) => setProfilesDialog({ profile })}
              onDelete={(name) => {
                if (window.confirm(`Delete the “${name}” profile?`)) deleteProfile.mutate(name);
              }}
              isUp={stack.isUp}
            />
          )}
          </div>
          <div className="mb-3 px-1">
            {installedCount === 0 && (
              <h2 id="services-heading" className="mb-1 text-base font-medium">
                Services
              </h2>
            )}
            <div className="flex flex-wrap items-center gap-2.5">
              <label className="flex cursor-pointer items-center gap-2 text-sm text-muted select-none">
                <input
                  type="checkbox"
                  className="size-4 accent-(--accent)"
                  checked={allSelected}
                  ref={(el) => {
                    if (el) el.indeterminate = chosen.length > 0 && !allSelected;
                  }}
                  onChange={() => setSelected(allSelected ? new Set() : new Set(selectable.map((s) => s.kind)))}
                  disabled={selectable.length === 0}
                />
                Select all
              </label>
              <span className="h-5 w-px bg-line ml-auto" aria-hidden />
              <span className="text-sm text-muted tabular-nums" aria-live="polite">
                {chosen.length} selected
              </span>
              {/* Only with something ticked: "Profile" saves a new one; with a profile of your own active and the ticks changed, it updates that profile. */}
              {chosen.length > 0 &&
                (stack.profile ? (
                  sameServices(chosen, stack.profile.services) ? null : (
                    <Button size="sm" variant="ghost" onClick={() => updateProfile.mutate({ name: stack.profile!.name, services: chosen })} disabled={updateProfile.isPending} title={`Save the ticked services as the ${stack.profile.name} profile`}>
                      <Save size={13} /> Update profile
                    </Button>
                  )
                ) : (
                  <Button size="sm" variant="ghost" onClick={() => setProfilesDialog({ initial: chosen })} title="Save the ticked services as a new profile">
                    <Save size={13} /> Profile
                  </Button>
                ))}
              <LayoutSwitch value={layout} onChange={setLayout} />
              {installedCount > 0 && <ProfilesTransfer menu />}
            </div>
          </div>

          <ul className={LAYOUTS.find((l) => l.id === layout)!.grid} aria-label="Services">
            {list.map((s, i) => (
              <li key={s.kind}>
                <ServiceRow
                  ref={(el) => {
                    rows.current[i] = el;
                  }}
                  service={s}
                  now={now}
                  sites={sites.data}
                  settings={settings.data}
                  ports={ports.data}
                  selected={selected.has(s.kind)}
                  onToggle={() => toggle(s.kind)}
                  tabIndex={i === focus ? 0 : -1}
                  onFocus={() => setFocus(i)}
                  inProfile={!!stack.profile?.services.includes(s.kind)}
                  profileActive={!!stack.profile}
                  onCopy={copy}
                  revealAt={revealed?.kind === s.kind ? revealed.at : undefined}
                  layout={layout}
                />
              </li>
            ))}
          </ul>
          <p className="mt-3 flex items-center gap-1.5 px-1 text-xs text-muted">
            <Keyboard size={13} aria-hidden /> Press <Kbd>?</Kbd> for keyboard shortcuts: <Kbd>J</Kbd>
            <Kbd>K</Kbd> move, <Kbd>Space</Kbd> select, <Kbd>Enter</Kbd> start or stop.
          </p>
        </section>

        <aside className="space-y-4" aria-label="Projects, shortcuts and connections">
          <ProjectList sites={sites.data?.sites ?? []} onNew={() => openNewProject(true)} />
          <QuickShortcuts sites={sites.data} services={services.data ?? []} settings={settings.data} />
          <ConnectPanel services={stack.usable} onCopy={copy} />
          <RecentActivity />
        </aside>
      </div>
    </div>

    {profilesDialog && <ProfilesDialog initial={profilesDialog.initial} profile={profilesDialog.profile} onClose={() => setProfilesDialog(null)} />}
  </>
  );
}

type ServiceLayout = "list" | "tiles" | "compact";

/** How Home lays out its services: rows (the default), big tiles three to a row, or compact tiles four to a row. Kept per device. */
const LAYOUTS: { id: ServiceLayout; label: string; icon: typeof LayoutList; grid: string }[] = [
  { id: "list", label: "List", icon: LayoutList, grid: "space-y-2.5" },
  { id: "tiles", label: "Tiles", icon: LayoutGrid, grid: "grid items-start gap-3 sm:grid-cols-2 lg:grid-cols-3" },
  { id: "compact", label: "Compact tiles", icon: Grid3x3, grid: "grid items-start gap-2.5 grid-cols-2 md:grid-cols-3 lg:grid-cols-4" },
];

const sameServices = (a: ServiceKind[], b: ServiceKind[]) => a.length === b.length && a.every((k) => b.includes(k));

/** One button that steps through the layouts; its icon is the current one. */
function LayoutSwitch({ value, onChange }: { value: ServiceLayout; onChange: (l: ServiceLayout) => void }) {
  const i = LAYOUTS.findIndex((l) => l.id === value);
  const next = LAYOUTS[(i + 1) % LAYOUTS.length];
  const Icon = LAYOUTS[i].icon;
  return (
    <Button size="sm" variant="ghost" onClick={() => onChange(next.id)} aria-label={`Layout: ${LAYOUTS[i].label}. Switch to ${next.label}`} title={`${LAYOUTS[i].label} · click for ${next.label.toLowerCase()}`}>
      <Icon size={14} />
    </Button>
  );
}

function ProfileTiles({
  profiles,
  active,
  onSelect,
  onManage,
  onEdit,
  onDelete,
  isUp,
}: {
  profiles: Settings["profiles"];
  active: string | null;
  onSelect: (name: string | null) => void;
  onManage: () => void;
  onEdit: (profile: Settings["profiles"][number]) => void;
  onDelete: (name: string) => void;
  /** Whether a profile's services (all services for []) are already running. */
  isUp: (services: ServiceKind[]) => boolean;
}) {
  const profileList = [{ name: null as string | null, label: "All services", services: [] as ServiceKind[] }, ...profiles.map((p) => ({ ...p, label: p.name }))];

  // One row of pills: picking one filters the list below and is what the header's Start
  // starts, so the cards' own Start buttons (and their height) are gone.
  return (
    // The section's heading: the pills are the services' presets, which needs no label of its own.
    <div className="flex flex-wrap items-center gap-x-3 gap-y-2 px-1">
      <h2 id="services-heading" className="text-base font-medium">
        Services
      </h2>
      <div className="flex min-w-0 flex-1 flex-wrap items-center gap-1.5" role="radiogroup" aria-label="Stack profiles">
        {profileList.map((profile) => {
          const selected = profile.name === active;
          const up = isUp(profile.services);
          const what = profile.name ? profile.services.map((k) => serviceLabels[k] ?? k).join(", ") : "Every installed service";
          return (
            <div
              key={profile.name ?? "all"}
              className={clsx(
                "group flex h-8 items-center rounded-full border text-sm transition-colors",
                selected ? "border-accent/60 bg-accent-soft/50 text-accent-text" : "border-line bg-surface text-fg hover:border-accent/40",
              )}
            >
              <button
                type="button"
                role="radio"
                aria-checked={selected}
                onClick={() => onSelect(profile.name)}
                title={`${what}${up ? " · running" : ""}`}
                className={clsx("flex h-full items-center gap-1.5 rounded-full pl-3", profile.name ? "pr-1" : "pr-3")}
              >
                {selected ? <Check size={13} strokeWidth={2.5} aria-hidden /> : <Layers size={13} className="text-muted" aria-hidden />}
                <span className="max-w-40 truncate font-medium">{profile.label}</span>
                {profile.name && <span className="text-xs text-muted tabular-nums">{profile.services.length}</span>}
                {up && <span className="size-1.5 rounded-full bg-ok" aria-label="Running" />}
              </button>
              {profile.name && (
                <span className="pr-1">
                  <Menu
                    label={`${profile.name} profile actions`}
                    plain
                    items={[
                      { label: "Edit profile", icon: <Pencil size={14} />, onSelect: () => onEdit(profile as Settings["profiles"][number]) },
                      { label: "Delete profile", icon: <Trash2 size={14} />, onSelect: () => onDelete(profile.name!) },
                    ]}
                  />
                </span>
              )}
            </div>
          );
        })}
        <button
          type="button"
          onClick={onManage}
          className="flex h-8 items-center gap-1 rounded-full border border-dashed border-line px-3 text-sm text-muted transition-colors hover:border-accent/60 hover:text-accent-text"
        >
          <Plus size={13} /> New profile
        </button>
      </div>
    </div>
  );
}

/** What "Open" does for a service: its site, inbox or database tool. */
function openAction(
  service: ServiceInfo,
  sites: SitesInfo | undefined,
  settings: Settings | undefined,
  _navigate: (p: "services") => void,
  catalog?: { runtimes: RuntimeInfo[] },
): { label: string; icon: typeof ExternalLink; run: () => void; always?: boolean } | null {
  if (isServer(service.kind)) {
    return { label: "Open in browser", icon: ExternalLink, run: () => openUrl(connection(service).url + "/").catch(notifyError) };
  }
  if (service.kind === "mariadb" || service.kind === "mysql" || service.kind === "postgres") {
    // The database's default app, chosen on the Services page (the ⋯ menu lists the others).
    const tool = preferredTool(service.kind, dbTools(service.kind, sites, settings), settings);
    if (!tool) return { label: `Install ${runtimeLabels.karotx ?? "KarotX"}`, icon: Table2, run: () => useAppStore.getState().openService("karotx"), always: true };
    return { label: `Open ${runtimeLabels[tool.tool]}`, icon: Table2, run: () => openUrl(tool.url).catch(notifyError) };
  }
  const client = preferredTool(service.kind, dbTools(service.kind, sites, settings), settings);
  if ((service.kind === "mongodb" || service.kind === "redis") && client) {
    return { label: `Open ${runtimeLabels[client.tool]}`, icon: Table2, run: () => openUrl(client.url).catch(notifyError) };
  }
  if (service.kind === "mongodb") {
    return settings?.versions.compass
      ? { label: "Open Compass", icon: Table2, run: () => api.openCompass().catch(notifyError) }
      : { label: `Install ${runtimeLabels.karotx ?? "KarotX"}`, icon: Table2, run: () => useAppStore.getState().openService("karotx"), always: true };
  }
  // Its own page, from its manifest (a web interface, a console).
  const link = serviceLink(infoFor(catalog, service.kind), sites, settings);
  if (link) {
    return { label: link.label, icon: ExternalLink, run: () => openUrl(link.url).catch(notifyError) };
  }
  return null;
}

function ServiceRow({
  ref,
  service,
  now,
  sites,
  settings,
  ports,
  selected,
  onToggle,
  tabIndex,
  onFocus,
  inProfile,
  profileActive,
  onCopy,
  revealAt,
  layout = "list",
}: {
  /** Set when Needs attention asks to show this row's fixes. */
  revealAt?: number;
  layout?: ServiceLayout;
  ref: (el: HTMLElement | null) => void;
  service: ServiceInfo;
  now: number;
  sites?: SitesInfo;
  settings?: Settings;
  ports?: PortStatus[];
  selected: boolean;
  onToggle: () => void;
  tabIndex: number;
  onFocus: () => void;
  inProfile?: boolean;
  profileActive?: boolean;
  onCopy: (text: string, what: string) => void;
}) {
  const catalog = useCatalog();
  const notify = useAppStore((s) => s.notify);
  const navigate = useAppStore((s) => s.navigate);
  const showLogs = useAppStore((s) => s.showLogs);
  const connMode = useAppStore((s) => s.connMode);
  const usage = useUsage((s) => s.series[service.kind]);
  const [expanded, setExpanded] = useState(false);
  useEffect(() => {
    if (revealAt) setExpanded(true);
  }, [revealAt]);
  // PHP's workers belong to no one service; the web servers show them, since that's
  // where a slow page's time goes.
  const php = useUsage((s) => (isServer(service.kind) ? s.latest.php : undefined));
  // A failed start shows on the row itself (ProblemPanel), so no toast for it.
  const start = useMutation({ mutationFn: () => api.startService(service.kind), onError: () => { } });
  const stop = useMutation({ mutationFn: () => api.stopService(service.kind), onError: notifyError });
  const restart = useMutation({ mutationFn: () => api.restartService(service.kind), onError: notifyError });
  // A service added from a file (or bundled, like KarotX) has no entry: the plain tile and
  // its manifest's blurb.
  const meta = serviceMeta[service.kind] ?? { icon: Package, tile: tileFor(service.kind).className, blurb: infoFor(catalog.data, service.kind)?.blurb ?? "" };
  const running = service.state === "running";
  // Spawned and waiting to answer (Elasticsearch, n8n… take up to a minute): it can be
  // stopped already.
  const loading = service.state === "starting" && service.pid != null;
  const busy = service.state === "starting" || service.state === "stopping" || restart.isPending || start.isPending || stop.isPending;
  const degraded = running && service.health?.status === "degraded";
  const conflict = conflictFor(service, ports);
  const hasIssues = !!(conflict || degraded || (service.problem && !running));
  const action = openAction(service, sites, settings, navigate, catalog.data);
  const conn = connection(service);
  const connValue = copyText(conn, connMode);

  const status = !service.installed
    ? { tone: "muted" as const, text: "Not installed" }
    : degraded
      ? { tone: "danger" as const, text: "Not responding" }
      : running
        ? { tone: "ok" as const, text: service.health?.status === "healthy" ? "Healthy" : "Running" }
        : service.problem?.port != null
          ? { tone: "danger" as const, text: `Port ${service.problem.port} in use` }
          : service.problem
            ? { tone: "danger" as const, text: "Failed" }
            : conflict
              ? { tone: "warn" as const, text: `Port ${conflict.port} in use` }
              : service.state === "starting" || service.state === "stopping"
                ? { tone: "warn" as const, text: service.state === "starting" ? "Starting" : "Stopping" }
                : { tone: "muted" as const, text: "Stopped" };

  const lastCpu = usage?.cpu?.length ? usage.cpu[usage.cpu.length - 1] : undefined;
  const lastMem = usage?.memory?.length ? usage.memory[usage.memory.length - 1] : undefined;

  // Every installed web tool for this database. Opening one here doesn't change the default,
  // which is chosen on the database's page in Services.
  const tools = dbTools(service.kind, sites, settings);
  const toolMenu =
    tools.length > 1
      ? tools.map((t) => ({
          label: `Open in ${runtimeLabels[t.tool]}${preferredTool(service.kind, tools, settings)?.tool === t.tool ? " (default)" : ""}`,
          icon: <Table2 size={15} />,
          onSelect: () => openUrl(t.url).catch(notifyError),
        }))
      : [];
  const dbMenu = [
    ...toolMenu,
    ...(service.kind === "mariadb" || service.kind === "mysql"
      ? [{ label: "Open command-line client", icon: <TerminalSquare size={15} />, onSelect: () => api.openDbClient(service.kind as DatabaseKind).catch(notifyError) }]
      : []),
  ];

  const tile = layout !== "list";
  const compact = layout === "compact";
  // Controls keep their own clicks, and selecting text isn't a click.
  const onRowClick = (e: React.MouseEvent) => {
    if ((e.target as HTMLElement).closest("button, a, input, select, label, [role=menu], [role=menuitem]")) return;
    if (window.getSelection()?.toString()) return;
    if (service.installed) setExpanded(!expanded);
  };
  const checkbox = (
    <input
      type="checkbox"
      className="size-4 shrink-0 accent-(--accent) disabled:opacity-30 cursor-pointer"
      checked={selected}
      onChange={onToggle}
      disabled={!service.installed}
      aria-label={`Select ${service.label}`}
      tabIndex={-1}
    />
  );
  // Running: the service's icon and name turn the accent colour in every layout.
  const iconBox = running ? "bg-accent-soft text-accent-text" : meta.tile;
  const nameTone = running ? "text-accent-text" : "text-fg";
  const startStop = !service.installed ? (
    <Button size="sm" variant="primary" onClick={() => useAppStore.getState().openService(service.kind)}>
      Install
    </Button>
  ) : service.external ? (
    <span className="px-2 text-xs text-muted" title="Started by the Ollama app; quit it there to stop it.">
      Ollama app
    </span>
  ) : running || loading || service.state === "stopping" ? (
    <>
      <Button size="sm" variant={tile ? "ghost" : "secondary"} onClick={() => stop.mutate()} disabled={busy && !loading} aria-label={`Stop ${service.label}`} title="Stop">
        <Square size={11} className="fill-current" /> Stop
      </Button>
      <Button size="sm" variant="ghost" onClick={() => restart.mutate()} disabled={busy} aria-label={`Restart ${service.label}`} title="Restart (R)">
        <RotateCw size={14} className={clsx(restart.isPending && "animate-spin")} />
      </Button>
    </>
  ) : (
    <Button size="sm" variant={tile ? "ghost" : "soft"} className={clsx(tile && "text-accent-text")} onClick={() => start.mutate()} disabled={busy} aria-label={`Start ${service.label}`} title="Start">
      <Play size={12} /> Start
    </Button>
  );
  const openButton = action && service.installed && (
    <Button size="sm" variant="ghost" onClick={action.run} disabled={!running && !action.always} aria-label={action.label} title={`${action.label} (O)`}>
      <action.icon size={14} />
    </Button>
  );
  const copyButton = service.installed && (
    <Button
      size="sm"
      variant="ghost"
      onClick={() => onCopy(connValue, `${service.label} ${connMode === "env" ? ".env" : "URL"}`)}
      title={connMode === "env" ? envText(conn.env) : `Copy ${conn.url} (C)`}
      aria-label={`Copy ${service.label} connection`}
    >
      <Copy size={13} className="text-muted" />
    </Button>
  );
  const menu = (
    <Menu
      plain={tile}
      items={[
        ...dbMenu,
        {
          label: service.kind === "mailpit" ? "Open data folder" : "Open config file",
          icon: <FileCog size={15} />,
          onSelect: () => openPath(service.configPath).catch(() => notify("error", "Created on first start")),
        },
        {
          label: "Open error log",
          icon: <FileText size={15} />,
          onSelect: () => openPath(service.logPath).catch(() => notify("error", "No log file yet")),
        },
        { label: "View logs", icon: <ScrollText size={15} />, onSelect: () => showLogs(service.kind) },
      ]}
    />
  );
  // Only rows fold out; tiles show their details only when something is wrong.
  const chevron = service.installed && !tile && (
    <button
      type="button"
      onClick={() => setExpanded(!expanded)}
      aria-expanded={expanded}
      aria-label={expanded ? `Collapse ${service.label} details` : `Expand ${service.label} details`}
      className={clsx("cursor-pointer rounded-lg p-1.5 text-muted transition-transform duration-200 hover:bg-surface-2 hover:text-fg", (expanded || hasIssues) && "rotate-180 text-accent-text")}
    >
      <ChevronDown size={15} />
    </button>
  );
  const selectName = (className: string) => (
    <button
      type="button"
      onClick={onToggle}
      disabled={!service.installed}
      tabIndex={-1}
      title={service.installed ? (selected ? `Unselect ${service.label}` : `Select ${service.label}`) : undefined}
      className={clsx("cursor-pointer rounded text-left text-[14px] font-medium hover:text-accent-text disabled:cursor-default", nameTone, className)}
    >
      {service.label}
    </button>
  );
  // Compact tiles: a running service's name opens it (its site, inbox or tool), underlined.
  const compactName =
    running && action ? (
      <button type="button" onClick={action.run} title={`${action.label} (O)`} className={clsx("flex max-w-full min-w-0 items-center gap-1 rounded text-left text-[14px] font-medium underline decoration-1 underline-offset-2 hover:decoration-2", nameTone)}>
        <span className="truncate">{service.label}</span>
        <ExternalLink size={12} className="shrink-0" aria-hidden />
      </button>
    ) : (
      selectName("block max-w-full truncate")
    );

  return (
    <article
      ref={ref}
      tabIndex={tabIndex}
      onFocus={(e) => e.target === e.currentTarget && onFocus()}
      onMouseDown={onFocus}
      aria-label={`${service.label}, ${status.text}${service.installed ? `, port ${service.port}` : ""}`}
      className={clsx(
        "rounded-xl border bg-surface transition-[border-color,background-color,box-shadow] duration-150 outline-none focus-visible:ring-2 focus-visible:ring-accent/50",
        selected ? "border-accent/50 bg-accent-soft/40" : "border-line shadow-card",
        !service.installed && "border-dashed",
        profileActive && !inProfile && "opacity-75",
      )}
    >
      {tile ? (
        // A click anywhere on a tile (but its controls) ticks it.
        <div
          className={clsx("flex flex-col", compact ? "gap-2 p-3" : "gap-3 p-4", service.installed && "cursor-pointer")}
          onClick={(e) => {
            if ((e.target as HTMLElement).closest("button, a, input, select, label, [role=menu], [role=menuitem]")) return;
            if (window.getSelection()?.toString()) return;
            if (service.installed) onToggle();
          }}
        >
          <div className="flex items-start gap-2.5">
            <div className={clsx("flex shrink-0 items-center justify-center rounded-lg transition-colors", compact ? "size-8" : "size-10", iconBox)} aria-hidden>
              <meta.icon size={compact ? 16 : 20} strokeWidth={1.75} />
            </div>
            <div className="min-w-0 flex-1">
              {compact ? compactName : selectName("block max-w-full truncate")}
              {compact ? (
                <>
                  <div className="mt-1 flex flex-wrap items-center gap-x-2 gap-y-0.5">
                    <Pill tone={status.tone}>{status.text}</Pill>
                    {service.installed && service.port > 0 && <span className="font-mono text-xs text-muted tabular-nums">:{service.port}</span>}
                  </div>
                  {/* One quiet line of detail: live usage while running, otherwise the version or what it is. */}
                  <p className="mt-1 truncate text-[11px] text-muted" title={running ? undefined : meta.blurb}>
                    {running && lastCpu !== undefined && lastMem !== undefined ? (
                      <span className="font-mono tabular-nums">
                        CPU <span className="text-fg">{formatCpu(lastCpu)}</span> · <span className="text-fg">{formatBytes(lastMem)}</span>
                      </span>
                    ) : service.version ? (
                      <span className="font-mono">v{service.version.replace(/^v/i, "")}</span>
                    ) : (
                      meta.blurb
                    )}
                  </p>
                </>
              ) : (
                <div className="mt-1 flex flex-wrap items-center gap-x-2 gap-y-1">
                  <Pill tone={status.tone}>{status.text}</Pill>
                  {service.installed && service.port > 0 && <span className="font-mono text-xs text-muted tabular-nums">:{service.port}</span>}
                </div>
              )}
            </div>
            {!compact && <span className="-mt-1.5 -mr-1">{openButton}</span>}
            {checkbox}
          </div>
          {!compact && (
            <p className="min-h-8 text-xs text-muted">
              {running && lastCpu !== undefined && lastMem !== undefined ? (
                <span className="font-mono tabular-nums">
                  CPU <span className="text-fg">{formatCpu(lastCpu)}</span> · RAM <span className="text-fg">{formatBytes(lastMem)}</span>
                  {service.version && <> · {service.version}</>}
                </span>
              ) : (
                <span className="line-clamp-2">
                  {meta.blurb}
                  {service.version && <span className="font-mono"> · {service.version}</span>}
                </span>
              )}
            </p>
          )}
          {/* Start/stop and restart on the left; copy and the menu on the right. */}
          <div className={clsx("-mx-1.5 flex items-center gap-0.5 border-t border-line/60", compact ? "pt-2" : "pt-2.5")}>
            {startStop}
            <span className="ml-auto flex items-center gap-0.5">
              {compact && running && openButton}
              {copyButton}
              {menu}
            </span>
          </div>
        </div>
      ) : (
      // The row: the name selects the service, the rest of the row opens its details.
      <div
        className={clsx("flex items-center gap-3 px-4 py-3", service.installed && "cursor-pointer")}
        onClick={onRowClick}
      >
        {checkbox}
        <div className={clsx("flex size-9 shrink-0 items-center justify-center rounded-lg transition-colors", iconBox)} aria-hidden>
          <meta.icon size={18} strokeWidth={1.75} />
        </div>
        <div className="min-w-0 flex-1 flex flex-wrap items-center gap-x-3 gap-y-1">
          {selectName("")}
          <Pill tone={status.tone}>{status.text}</Pill>
          {service.installed && service.port > 0 && (
            <span className="font-mono text-xs text-muted tabular-nums">:{service.port}</span>
          )}
          {service.version && <span className="font-mono text-xs text-muted hidden sm:inline">{service.version}</span>}
          {running && lastCpu !== undefined && lastMem !== undefined ? (
            <span className="hidden md:inline-flex items-center gap-3 font-mono text-xs text-muted tabular-nums">
              <span>CPU <span className="text-fg">{formatCpu(lastCpu)}</span></span>
              <span>RAM <span className="text-fg">{formatBytes(lastMem)}</span></span>
            </span>
          ) : (
            <span className="hidden lg:inline text-xs text-muted truncate max-w-xs">{meta.blurb}</span>
          )}
        </div>

        <div className="flex shrink-0 items-center gap-1.5">
          {startStop}
          {openButton}
          {copyButton}
          {menu}
          {chevron}
        </div>
      </div>
      )}

      {/* Expandable Details Drawer */}
      {service.installed && ((expanded && !tile) || hasIssues) && (
        <div className="border-t border-line/60 bg-surface-2/30 px-4 py-3 rounded-b-2xl space-y-3">
          <p className="text-xs text-muted">{meta.blurb}</p>

          <div className="flex flex-wrap items-end gap-x-6 gap-y-3">
            <Stat label="Port" value={service.port} live={running} />
            <Stat label="PID" value={service.pid ?? "—"} />
            <Stat label="Uptime" value={running && service.startedAt ? formatUptime(now - service.startedAt) : "—"} />
            {running && (
              <>
                <Sparkline label="CPU" values={usage?.cpu ?? []} format={formatCpu} floor={10} note="100% is one full CPU core" />
                <Sparkline label="RAM" values={usage?.memory ?? []} format={formatBytes} floor={64 * 1024 ** 2} />
                {php && (
                  <span title={`All PHP workers: ${php.processes} process${php.processes === 1 ? "" : "es"}`}>
                    <Stat label="PHP" value={`${formatCpu(php.cpu)} · ${formatBytes(php.memory)}`} />
                  </span>
                )}
              </>
            )}
            <button
              onClick={() => onCopy(connValue, `${service.label} ${connMode === "env" ? ".env" : "URL"}`)}
              className="group ml-auto flex max-w-full min-w-0 items-center gap-2 rounded-lg border border-line bg-surface px-2.5 py-1.5 text-left font-mono text-xs text-fg hover:border-accent/40 shadow-xs cursor-pointer"
              title={connMode === "env" ? envText(conn.env) : `Copy ${conn.url} (C)`}
              aria-label={`Copy ${service.label} connection ${connMode === "env" ? "as .env lines" : conn.url}`}
            >
              <span className="truncate">
                {connMode === "url" || conn.env.length === 0 ? conn.url : conn.env.length === 1 ? envText(conn.env) : `${conn.env[0][0]}=… +${conn.env.length - 1} more`}
              </span>
              <Copy size={13} className="shrink-0 text-muted group-hover:text-accent-text" />
            </button>
          </div>

          {conflict && (
            <div className="flex items-start gap-2.5 rounded-xl border border-line bg-surface-2/40 p-3 text-sm" role="status">
              <AlertTriangle size={16} className="mt-0.5 shrink-0 text-warn" aria-hidden />
              <div className="min-w-0 flex-1">
                <span className="font-medium">Port {conflict.port} is in use</span>
                <span className="text-muted">
                  {" "}
                  by {holder(conflict)}, so {service.label} can't start on it.
                  Stop that program, or pick another port in Settings → Ports.
                </span>
                {conflict.owner?.note && <p className="mt-1 text-xs text-muted">{conflict.owner.note}</p>}
                <div className="mt-2 flex flex-wrap gap-2">
                  <FreePort owner={conflict.owner} />
                  <Button size="sm" variant="ghost" onClick={() => navigate("settings")}>
                    Change port
                  </Button>
                </div>
              </div>
            </div>
          )}
          {degraded && (
            <div className="flex items-start gap-2.5 rounded-xl border border-line bg-surface-2/40 p-3 text-sm" role="status">
              <Pulse size={16} className="mt-0.5 shrink-0 text-warn" aria-hidden />
              <div>
                <span className="font-medium">The process is running but not answering.</span>{" "}
                <span className="text-muted">{service.health?.detail} Try Restart, or check its logs.</span>
              </div>
            </div>
          )}
          {service.problem && !running && (
            <ProblemPanel kind={service.kind} problem={service.problem} />
          )}
        </div>
      )}
    </article>
  );
}

function Stat({ label, value, live }: { label: string; value: string | number; live?: boolean }) {
  return (
    <div className="flex min-w-12 flex-col gap-0.5">
      <span className="text-xs font-medium text-muted">{label}</span>
      <span className={clsx("font-mono text-[13px] tabular-nums", live ? "font-medium text-accent-text" : "text-fg")}>{value}</span>
    </div>
  );
}

/** Windows: Apache and PHP need Microsoft's Visual C++ runtime. */
function VcRuntimeBanner() {
  const notify = useAppStore((s) => s.notify);
  const status = useQuery({ queryKey: ["vc-runtime"], queryFn: api.vcRuntimeStatus });
  const install = useMutation({
    mutationFn: api.vcRuntimeInstall,
    onSuccess: () => {
      notify("info", "Visual C++ runtime installed");
      status.refetch();
    },
    onError: notifyError,
  });
  if (!status.data?.applicable || status.data.installed) return null;
  return (
    <div className="flex flex-wrap items-center gap-4 rounded-2xl border border-warn/30 bg-warn/10 px-5 py-3.5">
      <p className="min-w-60 flex-1 text-sm">
        Apache and PHP need the Microsoft Visual C++ runtime, which isn't installed. Windows asks for administrator approval once.
      </p>
      <Button variant="primary" onClick={() => install.mutate()} disabled={install.isPending}>
        {install.isPending ? "Installing…" : "Install runtime"}
      </Button>
    </div>
  );
}

/**
 * A side panel whose body folds away under its title; the choice is kept per device. Home
 * showed everything at once, which buried the projects and the services people use most.
 */
function FoldCard({ id, defaultOpen, title, extra, children }: { id: string; defaultOpen: boolean; title: string; extra?: ReactNode; children: ReactNode }) {
  const [open, setOpen] = useState(() => {
    const saved = readPref(id);
    return saved === null ? defaultOpen : saved === "open";
  });
  const toggle = () => {
    setOpen(!open);
    writePref(id, open ? "closed" : "open");
  };
  return (
    <Card className="p-4">
      <div className={clsx("flex items-center justify-between gap-2 px-1", open && "mb-2")}>
        <button type="button" onClick={toggle} aria-expanded={open} className="flex items-center gap-1.5 rounded text-sm font-medium text-fg">
          <ChevronDown size={14} className={clsx("text-muted transition-transform", !open && "-rotate-90")} aria-hidden />
          <h2>{title}</h2>
        </button>
        {open && extra}
      </div>
      {open && children}
    </Card>
  );
}

/** One-click tools, each saying what it needs before it can open. */
function QuickShortcuts({ sites, services, settings }: { sites?: SitesInfo; services: ServiceInfo[]; settings?: Settings }) {
  const mailUi = settings?.ports.services["mailpit.ui"];
  const up = (kind: ServiceKind) => services.some((s) => s.kind === kind && s.state === "running");
  const web = primaryServer(settings);
  const db = up("mariadb") || up("mysql");

  const installed = (kind: ServiceKind) => services.some((s) => s.kind === kind && s.installed);
  const webTool = (tool: "phpmyadmin" | "adminer" | "phpredisadmin") => () => {
    const url = toolUrl(tool, sites, mailUi);
    if (url) openUrl(url).catch(notifyError);
  };
  const tools = [
    { tool: "mailpit" as const, label: "Mailpit inbox", ready: up("mailpit"), need: "Starts Mailpit, then opens", readyText: "Open inbox", installed: installed("mailpit"), open: () => openUrl(toolUrl("mailpit", sites, mailUi) ?? "http://localhost:8025/").catch(notifyError) },
    { tool: "phpmyadmin" as const, label: "phpMyAdmin", ready: up(web) && db, need: `Starts ${serviceLabels[web]} and a database, then opens`, readyText: "Browse databases", installed: !!toolUrl("phpmyadmin", sites, mailUi), open: webTool("phpmyadmin") },
    { tool: "adminer" as const, label: "Adminer", ready: up(web) && db, need: `Starts ${serviceLabels[web]} and a database, then opens`, readyText: "Lightweight database admin", installed: !!toolUrl("adminer", sites, mailUi), open: webTool("adminer") },
    { tool: "phpredisadmin" as const, label: "phpRedisAdmin", ready: up(web) && up("redis"), need: `Starts ${serviceLabels[web]} and Redis, then opens`, readyText: "Browse Redis keys", installed: !!toolUrl("phpredisadmin", sites, mailUi), open: webTool("phpredisadmin") },
  ];
  const readyCount = tools.filter((t) => t.installed && t.ready).length;

  return (
    <FoldCard id="home-shortcuts" defaultOpen={true} title="Quick shortcuts" extra={<span className="text-xs text-muted">{readyCount} of {tools.length} ready</span>}>
      <div className="space-y-1">
        {tools.map((t) => {
          const tiles = runtimeTiles[t.tool];
          const hint = !t.installed ? "Not installed" : t.ready ? t.readyText : t.need;
          return (
            <button
              key={t.tool}
              onClick={() => (t.installed ? t.open() : useAppStore.getState().openService(t.tool))}
              className="group flex w-full items-center gap-3 rounded-xl border border-transparent px-2 py-2 text-left hover:border-line hover:bg-surface-2/60 disabled:cursor-not-allowed disabled:opacity-70 disabled:hover:border-transparent disabled:hover:bg-transparent"
            >
              <span className={clsx("flex size-8 shrink-0 items-center justify-center rounded-lg", tiles.className)} aria-hidden>
                {tiles.icon(16)}
              </span>
              <span className="min-w-0 flex-1">
                <span className="block truncate text-sm font-medium">{t.label}</span>
                <span className={clsx("block truncate text-xs", t.installed && t.ready ? "font-medium text-accent-text" : "text-muted")}>{hint}</span>
              </span>
              <ChevronRight size={15} className="shrink-0 text-muted" aria-hidden />
            </button>
          );
        })}
      </div>
    </FoldCard>
  );
}

/** Connection details for the installed services, as URLs or ready-to-paste .env lines. */
function ConnectPanel({ services, onCopy }: { services: ServiceInfo[]; onCopy: (text: string, what: string) => void }) {
  const mode = useAppStore((s) => s.connMode);
  const setMode = useAppStore((s) => s.setConnMode);
  const [copied, setCopied] = useState<string | null>(null);
  // One web server's APP_URL is enough: the first one listed.
  const web = services.find((s) => isServer(s.kind));
  const shown = services.filter((s) => !isServer(s.kind) || s === web);
  if (shown.length === 0) return null;

  const copy = (key: string, text: string, what: string) => {
    onCopy(text, what);
    setCopied(key);
    setTimeout(() => setCopied((c) => (c === key ? null : c)), 1500);
  };
  const allEnv = shown.map((s) => `# ${s.label}\n${envText(connection(s).env)}`).join("\n\n");

  return (
    <FoldCard id="home-connect" defaultOpen={false}
        title="Connect your app"
        extra={
          <Segmented
            value={mode}
            onChange={setMode}
            className="p-0.5! text-xs"
            options={[
              { value: "url", label: "URL" },
              { value: "env", label: ".env" },
            ]}
          />
        }
      >
      <ul className="space-y-1.5">
        {shown.map((s) => {
          const c = connection(s);
          const text = copyText(c, mode);
          return (
            <li key={s.kind}>
              <button
                onClick={() => copy(s.kind, text, `${s.label} ${mode === "env" ? ".env" : "URL"}`)}
                className="group flex w-full items-start gap-2 rounded-lg border border-line bg-bg px-2.5 py-2 text-left hover:border-accent/40"
                aria-label={`Copy ${s.label} connection`}
              >
                <span className="min-w-0 flex-1">
                  <span className="block text-xs font-medium text-muted">{s.label}</span>
                  <span className="block font-mono text-xs break-all whitespace-pre-wrap text-fg">{text}</span>
                </span>
                {copied === s.kind ? (
                  <Check size={14} className="mt-0.5 shrink-0 text-accent-text" aria-hidden />
                ) : (
                  <Copy size={14} className="mt-0.5 shrink-0 text-muted group-hover:text-accent-text" aria-hidden />
                )}
              </button>
            </li>
          );
        })}
      </ul>
      <Button size="sm" variant="soft" className="mt-3 w-full" onClick={() => copy("all", allEnv, "all services as .env")}>
        {copied === "all" ? <Check size={14} /> : <Copy size={14} />} Copy everything as .env
      </Button>
    </FoldCard>
  );
}

/**
 * The most recent projects, at the top of the side column so the services keep the main
 * one: each opens its card on Sites, its site, or its folder.
 */
function ProjectList({ sites, onNew }: { sites: Site[]; onNew: () => void }) {
  const navigate = useAppStore((s) => s.navigate);
  const openSite = useAppStore((s) => s.openSite);
  const recent = [...sites].sort((a, b) => b.modified - a.modified).slice(0, 4);
  return (
    <FoldCard
      id="home-recent"
      defaultOpen={true}
      title={`Projects${sites.length ? ` · ${sites.length}` : ""}`}
      extra={
        <button onClick={() => navigate("sites")} className="text-xs font-medium text-accent-text hover:underline">
          All ›
        </button>
      }
    >
      {recent.length === 0 && <p className="px-1 py-2 text-sm text-muted">Each folder in www becomes a site.</p>}
      <ul className="space-y-0.5">
        {recent.map((site) => (
          <li key={site.path} className="group flex items-center gap-1 rounded-xl hover:bg-surface-2/60">
            <button type="button" onClick={() => openSite(site.path)} className="flex min-w-0 flex-1 items-center gap-3 px-2 py-2 text-left" title={`${site.name}: open its card on Sites`}>
              <span className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-surface-2 text-sm font-medium text-muted uppercase" aria-hidden>
                {site.name.slice(0, 1)}
              </span>
              <span className="min-w-0 flex-1">
                <span className="block truncate text-sm font-medium">{site.name}</span>
                <span className="block truncate text-xs text-muted">
                  {site.framework ?? site.language}
                  {site.gitBranch && ` · ${site.gitBranch}`}
                </span>
              </span>
            </button>
            <Button size="sm" variant="ghost" onClick={() => openUrl(site.url).catch(notifyError)} aria-label={`Open ${site.domain}`} title={`Open ${site.url}`}>
              <ExternalLink size={14} />
            </Button>
            <Button size="sm" variant="ghost" onClick={() => openPath(site.path).catch(notifyError)} aria-label={`Open ${site.name} folder`} title="Open the folder">
              <FolderOpen size={14} />
            </Button>
          </li>
        ))}
      </ul>
      <button
        onClick={onNew}
        className="mt-1.5 flex w-full items-center justify-center gap-1.5 rounded-xl border border-dashed border-line py-2 text-sm font-medium text-muted hover:border-accent/50 hover:bg-accent-soft/40 hover:text-accent-text"
        title="New project (N)"
      >
        <Plus size={14} /> New project
      </button>
    </FoldCard>
  );
}

const activityDot: Record<Activity["tone"], string> = {
  ok: "bg-ok",
  info: "bg-sky-500",
  muted: "bg-muted/50",
  error: "bg-danger",
};

function RecentActivity() {
  const activity = useAppStore((s) => s.activity);
  const navigate = useAppStore((s) => s.navigate);

  return (
    <FoldCard id="home-activity" defaultOpen={false}
        title="Recent activity"
        extra={
          <button onClick={() => navigate("logs")} className="text-xs font-medium text-accent-text hover:underline">
            Logs ›
          </button>
        }
      >
      <ul className="space-y-1.5 px-1">
        {activity.slice(0, 4).map((a) => (
          <li key={a.id} className="flex items-center gap-2.5 text-xs">
            <span className={clsx("size-2 shrink-0 rounded-full", activityDot[a.tone])} aria-hidden />
            <span className="font-mono text-muted tabular-nums">{formatClock(a.at)}</span>
            <span className="min-w-0 flex-1 truncate">{a.text}</span>
          </li>
        ))}
      </ul>
    </FoldCard>
  );
}
