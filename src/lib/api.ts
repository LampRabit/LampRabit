import { invoke } from "@tauri-apps/api/core";

export type ServiceKind = "apache" | "nginx" | "caddy" | "frankenphp" | "tomcat" | "postgres" | "pgadmin" | "mongodb" | "clickhouse" | "neo4j" | "meilisearch" | "typesense" | "rustfs" | "ollama" | "minio" | "elasticsearch" | "opensearch" | "n8n" | "rabbitmq" | "mariadb" | "mailpit" | "redis" | "memcached" | "mysql";
export type Server = "apache" | "nginx" | "caddy" | "frankenphp";
export const allServers: Server[] = ["apache", "nginx", "caddy", "frankenphp"];
export type ServiceState = "stopped" | "starting" | "running" | "stopping" | "error";
export type Runtime =
  | "apache"
  | "nginx"
  | "php"
  | "mariadb"
  | "mailpit"
  | "adminer"
  | "phpmyadmin"
  | "ngrok"
  | "cloudflared"
  | "redis"
  | "composer"
  | "mysql"
  | "phpredisadmin"
  | "memcached"
  | "caddy"
  | "java"
  | "tomcat"
  | "frankenphp"
  | "postgres"
  | "python"
  | "phppgadmin"
  | "pgadmin"
  | "mongodb"
  | "mongosh"
  | "mongotools"
  | "compass"
  | "sqlite"
  | "clickhouse"
  | "neo4j"
  | "meilisearch"
  | "typesense"
  | "rustfs"
  | "ollama"
  | "minio"
  | "elasticsearch"
  | "opensearch"
  | "n8n"
  | "rabbitmq"
  | "erlang"
  | "go"
  | "rust";
export const coreRuntimes: Runtime[] = ["apache", "nginx", "caddy", "frankenphp", "php", "mariadb", "mysql"];
export const toolRuntimes: Runtime[] = ["composer", "mailpit", "adminer", "phpmyadmin", "redis", "phpredisadmin", "memcached", "ngrok", "cloudflared"];

export const serviceLabels: Record<ServiceKind, string> = {
  apache: "Apache",
  nginx: "Nginx",
  caddy: "Caddy",
  frankenphp: "FrankenPHP",
  tomcat: "Tomcat",
  postgres: "PostgreSQL",
  pgadmin: "pgAdmin",
  mongodb: "MongoDB",
  clickhouse: "ClickHouse",
  neo4j: "Neo4j",
  meilisearch: "Meilisearch",
  typesense: "Typesense",
  rustfs: "RustFS",
  ollama: "Ollama",
  minio: "MinIO",
  elasticsearch: "Elasticsearch",
  opensearch: "OpenSearch",
  n8n: "n8n",
  rabbitmq: "RabbitMQ",
  mariadb: "MariaDB",
  mysql: "MySQL",
  mailpit: "Mailpit",
  redis: "Redis",
  memcached: "Memcached",
};

export const runtimeLabels: Record<Runtime, string> = {
  apache: "Apache",
  nginx: "Nginx",
  php: "PHP",
  mariadb: "MariaDB",
  mailpit: "Mailpit",
  adminer: "Adminer",
  phpmyadmin: "phpMyAdmin",
  ngrok: "Ngrok",
  cloudflared: "Cloudflare Tunnel",
  redis: "Redis",
  composer: "Composer",
  mysql: "MySQL",
  phpredisadmin: "phpRedisAdmin",
  memcached: "Memcached",
  caddy: "Caddy",
  java: "Java",
  tomcat: "Tomcat",
  frankenphp: "FrankenPHP",
  postgres: "PostgreSQL",
  python: "Python",
  phppgadmin: "phpPgAdmin",
  pgadmin: "pgAdmin",
  mongodb: "MongoDB",
  mongosh: "mongosh",
  mongotools: "MongoDB Database Tools",
  compass: "MongoDB Compass",
  sqlite: "SQLite",
  clickhouse: "ClickHouse",
  neo4j: "Neo4j",
  meilisearch: "Meilisearch",
  typesense: "Typesense",
  rustfs: "RustFS",
  ollama: "Ollama",
  minio: "MinIO",
  elasticsearch: "Elasticsearch",
  opensearch: "OpenSearch",
  n8n: "n8n",
  rabbitmq: "RabbitMQ",
  erlang: "Erlang",
  go: "Go",
  rust: "Rust",
};

export interface UpdateNotice {
  runtime: Runtime;
  installed: string;
  release: Release;
}

export interface ExtInfo {
  name: string;
  kind: "bundled" | "extra";
  description: string | null;
  enabled: boolean;
  installed: boolean;
  available: string | null;
  note: string | null;
}

export type TunnelProvider = "ngrok" | "cloudflare";

export interface Tunnel {
  id: string;
  provider: TunnelProvider;
  site: string | null;
  target: string;
  url: string | null;
  status: "starting" | "online" | "error";
  error: string | null;
}

export interface Release {
  runtime: Runtime;
  version: string;
  url: string;
  sha256: string | null;
  archive: "zip" | "targz";
  label: string | null;
}

export interface InstalledRuntime {
  runtime: Runtime;
  active: string | null;
  versions: string[];
}

export interface InstallProgress {
  runtime: Runtime;
  version: string;
  stage: "downloading" | "verifying" | "extracting" | "done" | "error";
  downloaded: number;
  total: number | null;
}

export interface ServiceInfo {
  kind: ServiceKind;
  label: string;
  state: ServiceState;
  pid: number | null;
  port: number;
  version: string | null;
  installed: boolean;
  startedAt: number | null;
  configPath: string;
  logPath: string;
  health: Health | null;
  problem: Problem | null;
}

export interface Health {
  status: "checking" | "healthy" | "degraded";
  detail: string | null;
  checkedAt: number;
}

export interface PortOwner {
  pid: number;
  name: string;
  note: string | null;
  stoppable: boolean;
}

export interface Problem {
  title: string;
  detail: string;
  fixes: string[];
  port: number | null;
  owner: PortOwner | null;
  at: number;
}

export interface PortStatus {
  key: keyof Settings["ports"];
  port: number;
  free: boolean;
  usedByService: string | null;
  owner: PortOwner | null;
}

export interface Site {
  name: string;
  domain: string;
  path: string;
  /** Directory Apache serves: the project's public/ folder when it has one. */
  docroot: string;
  ssl: boolean;
  url: string;
  modified: number;
  /** Set for built-in tools rather than www/ folders. */
  tool: "adminer" | "phpmyadmin" | "phpredisadmin" | "mailpit" | null;
  proxy: string | null;
  /** How the project's process runs, once set up (or started once). */
  app: NodeApp | null;
  /** Has a package.json, so it has scripts and can run a dev server. */
  nodeProject: boolean;
  language: Language;
  /** dynamic: the domain forwards to the project's dev server; static: the web server serves the files. */
  serveMode: ServeMode;
  /** Where the domain goes right now: a port the project listens on, or null for its files. */
  route: Route | null;
  /** Every port the project's processes listen on, whoever started them. */
  ports: DetectedPort[];
  /** The port picked for the domain when there are several; null chooses automatically. */
  routePort: number | null;
  /** PHP version chosen for the site; null follows the active version. */
  php: string | null;
  server: Server;
  /** Node version from the project's .nvmrc / .node-version. */
  node: string | null;
  /** "Laravel", "WordPress", "Symfony", "PHP", "Static"…, from the project's files. */
  framework: string | null;
  /** The checked-out git branch (or short commit). */
  gitBranch: string | null;
  /** The database the project's config names. */
  database: string | null;
}

export interface NodeStatus {
  manager: "nvm-windows" | "nvm" | null;
  managerVersion: string | null;
  current: string | null;
  installed: string[];
  needsElevation: boolean;
}

export interface NodeRelease {
  version: string;
  major: number;
  lts: string | null;
}

export interface NewProject {
  name: string;
  /** A template id from listTemplates. */
  template: string;
  php: string | null;
  server: Server | null;
  ssl: boolean;
  database: string | null;
  /** The server the database goes on; the default database when null. */
  engine: DbEngine | null;
  /** Values for the template's variables, by id. */
  variables: Record<string, string>;
  /** Optional steps by id: run or not. */
  optional: Record<string, boolean>;
  /** The user accepted the risky steps of a template that didn't ship with LampRabit. */
  approveRisky: boolean;
  /** Node templates. */
  node: string | null;
  packageManager: PackageManager | null;
}

export interface TemplateVariable {
  id: string;
  label: string;
  help: string | null;
  type: "text" | "choice" | "bool" | "multi";
  /** Options shown only while their `when` holds. */
  options: { value: string; label: string; when: string | null }[];
  /** multi: joins the picked values (the wizard stores them comma-separated). */
  separator: string | null;
  default: string | null;
  /** Shown only while this condition holds (same syntax as a step's if:). */
  when: string | null;
  required: boolean;
}

/** Whether a folder can hold the LampRabit folder (Settings → Move). */
export interface MoveCheck {
  problems: string[];
  size: number;
  free: number | null;
  sameDrive: boolean;
  fileSystem: string | null;
}

/** A program that keeps the LampRabit folder busy. */
export interface Holder {
  pid: number;
  name: string;
  path: string;
}

export type DbEngine = "mariadb" | "mysql" | "postgres" | "sqlite" | "mongodb";

/** A database dump in <root>/backups/<engine>/. */
export interface Backup {
  engine: DbEngine;
  database: string;
  path: string;
  size: number;
  /** Unix time in milliseconds. */
  created: number;
}

export interface ProjectTemplate {
  id: string;
  name: string;
  description: string;
  version: number;
  icon: string | null;
  color: string | null;
  duration: string | null;
  source: "builtin" | "user";
  path: string | null;
  usesPhp: boolean;
  usesNode: boolean;
  /** Lowest Python / Java version, for templates that run on LampRabit's own. */
  python: string | null;
  java: string | null;
  go: string | null;
  rust: string | null;
  minPhp: string | null;
  extensions: string[];
  database: "none" | "optional" | "required";
  /** Database servers the template can set a project up for. */
  engines: DbEngine[];
  variables: TemplateVariable[];
  optionalSteps: { id: string; label: string; default: boolean; when: string | null }[];
  /** Steps needing approval (templates you added yourself). */
  risks: { step: string; heading: string; command: string; reasons: string[] }[];
  done: string | null;
}

export interface TemplateCatalog {
  templates: ProjectTemplate[];
  broken: { file: string; error: string }[];
  folder: string;
}

export interface ProjectProgress {
  name: string;
  step: string | null;
  line: string | null;
}

export interface PoolStatus {
  version: string;
  port: number;
  workers: number;
  alive: number;
}

export interface SitesInfo {
  sites: Site[];
  tools: Site[];
  tld: string;
  needsApply: boolean;
  hostsOk: boolean;
  manageHosts: boolean;
  /** null while no site uses HTTPS. */
  caTrusted: boolean | null;
  caPath: string;
  /** Folders in www that can't be served, and why. */
  unserved: { name: string; reason: string }[];
  /** Tool domains (adminer.test…) that a folder of the same name has taken. */
  shadowedTools: string[];
}

export interface LogHistory {
  lines: RawLogLine[];
  cursor: number;
}

export interface StatusEvent {
  kind: ServiceKind;
  state: ServiceState;
  message: string | null;
}

export type LogService = "apache" | "nginx" | "caddy" | "frankenphp" | "tomcat" | "postgres" | "pgadmin" | "mongodb" | "clickhouse" | "neo4j" | "meilisearch" | "typesense" | "rustfs" | "ollama" | "minio" | "elasticsearch" | "opensearch" | "n8n" | "rabbitmq" | "php" | "mariadb" | "mysql" | "mailpit" | "redis" | "memcached" | "system" | "project";

/** A raw log line from a service's output or one of its log files. */
export interface RawLogLine {
  id: number;
  service: LogService;
  /** "command" is what LampRabit ran for a project, and how it ended. */
  stream: LogStream;
  line: string;
  /** For service "project": which project. */
  project?: string;
}

export type LogStream = "output" | "error" | "access" | "command";

/** What a project is written in; decides its runtime and how it's served. */
export type Language = "php" | "node" | "java" | "python" | "go" | "rust" | "static";
export type ServeMode = "dynamic" | "static";
export interface Route {
  port: number;
  /** 127.0.0.1, or [::1] for servers listening on IPv6 only. */
  host: string;
}

export interface DetectedPort extends Route {
  pid: number;
  /** node, php… */
  process: string;
  command: string;
}

export type AppMode = "command" | "file";
export type PackageManager = "npm" | "pnpm" | "yarn" | "bun";

/** How a Node project runs (Sites → Node app). */
export interface NodeApp {
  mode: AppMode;
  /** Command mode; `{port}` is filled in when it starts. */
  command: string;
  file: string;
  watch: boolean;
  port: number;
  node: string | null;
  packageManager: PackageManager | null;
  /** https://<project>.test forwards to the app. */
  serve: boolean;
}

export interface AppStatus {
  name: string;
  state: "stopped" | "starting" | "running" | "crashed";
  pid: number | null;
  port: number | null;
  since: number | null;
  node: string | null;
  exitCode: number | null;
  /** A command running for the project. */
  task: string | null;
}

export interface AppInfo {
  config: NodeApp;
  configured: boolean;
  scripts: Record<string, string>;
  detectedPackageManager: PackageManager;
  nodeVersions: string[];
  projectNode: string | null;
  framework: string | null;
  status: AppStatus;
}

export interface AppOutput {
  name: string;
  line: string;
  stream: LogStream;
}

export interface ProjectCommand {
  group: string;
  name: string;
  command: string;
  description: string | null;
  origin: "yours" | "template" | "discovered";
}

export interface ProjectCommands {
  commands: ProjectCommand[];
  problems: string[];
  sources: string[];
}

/** What the Overview shows about a project beyond the site list (project_facts). */
export interface ProjectFacts {
  package: string | null;
  description: string | null;
  version: string | null;
  /** Installed framework version, or the declared range when not installed. */
  frameworkVersion: string | null;
  phpRequirement: string | null;
  nodeRequirement: string | null;
  dependencies: { manager: string; count: number; devCount: number; installed: boolean }[];
  envFile: string | null;
  /** [key, value] pairs; secrets are never included. */
  env: [string, string][];
  git: { remote: string | null; commit: [string, string, number] | null; changes: number | null } | null;
  /** Name of the template it was created from. */
  template: string | null;
}

export interface UserCommand {
  name: string;
  run: string;
}

export interface Settings {
  rootDir: string;
  tld: string;
  ports: {
    http: number;
    https: number;
    /** MariaDB's port (the name predates MySQL support). */
    mysql: number;
    /** Oracle MySQL, next to MariaDB. */
    mysqlServer: number;
    phpFpm: number;
    /** Each web server's own ports, used while another server is the primary one. */
    nginxHttp: number;
    nginxHttps: number;
    apacheHttp: number;
    apacheHttps: number;
    caddyHttp: number;
    caddyHttps: number;
    /** Caddy's admin API (reload, stop). */
    caddyAdmin: number;
    frankenphpHttp: number;
    frankenphpHttps: number;
    frankenphpAdmin: number;
    /** Tomcat's HTTP connector, behind the web servers. */
    tomcatHttp: number;
    tomcatShutdown: number;
    postgres: number;
    pgadmin: number;
    mongodb: number;
    clickhouseHttp: number;
    clickhouseTcp: number;
    neo4jHttp: number;
    neo4jBolt: number;
    meilisearch: number;
    typesense: number;
    rustfs: number;
    rustfsConsole: number;
    ollama: number;
    minio: number;
    minioConsole: number;
    elasticsearch: number;
    elasticsearchTransport: number;
    opensearch: number;
    opensearchTransport: number;
    n8n: number;
    rabbitmq: number;
    rabbitmqManagement: number;
    mailSmtp: number;
    mailUi: number;
    redis: number;
    memcached: number;
  };
  versions: Partial<Record<Runtime, string | null>>;
  /** The web servers that run; the first is the primary one (main ports, localhost, tools). */
  webServers: Server[];
  phpWorkers: number;
  httpsByDefault: boolean;
  manageHosts: boolean;
  routeMail: boolean;
  checkUpdates: boolean;
  closeToTray: boolean;
  addToPath: boolean;
  startServicesOnLaunch: boolean;
  editor: string;
  sites: Record<string, { ssl: boolean | null; php: string | null; server: Server | null }>;
  profiles: StackProfile[];
  /** What `mysql`, new projects and the database tools use first. */
  defaultDatabase: Database;
  /** Object storage folders by service id (RustFS, MinIO); data/<id> when absent. */
  storage: Record<string, string>;
}

export type Database = "mariadb" | "mysql";

/** A named set of services started and stopped together. */
export interface StackProfile {
  name: string;
  services: ServiceKind[];
}

/** CPU (like `top`: 100 is one full core) and memory (bytes) of one service, or "php". */
export interface Usage {
  id: ServiceKind | "php";
  cpu: number;
  memory: number;
  processes: number;
}

/** Removing LampRabit: folders deleted and kept (Settings → Troubleshooting). */
export interface UninstallPlan {
  remove: string[];
  keep: string[];
}

export interface UninstallReport {
  kept: string[];
  /** What couldn't be undone, each saying what to do. */
  problems: string[];
}

export interface EnvPaths {
  root: string;
  www: string;
  logs: string;
  settingsFile: string;
  os: "windows" | "macos" | "linux";
}

export const api = {
  getSettings: () => invoke<Settings>("get_settings"),
  takeStartupWarnings: () => invoke<string[]>("take_startup_warnings"),
  saveSettings: (settings: Settings) => invoke<Settings>("save_settings", { settings }),
  envPaths: () => invoke<EnvPaths>("env_paths"),
  listServices: () => invoke<ServiceInfo[]>("list_services"),
  startService: (kind: ServiceKind) => invoke<void>("start_service", { kind }),
  stopService: (kind: ServiceKind) => invoke<void>("stop_service", { kind }),
  restartService: (kind: ServiceKind) => invoke<void>("restart_service", { kind }),
  listSites: () => invoke<SitesInfo>("list_sites"),
  applySites: () => invoke<void>("apply_sites"),
  setSiteSsl: (name: string, ssl: boolean) => invoke<void>("set_site_ssl", { name, ssl }),
  setSitePhp: (name: string, version: string | null) => invoke<void>("set_site_php", { name, version }),
  setSiteServer: (name: string, server: Server | null) => invoke<void>("set_site_server", { name, server }),
  phpStatus: () => invoke<PoolStatus[]>("php_status"),
  createProject: (request: NewProject) => invoke<{ url: string; note: string | null }>("create_project", { request }),
  listTemplates: () => invoke<TemplateCatalog>("list_templates"),
  appInfo: (name: string) => invoke<AppInfo>("app_info", { name }),
  appSave: (name: string, config: NodeApp) => invoke<void>("app_save", { name, config }),
  appStart: (name: string) => invoke<AppStatus>("app_start", { name }),
  appStop: (name: string) => invoke<void>("app_stop", { name }),
  appRestart: (name: string) => invoke<AppStatus>("app_restart", { name }),
  appInstall: (name: string) => invoke<void>("app_install", { name }),
  appStatuses: () => invoke<AppStatus[]>("app_statuses"),
  appLogs: (name: string) => invoke<string[]>("app_logs", { name }),
  appFreePort: () => invoke<number>("app_free_port"),
  projectCommands: (name: string) => invoke<ProjectCommands>("project_commands", { name }),
  projectFacts: (name: string) => invoke<ProjectFacts>("project_facts", { name }),
  setSiteRoute: (name: string, port: number | null) => invoke<void>("set_site_route", { name, port }),
  runProjectCommand: (name: string, label: string, command: string) => invoke<void>("run_project_command", { name, label, command }),
  stopProjectCommand: (name: string) => invoke<void>("stop_project_command", { name }),
  saveProjectCommands: (name: string, commands: UserCommand[]) => invoke<void>("save_project_commands", { name, commands }),
  openTemplatesFolder: () => invoke<void>("open_templates_folder"),
  tunnelStart: (provider: TunnelProvider, site: string | null, port: number | null) =>
    invoke<Tunnel>("tunnel_start", { provider, site, port }),
  tunnelStop: (id: string) => invoke<void>("tunnel_stop", { id }),
  tunnelList: () => invoke<Tunnel[]>("tunnel_list"),
  ngrokTokenSaved: () => invoke<boolean>("ngrok_token_saved"),
  nodeStatus: () => invoke<NodeStatus>("node_status"),
  nodeAvailable: () => invoke<NodeRelease[]>("node_available"),
  nodeInstall: (version: string) => invoke<void>("node_install", { version }),
  nodeUse: (version: string) => invoke<void>("node_use", { version }),
  nodeUninstall: (version: string) => invoke<void>("node_uninstall", { version }),
  nvmInstall: () => invoke<void>("nvm_install"),
  runtimeUpdates: () => invoke<UpdateNotice[]>("runtime_updates"),
  phpExtensions: (version: string) => invoke<ExtInfo[]>("php_extensions", { version }),
  setPhpExtension: (version: string, name: string, enabled: boolean) =>
    invoke<void>("set_php_extension", { version, name, enabled }),
  installPhpExtension: (version: string, name: string) => invoke<void>("install_php_extension", { version, name }),
  openTerminal: (dir: string | null) => invoke<void>("open_terminal", { dir }),
  openInEditor: (dir: string) => invoke<void>("open_in_editor", { dir }),
  vcRuntimeStatus: () => invoke<{ applicable: boolean; installed: boolean; version: string | null }>("vc_runtime_status"),
  vcRuntimeInstall: () => invoke<void>("vc_runtime_install"),
  setNgrokToken: (token: string | null) => invoke<void>("set_ngrok_token", { token }),
  /** Resolves to a note when browsers couldn't be covered (Linux without certutil). */
  trustCa: () => invoke<string | null>("trust_ca"),
  listDatabases: (engine: DbEngine) => invoke<string[]>("list_databases", { engine }),
  listBackups: () => invoke<Backup[]>("list_backups"),
  backupDatabase: (engine: DbEngine, database: string) => invoke<Backup>("backup_database", { engine, database }),
  restoreBackup: (engine: DbEngine, path: string, database: string) => invoke<void>("restore_backup", { engine, path, database }),
  deleteBackup: (path: string) => invoke<void>("delete_backup", { path }),
  openCompass: () => invoke<void>("open_compass"),
  ollamaElsewhere: () => invoke<string | null>("ollama_elsewhere"),
  checkMoveTarget: (target: string) => invoke<MoveCheck>("check_move_target", { target }),
  environmentInUse: () => invoke<Holder[]>("environment_in_use"),
  stopEnvironmentHolder: (pid: number) => invoke<void>("stop_environment_holder", { pid }),
  moveEnvironment: (target: string) => invoke<Settings>("move_environment", { target }),
  openDbClient: (database?: Database) => invoke<void>("open_db_client", { database: database ?? null }),
  startAll: () => invoke<string[]>("start_all"),
  /** Stops `stop`, then starts `start`; resolves to one message per failed start. */
  applyServices: (start: ServiceKind[], stop: ServiceKind[]) => invoke<string[]>("apply_services", { start, stop }),
  serviceUsage: () => invoke<Usage[]>("service_usage"),
  stopAll: () => invoke<void>("stop_all"),
  uninstallPlan: () => invoke<UninstallPlan>("uninstall_plan"),
  uninstall: () => invoke<UninstallReport>("uninstall"),
  checkPort: (port: number) => invoke<boolean>("check_port", { port }),
  portStatus: () => invoke<PortStatus[]>("port_status"),
  stopProcess: (pid: number) => invoke<void>("stop_process", { pid }),
  clearProblem: (kind: ServiceKind) => invoke<void>("clear_problem", { kind }),
  diagnostics: (redact: boolean) => invoke<string>("diagnostics", { redact }),
  saveText: (path: string, contents: string) => invoke<void>("save_text", { path, contents }),
  logHistory: () => invoke<LogHistory>("log_history"),
  clearLogs: () => invoke<void>("clear_logs"),
  runtimeCatalog: (runtime: Runtime) => invoke<Release[]>("runtime_catalog", { runtime }),
  installedRuntimes: () => invoke<InstalledRuntime[]>("installed_runtimes"),
  installRuntime: (release: Release) => invoke<void>("install_runtime", { release }),
  removeRuntime: (runtime: Runtime, version: string) => invoke<void>("remove_runtime", { runtime, version }),
  setActiveVersion: (runtime: Runtime, version: string) =>
    invoke<void>("set_active_version", { runtime, version }),
};
