use std::{collections::{HashMap, HashSet}, path::{Path, PathBuf}, process::Stdio, sync::Arc, time::Duration};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use tokio::{
    fs::{File, OpenOptions},
    io::{AsyncBufReadExt, AsyncRead, AsyncWriteExt, BufReader},
    process::Command,
    sync::{Mutex, mpsc, oneshot},
};

use crate::{
    config::Settings,
    config::Server,
    configs,
    diagnose::{self, Probe, Problem},
    manifest::{self, Check, Hook, Manifest},
    php::{self, PhpPools, PoolPlan},
    error::{Error, Result},
    logs::{self, Level},
    sites,
    layout::{Layout, exe},
    procgroup::{self, ProcGroup},
    runtimes,
};

/// How long a graceful stop may take before the process tree is killed.
const STOP_TIMEOUT: Duration = Duration::from_secs(15);
/// How long a slow service (JVM, Python, Node) may take to answer after it's spawned.
const READY_TIMEOUT: Duration = Duration::from_secs(90);
/// A service that crashes more often than this within `RESTART_WINDOW` is left stopped:
/// restarting it again would only hide a problem that needs looking at.
const MAX_RESTARTS: u32 = 3;
const RESTART_WINDOW: Duration = Duration::from_secs(300);

/// Serialized as its id (`"mariadb"`, or a non-built-in manifest's id).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ServiceKind {
    Apache,
    Nginx,
    Mariadb,
    /// Catches outgoing mail (SMTP) and shows it in a web inbox.
    Mailpit,
    /// Redis, for caches, sessions and queues.
    Redis,
    /// Memcached, an in-memory cache.
    Memcached,
    /// Oracle MySQL, next to MariaDB on its own port.
    Mysql,
    /// Caddy, a web server with a short config file of its own (Caddyfile).
    Caddy,
    /// Caddy with PHP built in.
    Frankenphp,
    /// Apache Tomcat for Java web apps, behind the web servers.
    Tomcat,
    Postgres,
    /// pgAdmin 4's web server, behind the web servers at pgadmin.test.
    Pgadmin,
    Mongodb,
    Clickhouse,
    /// Neo4j, with Neo4j Browser behind the web servers at neo4j.test.
    Neo4j,
    Meilisearch,
    Typesense,
    Rustfs,
    Ollama,
    Minio,
    Elasticsearch,
    Opensearch,
    /// n8n, behind the web servers at n8n.test.
    N8n,
    Rabbitmq,
    /// A service that exists only as a manifest (not built in): an index into
    /// `manifest::extras()`.
    Plugin(u16),
}

impl Serialize for ServiceKind {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(self.id())
    }
}

impl<'de> Deserialize<'de> for ServiceKind {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let id = String::deserialize(d)?;
        Self::from_id(&id).ok_or_else(|| serde::de::Error::custom(format!("unknown service \"{id}\"")))
    }
}


impl ServiceKind {
    pub fn all() -> Vec<Self> {
        let mut all = vec![Self::Apache, Self::Nginx, Self::Caddy, Self::Frankenphp, Self::Tomcat, Self::Mariadb, Self::Mysql, Self::Postgres, Self::Mongodb, Self::Clickhouse, Self::Neo4j, Self::Meilisearch, Self::Typesense, Self::Rustfs, Self::Minio, Self::Elasticsearch, Self::Opensearch, Self::Rabbitmq, Self::Ollama, Self::N8n, Self::Mailpit, Self::Redis, Self::Memcached, Self::Pgadmin];
        all.extend(manifest::extra_indexes().into_iter().map(Self::Plugin));
        all
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::all().into_iter().find(|k| k.id() == id)
    }

    /// The web servers' services, in `Server::ALL` order.
    pub const WEB: [ServiceKind; 4] = [ServiceKind::Apache, ServiceKind::Nginx, ServiceKind::Caddy, ServiceKind::Frankenphp];

    pub fn web_server(self) -> Option<Server> {
        match self {
            Self::Apache => Some(Server::Apache),
            Self::Nginx => Some(Server::Nginx),
            Self::Caddy => Some(Server::Caddy),
            Self::Frankenphp => Some(Server::Frankenphp),
            _ => None,
        }
    }

    pub fn for_server(server: Server) -> Self {
        match server {
            Server::Apache => Self::Apache,
            Server::Nginx => Self::Nginx,
            Server::Caddy => Self::Caddy,
            Server::Frankenphp => Self::Frankenphp,
        }
    }

    /// Its manifest (src-tauri/services/<id>.yaml): label, ports, health check, launch.
    pub fn manifest(self) -> &'static Manifest {
        manifest::get(self.id()).unwrap_or_else(|| panic!("no manifest for {}", self.id()))
    }

    /// Takes a while to come up: it stays Starting until its health check passes, so
    /// nothing (the UI, Start all, tools) treats it as ready while it's still loading.
    fn slow(self) -> bool {
        self.manifest().behaviour.slow
    }

    /// Services that need a lot of memory (a JVM, a model server, Erlang) or several seconds
    /// to start: "Start all", the tray and start-on-launch leave them out; they start from
    /// their own card or a stack profile that lists them.
    pub fn heavy(self) -> bool {
        self.manifest().behaviour.heavy
    }

    /// Web servers only run when Settings → Web server includes them.
    pub fn enabled(self, settings: &Settings) -> bool {
        self.web_server().is_none_or(|s| settings.server_enabled(s))
    }

    pub fn id(self) -> &'static str {
        match self {
            Self::Apache => "apache",
            Self::Nginx => "nginx",
            Self::Caddy => "caddy",
            Self::Frankenphp => "frankenphp",
            Self::Tomcat => "tomcat",
            Self::Postgres => "postgres",
            Self::Pgadmin => "pgadmin",
            Self::Mongodb => "mongodb",
            Self::Clickhouse => "clickhouse",
            Self::Neo4j => "neo4j",
            Self::Meilisearch => "meilisearch",
            Self::Typesense => "typesense",
            Self::Rustfs => "rustfs",
            Self::Ollama => "ollama",
            Self::Minio => "minio",
            Self::Elasticsearch => "elasticsearch",
            Self::Opensearch => "opensearch",
            Self::N8n => "n8n",
            Self::Rabbitmq => "rabbitmq",
            Self::Mariadb => "mariadb",
            Self::Mysql => "mysql",
            Self::Mailpit => "mailpit",
            Self::Redis => "redis",
            Self::Memcached => "memcached",
            Self::Plugin(i) => &manifest::extra(i).id,
        }
    }

    pub fn label(self) -> &'static str {
        &self.manifest().name
    }

    /// Each service runs its own runtime, under the same id.
    fn runtime(self) -> &'static str {
        self.id()
    }

    fn version(self, settings: &Settings) -> Option<String> {
        settings.versions.get(self.runtime()).cloned()
    }

    /// What "Open config" opens: the manifest's `config` (its config file, or the data
    /// folder for services configured on the command line).
    pub fn config_path(self, settings: &Settings, layout: &Layout) -> PathBuf {
        let m = self.manifest();
        match m.config.as_ref() {
            Some(config) => manifest::Ctx::any(m, settings, layout).text(config).ok().flatten().map(PathBuf::from).unwrap_or_else(|| layout.etc(self.id())),
            None => match self {
                Self::Tomcat => configs::tomcat_base(layout).join("conf").join("server.xml"),
                _ => layout.etc(self.id()),
            },
        }
    }

    /// The service's own error log (stdout/stderr go to `<id>-output.log`).
    pub fn log_path(self, layout: &Layout) -> PathBuf {
        layout.logs().join(self.manifest().log_file())
    }

    /// The main port: the web servers' come from Settings → Web server (which one is
    /// primary), the others' from their manifest.
    fn port(self, settings: &Settings) -> u16 {
        match self.web_server() {
            Some(server) => settings.server_ports(server).0,
            None => {
                let m = self.manifest();
                m.main_port().and_then(|p| m.port(settings, p)).unwrap_or_default()
            }
        }
    }

    /// How apps connect to it: a URL and .env lines.
    pub fn connection(self, settings: &Settings, layout: &Layout) -> Option<manifest::ConnectionInfo> {
        if self.web_server().is_some() {
            let port = self.port(settings);
            let url = format!("http://localhost{}", if port == 80 { String::new() } else { format!(":{port}") });
            return Some(manifest::ConnectionInfo { env: vec![("APP_URL".into(), url.clone())], url });
        }
        manifest::connection(self.id(), settings, layout)
    }

    /// The port the service listens on (its main one), for checks outside the supervisor.
    pub fn port_of(self, settings: &Settings) -> u16 {
        self.port(settings)
    }

    /// Whether the service answers right now (the smoke test's health check).
    pub async fn probe_now(self, settings: &Settings, client: &reqwest::Client) -> bool {
        matches!(self.probe(settings, client).await, Probe::Ok)
    }

    /// Asks the service whether it answers, the way its manifest's `health` says (on the
    /// port it names: Mailpit's main port is SMTP, its health endpoint the web inbox's).
    async fn probe(self, settings: &Settings, client: &reqwest::Client) -> Probe {
        let m = self.manifest();
        let port = |name: &str| if self.web_server().is_some() || name.is_empty() { self.port(settings) } else { m.port(settings, name).unwrap_or_default() };
        match m.check_kind() {
            Check::Http(p, path) => diagnose::probe_http(client, &format!("http://127.0.0.1:{}{path}", port(&p))).await,
            Check::Tcp(p) => diagnose::probe_tcp(port(&p)).await,
            Check::Mysql(p) => diagnose::probe_mysql(port(&p)).await,
            Check::Postgres(p) => diagnose::probe_postgres(port(&p)).await,
            Check::Line(p, send, expect) => diagnose::probe_line(port(&p), send.as_bytes(), &expect).await,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ServiceState {
    Stopped,
    Starting,
    Running,
    Stopping,
    Error,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceInfo {
    pub kind: ServiceKind,
    pub label: &'static str,
    pub state: ServiceState,
    pub pid: Option<u32>,
    pub port: u16,
    pub version: Option<String>,
    pub installed: bool,
    /// Unix time in milliseconds when the service came up.
    pub started_at: Option<u64>,
    pub config_path: PathBuf,
    pub log_path: PathBuf,
    /// Result of the latest health probe while running.
    pub health: Option<Health>,
    /// Why the last start failed or the service stopped unexpectedly.
    pub problem: Option<Problem>,
    /// Running, but not started by LampRabit (the Ollama app): it can't be stopped here.
    pub external: bool,
    /// Left out of "Start all" (`ServiceKind::heavy`).
    pub heavy: bool,
    /// How apps connect: a URL and .env lines (from its manifest).
    pub connection: Option<manifest::ConnectionInfo>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Health {
    /// "checking" right after start, then "healthy" or "degraded".
    pub status: &'static str,
    pub detail: Option<String>,
    pub checked_at: u64,
    #[serde(skip)]
    failures: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct StatusEvent {
    pub kind: ServiceKind,
    pub state: ServiceState,
    pub message: Option<String>,
}

struct LaunchSpec {
    program: PathBuf,
    args: Vec<String>,
    cwd: PathBuf,
    /// Directories prepended to PATH (e.g. PHP's DLLs for mod_php).
    path_prepend: Vec<PathBuf>,
    /// Command that asks the service to shut down cleanly.
    stop_cmd: Option<(PathBuf, Vec<String>)>,
    env: Vec<(&'static str, String)>,
    /// Arguments that check the config without starting (web servers): `httpd -t`,
    /// `nginx -t`, `caddy validate`.
    test: Option<Vec<String>>,
}

/// Caddy keeps state (autosave.json, certificates it would otherwise issue) in the user's
/// config and data folders; LampRabit's stays under tmp/<id>, removed on uninstall.
fn caddy_env(layout: &Layout, id: &str) -> Vec<(&'static str, String)> {
    let dir = layout.tmp().join(id).to_string_lossy().into_owned();
    vec![("XDG_CONFIG_HOME", dir.clone()), ("XDG_DATA_HOME", dir)]
}

/// The admin API port of Caddy and FrankenPHP, which reload and stop go through.
fn admin_port(kind: ServiceKind, settings: &Settings) -> Option<u16> {
    match kind {
        ServiceKind::Caddy => Some(settings.ports.caddy_admin),
        ServiceKind::Frankenphp => Some(settings.ports.frankenphp_admin),
        _ => None,
    }
}

/// Ollama installed outside LampRabit, used when LampRabit has none of its own. It keeps
/// its models where it always does, so the user's downloaded models stay available.
fn system_ollama(settings: &Settings) -> Option<LaunchSpec> {
    if settings.versions.get("ollama").is_some() {
        return None;
    }
    let program = runtimes::ollama_elsewhere()?;
    Some(LaunchSpec {
        cwd: program.parent().map(Path::to_path_buf).unwrap_or_default(),
        program,
        args: vec!["serve".into()],
        test: None,
        stop_cmd: None,
        path_prepend: vec![],
        env: vec![("OLLAMA_HOST", format!("127.0.0.1:{}", settings.ports.service("ollama.api")))],
    })
}

/// Whether something already answers on a service's port on this machine.
fn answers(port: u16) -> bool {
    std::net::TcpStream::connect_timeout(&std::net::SocketAddr::from(([127, 0, 0, 1], port)), Duration::from_millis(150)).is_ok()
}

fn launch_spec(kind: ServiceKind, settings: &Settings, layout: &Layout) -> Result<LaunchSpec> {
    let m = kind.manifest();
    if m.uses(Hook::External)
        && kind == ServiceKind::Ollama
        && let Some(spec) = system_ollama(settings) {
            return Ok(spec);
        }
    if m.uses(Hook::Launch) {
        return builtin_launch_spec(kind, settings, layout);
    }
    // A half-finished install (n8n's npm step, an interrupted download) has no marker yet.
    if let Some(binary) = manifest::Ctx::new(m, settings, layout)?.binary()
        && !binary.exists() {
            return Err(Error::msg(format!("{}'s install isn't complete (it may still be installing). Start it once it has finished, or reinstall it on the Services page.", m.name)));
        }
    manifest_spec(kind, settings, layout)
}

/// The command line of a service from its manifest's `run` and `stop`, with what it
/// `needs` found and checked first.
fn manifest_spec(kind: ServiceKind, settings: &Settings, layout: &Layout) -> Result<LaunchSpec> {
    let m: &'static Manifest = kind.manifest();
    let mut ctx = manifest::Ctx::new(m, settings, layout)?;
    let version = kind.version(settings);
    for need in m.needs.iter().filter(|n| n.applies(version.as_deref())) {
        let (home, bin) = need_home(m, need, settings, layout)?;
        ctx.set(format!("needs.{}.home", need.runtime), home.to_string_lossy());
        ctx.set(format!("needs.{}.bin", need.runtime), bin.to_string_lossy());
    }
    for need in m.system_needs.iter().filter(|n| manifest::matches(&n.on)) {
        if !std::env::var_os("PATH").is_some_and(|p| crate::apps::on_path(&need.command, &p)) {
            return Err(Error::msg(need.hint.clone()));
        }
    }
    if m.ui.is_some() {
        let url = sites::tools(settings, layout).into_iter().find(|t| t.tool == Some(kind.id())).map(|t| t.url).unwrap_or_default();
        ctx.set("ui.url", url);
    }
    let run = m.run.as_ref().ok_or_else(|| Error::msg(format!("{}'s manifest has no run section", m.name)))?;
    let program = ctx.pick(&run.program).ok_or_else(|| Error::msg(format!("{} has no program for {}", m.name, manifest::platform())))?;
    let stop_cmd = match &m.stop {
        Some(stop) => {
            let other = stop.if_installed.as_ref().map(|id| settings.versions.get(id).map(|v| (id, layout.bin(id, v))));
            match other {
                Some(None) => None,
                found => {
                    if let Some(Some((id, home))) = found {
                        ctx.set(format!("runtime.{id}.home"), home.to_string_lossy());
                    }
                    let mut command = ctx.args(&stop.command)?.into_iter();
                    let program = PathBuf::from(command.next().unwrap_or_default());
                    // A runtime that's set active but whose files are gone can't stop anything.
                    (stop.if_installed.is_none() || program.exists()).then(|| (program, command.collect()))
                }
            }
        }
        None => None,
    };
    let env = run.env.iter().filter_map(|(k, v)| ctx.pick(v).map(|v| ctx.render(v).map(|v| (k.as_str(), v)))).collect::<Result<_>>()?;
    Ok(LaunchSpec {
        program: PathBuf::from(ctx.render(program)?),
        args: ctx.args(&run.args)?,
        cwd: match run.cwd.as_ref().and_then(|c| ctx.pick(c)) {
            Some(cwd) => PathBuf::from(ctx.render(cwd)?),
            None => ctx.home(),
        },
        path_prepend: ctx.args(&run.path)?.into_iter().map(PathBuf::from).collect(),
        stop_cmd,
        env,
        test: None,
    })
}

/// The folder of a runtime a service runs on (its home, and where its programs are), when
/// it's installed in a version that works.
fn need_home(m: &Manifest, need: &manifest::Need, settings: &Settings, layout: &Layout) -> Result<(PathBuf, PathBuf)> {
    let label = match need.runtime.as_str() {
        "java" => "Java",
        "erlang" => "Erlang",
        "node" => "Node.js",
        "python" => "Python",
        "postgres" => "PostgreSQL",
        other => other,
    };
    let min = need.min.as_deref();
    let wanted = min.map(|v| format!("{label} {v}")).unwrap_or_else(|| label.to_string());
    // Node.js is NVM's, not a LampRabit runtime.
    if need.runtime == "node" {
        let major = min.and_then(|v| crate::version::numbers(v).first().copied()).unwrap_or(0) as u32;
        let bin = runtimes::n8n_node(major)?;
        return Ok((bin.clone(), bin));
    }
    let version = settings
        .versions
        .get(&need.runtime)
        .ok_or_else(|| Error::msg(format!("{}. Install {wanted} on the Services page first.", need.why)))?;
    let have = crate::version::numbers(version);
    if let Some(min) = min {
        let want = crate::version::numbers(min);
        if have.iter().take(want.len()).cloned().collect::<Vec<_>>() < want {
            return Err(Error::msg(format!("{} needs {label} {min} or newer, and the active {label} is {version}. Install {wanted} on the Services page.", m.name)));
        }
    }
    if let Some(max) = need.max.as_deref() {
        let top = crate::version::numbers(max);
        if have.iter().take(top.len()).cloned().collect::<Vec<_>>() > top {
            return Err(Error::msg(format!("{} works with {label} up to {max}, and the active {label} is {version}. Switch to an older {label} on the Services page.", m.name)));
        }
    }
    let home = layout.bin(&need.runtime, version);
    let bin = if need.runtime == "python" && cfg!(windows) { home.clone() } else { home.join("bin") };
    Ok((home, bin))
}

/// The command lines of the services whose manifest says `builtin: [launch]`: the web
/// servers (their config test and admin API), Tomcat (Java's classpath), PostgreSQL (the
/// elevated case) and pgAdmin (its package folder is found at start).
fn builtin_launch_spec(kind: ServiceKind, settings: &Settings, layout: &Layout) -> Result<LaunchSpec> {
    let version = kind
        .version(settings)
        .ok_or_else(|| Error::msg(format!("{} is not installed", kind.label())))?;
    let home = layout.bin(kind.runtime(), &version);
    let path = |p: PathBuf| p.to_string_lossy().into_owned();

    let spec = match kind {
        ServiceKind::Apache => LaunchSpec {
            program: home.join("bin").join(exe("httpd")),
            args: {
                let mut args = vec!["-d".into(), path(home.clone()), "-f".into(), path(layout.etc("apache").join("httpd.conf"))];
                // Outside Windows httpd forks into the background, and the supervisor would
                // see it exit while the real server runs on untracked.
                if !cfg!(windows) {
                    args.extend(["-D".into(), "FOREGROUND".into()]);
                }
                args
            },
            test: Some(vec!["-d".into(), path(home.clone()), "-f".into(), path(layout.etc("apache").join("httpd.conf")), "-t".into()]),
            cwd: home,
            path_prepend: vec![],
            stop_cmd: None,
            env: vec![],
        },
        ServiceKind::Nginx => {
            let prefix = format!("{}/", path(layout.tmp().join("nginx")));
            let conf = path(layout.etc("nginx").join("nginx.conf"));
            LaunchSpec {
                program: home.join(exe("nginx")),
                args: {
                    let mut args = vec!["-p".into(), prefix.clone(), "-c".into(), conf.clone()];
                    // Stay in the foreground, as on Windows (see Apache above).
                    if !cfg!(windows) {
                        args.extend(["-g".into(), "daemon off;".into()]);
                    }
                    args
                },
                test: Some(vec!["-p".into(), prefix.clone(), "-c".into(), conf.clone(), "-t".into()]),
                stop_cmd: Some((home.join(exe("nginx")), vec!["-p".into(), prefix, "-c".into(), conf, "-s".into(), "quit".into()])),
                cwd: home,
                path_prepend: vec![],
                env: vec![],
            }
        }
        // FrankenPHP is Caddy with PHP built in: the same commands, under its own name.
        ServiceKind::Caddy | ServiceKind::Frankenphp => {
            let id = kind.id();
            let conf = path(kind.config_path(settings, layout));
            let config = |cmd: &str| vec![cmd.into(), "--config".into(), conf.clone(), "--adapter".into(), "caddyfile".into()];
            let admin = format!("127.0.0.1:{}", admin_port(kind, settings).unwrap_or_default());
            let mut env = caddy_env(layout, id);
            if kind == ServiceKind::Frankenphp {
                env.push(("PHP_INI_SCAN_DIR", path(layout.etc("frankenphp").join("conf.d"))));
            }
            LaunchSpec {
                program: home.join(exe(id)),
                args: config("run"),
                test: Some(config("validate")),
                // Through the admin API: a clean shutdown on Windows too, where there's no SIGTERM.
                stop_cmd: Some((home.join(exe(id)), vec!["stop".into(), "--address".into(), admin])),
                cwd: home,
                path_prepend: vec![],
                env,
            }
        }
        // Tomcat's own scripts (catalina.bat/.sh) only assemble this command; running Java
        // directly keeps the supervisor on the real process and works the same everywhere.
        ServiceKind::Tomcat => {
            let java = configs::tomcat_java(settings, layout, &version)?;
            let java_home = java.parent().and_then(Path::parent).map(Path::to_path_buf).unwrap_or_default();
            let base = configs::tomcat_base(layout);
            let sep = if cfg!(windows) { ";" } else { ":" };
            let classpath = format!("{}{sep}{}", path(home.join("bin").join("bootstrap.jar")), path(home.join("bin").join("tomcat-juli.jar")));
            let args = |command: &str| -> Vec<String> {
                vec![
                    format!("-Dcatalina.home={}", path(home.clone())),
                    format!("-Dcatalina.base={}", path(base.clone())),
                    format!("-Djava.io.tmpdir={}", path(layout.tmp().join("tomcat").join("temp"))),
                    "-Djava.util.logging.manager=org.apache.juli.ClassLoaderLogManager".into(),
                    format!("-Djava.util.logging.config.file={}", path(base.join("conf").join("logging.properties"))),
                    "-cp".into(),
                    classpath.clone(),
                    "org.apache.catalina.startup.Bootstrap".into(),
                    command.into(),
                ]
            };
            LaunchSpec {
                program: java.clone(),
                args: args("start"),
                test: None,
                // Bootstrap stop sends SHUTDOWN to the shutdown port: Tomcat undeploys its apps cleanly.
                stop_cmd: Some((java, args("stop"))),
                cwd: home.clone(),
                path_prepend: vec![],
                env: vec![("JAVA_HOME", path(java_home)), ("CATALINA_HOME", path(home.clone())), ("CATALINA_BASE", path(base.clone()))],
            }
        }
        // PostgreSQL won't run with administrator rights, and LampRabit has them when UAC is off
        // or it was started with "Run as administrator". pg_ctl starts it with those rights
        // dropped; LampRabit's `--pg-elevated` mode runs pg_ctl, then stands in for the server
        // (its output, and exiting when the server does) so the supervisor works as usual.
        ServiceKind::Postgres if cfg!(windows) && procgroup::elevated() => {
            let data = path(configs::postgres_data(settings, layout));
            let bin = home.join("bin");
            LaunchSpec {
                program: std::env::current_exe()?,
                args: vec![
                    "--pg-elevated".into(),
                    path(bin.join(exe("pg_ctl"))),
                    data.clone(),
                    settings.ports.service("postgres.postgres").to_string(),
                    path(layout.tmp().join("postgres-pgctl.log")),
                ],
                test: None,
                stop_cmd: Some((bin.join(exe("pg_ctl")), vec!["stop".into(), "-D".into(), data, "-m".into(), "fast".into(), "-s".into()])),
                env: vec![],
                cwd: home,
                path_prepend: vec![],
            }
        }
        ServiceKind::Postgres => {
            let data = path(configs::postgres_data(settings, layout));
            LaunchSpec {
                program: home.join("bin").join(exe("postgres")),
                // Only on 127.0.0.1, and no Unix socket: its default folder (/var/run/postgresql)
                // usually doesn't exist or isn't writable, and clients get PGHOST from the shims.
                args: vec![
                    "-D".into(),
                    data.clone(),
                    "-p".into(),
                    settings.ports.service("postgres.postgres").to_string(),
                    "-c".into(),
                    "listen_addresses=127.0.0.1".into(),
                    "-c".into(),
                    "unix_socket_directories=".into(),
                ],
                test: None,
                // A fast shutdown: open connections are closed, nothing is lost.
                stop_cmd: Some((home.join("bin").join(exe("pg_ctl")), vec!["stop".into(), "-D".into(), data, "-m".into(), "fast".into(), "-s".into()])),
                env: if cfg!(target_os = "linux") { vec![("LD_LIBRARY_PATH", path(home.join("lib")))] } else { vec![] },
                cwd: home,
                path_prepend: vec![],
            }
        }
        // pgAdmin's own entry point, on the Python of its virtual environment.
        ServiceKind::Pgadmin => {
            let package = runtimes::pgadmin_package(&home)
                .ok_or_else(|| Error::msg("pgAdmin's install is incomplete. Remove it on the Services page and install it again."))?;
            LaunchSpec {
                program: runtimes::venv_python(&home.join("venv")),
                args: vec![path(package.join("pgAdmin4.py"))],
                test: None,
                stop_cmd: None,
                cwd: package,
                path_prepend: vec![],
                // Its output in our log right away rather than when Python's buffer fills.
                // Desktop mode (no pgAdmin login), as its desktop app starts it.
                env: vec![("PYTHONUNBUFFERED", "1".into()), ("PGADMIN_SERVER_MODE", "OFF".into())],
            }
        }
        _ => return Err(Error::msg(format!("{} is started from its manifest, not by builtin code", kind.label()))),
    };
    Ok(spec)
}

/// Writes configs and does first-run setup for a service. Web servers return the PHP
/// pools their sites need.
async fn prepare(app: &AppHandle, kind: ServiceKind, settings: &Settings, layout: &Layout) -> Result<Vec<PoolPlan>> {
    layout.ensure()?;
    match kind {
        ServiceKind::Apache | ServiceKind::Nginx | ServiceKind::Caddy | ServiceKind::Frankenphp => {
            let server = kind.web_server().unwrap();
            configs::write_tools(settings, layout)?;
            let sites = sites::served(settings, layout);
            let plan = php::plan(settings, layout, &sites);
            if sites::write_config(settings, layout, &sites, &plan, server)? {
                logs::system(app, Level::Info, "Created the local certificate authority for HTTPS sites");
            }
            match kind {
                ServiceKind::Apache => {
                    configs::write_apache(settings, layout, &plan)?;
                    let _ = std::fs::remove_file(configs::apache_pid_file(layout));
                }
                ServiceKind::Nginx => {
                    configs::nginx_prefix(layout)?;
                }
                ServiceKind::Frankenphp => {
                    configs::write_frankenphp_ini(settings, layout)?;
                    std::fs::create_dir_all(layout.tmp().join("frankenphp"))?;
                }
                _ => std::fs::create_dir_all(layout.tmp().join("caddy"))?,
            }
            // In the background: the admin prompt can sit unanswered for minutes.
            let (app, settings, layout) = (app.clone(), settings.clone(), layout.clone());
            tauri::async_runtime::spawn(async move { sync_hosts(&app, &settings, &layout, &sites).await });
            Ok(plan)
        }
        ServiceKind::Mariadb => {
            configs::write_mariadb(settings, layout)?;
            configs::init_mariadb_data(settings, layout).await?;
            Ok(vec![])
        }
        ServiceKind::Mysql => {
            configs::mysql_libraries(&configs::mysql_home(settings, layout)?)?;
            configs::write_mysql(settings, layout)?;
            configs::init_mysql_data(settings, layout).await?;
            Ok(vec![])
        }
        ServiceKind::Tomcat => {
            configs::write_tomcat(settings, layout, &sites::tomcat_apps(settings, layout))?;
            Ok(vec![])
        }
        ServiceKind::Postgres => {
            configs::init_postgres_data(settings, layout).await?;
            Ok(vec![])
        }
        ServiceKind::Pgadmin => {
            let version = kind.version(settings).unwrap_or_default();
            let package = runtimes::pgadmin_package(&layout.bin("pgadmin", &version)).ok_or_else(|| Error::msg("pgAdmin's install is incomplete"))?;
            configs::write_pgadmin(settings, layout, &package)?;
            let python = runtimes::venv_python(&layout.bin("pgadmin", &version).join("venv"));
            if let Err(e) = configs::register_pgadmin_server(settings, layout, &python, &package).await {
                logs::system(app, Level::Warn, format!("Couldn't add LampRabit's PostgreSQL to pgAdmin; add it yourself (127.0.0.1, user root): {e}"));
            }
            Ok(vec![])
        }
        _ => {
            // Its manifest's `dirs` and `files`.
            manifest::Ctx::new(kind.manifest(), settings, layout)?.prepare()?;
            Ok(vec![])
        }
    }
}

/// Every port a service will listen on: its manifest's, or for a web server its HTTP
/// port, its HTTPS one when a site uses it, and Caddy's admin API.
pub fn required_ports(kind: ServiceKind, settings: &Settings, layout: &Layout) -> Vec<u16> {
    let Some(server) = kind.web_server() else {
        return kind.manifest().port_numbers(settings);
    };
    let mut ports = vec![kind.port(settings)];
    if sites::served(settings, layout).iter().any(|s| s.ssl && s.server == server) {
        ports.push(settings.server_ports(server).1);
    }
    ports.extend(admin_port(kind, settings));
    ports
}

/// `httpd -t`: catches config mistakes (including user files in etc/apache/sites/)
/// before Apache is started for real.
/// Apache's config test for the diagnostics report: None when Apache isn't installed or
/// has never been started (no generated config yet).
pub async fn config_test(kind: ServiceKind, settings: &Settings, layout: &Layout) -> Option<std::result::Result<(), String>> {
    let spec = launch_spec(kind, settings, layout).ok()?;
    if !spec.program.exists() || !kind.config_path(settings, layout).exists() {
        return None;
    }
    Some(match server_config_test(&spec).await {
        None => Ok(()),
        Some(err) => Err(err),
    })
}

/// `httpd -t` / `nginx -t` / `caddy validate`. None when the config is valid, otherwise
/// the error output.
async fn server_config_test(spec: &LaunchSpec) -> Option<String> {
    let mut cmd = Command::new(&spec.program);
    cmd.args(spec.test.as_ref()?).current_dir(&spec.cwd).envs(spec.env.iter().map(|(k, v)| (*k, v)));
    if !spec.path_prepend.is_empty() {
        let existing = std::env::var_os("PATH").unwrap_or_default();
        let paths = spec.path_prepend.iter().cloned().chain(std::env::split_paths(&existing));
        cmd.env("PATH", std::env::join_paths(paths).unwrap_or(existing));
    }
    procgroup::prepare(&mut cmd);
    let out = cmd.output().await.ok()?;
    if out.status.success() {
        return None;
    }
    let text = format!("{}{}", String::from_utf8_lossy(&out.stderr), String::from_utf8_lossy(&out.stdout));
    Some(text.trim().to_string())
}

/// Brings the hosts file in line with the site list. A failure (e.g. a cancelled admin
/// prompt) is logged but never blocks Apache from starting.
pub async fn sync_hosts(app: &AppHandle, settings: &Settings, layout: &Layout, sites: &[sites::Site]) {
    // Apache and Nginx starting together would otherwise ask for admin rights twice.
    static HOSTS: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    let _one_at_a_time = HOSTS.lock().await;
    if !settings.manage_hosts || !sites::needs_hosts_file(settings) || sites::hosts_in_sync(sites) {
        return;
    }
    logs::system(app, Level::Info, format!("Updating the hosts file for {} site(s)", sites.len()));
    match sites::update_hosts(layout, sites).await {
        Ok(()) => logs::system(app, Level::Info, "Hosts file updated"),
        Err(e) => logs::system(app, Level::Warn, format!("{e} Sites may not resolve until the hosts file is updated.")),
    }
}

/// Server processes left running from LampRabit's bin/ folder by a previous run that
/// ended without shutting them down (a crash, or the app being killed). They hold ports
/// and files, so they're stopped when LampRabit starts. Only one LampRabit runs at a time,
/// so none of them can belong to another copy. CLI tools (`php`, `mysql`…) that the user
/// may be running from a terminal are left alone. Returns what was stopped.
pub fn reap_orphans(layout: &Layout) -> Vec<String> {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
    const SERVERS: &[&str] = &["httpd", "nginx", "caddy", "frankenphp", "postgres", "mongod", "clickhouse", "meilisearch", "typesense-server", "rustfs", "ollama", "minio", "epmd", "erl", "beam.smp", "mariadbd", "mysqld", "php-fpm", "php-cgi", "mailpit", "redis-server", "memcached",
        // Redis before 0.2, so one left running is stopped before its folder is removed.
        "GarnetServer"];
    let bin = layout.root().join("bin");
    let mut system = System::new();
    system.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing().with_exe(UpdateKind::Always).with_cmd(UpdateKind::Always));
    let mut stopped = Vec::new();
    // Threads are listed as processes on Linux; only whole processes are stopped.
    for (pid, process) in system.processes().iter().filter(|(_, p)| p.thread_kind().is_none()) {
        let Some(exe) = process.exe() else { continue };
        // Linux marks a program whose file was replaced (a reinstalled runtime) "(deleted)".
        let exe_text = exe.to_string_lossy();
        let exe = Path::new(exe_text.strip_suffix(" (deleted)").unwrap_or(&exe_text));
        let name = exe.file_stem().and_then(|n| n.to_str()).unwrap_or_default();
        // Tomcat runs as LampRabit's java; Java programs started from the terminal don't
        // run Tomcat's Bootstrap class, so they're left alone.
        // Tomcat and Neo4j run on LampRabit's java, recognised by their main classes.
        let tomcat = name == "java"
            && process.cmd().iter().any(|a| {
                let a = a.to_string_lossy();
                a == "org.apache.catalina.startup.Bootstrap" || a.starts_with("org.neo4j.server.")
            });
        // pgAdmin's virtual environment only ever runs pgAdmin.
        let pgadmin = exe.starts_with(bin.join("pgadmin"));
        // Elasticsearch and OpenSearch run on the Java bundled in their own folders.
        let search = exe.starts_with(bin.join("elasticsearch")) || exe.starts_with(bin.join("opensearch"));
        // n8n runs on NVM's Node, which may live elsewhere; its script is LampRabit's. An
        // n8n the user runs from their own folder is left alone.
        let n8n = process.cmd().iter().any(|a| Path::new(a).starts_with(bin.join("n8n")));
        if ((exe.starts_with(&bin) && (SERVERS.contains(&name) || tomcat || pgadmin || search)) || n8n) && process.kill() {
            stopped.push(format!("{name} (PID {pid})"));
        }
    }
    stopped
}

pub fn port_free(port: u16) -> bool {
    use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, TcpListener, TcpStream};
    // A test bind alone can succeed on Windows while another program listens on the
    // IPv6 or loopback address, so first see whether anything answers.
    let answers = |addr: SocketAddr| TcpStream::connect_timeout(&addr, Duration::from_millis(150)).is_ok();
    if answers((Ipv4Addr::LOCALHOST, port).into()) || answers((Ipv6Addr::LOCALHOST, port).into()) {
        return false;
    }
    // Nobody answered, so "permission denied" only means a privileged port (below 1024 on
    // Linux), which allow_low_ports deals with; it isn't taken.
    let bindable = |addr: (&str, u16)| match TcpListener::bind(addr) {
        Ok(_) => true,
        Err(e) => e.kind() == std::io::ErrorKind::PermissionDenied,
    };
    bindable(("0.0.0.0", port)) && bindable(("127.0.0.1", port))
}

/// Linux reserves ports below 1024 for root, so a web server on 80/443 needs that limit
/// lowered. Asked for once (pkexec) and kept in /etc/sysctl.d, so it survives reboots.
#[cfg(target_os = "linux")]
async fn allow_low_ports(app: &AppHandle, label: &str, ports: &[u16]) -> Result<()> {
    const LIMIT: &str = "/proc/sys/net/ipv4/ip_unprivileged_port_start";
    let Some(lowest) = ports.iter().copied().min() else { return Ok(()) };
    let limit: u16 = std::fs::read_to_string(LIMIT).ok().and_then(|s| s.trim().parse().ok()).unwrap_or(1024);
    // SAFETY: geteuid has no preconditions and can't fail.
    if lowest >= limit || unsafe { libc::geteuid() } == 0 {
        return Ok(());
    }
    logs::system(app, Level::Info, format!("Asking for administrator rights to let {label} use port {lowest}"));
    let script = format!(
        "printf '# Lets LampRabit serve sites on standard ports without root.\nnet.ipv4.ip_unprivileged_port_start = {lowest}\n' > /etc/sysctl.d/50-lamprabit.conf && sysctl -q -w net.ipv4.ip_unprivileged_port_start={lowest}"
    );
    let allowed = Command::new("pkexec").args(["sh", "-c", &script]).status().await.is_ok_and(|s| s.success());
    if allowed {
        logs::system(app, Level::Info, format!("Ports from {lowest} up can now be used without administrator rights"));
        Ok(())
    } else {
        Err(Error::msg(format!(
            "{label} needs port {lowest}, and Linux only lets administrators use ports below 1024. Allow it when asked, or choose ports above 1024 (such as 8080 and 8443) in Settings → Ports."
        )))
    }
}

use crate::diagnose::now_ms;

struct Handle {
    pid: Option<u32>,
    state: ServiceState,
    started_at: Option<u64>,
    /// Taken when a stop has been requested.
    stop_tx: Option<oneshot::Sender<()>>,
}

/// Crash restarts counted against `MAX_RESTARTS`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Budget {
    used: u32,
    /// When the first restart of the current window happened (ms since epoch).
    since: u64,
}

impl Budget {
    /// Takes one restart at `now`, returning the attempt number, or None when the service
    /// already crashed `MAX_RESTARTS` times within `RESTART_WINDOW`. Uptime doesn't
    /// refill it: a service that comes up and crashes every minute isn't fixed by
    /// restarting it forever.
    fn take(&mut self, now: u64) -> Option<u32> {
        if self.used == 0 || now.saturating_sub(self.since) > RESTART_WINDOW.as_millis() as u64 {
            *self = Budget { used: 0, since: now };
        }
        if self.used >= MAX_RESTARTS {
            return None;
        }
        self.used += 1;
        Some(self.used)
    }
}

pub struct Supervisor {
    running: Arc<Mutex<HashMap<ServiceKind, Handle>>>,
    /// PHP FastCGI pools, started and stopped with the web servers.
    pub php: Arc<PhpPools>,
    problems: Arc<std::sync::Mutex<HashMap<ServiceKind, Problem>>>,
    health: Arc<std::sync::Mutex<HashMap<ServiceKind, Health>>>,
    /// Services that crashed and wait for `restart_crashed`; Stop or Start clears them.
    pending: Arc<std::sync::Mutex<HashSet<ServiceKind>>>,
    budgets: Arc<std::sync::Mutex<HashMap<ServiceKind, Budget>>>,
    crashed_tx: mpsc::UnboundedSender<ServiceKind>,
    crashed_rx: std::sync::Mutex<Option<mpsc::UnboundedReceiver<ServiceKind>>>,
}

impl Default for Supervisor {
    fn default() -> Self {
        let (crashed_tx, crashed_rx) = mpsc::unbounded_channel();
        Self {
            running: Arc::default(),
            php: Arc::default(),
            problems: Arc::default(),
            health: Arc::default(),
            pending: Arc::default(),
            budgets: Arc::default(),
            crashed_tx,
            crashed_rx: std::sync::Mutex::new(Some(crashed_rx)),
        }
    }
}

impl Supervisor {
    /// Services that exited unexpectedly, for the app to restart when Settings says so
    /// (it has the settings; the exit watcher doesn't). Can be taken once.
    pub fn crashes(&self) -> Option<mpsc::UnboundedReceiver<ServiceKind>> {
        self.crashed_rx.lock().unwrap().take()
    }

    /// Restarts a crashed service, unless it was stopped or started by hand meanwhile or
    /// it keeps crashing.
    pub async fn restart_crashed(&self, app: &AppHandle, kind: ServiceKind, settings: &Settings, layout: &Layout) -> Result<()> {
        if !self.pending.lock().unwrap().remove(&kind) || !settings.restarts_after_crash(kind.id()) {
            return Ok(());
        }
        let Some(attempt) = self.budgets.lock().unwrap().entry(kind).or_default().take(now_ms()) else {
            logs::system(
                app,
                Level::Error,
                format!("{} crashed {MAX_RESTARTS} times in a few minutes, so it isn't restarted again; check its log, then start it yourself", kind.label()),
            );
            return Ok(());
        };
        logs::system(app, Level::Warn, format!("Restarting {} after a crash (attempt {attempt} of {MAX_RESTARTS})", kind.label()));
        self.start_service(app, kind, settings, layout).await
    }

    pub async fn list(&self, settings: &Settings, layout: &Layout) -> Vec<ServiceInfo> {
        let running = self.running.lock().await;
        let problems = self.problems.lock().unwrap().clone();
        let health = self.health.lock().unwrap().clone();
        ServiceKind::all()
            .into_iter()
            .map(|kind| {
                let handle = running.get(&kind);
                let installed = launch_spec(kind, settings, layout)
                    .map(|s| s.program.exists())
                    .unwrap_or(false);
                // The Ollama app usually serves on its own; that counts as running.
                let external = handle.is_none() && kind == ServiceKind::Ollama && installed && answers(kind.port(settings));
                ServiceInfo {
                    kind,
                    label: kind.label(),
                    external,
                    state: if external { ServiceState::Running } else { handle.map_or(ServiceState::Stopped, |h| h.state) },
                    pid: handle.and_then(|h| h.pid),
                    port: kind.port(settings),
                    version: kind.version(settings),
                    installed,
                    started_at: handle.and_then(|h| h.started_at),
                    config_path: kind.config_path(settings, layout),
                    log_path: kind.log_path(layout),
                    health: handle.filter(|h| h.state == ServiceState::Running).and_then(|_| health.get(&kind).cloned()),
                    problem: problems.get(&kind).cloned(),
                    heavy: kind.heavy(),
                    connection: kind.connection(settings, layout),
                }
            })
            .collect()
    }

    pub fn clear_problem(&self, kind: ServiceKind) {
        self.problems.lock().unwrap().remove(&kind);
    }

    fn set_problem(&self, kind: ServiceKind, problem: Problem) {
        self.problems.lock().unwrap().insert(kind, problem);
    }

    /// PIDs of everything LampRabit runs, to recognise its own ports.
    pub async fn pids(&self) -> HashMap<u32, ServiceKind> {
        self.running.lock().await.iter().filter_map(|(k, h)| Some((h.pid?, *k))).collect()
    }

    /// Probes every running service and emits `service-health` when a status changes.
    pub async fn check_health(&self, app: &AppHandle, settings: &Settings, client: &reqwest::Client) {
        let running: Vec<(ServiceKind, u64)> = self
            .running
            .lock()
            .await
            .iter()
            .filter(|(_, h)| h.state == ServiceState::Running)
            .filter_map(|(k, h)| Some((*k, h.started_at?)))
            .collect();
        self.health.lock().unwrap().retain(|k, _| running.iter().any(|(r, _)| r == k));

        for (kind, started_at) in running {
            let probe = kind.probe(settings, client).await;
            // Slow services are only Running once they answered; give the rest a moment.
            let young = now_ms().saturating_sub(started_at) < 15_000;
            let changed = {
                let mut map = self.health.lock().unwrap();
                let prev = map.get(&kind).cloned();
                let failures = match &probe {
                    Probe::Ok => 0,
                    Probe::Bad(_) => prev.as_ref().map_or(0, |p| p.failures) + 1,
                };
                // One failed probe can be a blip; two in a row is a real problem.
                let (status, detail) = match probe {
                    Probe::Ok => ("healthy", None),
                    Probe::Bad(_) if young => ("checking", None),
                    Probe::Bad(d) if failures >= 2 => ("degraded", Some(d)),
                    Probe::Bad(_) => prev.as_ref().map_or(("checking", None), |p| (p.status, p.detail.clone())),
                };
                let changed = prev.as_ref().is_none_or(|p| p.status != status || p.detail != detail);
                if changed && status == "degraded" {
                    logs::system(app, Level::Warn, format!("{} is not responding: {}", kind.label(), detail.clone().unwrap_or_default()));
                }
                if changed && status == "healthy" && prev.as_ref().is_some_and(|p| p.status == "degraded") {
                    logs::system(app, Level::Info, format!("{} is responding again", kind.label()));
                }
                map.insert(kind, Health { status, detail, checked_at: now_ms(), failures });
                changed
            };
            if changed {
                let _ = app.emit("service-health", kind);
            }
        }
    }

    pub async fn is_running(&self, kind: ServiceKind) -> bool {
        self.running.lock().await.contains_key(&kind)
    }

    /// Waits until a stopping service has fully exited.
    pub async fn wait_stopped(&self, kind: ServiceKind) {
        let deadline = tokio::time::Instant::now() + STOP_TIMEOUT + Duration::from_secs(2);
        while self.is_running(kind).await && tokio::time::Instant::now() < deadline {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    pub async fn start(
        &self,
        app: &AppHandle,
        kind: ServiceKind,
        settings: &Settings,
        layout: &Layout,
    ) -> Result<()> {
        // Started by hand: a restart that was waiting isn't needed, and the crash count
        // starts over since someone is looking at it.
        self.pending.lock().unwrap().remove(&kind);
        self.budgets.lock().unwrap().remove(&kind);
        self.start_service(app, kind, settings, layout).await
    }

    async fn start_service(&self, app: &AppHandle, kind: ServiceKind, settings: &Settings, layout: &Layout) -> Result<()> {
        // The Ollama app already serving is what's wanted; there's nothing to start.
        if kind == ServiceKind::Ollama && !self.is_running(kind).await && answers(kind.port(settings)) {
            logs::system(app, Level::Info, "Ollama is already running (the Ollama app); LampRabit uses it as is");
            return Ok(());
        }
        {
            let mut running = self.running.lock().await;
            if running.contains_key(&kind) {
                return Ok(());
            }
            // Reserve the slot so concurrent starts don't race during setup.
            running.insert(
                kind,
                Handle { pid: None, state: ServiceState::Starting, started_at: None, stop_tx: None },
            );
        }
        self.clear_problem(kind);
        self.health.lock().unwrap().remove(&kind);
        emit_status(app, kind, ServiceState::Starting, None);
        let version = kind.version(settings).unwrap_or_default();
        logs::system(app, Level::Info, format!("Starting {} {version}", kind.label()));

        match self.spawn(app, kind, settings, layout).await {
            Ok(()) => Ok(()),
            Err(e) => {
                self.running.lock().await.remove(&kind);
                if !self.problems.lock().unwrap().contains_key(&kind) {
                    self.set_problem(kind, Problem::new(format!("{} couldn't start", kind.label()), e.to_string(), &[]));
                }
                // Error without a message: the UI records it as a failed start; the caller shows the error.
                emit_status(app, kind, ServiceState::Error, None);
                logs::system(app, Level::Error, format!("{} failed to start: {e}", kind.label()));
                Err(e)
            }
        }
    }

    async fn spawn(&self, app: &AppHandle, kind: ServiceKind, settings: &Settings, layout: &Layout) -> Result<()> {
        let spec = launch_spec(kind, settings, layout)?;
        if !spec.program.exists() {
            return Err(Error::msg(format!(
                "{} binary not found at {}",
                kind.label(),
                spec.program.display()
            )));
        }
        let port = kind.port(settings);
        #[cfg(target_os = "linux")]
        if kind.web_server().is_some() {
            allow_low_ports(app, kind.label(), &required_ports(kind, settings, layout)).await?;
        }
        for p in required_ports(kind, settings, layout) {
            if !port_free(p) {
                let problem = diagnose::port_problem(kind.label(), p, diagnose::port_owner(p).await);
                let message = problem.detail.clone();
                self.set_problem(kind, problem);
                return Err(Error::msg(message));
            }
        }
        let upgrade_from = if kind == ServiceKind::Mariadb {
            match configs::mariadb_version_guard(settings, layout) {
                Ok(previous) => previous,
                Err(e) => {
                    self.set_problem(
                        kind,
                        Problem::new("MariaDB data is from a newer version", e.to_string(), &["Switch back on the Services page."]),
                    );
                    return Err(e);
                }
            }
        } else {
            None
        };
        let plan = prepare(app, kind, settings, layout).await?;
        if kind == ServiceKind::Mariadb && upgrade_from.is_none() && !configs::mariadb_marker_exists(layout) {
            configs::write_mariadb_marker(settings, layout);
        }
        if kind.web_server().is_some() {
            if let Some(output) = server_config_test(&spec).await {
                let line = output
                    .lines()
                    .find(|l| l.contains("rror") || l.contains("emerg"))
                    .unwrap_or(&output)
                    .to_string();
                self.set_problem(
                    kind,
                    Problem::new(
                        format!("{} configuration error", kind.label()),
                        line.clone(),
                        &["If the error points into etc/apache/sites/, fix or remove that file. LampRabit's own config files are regenerated on every start."],
                    ),
                );
                return Err(Error::msg(format!("{} configuration test failed: {line}", kind.label())));
            }
            // PHP has to be listening before the web server takes requests. FrankenPHP
            // runs PHP itself.
            let plan = if kind == ServiceKind::Frankenphp { Vec::new() } else { plan };
            if let Err(e) = self.php.ensure(app, settings, layout, &plan).await {
                self.set_problem(kind, Problem::new("PHP couldn't start", e.to_string(), &[]));
                return Err(e);
            }
        }

        let log = OpenOptions::new()
            .create(true)
            .append(true)
            .open(layout.logs().join(format!("{}-output.log", kind.id())))
            .await?;
        let log = Arc::new(Mutex::new(log));

        let mut cmd = Command::new(&spec.program);
        cmd.args(&spec.args)
            .current_dir(&spec.cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        if !spec.path_prepend.is_empty() {
            let existing = std::env::var_os("PATH").unwrap_or_default();
            let paths = spec.path_prepend.iter().cloned().chain(std::env::split_paths(&existing));
            cmd.env("PATH", std::env::join_paths(paths).unwrap_or(existing));
        }
        for (k, v) in &spec.env {
            cmd.env(k, v);
        }
        procgroup::prepare(&mut cmd);

        let mut child = cmd.spawn()?;
        let group = ProcGroup::attach(&child);
        if let Err(e) = &group {
            eprintln!("could not attach {} to a process group: {e}", kind.label());
        }
        pipe_output(app.clone(), kind, child.stdout.take(), log.clone());
        pipe_output(app.clone(), kind, child.stderr.take(), log);

        let (stop_tx, stop_rx) = oneshot::channel();
        logs::system(
            app,
            Level::Info,
            format!("{} started (PID {}, port {port})", kind.label(), child.id().unwrap_or_default()),
        );
        let slow = kind.slow();
        let state = if slow { ServiceState::Starting } else { ServiceState::Running };
        self.running.lock().await.insert(
            kind,
            Handle { pid: child.id(), state, started_at: Some(now_ms()), stop_tx: Some(stop_tx) },
        );
        emit_status(app, kind, state, None);
        if slow {
            let (running, health, app, settings) = (self.running.clone(), self.health.clone(), app.clone(), settings.clone());
            tauri::async_runtime::spawn(async move {
                let client = reqwest::Client::builder().timeout(Duration::from_secs(2)).build().unwrap_or_default();
                let deadline = tokio::time::Instant::now() + READY_TIMEOUT;
                let mut answered = false;
                while !answered && tokio::time::Instant::now() < deadline {
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    // Stopped, crashed or replaced meanwhile: the exit watcher reports that.
                    if !running.lock().await.get(&kind).is_some_and(|h| h.state == ServiceState::Starting) {
                        return;
                    }
                    answered = matches!(kind.probe(&settings, &client).await, Probe::Ok);
                }
                {
                    let mut running = running.lock().await;
                    let Some(h) = running.get_mut(&kind).filter(|h| h.state == ServiceState::Starting) else { return };
                    h.state = ServiceState::Running;
                }
                // Still running but not answering: it's up, though not healthy, and the
                // health checks keep trying.
                let (status, detail) = if answered {
                    ("healthy", None)
                } else {
                    let secs = READY_TIMEOUT.as_secs();
                    logs::system(&app, Level::Warn, format!("{} didn't answer within {secs} s; check its log", kind.label()));
                    ("degraded", Some(format!("No answer within {secs} s of starting")))
                };
                health.lock().unwrap().insert(kind, Health { status, detail, checked_at: now_ms(), failures: if answered { 0 } else { 2 } });
                emit_status(&app, kind, ServiceState::Running, None);
                let _ = app.emit("service-health", kind);
            });
        }
        if kind == ServiceKind::Postgres {
            let (app, settings, layout) = (app.clone(), settings.clone(), layout.clone());
            tauri::async_runtime::spawn(async move {
                if let Err(e) = configs::postgres_root_role(&settings, &layout).await {
                    logs::system(&app, Level::Warn, format!("Couldn't add the root role to PostgreSQL: {e}"));
                }
            });
        }
        if let Some(previous) = upgrade_from {
            let (app, settings, layout) = (app.clone(), settings.clone(), layout.clone());
            let port = settings.ports.service("mariadb.mysql");
            tauri::async_runtime::spawn(async move {
                for _ in 0..60 {
                    if matches!(diagnose::probe_mysql(port).await, Probe::Ok) {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
                logs::system(&app, Level::Info, format!("Upgrading MariaDB system tables from {previous}"));
                match configs::mariadb_upgrade(&settings, &layout).await {
                    Ok(_) => logs::system(&app, Level::Info, "MariaDB upgrade finished"),
                    Err(e) => logs::system(&app, Level::Error, e.to_string()),
                }
            });
        }

        let running = self.running.clone();
        let problems = self.problems.clone();
        let (pending, crashed) = (self.pending.clone(), self.crashed_tx.clone());
        let pools = self.php.clone();
        let logs_to_read = [kind.log_path(layout), layout.logs().join(format!("{}-output.log", kind.id()))];
        let app = app.clone();
        let stop_cmd = spec.stop_cmd;
        // The stop command (mysqladmin, nginx -s quit) needs the same library path.
        let stop_env = spec.env.clone();
        tauri::async_runtime::spawn(async move {
            let group = group.ok();
            let (state, message) = tokio::select! {
                status = child.wait() => match status {
                    Ok(s) if s.success() => (ServiceState::Stopped, None),
                    Ok(s) => {
                        let tail: String = logs_to_read.iter().map(|p| diagnose::tail(p, 8 * 1024)).collect();
                        let problem = diagnose::explain_exit(kind.label(), s.code(), &tail);
                        let message = format!("{}: {}", problem.title, problem.detail);
                        problems.lock().unwrap().insert(kind, problem);
                        (ServiceState::Error, Some(message))
                    }
                    Err(e) => (ServiceState::Error, Some(e.to_string())),
                },
                _ = stop_rx => {
                    let asked = match &stop_cmd {
                        Some((program, args)) => {
                            let mut c = Command::new(program);
                            c.args(args).envs(stop_env.iter().map(|(k, v)| (*k, v))).stdout(Stdio::null()).stderr(Stdio::null());
                            procgroup::prepare(&mut c);
                            c.status().await.is_ok_and(|s| s.success())
                        }
                        None => group.as_ref().is_some_and(|g| g.signal_term()),
                    };
                    if !asked || tokio::time::timeout(STOP_TIMEOUT, child.wait()).await.is_err() {
                        match &group {
                            Some(g) => g.kill(),
                            None => { let _ = child.start_kill(); }
                        }
                    }
                    let _ = child.wait().await;
                    (ServiceState::Stopped, None)
                }
            };
            // Clean up anything the service left behind (e.g. Apache's worker process).
            if let Some(g) = &group {
                g.kill();
            }
            drop(group);
            let web_servers_left = {
                let mut running = running.lock().await;
                running.remove(&kind);
                running.keys().any(|k| k.web_server().is_some())
            };
            if kind.web_server().is_some() && !web_servers_left {
                pools.stop_all(&app).await;
            }
            match &message {
                Some(m) => logs::system(&app, Level::Error, m.clone()),
                None => logs::system(&app, Level::Info, format!("{} stopped", kind.label())),
            }
            emit_status(&app, kind, state, message);
            // Only a crash: a clean exit or a requested stop ends with Stopped.
            if state == ServiceState::Error {
                pending.lock().unwrap().insert(kind);
                let _ = crashed.send(kind);
            }
        });

        Ok(())
    }

    pub async fn stop(&self, app: &AppHandle, kind: ServiceKind) {
        if self.pending.lock().unwrap().remove(&kind) {
            logs::system(app, Level::Info, format!("{} won't be restarted after its crash", kind.label()));
        }
        let mut running = self.running.lock().await;
        if let Some(handle) = running.get_mut(&kind)
            && let Some(tx) = handle.stop_tx.take() {
                handle.state = ServiceState::Stopping;
                let _ = tx.send(());
                emit_status(app, kind, ServiceState::Stopping, None);
                logs::system(app, Level::Info, format!("Stopping {}", kind.label()));
            }
    }

    /// Applies changed sites to a running web server without dropping its connections:
    /// Nginx re-reads its config (`-s reload`, every OS) and Apache on Linux and macOS
    /// restarts gracefully (SIGUSR1). The PHP pools are matched to the sites first, and the
    /// new config is tested before the server sees it, since Apache exits on a graceful
    /// restart with a broken config. Returns false when the server has to be restarted
    /// instead: Apache on Windows has no graceful restart as a console process.
    pub async fn reload(&self, app: &AppHandle, kind: ServiceKind, settings: &Settings, layout: &Layout) -> Result<bool> {
        let Some(server) = kind.web_server() else { return Ok(false) };
        let pid = match self.running.lock().await.get(&kind) {
            Some(h) if h.state == ServiceState::Running => h.pid,
            _ => return Ok(true),
        };
        if cfg!(windows) && kind == ServiceKind::Apache {
            return Ok(false);
        }
        let sites = sites::served(settings, layout);
        let plan = php::plan(settings, layout, &sites);
        if kind != ServiceKind::Frankenphp {
            self.php.ensure(app, settings, layout, &plan).await?;
        }
        sites::write_config(settings, layout, &sites, &plan, server)?;
        if kind == ServiceKind::Apache {
            configs::write_apache(settings, layout, &plan)?;
        }
        let spec = launch_spec(kind, settings, layout)?;
        if let Some(output) = server_config_test(&spec).await {
            return Err(Error::msg(format!("{} kept its current sites because the new config has an error:\n{output}", kind.label())));
        }
        if let Some(admin) = admin_port(kind, settings) {
            // Through the admin API: Caddy swaps configs without dropping connections.
            let mut args = spec.test.clone().unwrap_or_default();
            args[0] = "reload".into();
            args.extend(["--address".into(), format!("127.0.0.1:{admin}")]);
            let mut cmd = Command::new(&spec.program);
            cmd.args(args).current_dir(&spec.cwd).envs(spec.env.iter().map(|(k, v)| (*k, v)));
            procgroup::prepare(&mut cmd);
            let out = cmd.output().await?;
            if !out.status.success() {
                return Err(Error::msg(format!("{} didn't reload: {}", kind.label(), String::from_utf8_lossy(&out.stderr).trim())));
            }
        } else if kind == ServiceKind::Nginx {
            let (program, mut args) = spec.stop_cmd.ok_or_else(|| Error::msg("Nginx has no control command"))?;
            if let Some(last) = args.last_mut() {
                *last = "reload".into();
            }
            let mut cmd = Command::new(program);
            cmd.args(args).current_dir(&spec.cwd);
            procgroup::prepare(&mut cmd);
            let out = cmd.output().await?;
            if !out.status.success() {
                return Err(Error::msg(format!("Nginx didn't reload: {}", String::from_utf8_lossy(&out.stderr).trim())));
            }
        } else {
            #[cfg(unix)]
            if let Some(pid) = pid {
                // SAFETY: signals the Apache parent process this supervisor started.
                unsafe { libc::kill(pid as i32, libc::SIGUSR1) };
            }
            #[cfg(not(unix))]
            let _ = pid;
        }
        logs::system(app, Level::Info, format!("{} reloaded its sites", kind.label()));
        Ok(true)
    }

    /// Stops everything and waits (bounded) for the processes to exit.
    pub async fn shutdown(&self) {
        {
            let mut running = self.running.lock().await;
            for handle in running.values_mut() {
                if let Some(tx) = handle.stop_tx.take() {
                    let _ = tx.send(());
                }
            }
        }
        let deadline = tokio::time::Instant::now() + STOP_TIMEOUT + Duration::from_secs(2);
        while tokio::time::Instant::now() < deadline {
            if self.running.lock().await.is_empty() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }
}

fn emit_status(app: &AppHandle, kind: ServiceKind, state: ServiceState, message: Option<String>) {
    let _ = app.emit("service-status", StatusEvent { kind, state, message });
}

fn pipe_output<R>(app: AppHandle, kind: ServiceKind, reader: Option<R>, log: Arc<Mutex<File>>)
where
    R: AsyncRead + Unpin + Send + 'static,
{
    let Some(reader) = reader else { return };
    tauri::async_runtime::spawn(async move {
        let mut lines = BufReader::new(reader).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let _ = log.lock().await.write_all(format!("{line}\n").as_bytes()).await;
            crate::logs::push(&app, kind.id(), "output", line);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtimes::Runtime;

    const MIN: u64 = 60_000;

    const ECHO: &str = r#"
schema: 1
id: echo
name: Echo
category: tools
blurb: Says things back
source:
  github: someone/echo
  assets:
    windows: { file: "echo-{version}-windows.zip", archive: zip }
    linux: { file: "echo-{version}-linux.tar.gz", archive: targz }
    macos: { file: "echo-{version}-macos.tar.gz", archive: targz }
binary: "echo{{exe}}"
ports:
  http: { default: 7000, label: Web }
run:
  program: "{{home}}/echo{{exe}}"
  args: [--port, "{{port.http}}"]
health: { http: { path: /health } }
"#;

    /// Registers the community service once for every test that needs it (they share the
    /// process, and ids must be unique).
    fn echo() -> u16 {
        static INDEX: std::sync::OnceLock<u16> = std::sync::OnceLock::new();
        *INDEX.get_or_init(|| manifest::register(manifest::parse(ECHO, false).unwrap()).unwrap())
    }

    #[test]
    fn a_manifest_only_service_works_without_an_enum_variant() {
        let i = echo();
        let kind = ServiceKind::from_id("echo").unwrap();
        assert_eq!(kind, ServiceKind::Plugin(i));
        assert_eq!(serde_json::to_string(&kind).unwrap(), "\"echo\"");
        assert_eq!(serde_json::from_str::<ServiceKind>("\"echo\"").unwrap(), kind);
        assert_eq!(kind.label(), "Echo");

        let runtime = Runtime::from_id("echo").unwrap();
        assert_eq!(runtime, Runtime::Plugin(i));
        assert_eq!(serde_json::from_str::<Runtime>("\"echo\"").unwrap(), runtime);
        assert_eq!(runtime.marker(), PathBuf::from(if cfg!(windows) { "echo.exe" } else { "echo" }));
        assert_eq!(crate::cmd::dependents(runtime), vec![kind]);

        // Its default port is another service's and 7001 is busy: it gets 7002, kept on reinstall.
        let mut settings = Settings::default();
        settings.ports.services.insert("other.http".into(), 7000);
        kind.manifest().assign_ports(&mut settings, |p| p != 7001);
        assert_eq!(settings.ports.services.get("echo.http"), Some(&7002));
        kind.manifest().assign_ports(&mut settings, |_| true);
        assert_eq!(kind.port_of(&settings), 7002);

        // It launches from its manifest once installed.
        settings.versions.set("echo", Some("1.0".into()));
        let layout = Layout::new(std::env::temp_dir().join("lr-plugin-service"));
        let home = layout.bin("echo", "1.0");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::write(home.join(runtime.marker()), "").unwrap();
        let spec = launch_spec(kind, &settings, &layout).unwrap();
        assert_eq!(spec.program, home.join(runtime.marker()));
        assert_eq!(spec.args, vec!["--port".to_string(), "7002".to_string()]);

        // The Services page lists it in its category, with its port to edit.
        let catalog = crate::catalog::catalog();
        assert!(catalog.groups.iter().any(|g| g.id == "tools" && g.items.contains(&"echo")));
        let info = catalog.runtimes.iter().find(|r| r.id == "echo").unwrap();
        assert_eq!((info.ports[0].key.as_str(), info.ports[0].label.as_deref(), info.ports[0].default), ("echo.http", Some("Web"), 7000));
    }

    #[test]
    fn local_service_files_load_update_and_hide() {
        let dir = std::env::temp_dir().join(format!("lr-local-services-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let put = |folder: &str, text: &str| {
            std::fs::create_dir_all(dir.join(folder)).unwrap();
            std::fs::write(dir.join(folder).join("service.yaml"), text).unwrap();
        };
        let local = ECHO.replace("id: echo", "id: echo-local");
        put("echo-local", &local);
        put("elsewhere", &ECHO.replace("id: echo", "id: echo-elsewhere"));
        put("broken", "schema: 1\nid: broken\n");
        let problems = manifest::load_local(&dir);
        assert_eq!(problems.len(), 2, "{problems:?}");
        assert!(problems.iter().any(|p| p.contains("doesn't match its folder")));
        assert!(ServiceKind::from_id("echo-elsewhere").is_none());
        let kind = ServiceKind::from_id("echo-local").unwrap();

        // Adding the same id again updates that service in place.
        let i = manifest::register(manifest::parse(&local.replace("name: Echo", "name: Echo 2"), false).unwrap()).unwrap();
        assert_eq!(ServiceKind::Plugin(i), kind);
        assert_eq!(kind.label(), "Echo 2");
        // Removing hides it everywhere; adding it back reuses its index.
        manifest::unregister("echo-local");
        assert!(ServiceKind::from_id("echo-local").is_none() && Runtime::from_id("echo-local").is_none());
        assert!(!crate::catalog::catalog().runtimes.iter().any(|r| r.id == "echo-local"));
        assert_eq!(manifest::register(manifest::parse(&local, false).unwrap()).unwrap(), i);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_service_file_review_says_what_it_does() {
        let r = manifest::parse(ECHO, false).unwrap().review();
        assert_eq!(r.downloads.as_deref(), Some("GitHub releases of someone/echo"));
        assert_eq!(r.checksum.as_deref(), Some("The SHA-256 GitHub reports for each file"));
        assert_eq!(r.program.as_deref(), Some("{{home}}/echo{{exe}}"));
        assert_eq!(r.args, vec!["--port", "{{port.http}}"]);
        assert_eq!(r.ports, vec![("Web".to_string(), 7000)]);
    }

    #[test]
    fn a_manifest_only_service_cant_take_a_built_in_id() {
        let m = manifest::parse(&ECHO.replace("id: echo", "id: redis"), false).unwrap();
        assert!(manifest::register(m).unwrap_err().to_string().contains("already has a service"));
    }

    #[test]
    fn restart_budget_allows_three_then_stops() {
        let mut b = Budget::default();
        assert_eq!(b.take(0), Some(1));
        assert_eq!(b.take(MIN), Some(2));
        assert_eq!(b.take(2 * MIN), Some(3));
        assert_eq!(b.take(3 * MIN), None);
        // Being refused doesn't extend the window.
        assert_eq!(b.since, 0);
    }

    #[test]
    fn restart_budget_refills_after_the_window() {
        let mut b = Budget::default();
        for t in 0..3 {
            b.take(t * MIN);
        }
        assert_eq!(b.take(4 * MIN), None);
        assert_eq!(b.take(6 * MIN), Some(1));
    }

    #[test]
    fn restart_budget_window_starts_at_the_first_crash() {
        // A service that ran for an hour before crashing gets a fresh window.
        let mut b = Budget::default();
        assert_eq!(b.take(60 * MIN), Some(1));
        assert_eq!(b.since, 60 * MIN);
    }

    #[test]
    fn slow_services_have_a_real_check() {
        // A bare TCP connect answers before a JVM service is ready; only RabbitMQ (no
        // HTTP on its AMQP port) is allowed that.
        for kind in ServiceKind::all().into_iter().filter(|k| k.slow()) {
            assert!(matches!(kind.manifest().check_kind(), Check::Http(..)) || kind == ServiceKind::Rabbitmq, "{kind:?}");
        }
    }
}
