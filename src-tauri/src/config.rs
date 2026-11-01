use std::{
    collections::BTreeMap,
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

    pub fn database_version(&self, db: Database) -> Option<&String> {
        match db {
            Database::Mariadb => self.versions.mariadb.as_ref(),
            Database::Mysql => self.versions.mysql.as_ref(),
        }
    }

    pub fn database_port(&self, db: Database) -> u16 {
        match db {
            Database::Mariadb => self.ports.mysql,
            Database::Mysql => self.ports.mysql_server,
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
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Ports {
    pub http: u16,
    pub https: u16,
    /// MariaDB (the name predates MySQL support).
    pub mysql: u16,
    /// Oracle MySQL, next to MariaDB.
    pub mysql_server: u16,
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
    /// Tomcat's HTTP connector; the web servers proxy Java sites' domains to it.
    pub tomcat_http: u16,
    /// The port Tomcat listens on for its shutdown command (127.0.0.1 only).
    pub tomcat_shutdown: u16,
    pub postgres: u16,
    /// pgAdmin's web server (127.0.0.1 only), proxied at pgadmin.test.
    pub pgadmin: u16,
    pub mongodb: u16,
    /// ClickHouse's HTTP interface (and its /play page) and native protocol; the native
    /// one isn't ClickHouse's usual 9000, which PHP's FastCGI ports use.
    pub clickhouse_http: u16,
    pub clickhouse_tcp: u16,
    /// Neo4j Browser (HTTP) and the Bolt protocol drivers use.
    pub neo4j_http: u16,
    pub neo4j_bolt: u16,
    /// Mailpit's SMTP server, where PHP and frameworks send mail.
    pub mail_smtp: u16,
    pub redis: u16,
    pub memcached: u16,
    /// Mailpit's web inbox.
    pub mail_ui: u16,
}

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
            add_to_path: false,
            start_services_on_launch: false,
            editor: "code".into(),
            sites: BTreeMap::new(),
            php_extensions: BTreeMap::new(),
            profiles: Vec::new(),
            default_database: Database::Mariadb,
        }
    }
}

impl Default for Ports {
    fn default() -> Self {
        Self {
            http: 80,
            https: 443,
            mysql: 3306,
            mysql_server: 3307,
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
            tomcat_http: 8180,
            tomcat_shutdown: 8105,
            postgres: 5432,
            pgadmin: 5050,
            mongodb: 27017,
            clickhouse_http: 8123,
            clickhouse_tcp: 19000,
            neo4j_http: 7474,
            neo4j_bolt: 7687,
            mail_smtp: 1025,
            redis: 6379,
            memcached: 11211,
            mail_ui: 8025,
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
