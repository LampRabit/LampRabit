/**
 * The real app with a fake backend, for layout checks in a plain browser: every Tauri
 * command answers from realistic fixtures (default settings and the catalog come from the
 * backend: `cargo test --lib write_ui_fixtures -- --ignored`). Query parameters pick what
 * to show: ?page=sites&tab=git&gitView=history&theme=dark&width=…
 * Run `node scripts/screenshots/shoot.mjs` to capture every view with headless Edge/Chrome.
 */
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import fixtures from "./fixtures.json";

const q = new URLSearchParams(location.search);
const now = Math.floor(Date.now() / 1000);
const settings = structuredClone(fixtures.settings) as Record<string, unknown> & { versions: Record<string, string>; sites: Record<string, unknown> };
settings.rootDir = "C:\\lampRabit";
settings.versions = { ...settings.versions, apache: "2.4.65", php: "8.4.14", mariadb: "11.8.3", mysql: "8.4.7", mailpit: "1.27.10", redis: "8.2.2", composer: "2.8.12", phpmyadmin: "5.2.3" };

const site = (name: string, framework: string | null, language: string, extra: Record<string, unknown> = {}) => ({
  name,
  domain: `${name.toLowerCase()}.test`,
  path: `C:\\lampRabit\\www\\${name}`,
  docroot: `C:\\lampRabit\\www\\${name}\\public`,
  ssl: true,
  url: `https://${name.toLowerCase()}.test`,
  modified: Date.now() - 600_000,
  tool: null,
  proxy: null,
  app: null,
  nodeProject: language === "node",
  language,
  serveMode: "dynamic",
  route: null,
  ports: [],
  routePort: null,
  php: language === "php" ? "8.4.14" : null,
  server: "apache",
  node: null,
  framework,
  gitBranch: "master",
  database: framework === "laravel" ? "dummy_project" : null,
  versionRequest: null,
  languageVersion: null,
  ...extra,
});
const sites = [site("dummy-project", "laravel", "php"), site("LampVites", "vite", "node"), site("MyWordpressSite", "wordpress", "php", { gitBranch: null })];

const service = (kind: string, label: string, state: string, port: number, version: string) => ({
  kind,
  label,
  state,
  pid: state === "running" ? 4242 : null,
  port,
  version,
  installed: true,
  startedAt: state === "running" ? Date.now() - 3_600_000 : null,
  configPath: `C:\\lampRabit\\etc\\${kind}`,
  logPath: `C:\\lampRabit\\logs\\${kind}-error.log`,
  health: state === "running" ? { status: "healthy", detail: null, checkedAt: Date.now() } : null,
  problem: null,
  external: false,
});
const services = [
  service("apache", "Apache", "running", 80, "2.4.65"),
  service("mariadb", "MariaDB", "stopped", 3306, "11.8.3"),
  service("mysql", "MySQL", "running", 3307, "8.4.7"),
  service("mailpit", "Mailpit", "stopped", 8025, "1.27.10"),
  service("redis", "Redis", "stopped", 6379, "8.2.2"),
];

// Logs: MySQL's own ISO lines from days ago, repeats to fold, an error, project output.
const day = (d: number, t: string) => new Date(Date.now() - d * 86_400_000).toISOString().slice(0, 10) + t;
const logLines = [
  ...Array.from({ length: 6 }, (_, i) => ({ service: "mysql", stream: "output", line: `${day(11, `T12:17:1${i}.188184Z`)} 0 [System] [MY-015015] [Server] MySQL Server - start.` })),
  { service: "mysql", stream: "output", line: `${day(11, "T12:17:20.000000Z")} 0 [Warning] [MY-010068] [Server] CA certificate ca.pem is self signed.` },
  { service: "mysql", stream: "error", line: `${day(1, "T09:00:00.000000Z")} 1 [ERROR] [MY-012574] [InnoDB] Unable to lock ./ibdata1 error: 11` },
  { service: "apache", stream: "error", line: `[${new Date().toDateString().replace(/^(\w+) (\w+) (\d+) (\d+)$/, "$1 $2 $3 10:00:00.000000 $4")}] [core:warn] [pid 1:tid 2] AH00098: pid file overwritten` },
  { service: "project", project: "dummy-project", stream: "output", line: `[${day(0, " 10:00:00")}] output: Server running on http://127.0.0.1:8000` },
  ...Array.from({ length: 40 }, (_, i) => ({ service: "apache", stream: "access", line: `127.0.0.1 - - [12/Nov/2026:10:${String(i).padStart(2, "0")}:00 +0530] "GET /api/items/${i} HTTP/1.1" 200 ${100 + i}` })),
].map((l, i) => ({ id: 0, ...l, _i: i }));

const gitFiles = [".gitignore", ".oxlintrc.json", "README.md", "bun.lock", "index.html", "package.json", "public/favicon.svg", "src/App.css", "src/App.tsx", "src/assets/hero.png", "src/main.tsx"].map((path) => ({ path, from: null, staged: "?", unstaged: "?", conflicted: false }));
const gitStatus = {
  repo: true,
  branch: "master",
  head: q.get("gitNew") ? null : "2e8a8fdc2a4f",
  upstream: q.get("gitNew") ? null : "origin/master",
  ahead: Number(q.get("ahead") ?? 0),
  behind: 0,
  files: q.get("clean") ? [] : gitFiles,
  operation: null,
  identity: { name: "shivam-sv", email: "dev@example.com", local: false },
  remotes: q.get("noRemote") ? [] : [{ name: "origin", url: "https://github.com/acme/lampvites.git", pushUrl: null }],
};
const readme = ["@@ -0,0 +1,12 @@", "+# React + TypeScript + Vite", "+", "+This template provides a minimal setup to get React working in Vite with HMR and some ESLint rules.", "+", "+Currently, two official plugins are available:", "+", "+- [@vitejs/plugin-react](https://github.com/vitejs/vite-plugin-react) uses Babel", "+- [@vitejs/plugin-react-swc](https://github.com/vitejs/vite-plugin-react-swc) uses SWC", "+", "+## React Compiler", "+", "+The React Compiler is not enabled on this template because of its impact on dev & build performances."].join("\n");
const edit = ["@@ -1,7 +1,8 @@", " root = true", " ", " [*]", "-charset = latin1", "+charset = utf-8", "+end_of_line = lf", " indent_size = 4", " indent_style = space", " insert_final_newline = true"].join("\n");

const answers: Record<string, unknown> = {
  runtime_info: fixtures.catalog,
  get_settings: settings,
  take_startup_warnings: [],
  env_paths: { root: "C:\\lampRabit", www: "C:\\lampRabit\\www", logs: "C:\\lampRabit\\logs", settingsFile: "C:\\Users\\dev\\AppData\\Roaming\\lamprabit\\settings.toml", os: q.get("os") ?? "windows" },
  list_services: services,
  list_sites: { sites, tools: [], tld: "test", needsApply: false, hostsOk: true, manageHosts: true, caTrusted: true, caPath: "C:\\lampRabit\\ssl\\ca.pem", unserved: [], shadowedTools: [] },
  php_status: [{ version: "8.4.14", port: 9084, workers: 4, alive: 4 }],
  list_templates: { templates: [], problems: [] },
  app_statuses: [],
  project_commands: { commands: [], problems: [], sources: [] },
  project_facts: {
    package: "laravel/laravel",
    description: "The skeleton application for the Laravel framework.",
    version: null,
    frameworkVersion: "13.38.0",
    phpRequirement: "^8.3",
    nodeRequirement: null,
    dependencies: [{ manager: "composer", count: 12, devCount: 8, installed: true, outdated: false }],
    envFile: ".env",
    env: [["APP_ENV", "local"], ["DB_CONNECTION", "mysql"], ["DB_PORT", "3306"]],
    git: { remote: "https://github.com/acme/shop.git", commit: ["2e8a8fd", "Some changes", now - 660], changes: 11 },
    template: "laravel@1",
  },
  database_status: { engine: "mariadb", label: "MariaDB", database: "dummy_project", installed: false, running: false, exists: null, port: 3306, reason: "The project connects to port 3306, which is MariaDB's in LampRabit (MySQL uses 3307).", alternative: { engine: "mysql", label: "MySQL", port: 3307 } },
  project_doctor: q.get("healthy")
    ? []
    : [
        { id: "php-version", severity: "error", title: "PHP 8.1.31 doesn't match the project", detail: "composer.json asks for PHP ^8.3.", fix: { label: "Use PHP 8.4.14", preview: "Sets dummy-project's PHP to 8.4.14 (other projects keep theirs) and restarts the web server.", action: { kind: "setPhp", version: "8.4.14" } } },
        { id: "db-missing", severity: "error", title: "MariaDB isn't installed", detail: "The project connects to port 3306, which is MariaDB's in LampRabit (MySQL uses 3307).", fix: { label: "Use MySQL", preview: "Points the project at MySQL (port 3307) in its config.", action: { kind: "switchDatabase", engine: "mysql" } } },
        { id: "deps-outdated-composer", severity: "warning", title: "Composer dependencies are out of date", detail: "The lock file changed since vendor/ was installed (a pull or a branch switch), so the code may expect packages that aren't there.", fix: { label: "Run composer install", preview: "Runs composer install in the project (its output shows under Commands).", action: { kind: "runCommand" } } },
        { id: "git-identity", severity: "info", title: "Git doesn't know who you are", detail: "Commits will fail until Git has a name and email: set them in the Git tab.", fix: null },
      ],
  mcp_status: {
    running: q.get("mcpOn") === "1",
    url: "http://127.0.0.1:39813/mcp",
    token: "3f9c2a7be41d08c6a5e19f7d0b2c4e8a",
    command: String.raw`C:\Program Files\LampRabit\LampRabit.exe`,
    args: ["--cli", "mcp"],
    clients: [
      { id: "claude-desktop", name: "Claude Desktop", installed: true, connected: true, path: String.raw`C:\Users\dev\AppData\Roaming\Claude\claude_desktop_config.json`, stale: false, note: null, unsupported: null },
      { id: "claude-code", name: "Claude Code", installed: true, connected: false, path: String.raw`C:\Users\dev\.claude.json`, stale: false, note: null, unsupported: null },
      { id: "codex", name: "Codex", installed: true, connected: false, path: String.raw`C:\Users\dev\.codex\config.toml`, stale: false, note: "The Codex app, CLI and IDE extension", unsupported: null },
      { id: "cursor", name: "Cursor", installed: true, connected: false, path: null, stale: false, note: null, unsupported: null },
      { id: "vscode", name: "VS Code", installed: true, connected: false, path: null, stale: false, note: "Copilot's agent mode", unsupported: null },
      { id: "lm-studio", name: "LM Studio", installed: true, connected: false, path: null, stale: false, note: "Local models, Ollama's included", unsupported: null },
      { id: "windsurf", name: "Windsurf", installed: false, connected: false, path: null, stale: false, note: null, unsupported: null },
      { id: "kiro", name: "Kiro", installed: false, connected: false, path: null, stale: false, note: null, unsupported: null },
      { id: "chatgpt", name: "ChatGPT", installed: false, connected: false, stale: false, path: null, note: null, unsupported: "ChatGPT's connectors reach only MCP servers on the internet, and LampRabit's stays on this computer. Codex, OpenAI's coding app, runs local ones: connect Codex." },
      { id: "ollama", name: "Ollama", installed: false, connected: false, stale: false, path: null, note: null, unsupported: "Ollama runs models but has no MCP client of its own. Use its models from an app that has one, such as LM Studio, Cline or opencode, and connect that app." },
    ],
  },
  git_version: "2.47.1",
  git_status: gitStatus,
  git_diff: (args: { path: string }) => (args.path === "README.md" ? readme : edit),
  git_branches: [
    { name: "master", remote: false, current: true, upstream: "origin/master", track: "", time: now - 660, subject: "Some changes" },
    { name: "feature/cart", remote: false, current: false, upstream: null, track: "", time: now - 86_400, subject: "Cart totals" },
    { name: "origin/master", remote: true, current: false, upstream: null, track: "", time: now - 660, subject: "Some changes" },
  ],
  git_log: [
    { hash: "2e8a8fdc2a4f05d15f9a1cf41362b49710eaaa9c", short: "2e8a8fd", author: "shivam-sv", email: "dev@example.com", time: now - 660, refs: "HEAD -> master, origin/master", subject: "Some changes" },
    { hash: "91ec0cd00000000000000000000000000000000a", short: "91ec0cd", author: "shivam-sv", email: "dev@example.com", time: now - 720, refs: "tag: v0.1.0", subject: "Some commit" },
  ],
  git_commit_details: {
    hash: "2e8a8fdc2a4f05d15f9a1cf41362b49710eaaa9c",
    parents: ["91ec0cd00000000000000000000000000000000a"],
    author: "shivam-sv",
    authorEmail: "dev@example.com",
    authorTime: now - 660,
    committer: "shivam-sv",
    committerEmail: "dev@example.com",
    committerTime: now - 660,
    refs: "HEAD -> master, origin/master",
    subject: "Some changes",
    body: "Adds the editor config and the cart controller.",
    files: [
      { path: ".editorconfig", from: null, status: "A", additions: 18, deletions: 0, binary: false },
      { path: "app/Http/Controllers/CartController.php", from: null, status: "M", additions: 42, deletions: 7, binary: false },
      { path: "public/logo.png", from: null, status: "M", additions: 0, deletions: 0, binary: true },
    ],
  },
  git_commit_file_diff: edit,
  git_repo_settings: { pull: null, prune: false, autoSetupRemote: false },
  installed_runtimes: Object.entries(settings.versions).map(([runtime, active]) => ({ runtime, active, versions: [active] })),
  runtime_catalog: [{ runtime: "php", version: "8.4.14", url: "https://example.com/php.zip", sha256: null, archive: "zip", label: null }],
  runtime_requirements: [],
  runtime_updates: [],
  node_status: { manager: "nvm-windows", managerVersion: "1.2.2", current: "24.11.1", installed: ["24.11.1"], needsElevation: false },
  node_available: [],
  service_usage: services.map((s) => ({ id: s.kind, cpu: s.state === "running" ? 1.2 : 0, memory: s.state === "running" ? 120_000_000 : 0, processes: 1 })),
  port_status: [],
  log_history: { lines: logLines, cursor: 0 },
  list_dumps: [],
  tunnel_list: [],
  ngrok_token_saved: false,
  vc_runtime_status: { applicable: false, installed: true, version: null },
  first_run: q.get("firstRun") === "1",
  // Every component downloads, except the ones installed already.
  starter_plan: ({ runtimes }: { runtimes: string[] }) =>
    runtimes.map((runtime) => ({
      runtime,
      label: ({ php: "PHP", apache: "Apache", mariadb: "MariaDB", composer: "Composer", mailpit: "Mailpit", phpmyadmin: "phpMyAdmin", git: "Git", redis: "Redis", postgres: "PostgreSQL" } as Record<string, string>)[runtime] ?? runtime,
      blurb: "",
      action:
        runtime in settings.versions
          ? { kind: "ready", version: (settings.versions as Record<string, string>)[runtime] }
          : { kind: "install", release: { runtime, version: "8.4.14", url: "", sha256: null, archive: "zip", label: null } },
      size: 34_000_000,
    })),
  import_sources: [
    {
      id: "laragon",
      name: "Laragon",
      folders: [String.raw`C:\laragon\www`],
      projects: [
        { name: "crm", path: String.raw`C:\laragon\www\crm`, framework: "Laravel", database: "crm", target: "crm" },
        { name: "lampvites", path: String.raw`C:\laragon\www\lampvites`, framework: "Laravel", database: null, target: "lampvites-2" },
        { name: "landing page", path: String.raw`C:\laragon\www\landing page`, framework: "Static", database: null, target: "landing-page" },
      ],
      databases: ["crm", "wp_blog"],
    },
    {
      id: "xampp",
      name: "XAMPP",
      folders: [String.raw`C:\xampp\htdocs`],
      projects: [{ name: "blog", path: String.raw`C:\xampp\htdocs\blog`, framework: "WordPress", database: "wp_blog", target: "blog" }],
      databases: [],
    },
  ],
  ollama_elsewhere: null,
  app_info: { version: "0.2.0" },
  project_folders: [],
  command_history: [],
  php_extensions: [],
  list_backups: [],
  list_databases: [],
  project_file: null,
  ports_listening: [],
  check_port: true,
};

mockWindows("main");
mockIPC(
  (cmd, args) => {
    if (cmd.startsWith("plugin:")) return cmd.includes("version") ? "0.2.0" : null;
    const a = answers[cmd];
    return typeof a === "function" ? (a as (x: unknown) => unknown)(args) : (a ?? null);
  },
  { shouldMockEvents: true },
);

// View options, applied before the app renders.
const theme = q.get("theme");
if (theme) document.documentElement.dataset.theme = theme;
localStorage.clear();
if (q.get("all")) localStorage.setItem("services-scope", "all");
if (q.get("nav")) localStorage.setItem("navPosition", q.get("nav")!);

const { useAppStore } = await import("../../src/lib/store");
const page = q.get("page");
if (page) useAppStore.setState({ page: page as never });
const guide = q.get("guide");
if (guide === "guide" || guide === "import") useAppStore.setState({ setupGuideOpen: guide });
if (q.get("templateEditor")) useAppStore.setState({ templateEditor: {} });
await import("../../src/main");

// Click through to deeper views once the page has drawn: a tab, then a Git view.
const click = async (match: (el: HTMLElement) => boolean) => {
  for (let i = 0; i < 50; i++) {
    const el = [...document.querySelectorAll<HTMLElement>("button, [role=tab]")].find(match);
    if (el) {
      el.click();
      return;
    }
    await new Promise((r) => setTimeout(r, 100));
  }
};
const tab = q.get("tab");
if (tab) await click((el) => el.getAttribute("role") === "tab" && el.textContent?.trim().toLowerCase() === tab);
const gitView = q.get("gitView");
if (gitView) await click((el) => el.getAttribute("role") === "tab" && !!el.textContent?.toLowerCase().startsWith(gitView));
const file = q.get("file");
if (file) await click((el) => el.textContent?.includes(file) ?? false);
if (q.get("filters")) await click((el) => el.textContent?.trim().startsWith("Filters") ?? false);
// macOS draws its traffic lights over the title bar; stand-ins where trafficLightPosition puts them
// (x 18, y 18; 12px buttons 20px apart) show whether they clear the logo.
if (q.get("os") === "macos") {
  const lights = document.createElement("div");
  lights.style.cssText = "position:fixed;left:18px;top:18px;display:flex;gap:8px;z-index:9999;pointer-events:none";
  for (const c of ["#ff5f57", "#febc2e", "#28c840"]) lights.insertAdjacentHTML("beforeend", `<span style="width:12px;height:12px;border-radius:50%;background:${c}"></span>`);
  document.body.append(lights);
}
document.body.dataset.ready = "1";
