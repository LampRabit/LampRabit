import { useEffect, useMemo, useRef, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { openPath } from "@tauri-apps/plugin-opener";
import { openUrl } from "../lib/open";
import { Search } from "lucide-react";
import clsx from "clsx";
import { api, type ServiceKind } from "../lib/api";
import { useCatalog } from "../lib/catalog";
import { pushRecent, readRecent, search, type PaletteItem } from "../lib/palette";
import { sections } from "../lib/settingsSections";
import { loadSettingsIndex } from "../lib/settingsIndex";
import { keysFor } from "../lib/shortcuts";
import { useStack } from "../lib/stack";
import { notifyError, useAppStore, type PageId } from "../lib/store";
import { useTheme } from "../lib/theme";
import { useAppUpdate } from "../lib/updates";
import Dialog, { Kbd } from "./Dialog";

const REPO = "https://github.com/LampRabit/LampRabit/blob/main";

/** LampRabit's documentation, found by title and the words people search with. */
const DOCS: { title: string; url: string; keywords: string[] }[] = [
  { title: "Services and service files", url: `${REPO}/docs/services.md`, keywords: ["manifest", "yaml", "add a service", "plugin"] },
  { title: "Command line", url: `${REPO}/docs/cli.md`, keywords: ["cli", "lamprabit command", "mcp", "terminal"] },
  { title: "Git in a project", url: `${REPO}/docs/git.md`, keywords: ["commit", "branch", "push", "pull", "stash"] },
  { title: "Project doctor", url: `${REPO}/docs/project-doctor.md`, keywords: ["health", "problems", "fix", "check"] },
  { title: "Project templates", url: `${REPO}/docs/templates.md`, keywords: ["new project", "laravel", "wordpress", "template"] },
  { title: "Moving from Laragon", url: `${REPO}/docs/website/migrate-from-laragon.md`, keywords: ["import", "laragon", "migrate"] },
  { title: "Moving from XAMPP", url: `${REPO}/docs/website/migrate-from-xampp.md`, keywords: ["import", "xampp", "migrate"] },
  { title: "Questions and answers", url: `${REPO}/docs/website/faq.md`, keywords: ["faq", "help", "question"] },
];

/** A project's tabs, for "blog: Git" and the like: [tab id, label, other words]. */
const PROJECT_TABS: [string, string, string[]][] = [
  ["git", "Git", ["commit", "branch", "push", "pull", "changes"]],
  ["commands", "commands and processes", ["artisan", "npm", "scripts", "queue", "cron", "processes"]],
  ["dependencies", "dependencies", ["composer", "packages", "npm", "outdated", "vendor"]],
  ["domains", "domains", ["alias", "subdomain", "url", "https"]],
  ["settings", "project settings", ["php version", "web server", "docroot", "public"]],
  ["overview", "health", ["doctor", "problems", "fix", "check"]],
];

const PAGES: { id: PageId; title: string; keywords: string[] }[] = [
  { id: "overview", title: "Home", keywords: ["dashboard", "overview", "status"] },
  { id: "sites", title: "Sites", keywords: ["projects", "domains"] },
  { id: "services", title: "Services", keywords: ["runtimes", "versions", "install"] },
  { id: "logs", title: "Logs", keywords: ["errors", "output", "dumps"] },
  { id: "settings", title: "Settings", keywords: ["preferences", "options"] },
];

/**
 * Ctrl/Cmd+K: one box for every action, project, service, setting, config file and docs
 * page. Enter runs the highlighted item; recently used ones come first.
 */
export default function CommandPalette() {
  const open = useAppStore((s) => s.paletteOpen);
  const setOpen = useAppStore((s) => s.setPaletteOpen);
  return (
    <Dialog open={open} onClose={() => setOpen(false)} title="Search everything" width="max-w-2xl" bare>
      {open && <Palette onClose={() => setOpen(false)} />}
    </Dialog>
  );
}

function Palette({ onClose }: { onClose: () => void }) {
  const items = useItems();
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const [recent] = useState(readRecent);
  const results = useMemo(() => search(items, query, recent), [items, query, recent]);
  const list = useRef<HTMLUListElement>(null);
  useEffect(() => setActive(0), [query]);
  useEffect(() => {
    list.current?.querySelector(`[data-index="${active}"]`)?.scrollIntoView?.({ block: "nearest" });
  }, [active]);

  const run = (item: PaletteItem | undefined) => {
    if (!item) return;
    pushRecent(item.id);
    onClose();
    item.run();
  };

  return (
    <div className="flex max-h-[70vh] min-h-0 flex-col">
      <div className="flex items-center gap-3 border-b border-line px-4">
        <Search size={18} className="shrink-0 text-muted" aria-hidden />
        <input
          data-autofocus
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "ArrowDown") setActive((i) => Math.min(results.length - 1, i + 1));
            else if (e.key === "ArrowUp") setActive((i) => Math.max(0, i - 1));
            else if (e.key === "Enter") run(results[active]);
            else return;
            e.preventDefault();
          }}
          placeholder="Search actions, projects, services, settings"
          aria-label="Search everything"
          role="combobox"
          aria-expanded="true"
          aria-controls="palette-results"
          aria-activedescendant={results[active] ? `palette-${results[active].id}` : undefined}
          className="h-14 min-w-0 flex-1 bg-transparent text-base text-fg outline-none placeholder:text-muted"
        />
      </div>
      <ul ref={list} id="palette-results" role="listbox" aria-label="Results" className="min-h-0 flex-1 overflow-y-auto p-2">
        {results.length === 0 && <li className="px-3 py-8 text-center text-sm text-muted">Nothing matches “{query.trim()}”</li>}
        {results.map((item, i) => (
          <li
            key={item.id}
            id={`palette-${item.id}`}
            data-index={i}
            role="option"
            aria-selected={i === active}
            onMouseMove={() => setActive(i)}
            onClick={() => run(item)}
            className={clsx("flex h-10 cursor-pointer items-center gap-3 rounded-lg px-3 text-sm", i === active ? "bg-surface-2 text-fg" : "text-fg")}
          >
            <span className="min-w-0 flex-1 truncate">
              {item.title}
              {item.hint && <span className="ml-2 font-mono text-xs text-muted">{item.hint}</span>}
            </span>
            {item.keys && (
              <span className="flex shrink-0 items-center gap-1">
                {item.keys.map((k) => (
                  <Kbd key={k}>{k}</Kbd>
                ))}
              </span>
            )}
            <span className="w-24 shrink-0 text-right text-xs text-muted">{item.group}</span>
          </li>
        ))}
      </ul>
      <div className="flex items-center gap-4 border-t border-line px-4 py-2 text-xs text-muted">
        <span className="flex items-center gap-1.5">
          <Kbd>↑</Kbd>
          <Kbd>↓</Kbd> move
        </span>
        <span className="flex items-center gap-1.5">
          <Kbd>Enter</Kbd> run
        </span>
        <span className="flex items-center gap-1.5">
          <Kbd>Esc</Kbd> close
        </span>
      </div>
    </div>
  );
}

/** Everything the palette offers, from what the app already loads. */
function useItems(): PaletteItem[] {
  const queryClient = useQueryClient();
  const store = useAppStore.getState();
  const stack = useStack();
  const { theme, setTheme } = useTheme();
  const catalog = useCatalog();
  const services = useQuery({ queryKey: ["services"], queryFn: api.listServices });
  const installed = useQuery({ queryKey: ["installed"], queryFn: api.installedRuntimes });
  const sites = useQuery({ queryKey: ["sites"], queryFn: api.listSites });
  const settings = useQuery({ queryKey: ["settings"], queryFn: api.getSettings });
  const configFiles = useQuery({ queryKey: ["all-config-files"], queryFn: api.allConfigFiles, staleTime: 60_000 });
  const settingRows = useQuery({ queryKey: ["settings-index"], queryFn: loadSettingsIndex, staleTime: Infinity });
  const php = installed.data?.find((r) => r.runtime === "php")?.active ?? null;
  const extensions = useQuery({ queryKey: ["php-extensions", php], queryFn: () => api.phpExtensions(php!), enabled: !!php, staleTime: 60_000 });

  return useMemo(() => {
    const out: PaletteItem[] = [];
    const action = (id: string, title: string, run: () => void, keywords: string[] = [], hint?: string) => out.push({ id, title, group: "Actions", keywords, hint, keys: keysFor(id), run });
    const refresh = () => queryClient.invalidateQueries({ queryKey: ["services"] });
    const fail = (p: Promise<unknown>) => p.then(refresh, notifyError);

    const keywordsOf = (id: string) => catalog.data?.runtimes.find((r) => r.id === id)?.keywords ?? [];
    for (const p of PAGES) out.push({ id: `page-${p.id}`, title: `Go to ${p.title}`, group: "Pages", keywords: p.keywords, keys: keysFor(`page-${p.id}`), run: () => store.navigate(p.id) });
    action("new-project", "New project", () => store.setNewProjectOpen(true), ["create", "laravel", "wordpress", "template"]);
    action("terminal", "Open terminal", () => api.openTerminal(null).catch(notifyError), ["shell", "console"]);
    action("www", "Open the www folder", () => api.envPaths().then((p) => openPath(p.www)).catch(notifyError), ["folder", "explorer", "finder"]);
    action("start-all", stack.profile ? `Start ${stack.profile.name}` : "Start all services", stack.startStack, ["stack", "everything"]);
    if (stack.stoppable.length) action("stop-all", "Stop all services", stack.stopAll, ["stack", "everything"]);
    if (stack.running.length) action("restart-running", "Restart the running services", stack.restartRunning);
    action("shortcuts", "Keyboard shortcuts", () => store.setShortcutsOpen(true), ["keys", "hotkeys", "help"]);
    action("diagnostics", "Run diagnostics", () => store.setDiagnosticsOpen(true), ["report", "debug", "problem", "issue"]);
    action("setup-guide", "Open the setup guide", () => store.setSetupGuideOpen("guide"), ["onboarding", "install", "recommended"]);
    action("import", "Import projects", () => store.setSetupGuideOpen("import"), ["laragon", "xampp", "herd", "copy"]);
    action("theme", theme === "dark" ? "Use the light theme" : "Use the dark theme", () => setTheme(theme === "dark" ? "light" : "dark"), ["dark mode", "light mode", "appearance"]);
    action("updates", "Check for updates", () => useAppUpdate.getState().checkNow().catch(notifyError), ["upgrade", "new version"]);
    action("problems", "Show errors in the logs", () => store.navigate("logs"), ["errors", "warnings", "problems"]);

    for (const site of sites.data?.sites ?? []) {
      const words = [site.domain, site.framework ?? "", site.language];
      out.push({ id: `site-${site.path}`, title: site.name, group: "Projects", hint: site.domain, keywords: words, run: () => store.openSite(site.path) });
      out.push({ id: `site-open-${site.path}`, title: `Open ${site.name} in the browser`, group: "Projects", keywords: [...words, "browser", "visit"], run: () => openUrl(site.url).catch(notifyError) });
      out.push({ id: `site-editor-${site.path}`, title: `Open ${site.name} in the editor`, group: "Projects", keywords: [...words, "code", "vscode", "ide"], run: () => api.openInEditor(site.path).catch(notifyError) });
      out.push({ id: `site-terminal-${site.path}`, title: `Terminal in ${site.name}`, group: "Projects", keywords: [...words, "shell", "console"], run: () => api.openTerminal(site.path).catch(notifyError) });
      // Straight to one of its tabs.
      for (const [tab, label, more] of PROJECT_TABS) {
        out.push({ id: `site-${tab}-${site.path}`, title: `${site.name}: ${label}`, group: "Projects", keywords: [...words, ...more], run: () => store.openSite(site.path, tab) });
      }
      out.push({ id: `site-logs-${site.path}`, title: `${site.name}: logs`, group: "Projects", keywords: [...words, "output", "errors"], run: () => store.showLogs(`project:${site.name}`) });
      if (site.app) {
        out.push({ id: `site-start-${site.path}`, title: `Start the ${site.name} dev server`, group: "Projects", keywords: [...words, "run", "serve", "npm run dev"], run: () => api.appStart(site.name).catch(notifyError) });
        out.push({ id: `site-stop-${site.path}`, title: `Stop the ${site.name} dev server`, group: "Projects", keywords: [...words, "kill"], run: () => api.appStop(site.name).catch(notifyError) });
      }
    }
    // Web tools (phpMyAdmin, Adminer, KarotX, Mailpit…) at their own addresses.
    if (sites.data?.hostsOk) {
      for (const t of sites.data.tools) out.push({ id: `tool-${t.name}`, title: `Open ${catalog.data?.runtimes.find((r) => r.id === t.name)?.label ?? t.name}`, group: "Tools", hint: t.domain, keywords: [t.name, ...keywordsOf(t.name), "browser"], run: () => openUrl(t.url).catch(notifyError) });
    }

    for (const s of services.data ?? []) {
      if (!s.installed) continue;
      const kind = s.kind as ServiceKind;
      const words = [s.kind, ...keywordsOf(s.kind)];
      const running = s.state === "running" || s.state === "starting";
      if (!s.external) {
        if (running) out.push({ id: `stop-${kind}`, title: `Stop ${s.label}`, group: "Services", keywords: words, hint: "running", run: () => fail(api.stopService(kind)) });
        else out.push({ id: `start-${kind}`, title: `Start ${s.label}`, group: "Services", keywords: words, hint: "stopped", run: () => fail(api.startService(kind)) });
        if (running) out.push({ id: `restart-${kind}`, title: `Restart ${s.label}`, group: "Services", keywords: words, run: () => fail(api.restartService(kind)) });
      }
      out.push({ id: `logs-${kind}`, title: `${s.label} logs`, group: "Services", keywords: [...words, "errors", "output"], run: () => store.showLogs(kind) });
    }
    // Every runtime and service: its versions and settings on the Services page, or its install.
    const has = (id: string) => (installed.data ?? []).some((r) => r.runtime === id && r.versions.length > 0);
    for (const r of catalog.data?.runtimes ?? []) {
      if (r.unavailable) continue;
      out.push({ id: `service-${r.id}`, title: has(r.id) ? `${r.label} versions and settings` : `Install ${r.label}`, group: "Services", hint: has(r.id) ? undefined : r.blurb, keywords: [r.id, ...r.keywords], run: () => store.openService(r.id) });
    }

    // Another installed version of a runtime: switch to it.
    for (const r of installed.data ?? []) {
      const label = catalog.data?.runtimes.find((c) => c.id === r.runtime)?.label ?? r.runtime;
      for (const v of r.versions.filter((v) => v !== r.active)) {
        out.push({ id: `use-${r.runtime}-${v}`, title: `Use ${label} ${v}`, group: "Services", hint: r.active ? `now ${r.active}` : undefined, keywords: [r.runtime, "switch", "version", "change"], run: () => api.setActiveVersion(r.runtime, v).then(() => queryClient.invalidateQueries(), notifyError) });
      }
    }
    for (const p of settings.data?.profiles ?? []) {
      out.push({ id: `profile-${p.name}`, title: `Start the ${p.name} profile`, group: "Profiles", hint: p.services.join(", "), keywords: ["profile", "stack", ...p.services], run: () => stack.startProfile(p.services) });
    }

    // PHP's extensions (the active version): turn one on or off, or download it.
    if (php) {
      out.push({ id: "php-extensions", title: "PHP extensions", group: "PHP", hint: `PHP ${php}`, keywords: ["extension", "ext", "module", "xdebug", "php.ini"], run: () => store.openExtensions(true) });
      const toggle = (name: string, on: boolean, install: boolean) =>
        (install ? api.installPhpExtension(php, name) : api.setPhpExtension(php, name, on)).then(() => {
          store.notify("info", `PHP ${php}: ${name} ${on ? "on" : "off"}; the web servers restart to apply it`);
          queryClient.invalidateQueries({ queryKey: ["php-extensions"] });
        }, notifyError);
      for (const e of extensions.data ?? []) {
        const words = ["extension", "ext", "php", e.description ?? ""];
        if (e.installed) out.push({ id: `ext-${e.name}`, title: `${e.enabled ? "Turn off" : "Turn on"} ${e.name}`, group: "PHP extensions", hint: e.enabled ? "on" : "off", keywords: [...words, e.enabled ? "disable" : "enable"], run: () => toggle(e.name, !e.enabled, false) });
        else if (e.available) out.push({ id: `ext-${e.name}`, title: `Install ${e.name}`, group: "PHP extensions", hint: e.description ?? e.available, keywords: [...words, "enable", "download", "pecl"], run: () => toggle(e.name, true, true) });
      }
    }

    for (const sec of sections) out.push({ id: `settings-${sec.id}`, title: `${sec.label} settings`, group: "Settings", keywords: sec.keywords, run: () => store.openSettings(sec.id) });
    // Each setting on its own, found by its name and its explanation.
    for (const row of settingRows.data ?? []) {
      const section = sections.find((s) => s.id === row.section);
      out.push({ id: `setting-${row.section}-${row.label}`, title: row.label, group: "Settings", hint: section?.label, keywords: [row.hint, ...(section?.keywords ?? [])], run: () => store.openSettings(`${row.section}#${row.label}`) });
    }

    for (const f of (configFiles.data ?? []).filter((f) => f.exists)) {
      out.push({ id: `config-${f.path}`, title: f.label, group: "Config files", hint: f.owner, keywords: [f.path, f.owner, "config", "edit", "where is"], run: () => api.openInEditor(f.path).catch(notifyError) });
    }

    for (const d of DOCS) out.push({ id: `docs-${d.url}`, title: d.title, group: "Docs", keywords: ["docs", "documentation", "how to", ...d.keywords], run: () => openUrl(d.url).catch(notifyError) });
    return out;
  }, [queryClient, store, stack, theme, setTheme, catalog.data, services.data, installed.data, sites.data, settings.data, configFiles.data, settingRows.data, php, extensions.data]);
}
