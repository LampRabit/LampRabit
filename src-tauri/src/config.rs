use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    /// Root of the managed environment: binaries, configs, www, data, logs.
    pub root_dir: PathBuf,
    /// Top-level domain used for auto virtual hosts, e.g. `myapp.test`.
    pub tld: String,
    /// The user's domain on Cloudflare, for sharing projects at <project>.<domain> through
    /// named tunnels. Empty until they set it in Settings → Sharing.
    pub cloudflare_domain: String,
    pub ports: Ports,
    pub versions: Versions,
    /// The web servers that run, side by side. The first is the primary one: it has the
    /// main ports, localhost and the developer tools; the others use their own ports.
    pub web_servers: Vec<Server>,
    /// Before 0.3 the choice was Apache, Nginx or both; read once and turned into
    /// `web_servers` (see `Settings::load`).
    #[serde(skip_serializing)]
    pub web_server: Option<WebServer>,
    /// php-cgi processes per PHP version on Windows (php-fpm manages its own pool elsewhere).
    pub php_workers: u8,
    /// HTTPS for sites that have no setting of their own.
    pub https_by_default: bool,
    /// Keep the system hosts file in sync with the site list when Apache starts.
    pub manage_hosts: bool,
    /// Point PHP's mail() at Mailpit while Mailpit is installed.
    pub route_mail: bool,
    /// Look for a new LampRabit release when the app starts.
    pub check_updates: bool,
    /// Closing the window keeps LampRabit (and its services) running in the tray.
    pub close_to_tray: bool,
    /// Put the shims folder (php, composer, mariadb…) on the user's PATH.
    pub add_to_path: bool,
    /// Start every installed service when LampRabit opens.
    pub start_services_on_launch: bool,
    /// Command used for "Open in editor" (e.g. "code", "cursor", "phpstorm").
    pub editor: String,
    /// Per-site overrides keyed by folder name.
    pub sites: BTreeMap<String, SiteSettings>,
    /// Extension choices per installed PHP version, on top of the defaults.
    pub php_extensions: BTreeMap<String, ExtensionChoices>,
    /// Named sets of services to start and stop together ("Laravel", "WordPress"…).
    pub profiles: Vec<StackProfile>,
    /// What `mysql`, new projects and the database tools use first.
    pub default_database: Database,
    /// The app each database opens in, by service id (`postgres` → `pgadmin`), chosen on the
    /// Services page. Absent: KarotX when installed, else the first installed web tool.
    pub db_clients: BTreeMap<String, String>,
    /// Where object storage (RustFS, MinIO) keeps its buckets, by service id; data/<id>
    /// when absent. Can be outside the LampRabit folder (another drive, a NAS): it isn't
    /// moved with it, and uninstall never deletes it.
    pub storage: BTreeMap<String, PathBuf>,
    /// Values of the options services declare in their manifest (`settings:`), by service id
    /// then option name; only what differs from the default is kept.
    pub service_options: BTreeMap<String, BTreeMap<String, String>>,
    /// Services (by id) started again when they crash, up to a few times in a row.
    pub auto_restart: BTreeSet<String>,
    /// PHP's dump() and dd() go to Logs (VarDumper's server mode) instead of the page.
    pub catch_dumps: bool,
    /// LampRabit runs its MCP server over HTTP (ports.mcp) for AI assistants to connect to.
    pub mcp_server: bool,
    /// A log file bigger than this (MB) is rotated; `log_keep` older copies are kept, and
    /// the logs folder as a whole stays under `log_total_mb`.
    pub log_max_mb: u32,
    pub log_keep: u32,
    pub log_total_mb: u32,
    /// What Home greets the user with; empty until the setup guide asks.
    pub developer_name: String,
}

/// The two MySQL-compatible servers LampRabit runs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Database {
    #[default]
    Mariadb,
    Mysql,
}

impl Database {
    pub fn id(self) -> &'static str {
        match self {
            Self::Mariadb => "mariadb",
            Self::Mysql => "mysql",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Mariadb => "MariaDB",
            Self::Mysql => "MySQL",
        }
    }

    pub fn other(self) -> Self {
        match self {
            Self::Mariadb => Self::Mysql,
            Self::Mysql => Self::Mariadb,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackProfile {
    pub name: String,
    pub services: Vec<crate::services::ServiceKind>,
    /// Project processes it starts too, as "project/process". Only ones listed here: Start
    /// all and plain services never start projects.
    #[serde(default)]
    pub processes: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ExtensionChoices {
    pub enabled: std::collections::BTreeSet<String>,
    pub disabled: std::collections::BTreeSet<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SiteSettings {
    pub ssl: Option<bool>,
    /// PHP version for this site; the active version when unset.
    pub php: Option<String>,
    /// Web server for this site when several are enabled.
    pub server: Option<Server>,
    /// The template it was created from, as "id@version".
    pub template: Option<String>,
    /// How the project runs as a Node app; None until it's set up on the Sites page.
    pub app: Option<NodeApp>,
    /// Commands the user added to the project's Commands panel.
    pub commands: Vec<crate::commands::UserCommand>,
    /// The port its domain goes to when the project listens on several (or, for PHP
    /// projects, at all); used while that port is listening.
    pub route_port: Option<u16>,
    /// FrankenPHP keeps the app in memory between requests (worker mode). The app has to
    /// support it (Laravel Octane, Symfony Runtime…); code changes need "Restart workers".
    pub worker: bool,
    /// Where the main domain goes when it's set by hand; None follows what's running
    /// (the dev server while one listens, the files otherwise). `Main` isn't used here.
    pub main: Option<DomainTarget>,
    /// More domains for the project (subdomains of its own), each pointing at its files,
    /// a port or a process.
    pub domains: Vec<ExtraDomain>,
    /// Long-running commands next to the app: a queue worker, a scheduler, Vite.
    pub processes: Vec<ProjectProcess>,
    /// php.ini settings for this site only (memory_limit, upload_max_filesize…), written
    /// as a [PATH=] section of php.ini, which php-cgi and php-fpm apply per folder.
    pub php_ini: std::collections::BTreeMap<String, String>,
    /// Xdebug starts with every request of this site, not only with XDEBUG_TRIGGER.
    pub xdebug: bool,
}

/// Another domain of a project (admin.shop.test), with the same choices as the main one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtraDomain {
    pub host: String,
    pub target: DomainTarget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum DomainTarget {
    /// Wherever the main domain goes right now (its files or its dev server).
    Main,
    /// A folder in the project ("" for the folder the main domain serves).
    Files { root: String },
    /// A local port, whoever listens on it.
    Port { port: u16 },
    /// One of the project's processes, at its port.
    Process { name: String },
}

/// A long-running command of a project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ProjectProcess {
    /// Letters, digits and dashes; "queue", "vite".
    pub name: String,
    /// What runs; `{port}` is filled in.
    pub command: String,
    /// Where it runs, relative to the project ("" for the project folder): `api` in a
    /// monorepo.
    pub cwd: String,
    /// Where it listens, if it serves anything; also passed as $PORT.
    pub port: Option<u16>,
    pub env: BTreeMap<String, String>,
    pub restart: RestartPolicy,
    /// Starts and stops with the project's app (its Run button).
    pub with_app: bool,
    /// Starts when LampRabit starts (so at login, when LampRabit starts with the system).
    pub at_launch: bool,
    /// A crontab line (`0 3 * * *`): started at those times if it isn't running. Empty for none.
    pub schedule: String,
}

impl Default for ProjectProcess {
    fn default() -> Self {
        Self { name: String::new(), command: String::new(), cwd: String::new(), port: None, env: BTreeMap::new(), restart: RestartPolicy::OnFailure, with_app: true, at_launch: false, schedule: String::new() }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RestartPolicy {
    Never,
    /// After a crash (a non-zero exit), a few times in a row at most.
    #[default]
    OnFailure,
    /// Whenever it exits without being stopped (workers that exit to reload).
    Always,
}

/// A Node project's process (apps.rs) and how its domain reaches it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct NodeApp {
    pub mode: AppMode,
    /// Command mode: what runs, e.g. `npm run dev -- --port {port}`; `{port}` is filled in.
    pub command: String,
    /// File mode: the script `node` runs (relative to the project).
    pub file: String,
    /// File mode: restart when files change (`node --watch`).
    pub watch: bool,
    /// Where the app listens; also passed as $PORT. 0 until LampRabit picks a free one
    /// when the app first starts.
    pub port: u16,
    /// An NVM-installed version; None: the project's .nvmrc, else NVM's default.
    pub node: Option<String>,
    /// None: worked out from the lock file.
    pub package_manager: Option<PackageManager>,
    /// https://<project>.test forwards to the app instead of serving the folder.
    pub serve: bool,
}

/// How a project's domain is served. Dynamic: a process the project runs (a dev server)
/// behind the web server, on a port LampRabit picks. Static: the web server serves the
/// folder's files itself (with PHP for .php files).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServeMode {
    #[default]
    Dynamic,
    Static,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AppMode {
    #[default]
    Command,
    File,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackageManager {
    #[default]
    Npm,
    Pnpm,
    Yarn,
    Bun,
}

impl PackageManager {
    pub fn id(self) -> &'static str {
        match self {
            Self::Npm => "npm",
            Self::Pnpm => "pnpm",
            Self::Yarn => "yarn",
            Self::Bun => "bun",
        }
    }
}

/// How a new Python project gets its environment and packages.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PythonManager {
    /// `python -m venv .venv` on LampRabit's Python, packages with pip, a requirements.txt.
    #[default]
    Pip,
    /// `uv init` + `uv add`: a pyproject.toml and uv.lock, the environment in .venv.
    Uv,
    /// A conda environment in .venv (`conda create -p`), packages with its pip.
    Conda,
}

impl PythonManager {
    pub fn id(self) -> &'static str {
        match self {
            Self::Pip => "pip",
            Self::Uv => "uv",
            Self::Conda => "conda",
        }
    }
}

/// The pre-0.3 web server choice, only read from old settings files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WebServer {
    Apache,
    Nginx,
    Both,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Server {
    Apache,
    Nginx,
    Caddy,
    /// Caddy with PHP built in; its sites don't use LampRabit's PHP versions.
    Frankenphp,
}

impl Server {
    pub const ALL: [Server; 4] = [Server::Apache, Server::Nginx, Server::Caddy, Server::Frankenphp];

    pub fn label(self) -> &'static str {
        match self {
            Self::Apache => "Apache",
            Self::Nginx => "Nginx",
            Self::Caddy => "Caddy",
            Self::Frankenphp => "FrankenPHP",
        }
    }
}

impl Settings {
    pub fn server_enabled(&self, server: Server) -> bool {
        self.web_servers.contains(&server)
    }

    /// (http, https) for a web server. The primary server has the main ports; the others
    /// their own, so they can all run at once.
    pub fn server_ports(&self, server: Server) -> (u16, u16) {
        let p = &self.ports;
        if server == self.primary_server() {
            return (p.http, p.https);
        }
        match server {
            Server::Apache => (p.apache_http, p.apache_https),
            Server::Nginx => (p.nginx_http, p.nginx_https),
            Server::Caddy => (p.caddy_http, p.caddy_https),
            Server::Frankenphp => (p.frankenphp_http, p.frankenphp_https),
        }
    }

    /// Where an object storage service keeps its buckets.
    pub fn storage_dir(&self, layout: &crate::layout::Layout, id: &str) -> PathBuf {
        self.storage.get(id).cloned().unwrap_or_else(|| layout.data(id))
    }

    pub fn database_version(&self, db: Database) -> Option<&String> {
        match db {
            Database::Mariadb => self.versions.mariadb.as_ref(),
            Database::Mysql => self.versions.mysql.as_ref(),
        }
    }

    pub fn database_port(&self, db: Database) -> u16 {
        match db {
            Database::Mariadb => self.ports.service("mariadb.mysql"),
            Database::Mysql => self.ports.service("mysql.mysql"),
        }
    }

    /// The installed databases, the default one first.
    pub fn databases(&self) -> Vec<Database> {
        [self.default_database, self.default_database.other()]
            .into_iter()
            .filter(|&d| self.database_version(d).is_some())
            .collect()
    }

    /// The default database if it's installed, otherwise the other one if that is.
    pub fn database(&self) -> Option<Database> {
        self.databases().first().copied()
    }

    /// The server that also hosts localhost and the developer tools.
    pub fn primary_server(&self) -> Server {
        self.web_servers.first().copied().unwrap_or(Server::Apache)
    }

    /// Turns the pre-0.3 `web_server` choice into `web_servers`, and makes sure at least
    /// one server is enabled and none twice.
    pub fn restarts_after_crash(&self, service: &str) -> bool {
        self.auto_restart.contains(service)
    }

    pub fn normalize(&mut self) {
        if let Some(old) = self.web_server.take() {
            self.web_servers = match old {
                WebServer::Apache => vec![Server::Apache],
                WebServer::Nginx => vec![Server::Nginx],
                WebServer::Both => vec![Server::Apache, Server::Nginx],
            };
        }
        let mut seen = Vec::new();
        self.web_servers.retain(|s| !seen.contains(s) && { seen.push(*s); true });
        if self.web_servers.is_empty() {
            self.web_servers.push(Server::Apache);
        }
        // Ports saved as their own fields before 1.0, and any built-in service's port a
        // file doesn't list yet (a service added in a newer release) at its default.
        let legacy = std::mem::take(&mut self.ports.legacy);
        for (old, new) in LEGACY_PORTS {
            if let Some(&port) = legacy.get(*old) {
                self.ports.services.insert(new.to_string(), port);
            }
        }
        for (key, port) in Ports::default().services {
            self.ports.services.entry(key).or_insert(port);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Ports {
    /// The primary web server's.
    pub http: u16,
    pub https: u16,
    /// First port for PHP FastCGI workers; each PHP version in use takes a block of 10.
    pub php_fpm: u16,
    /// Each web server's own ports, used while another server is the primary one.
    pub nginx_http: u16,
    pub nginx_https: u16,
    pub apache_http: u16,
    pub apache_https: u16,
    pub caddy_http: u16,
    pub caddy_https: u16,
    /// Caddy's admin API, which `caddy reload` and `caddy stop` talk to.
    pub caddy_admin: u16,
    pub frankenphp_http: u16,
    pub frankenphp_https: u16,
    /// FrankenPHP's admin API (it is Caddy underneath).
    pub frankenphp_admin: u16,
    /// Where PHP's dump() sends dumps for Logs (Symfony's dump server uses 9912 too).
    pub dumps: u16,
    /// LampRabit's own MCP server over HTTP, when it's switched on.
    pub mcp: u16,
    /// Every service's ports, as `<service id>.<port name>` (`mariadb.mysql`): the defaults
    /// come from their manifests; a service added later gets free ones when it's installed.
    pub services: BTreeMap<String, u16>,
    /// Service ports saved by releases before 1.0 as fields of their own (`mysql`,
    /// `mailUi`…); moved into `services` when the settings load.
    #[serde(flatten, skip_serializing)]
    legacy: BTreeMap<String, u16>,
}

/// Where each pre-1.0 port field went in `Ports.services`.
const LEGACY_PORTS: &[(&str, &str)] = &[
    ("mysql", "mariadb.mysql"),
    ("mysqlServer", "mysql.mysql"),
    ("tomcatHttp", "tomcat.http"),
    ("tomcatShutdown", "tomcat.shutdown"),
    ("postgres", "postgres.postgres"),
    ("pgadmin", "pgadmin.http"),
    ("mongodb", "mongodb.mongodb"),
    ("clickhouseHttp", "clickhouse.http"),
    ("clickhouseTcp", "clickhouse.tcp"),
    ("neo4jHttp", "neo4j.http"),
    ("neo4jBolt", "neo4j.bolt"),
    ("meilisearch", "meilisearch.http"),
    ("typesense", "typesense.api"),
    ("rustfs", "rustfs.api"),
    ("rustfsConsole", "rustfs.console"),
    ("ollama", "ollama.api"),
    ("minio", "minio.api"),
    ("minioConsole", "minio.console"),
    ("elasticsearch", "elasticsearch.http"),
    ("elasticsearchTransport", "elasticsearch.transport"),
    ("opensearch", "opensearch.http"),
    ("opensearchTransport", "opensearch.transport"),
    ("n8n", "n8n.http"),
    ("rabbitmq", "rabbitmq.amqp"),
    ("rabbitmqManagement", "rabbitmq.management"),
    ("mailSmtp", "mailpit.smtp"),
    ("mailUi", "mailpit.ui"),
    ("redis", "redis.redis"),
    ("memcached", "memcached.memcached"),
];

/// Active version per runtime; `None` means not installed yet.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Versions {
    pub apache: Option<String>,
    pub nginx: Option<String>,
    pub php: Option<String>,
    pub mariadb: Option<String>,
    pub mailpit: Option<String>,
    pub adminer: Option<String>,
    pub phpmyadmin: Option<String>,
    pub ngrok: Option<String>,
    pub cloudflared: Option<String>,
    pub redis: Option<String>,
    pub composer: Option<String>,
    pub mysql: Option<String>,
    pub phpredisadmin: Option<String>,
    pub memcached: Option<String>,
    /// Runtimes added since 0.3, by id (`caddy = "2.10.2"`). Same place in settings.toml
    /// as the fields above, so a runtime can move between the two without a migration.
    #[serde(flatten)]
    pub other: BTreeMap<String, String>,
}

impl Versions {
    /// The active version of a runtime, by its id.
    pub fn get(&self, id: &str) -> Option<&String> {
        match id {
            "apache" => self.apache.as_ref(),
            "nginx" => self.nginx.as_ref(),
            "php" => self.php.as_ref(),
            "mariadb" => self.mariadb.as_ref(),
            "mailpit" => self.mailpit.as_ref(),
            "adminer" => self.adminer.as_ref(),
            "phpmyadmin" => self.phpmyadmin.as_ref(),
            "ngrok" => self.ngrok.as_ref(),
            "cloudflared" => self.cloudflared.as_ref(),
            "redis" => self.redis.as_ref(),
            "composer" => self.composer.as_ref(),
            "mysql" => self.mysql.as_ref(),
            "phpredisadmin" => self.phpredisadmin.as_ref(),
            "memcached" => self.memcached.as_ref(),
            _ => self.other.get(id),
        }
    }

    pub fn set(&mut self, id: &str, version: Option<String>) {
        let field = match id {
            "apache" => &mut self.apache,
            "nginx" => &mut self.nginx,
            "php" => &mut self.php,
            "mariadb" => &mut self.mariadb,
            "mailpit" => &mut self.mailpit,
            "adminer" => &mut self.adminer,
            "phpmyadmin" => &mut self.phpmyadmin,
            "ngrok" => &mut self.ngrok,
            "cloudflared" => &mut self.cloudflared,
            "redis" => &mut self.redis,
            "composer" => &mut self.composer,
            "mysql" => &mut self.mysql,
            "phpredisadmin" => &mut self.phpredisadmin,
            "memcached" => &mut self.memcached,
            _ => {
                match version {
                    Some(v) => self.other.insert(id.to_string(), v),
                    None => self.other.remove(id),
                };
                return;
            }
        };
        *field = version;
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            root_dir: default_root(),
            tld: "test".into(),
            cloudflare_domain: String::new(),
            ports: Ports::default(),
            versions: Versions::default(),
            web_servers: vec![Server::Apache],
            web_server: None,
            php_workers: 4,
            https_by_default: true,
            manage_hosts: true,
            route_mail: true,
            check_updates: true,
            close_to_tray: true,
            add_to_path: true,
            start_services_on_launch: false,
            editor: "code".into(),
            sites: BTreeMap::new(),
            php_extensions: BTreeMap::new(),
            profiles: Vec::new(),
            default_database: Database::Mariadb,
            db_clients: BTreeMap::new(),
            storage: BTreeMap::new(),
            service_options: BTreeMap::new(),
            // n8n's Node process is the one that falls over on large workflows.
            auto_restart: BTreeSet::from(["n8n".to_string()]),
            catch_dumps: false,
            mcp_server: false,
            log_max_mb: 10,
            log_keep: 3,
            log_total_mb: 500,
            developer_name: String::new(),
        }
    }
}

impl Ports {
    /// A service's port by `<service id>.<port name>`, else its manifest's default.
    pub fn service(&self, key: &str) -> u16 {
        self.services.get(key).copied().unwrap_or_else(|| {
            let (id, name) = key.split_once('.').unwrap_or((key, ""));
            crate::manifest::get(id).and_then(|m| m.ports.get(name)).map_or(0, |d| d.default)
        })
    }

}

impl Default for Ports {
    fn default() -> Self {
        Self {
            http: 80,
            https: 443,
            php_fpm: 9000,
            nginx_http: 8080,
            nginx_https: 8443,
            apache_http: 8081,
            apache_https: 8444,
            caddy_http: 8082,
            caddy_https: 8445,
            caddy_admin: 2019,
            frankenphp_http: 8083,
            frankenphp_https: 8446,
            frankenphp_admin: 2020,
            dumps: 9912,
            mcp: 39813,
            services: crate::manifest::built_ins().iter().flat_map(|m| m.ports.iter().map(|(name, d)| (format!("{}.{name}", m.id), d.default))).collect(),
            legacy: BTreeMap::new(),
        }
    }
}

fn default_root() -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(r"C:\lampRabit")
    } else {
        dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join("lampRabit")
    }
}

impl Settings {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let mut settings: Self = toml::from_str(&std::fs::read_to_string(path)?)?;
        settings.normalize();
        Ok(settings)
    }

    /// Like `load`, but a damaged file never stops the app from opening: it is moved aside
    /// (settings.broken-<time>.toml) and defaults are used. Returns where it went, if so.
    pub fn load_or_recover(path: &Path) -> (Self, Option<PathBuf>) {
        match Self::load(path) {
            Ok(settings) => (settings, None),
            Err(_) => {
                let stamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |d| d.as_secs());
                let aside = path.with_file_name(format!("settings.broken-{stamp}.toml"));
                let moved = std::fs::rename(path, &aside).is_ok();
                (Self::default(), moved.then_some(aside))
            }
        }
    }

    /// Writes a temporary file and renames it over the old one, so a crash or power cut
    /// mid-save can't leave half a settings file behind.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("toml.tmp");
        std::fs::write(&tmp, toml::to_string_pretty(self)?)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Settings saved before 1.0 had a field per service port; they load into
    /// `ports.services`, and saving writes only the new form.
    #[test]
    fn old_port_fields_move_into_services() {
        let mut s: Settings = toml::from_str("[ports]\nhttp = 8000\nmysql = 3316\nmailUi = 8026\n").unwrap();
        s.normalize();
        assert_eq!(s.ports.http, 8000);
        assert_eq!(s.ports.service("mariadb.mysql"), 3316);
        assert_eq!(s.ports.service("mailpit.ui"), 8026);
        // One the file didn't have gets its manifest's default.
        assert_eq!(s.ports.service("redis.redis"), 6379);
        let saved = toml::to_string_pretty(&s).unwrap();
        assert!(!saved.contains("mailUi") && saved.contains("\"mailpit.ui\" = 8026"), "{saved}");
    }
}
