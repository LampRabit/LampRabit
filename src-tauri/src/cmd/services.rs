//! Services: start, stop, restart, Start all, usage and ports.

use crate::*;

/// Services that load the given runtime and must restart when it changes.
pub(crate) fn dependents(runtime: Runtime) -> Vec<ServiceKind> {
    let kinds: &[ServiceKind] = match runtime {
        Runtime::Apache => &[ServiceKind::Apache],
        Runtime::Nginx => &[ServiceKind::Nginx],
        Runtime::Caddy => &[ServiceKind::Caddy],
        Runtime::Frankenphp => &[ServiceKind::Frankenphp],
        Runtime::Tomcat => &[ServiceKind::Tomcat],
        Runtime::Postgres => &[ServiceKind::Postgres],
        Runtime::Pgadmin => &[ServiceKind::Pgadmin],
        Runtime::Mongodb => &[ServiceKind::Mongodb],
        // The shell runs from the terminal; MongoDB uses it only to shut down.
        Runtime::Mongosh => &[],
        // Command-line tools and a desktop app: nothing runs them in the background.
        Runtime::Mongotools | Runtime::Compass | Runtime::Sqlite => &[],
        Runtime::Clickhouse => &[ServiceKind::Clickhouse],
        Runtime::Neo4j => &[ServiceKind::Neo4j],
        Runtime::Meilisearch => &[ServiceKind::Meilisearch],
        Runtime::Typesense => &[ServiceKind::Typesense],
        Runtime::Rustfs => &[ServiceKind::Rustfs],
        Runtime::Ollama => &[ServiceKind::Ollama],
        Runtime::Minio => &[ServiceKind::Minio],
        Runtime::Elasticsearch => &[ServiceKind::Elasticsearch],
        Runtime::Opensearch => &[ServiceKind::Opensearch],
        Runtime::N8n => &[ServiceKind::N8n],
        Runtime::Rabbitmq | Runtime::Erlang => &[ServiceKind::Rabbitmq],
        // Languages run from the terminal and projects' own processes.
        Runtime::Go | Runtime::Rust | Runtime::Maven | Runtime::Gradle | Runtime::Git => &[],
        Runtime::Bun | Runtime::Deno | Runtime::Uv | Runtime::Conda => &[],
        // Projects run Python themselves; pgAdmin keeps the Python it was installed with.
        Runtime::Python => &[],
        Runtime::Phppgadmin => &ServiceKind::WEB,
        // PHP pools are owned by the web servers; restarting them restarts PHP.
        Runtime::Php => &ServiceKind::WEB,
        Runtime::Mariadb => &[ServiceKind::Mariadb],
        Runtime::Mysql => &[ServiceKind::Mysql],
        Runtime::Mailpit => &[ServiceKind::Mailpit],
        // Served by the primary web server through their own virtual hosts.
        Runtime::Adminer | Runtime::Phpmyadmin | Runtime::Phpredisadmin => &ServiceKind::WEB,
        // Tunnels start on demand; Composer runs from the terminal.
        Runtime::Ngrok | Runtime::Cloudflared | Runtime::Composer => &[],
        // Tomcat runs on the active Java.
        Runtime::Java => &[ServiceKind::Tomcat, ServiceKind::Neo4j],
        Runtime::Redis => &[ServiceKind::Redis],
        Runtime::Memcached => &[ServiceKind::Memcached],
        // A manifest-only service runs its own runtime, under the same index.
        Runtime::Plugin(i) => return vec![ServiceKind::Plugin(i)],
    };
    kinds.to_vec()
}

/// Restarts whichever web servers are running, so they (and their PHP pools) pick up
/// changed sites or PHP versions together.
pub(crate) async fn restart_web_servers(app: &AppHandle, state: &AppState) -> Result<()> {
    let mut running = Vec::new();
    for kind in ServiceKind::WEB {
        if state.supervisor.is_running(kind).await {
            state.supervisor.stop(app, kind).await;
            running.push(kind);
        }
    }
    // Tomcat reads its hosts only at start; restart it when the Java apps changed.
    {
        let (settings, layout) = state.snapshot().await;
        if state.supervisor.is_running(ServiceKind::Tomcat).await
            && configs::tomcat_stale(&settings, &layout, &sites::tomcat_apps(&settings, &layout))
        {
            state.supervisor.stop(app, ServiceKind::Tomcat).await;
            running.push(ServiceKind::Tomcat);
        }
    }
    for &kind in &running {
        state.supervisor.wait_stopped(kind).await;
    }
    let (settings, layout) = state.snapshot().await;
    for kind in running {
        state.supervisor.start(app, kind, &settings, &layout).await?;
    }
    Ok(())
}

#[tauri::command]
pub(crate) async fn list_services(state: State<'_, AppState>) -> Result<Vec<ServiceInfo>> {
    let (settings, layout) = state.snapshot().await;
    Ok(state.supervisor.list(&settings, &layout).await)
}

#[tauri::command]
pub(crate) async fn start_service(app: AppHandle, state: State<'_, AppState>, kind: ServiceKind) -> Result<()> {
    let _op = state.ops.lock().await;
    let (settings, layout) = state.snapshot().await;
    state.supervisor.start(&app, kind, &settings, &layout).await?;
    serve_tool_domain(&app, &state, kind).await;
    Ok(())
}

/// Services with a web UI of their own at a .test domain (n8n.test, pgadmin.test…), which
/// the primary web server proxies.
pub(crate) fn tool_domain_service(kind: ServiceKind) -> bool {
    matches!(kind, ServiceKind::N8n | ServiceKind::Pgadmin | ServiceKind::Neo4j | ServiceKind::Mailpit)
}

/// Makes a just-started service's .test domain answer: the primary web server is started
/// when it isn't running (which also brings the hosts file up to date), or reloaded when
/// its config doesn't have the domain yet. A failure is logged; the service itself runs.
pub(crate) async fn serve_tool_domain(app: &AppHandle, state: &AppState, kind: ServiceKind) {
    if !tool_domain_service(kind) {
        return;
    }
    let (settings, layout) = state.snapshot().await;
    let web = server_kind(settings.primary_server());
    let result = if !state.supervisor.is_running(web).await {
        let installed = state.supervisor.list(&settings, &layout).await.iter().any(|s| s.kind == web && s.installed);
        if !installed {
            return;
        }
        logs::system(app, logs::Level::Info, format!("Starting {} so {}'s .test address works", web.label(), kind.label()));
        state.supervisor.start(app, web, &settings, &layout).await
    } else {
        let served = sites::served(&settings, &layout);
        let plan = php::plan(&settings, &layout, &served);
        if !sites::config_stale(&settings, &layout, &served, &plan, settings.primary_server()) {
            return;
        }
        // A reload doesn't touch the hosts file the way a start does.
        services::sync_hosts(app, &settings, &layout, &served).await;
        match state.supervisor.reload(app, web, &settings, &layout).await {
            Ok(true) => Ok(()),
            // No graceful reload (Apache on Windows): a quick restart instead.
            Ok(false) => {
                state.supervisor.stop(app, web).await;
                state.supervisor.wait_stopped(web).await;
                state.supervisor.start(app, web, &settings, &layout).await
            }
            Err(e) => Err(e),
        }
    };
    if let Err(e) = result {
        logs::system(app, logs::Level::Warn, format!("{}'s .test address may not work yet: {e}", kind.label()));
    }
}

#[tauri::command]
pub(crate) async fn stop_service(app: AppHandle, state: State<'_, AppState>, kind: ServiceKind) -> Result<()> {
    let _op = state.ops.lock().await;
    state.supervisor.stop(&app, kind).await;
    // Finish stopping before the next queued operation looks at the state.
    state.supervisor.wait_stopped(kind).await;
    Ok(())
}

/// Starts every installed service except the heavy ones (`ServiceKind::heavy`), for the
/// tray, start-on-launch and `lamprabit start`; returns one error message per failure.
#[tauri::command]
pub(crate) async fn start_all(app: AppHandle, state: State<'_, AppState>) -> Result<Vec<String>> {
    let _op = state.ops.lock().await;
    let (settings, layout) = state.snapshot().await;
    let mut errors = Vec::new();
    for info in state.supervisor.list(&settings, &layout).await {
        if !info.installed || !info.kind.enabled(&settings) || info.heavy {
            continue;
        }
        if let Err(e) = state.supervisor.start(&app, info.kind, &settings, &layout).await {
            errors.push(format!("{}: {e}", info.label));
        }
    }
    Ok(errors)
}

/// Stops tunnels, project processes and services the way quitting does, before an app
/// update installs: on Windows the updater ends the app without the usual exit, which would
/// leave the databases running until the next start cleans them up.
#[tauri::command]
pub(crate) async fn prepare_for_update(state: State<'_, AppState>) -> Result<()> {
    let _op = state.ops.lock().await;
    state.tunnels.stop_all().await;
    state.apps.stop_all().await;
    state.supervisor.shutdown().await;
    Ok(())
}

#[tauri::command]
pub(crate) async fn stop_all(app: AppHandle, state: State<'_, AppState>) -> Result<()> {
    let _op = state.ops.lock().await;
    for kind in ServiceKind::all() {
        state.supervisor.stop(&app, kind).await;
    }
    for kind in ServiceKind::all() {
        state.supervisor.wait_stopped(kind).await;
    }
    Ok(())
}

/// Stops the project processes in `stop`, then starts those in `start` ("project/process"),
/// for stack profiles. Returns one message per process that failed to start; a process
/// that no longer exists is skipped.
#[tauri::command]
pub(crate) async fn apply_processes(app: AppHandle, state: State<'_, AppState>, start: Vec<String>, stop: Vec<String>) -> Result<Vec<String>> {
    for key in &stop {
        state.apps.stop(&app, key).await;
    }
    let (settings, layout) = state.snapshot().await;
    let mut errors = Vec::new();
    for key in &start {
        let Some((project, process)) = key.split_once('/') else { continue };
        let exists = settings.sites.get(project).is_some_and(|s| s.processes.iter().any(|p| p.name == process));
        if !exists {
            continue;
        }
        if let Err(e) = state.apps.start_process(&app, &settings, &layout, project, process).await {
            errors.push(format!("{key}: {e}"));
        }
    }
    Ok(errors)
}

/// Stops `stop`, then starts `start` (installed and enabled ones only), in one go: what
/// selection actions, stack profiles and Start/Stop all run. Returns one message per
/// service that failed to start.
#[tauri::command]
pub(crate) async fn apply_services(
    app: AppHandle,
    state: State<'_, AppState>,
    start: Vec<ServiceKind>,
    stop: Vec<ServiceKind>,
) -> Result<Vec<String>> {
    let _op = state.ops.lock().await;
    for &kind in &stop {
        state.supervisor.stop(&app, kind).await;
    }
    for &kind in &stop {
        state.supervisor.wait_stopped(kind).await;
    }
    let (settings, layout) = state.snapshot().await;
    let mut errors = Vec::new();
    for info in state.supervisor.list(&settings, &layout).await {
        if !start.contains(&info.kind) || !info.installed || !info.kind.enabled(&settings) {
            continue;
        }
        if let Err(e) = state.supervisor.start(&app, info.kind, &settings, &layout).await {
            errors.push(format!("{}: {e}", info.label));
        }
    }
    // Started together with the web server (Start all), the domains are already in its
    // config; on their own (a profile without it), it's brought up for them.
    for kind in start.into_iter().filter(|k| tool_domain_service(*k)) {
        if state.supervisor.is_running(kind).await {
            serve_tool_domain(&app, &state, kind).await;
        }
    }
    Ok(errors)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Usage {
    /// A service id ("apache", "mariadb"…) or "php" for the PHP workers.
    pub(crate) id: String,
    /// CPU like `top` shows it: 100 is one full core.
    pub(crate) cpu: f32,
    /// Resident memory in bytes.
    pub(crate) memory: u64,
    pub(crate) processes: usize,
}

/// CPU and memory of each running service, counting its child processes (Apache and
/// Nginx workers), plus PHP's workers as their own entry.
#[tauri::command]
pub(crate) async fn service_usage(state: State<'_, AppState>) -> Result<Vec<Usage>> {
    use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, UpdateKind};
    let (settings, layout) = state.snapshot().await;
    let roots: Vec<(ServiceKind, u32)> =
        state.supervisor.list(&settings, &layout).await.into_iter().filter_map(|s| Some((s.kind, s.pid?))).collect();
    let php_bin = layout.bin("php", "");

    if roots.is_empty() {
        // PHP's pools only run with a web server, so nothing to measure.
        return Ok(Vec::new());
    }
    let mut system = state.processes.lock().unwrap_or_else(|e| e.into_inner());
    // Reading every process takes tens of milliseconds (more on macOS); off the async
    // threads, so other commands don't wait behind it.
    tokio::task::block_in_place(|| {
        system.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing().with_cpu().with_memory().with_exe(UpdateKind::OnlyIfNotSet),
        )
    });
    let mut children: std::collections::HashMap<Pid, Vec<Pid>> = std::collections::HashMap::new();
    // On Linux every thread is listed too, each with its whole process's memory.
    let processes = || system.processes().iter().filter(|(_, p)| p.thread_kind().is_none());
    for (pid, process) in processes() {
        if let Some(parent) = process.parent() {
            children.entry(parent).or_default().push(*pid);
        }
    }
    let mut out = Vec::new();
    for (kind, root) in roots {
        let mut usage = Usage { id: kind.id().to_string(), cpu: 0.0, memory: 0, processes: 0 };
        let mut queue = vec![Pid::from_u32(root)];
        while let Some(pid) = queue.pop() {
            if let Some(p) = system.process(pid).filter(|p| p.thread_kind().is_none()) {
                usage.cpu += p.cpu_usage();
                usage.memory += p.memory();
                usage.processes += 1;
            }
            queue.extend(children.get(&pid).into_iter().flatten().copied());
        }
        out.push(usage);
    }
    let mut php = Usage { id: "php".into(), cpu: 0.0, memory: 0, processes: 0 };
    for (_, p) in processes().filter(|(_, p)| p.exe().is_some_and(|e| e.starts_with(&php_bin))) {
        php.cpu += p.cpu_usage();
        php.memory += p.memory();
        php.processes += 1;
    }
    if php.processes > 0 {
        out.push(php);
    }
    Ok(out)
}

/// The options a service's manifest declares (`settings:`), with their values.
#[tauri::command]
pub(crate) async fn service_options(state: State<'_, AppState>, kind: ServiceKind) -> Result<Vec<manifest::OptionInfo>> {
    Ok(kind.manifest().options(&*state.settings.read().await))
}

/// Changes one of them. It applies the next time the service starts; the card offers a
/// restart when it's running.
#[tauri::command]
pub(crate) async fn set_service_option(state: State<'_, AppState>, kind: ServiceKind, name: String, value: String) -> Result<Vec<manifest::OptionInfo>> {
    let mut settings = state.settings.write().await;
    let mut next = settings.clone();
    kind.manifest().set_option(&mut next, &name, &value)?;
    next.save(&state.settings_path)?;
    *settings = next;
    Ok(kind.manifest().options(&settings))
}

#[tauri::command]
pub(crate) async fn restart_service(app: AppHandle, state: State<'_, AppState>, kind: ServiceKind) -> Result<()> {
    let _op = state.ops.lock().await;
    logs::system(&app, logs::Level::Info, format!("Restarting {}", kind.label()));
    state.supervisor.stop(&app, kind).await;
    state.supervisor.wait_stopped(kind).await;
    let (settings, layout) = state.snapshot().await;
    state.supervisor.start(&app, kind, &settings, &layout).await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PortStatus {
    /// Settings key, e.g. "http" or "mailSmtp".
    pub(crate) key: &'static str,
    pub(crate) port: u16,
    pub(crate) free: bool,
    /// Set when one of LampRabit's own services holds the port.
    pub(crate) used_by_service: Option<&'static str>,
    pub(crate) owner: Option<diagnose::PortOwner>,
}

/// Who, if anyone, is on each configured port.
#[tauri::command]
pub(crate) async fn port_status(state: State<'_, AppState>) -> Result<Vec<PortStatus>> {
    let (settings, layout) = state.snapshot().await;
    let p = &settings.ports;
    let mut ports = vec![("http", p.http), ("https", p.https), ("mysql", p.mysql), ("mysqlServer", p.mysql_server), ("mailSmtp", p.mail_smtp), ("mailUi", p.mail_ui), ("phpFpm", p.php_fpm), ("redis", p.redis), ("memcached", p.memcached), ("postgres", p.postgres), ("pgadmin", p.pgadmin), ("mongodb", p.mongodb), ("clickhouseHttp", p.clickhouse_http), ("clickhouseTcp", p.clickhouse_tcp), ("neo4jHttp", p.neo4j_http), ("neo4jBolt", p.neo4j_bolt), ("meilisearch", p.meilisearch), ("typesense", p.typesense), ("rustfs", p.rustfs), ("rustfsConsole", p.rustfs_console), ("ollama", p.ollama), ("minio", p.minio), ("minioConsole", p.minio_console), ("elasticsearch", p.elasticsearch), ("elasticsearchTransport", p.elasticsearch_transport), ("opensearch", p.opensearch), ("opensearchTransport", p.opensearch_transport), ("n8n", p.n8n), ("rabbitmq", p.rabbitmq), ("rabbitmqManagement", p.rabbitmq_management)];
    // Servers that aren't the primary one have their own pair of ports.
    for &server in settings.web_servers.iter().skip(1) {
        let (http, https) = settings.server_ports(server);
        ports.extend(match server {
            config::Server::Apache => [("apacheHttp", http), ("apacheHttps", https)],
            config::Server::Nginx => [("nginxHttp", http), ("nginxHttps", https)],
            config::Server::Caddy => [("caddyHttp", http), ("caddyHttps", https)],
            config::Server::Frankenphp => [("frankenphpHttp", http), ("frankenphpHttps", https)],
        });
    }
    if settings.server_enabled(config::Server::Caddy) {
        ports.push(("caddyAdmin", p.caddy_admin));
    }
    if settings.server_enabled(config::Server::Frankenphp) {
        ports.push(("frankenphpAdmin", p.frankenphp_admin));
    }
    if settings.versions.get("tomcat").is_some() {
        ports.extend([("tomcatHttp", p.tomcat_http), ("tomcatShutdown", p.tomcat_shutdown)]);
    }
    if settings.catch_dumps {
        ports.push(("dumps", p.dumps));
    }
    let mut out = Vec::new();
    for (key, port) in ports {
        let free = services::port_free(port);
        let mut used_by_service = None;
        for kind in ServiceKind::all() {
            if state.supervisor.is_running(kind).await && services::required_ports(kind, &settings, &layout).contains(&port) {
                used_by_service = Some(kind.label());
            }
        }
        if state.supervisor.php.status().await.iter().any(|p| (p.port..p.port + p.workers).contains(&port)) {
            used_by_service = Some("PHP");
        }
        let owner = if free || used_by_service.is_some() { None } else { diagnose::port_owner(port).await };
        out.push(PortStatus { key, port, free, used_by_service, owner });
    }
    Ok(out)
}

/// Stops a process that is blocking a port. Refuses system processes and LampRabit's own.
#[tauri::command]
pub(crate) async fn stop_process(app: AppHandle, state: State<'_, AppState>, pid: u32) -> Result<()> {
    if pid == 0 || pid == 4 || pid == std::process::id() || state.supervisor.pids().await.contains_key(&pid) {
        return Err(Error::msg("LampRabit won't stop that process."));
    }
    let status = if cfg!(windows) {
        let mut cmd = tokio::process::Command::new("taskkill");
        cmd.args(["/PID", &pid.to_string(), "/F"]);
        procgroup::prepare(&mut cmd);
        cmd.status().await?
    } else {
        tokio::process::Command::new("kill").arg(pid.to_string()).status().await?
    };
    if !status.success() {
        return Err(Error::msg(format!(
            "Couldn't stop process {pid}. It may belong to another user or need administrator rights."
        )));
    }
    logs::system(&app, logs::Level::Info, format!("Stopped process {pid} to free its port"));
    Ok(())
}

#[tauri::command]
pub(crate) fn clear_problem(state: State<'_, AppState>, kind: ServiceKind) {
    state.supervisor.clear_problem(kind);
}

#[tauri::command]
pub(crate) fn check_port(port: u16) -> bool {
    services::port_free(port)
}

pub(crate) async fn restart_if_running(app: &AppHandle, state: &AppState, kind: ServiceKind) -> Result<()> {
    if !state.supervisor.is_running(kind).await {
        return Ok(());
    }
    state.supervisor.stop(app, kind).await;
    state.supervisor.wait_stopped(kind).await;
    let (settings, layout) = state.snapshot().await;
    state.supervisor.start(app, kind, &settings, &layout).await
}

/// Crashed services come back after a short pause when Settings says so; the pause lets
/// ports and lock files go first.
pub(crate) fn spawn_crash_restarts(app: &AppHandle) {
    let handle = app.clone();
    let crashes = app.state::<AppState>().supervisor.crashes();
    tauri::async_runtime::spawn(async move {
        let Some(mut crashes) = crashes else { return };
        while let Some(kind) = crashes.recv().await {
            tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
            let state = handle.state::<AppState>();
            let _op = state.ops.lock().await;
            let (settings, layout) = state.snapshot().await;
            // start() already logged and recorded why it failed.
            let _ = state.supervisor.restart_crashed(&handle, kind, &settings, &layout).await;
        }
    });
}
