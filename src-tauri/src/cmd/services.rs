//! Services: start, stop, restart, Start all, usage and ports.

use crate::*;

/// Services that load the given runtime and must restart when it changes: a service runs
/// its own runtime, and one that `needs` another (Tomcat and Neo4j run on Java) restarts
/// with it. PHP and the PHP tools are served by the web servers.
pub(crate) fn dependents(runtime: Runtime) -> Vec<ServiceKind> {
    if matches!(runtime, Runtime::Php | Runtime::Phppgadmin | Runtime::Adminer | Runtime::Phpmyadmin | Runtime::Phpredisadmin) {
        return ServiceKind::WEB.to_vec();
    }
    ServiceKind::all().into_iter().filter(|k| k.id() == runtime.id() || k.manifest().needs.iter().any(|n| n.runtime == runtime.id())).collect()
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

/// One of a service's config files, for its card's Open / Reveal / Copy path.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConfigFileInfo {
    pub(crate) label: String,
    pub(crate) path: PathBuf,
    pub(crate) exists: bool,
}

/// Its manifest's `config_files`, or its single `config` when it lists none.
#[tauri::command]
pub(crate) async fn service_config_files(state: State<'_, AppState>, kind: ServiceKind) -> Result<Vec<ConfigFileInfo>> {
    let (settings, layout) = state.snapshot().await;
    let m = kind.manifest();
    let ctx = manifest::Ctx::any(m, &settings, &layout);
    let mut out = Vec::new();
    for f in &m.config_files {
        let path = PathBuf::from(ctx.render(&f.path)?);
        let label = f.label.clone().unwrap_or_else(|| path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default());
        out.push(ConfigFileInfo { exists: path.exists(), label, path });
    }
    if out.is_empty() && m.config.is_some() {
        let path = kind.config_path(&settings, &layout);
        out.push(ConfigFileInfo { label: path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(), exists: path.exists(), path });
    }
    Ok(out)
}

/// One config file anywhere in LampRabit, with what it belongs to: the command palette and
/// `lamprabit files` list them all.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConfigFileEntry {
    pub(crate) owner: String,
    pub(crate) label: String,
    pub(crate) path: PathBuf,
    pub(crate) exists: bool,
}

/// Every installed service's config files, each installed PHP version's php.ini (the active
/// one first), Composer's global config, LampRabit's own settings and the hosts file.
#[tauri::command]
pub(crate) async fn all_config_files(state: State<'_, AppState>) -> Result<Vec<ConfigFileEntry>> {
    let (settings, layout) = state.snapshot().await;
    let mut out = Vec::new();
    let mut add = |owner: &str, label: &str, path: PathBuf| out.push(ConfigFileEntry { owner: owner.to_string(), label: label.to_string(), exists: path.exists(), path });
    let active = settings.versions.get("php").cloned();
    let mut php = runtimes::installed(&layout, Runtime::Php);
    php.sort_by_key(|v| Some(v) != active.as_ref());
    for v in &php {
        let etc = layout.etc("php").join(v);
        add(&format!("PHP {v}"), "php.ini", etc.join("php.ini"));
        if !cfg!(windows) {
            add(&format!("PHP {v}"), "php-fpm.conf", etc.join("php-fpm.conf"));
        }
    }
    add("Composer", "config.json", layout.data("composer").join("config.json"));
    add("LampRabit", "settings.toml", state.settings_path.clone());
    add("System", "hosts", sites::hosts_file());
    for kind in ServiceKind::all() {
        if settings.versions.get(kind.id()).is_none() {
            continue;
        }
        for f in service_config_files(state.clone(), kind).await.unwrap_or_default() {
            out.push(ConfigFileEntry { owner: kind.label().to_string(), label: f.label, path: f.path, exists: f.exists });
        }
    }
    Ok(out)
}

/// A service file's summary for the user to approve, and whether it replaces one they added.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ServiceFile {
    pub(crate) review: manifest::Review,
    pub(crate) replaces: bool,
}

/// Reads and checks a service file without adding it.
#[tauri::command]
pub(crate) async fn inspect_service_file(path: PathBuf) -> Result<ServiceFile> {
    let text = std::fs::read_to_string(&path).map_err(|e| Error::msg(format!("Couldn't read {}: {e}", path.display())))?;
    let m = manifest::parse(&text, false)?;
    Ok(ServiceFile { replaces: manifest::extras().iter().any(|e| e.id == m.id), review: m.review() })
}

/// Adds (or updates) a service from a file the user reviewed: it's copied to
/// `<root>/services/<id>/` and loaded from there on every start. Nobody has checked a local
/// file, so it needs the user's approval, like a user template's risky steps.
#[tauri::command]
pub(crate) async fn add_service_file(app: AppHandle, state: State<'_, AppState>, path: PathBuf, approve: bool) -> Result<String> {
    if !approve {
        return Err(Error::msg("Review the service and approve it before adding it."));
    }
    let _op = state.ops.lock().await;
    let text = std::fs::read_to_string(&path).map_err(|e| Error::msg(format!("Couldn't read {}: {e}", path.display())))?;
    let m = manifest::parse(&text, false)?;
    if let Some(kind) = ServiceKind::from_id(&m.id)
        && state.supervisor.is_running(kind).await
    {
        return Err(Error::msg(format!("Stop {} before updating it.", kind.label())));
    }
    let (_, layout) = state.snapshot().await;
    let folder = layout.services().join(&m.id);
    std::fs::create_dir_all(&folder)?;
    std::fs::write(folder.join("service.yaml"), &text)?;
    // Its icon, when it names a file next to it (a bare name: nothing outside that folder).
    if let (Some(icon), Some(from)) = (m.icon.as_deref(), path.parent())
        && std::path::Path::new(icon).file_name().is_some_and(|n| n == icon)
        && from.join(icon).is_file()
    {
        std::fs::copy(from.join(icon), folder.join(icon))?;
    }
    let (id, name) = (m.id.clone(), m.name.clone());
    manifest::register(m)?;
    logs::system(&app, logs::Level::Info, format!("Added the service {name} from {}", path.display()));
    Ok(id)
}

/// Removes a service the user added. Its installed versions go first (through the normal
/// remove), so nothing is left running from a folder LampRabit no longer knows; its data
/// folder stays, like every service's.
#[tauri::command]
pub(crate) async fn remove_local_service(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<()> {
    let _op = state.ops.lock().await;
    let kind = ServiceKind::from_id(&id).filter(|k| matches!(k, ServiceKind::Plugin(_)) && !manifest::is_bundled(&id)).ok_or_else(|| Error::msg(format!("{id} isn't a service you added")))?;
    if state.supervisor.is_running(kind).await {
        return Err(Error::msg(format!("Stop {} before removing it.", kind.label())));
    }
    let (_, layout) = state.snapshot().await;
    if std::fs::read_dir(layout.root().join("bin").join(&id)).is_ok_and(|mut d| d.next().is_some()) {
        return Err(Error::msg(format!("Remove {}'s installed versions first.", kind.label())));
    }
    let label = kind.label();
    let folder = layout.services().join(&id);
    if folder.exists() {
        std::fs::remove_dir_all(&folder).map_err(|e| Error::msg(format!("Couldn't delete {}: {e}", folder.display())))?;
    }
    manifest::unregister(&id);
    state.update_settings(|s| s.ports.services.retain(|k, _| !k.starts_with(&format!("{id}.")))).await?;
    logs::system(&app, logs::Level::Info, format!("Removed the service {label}"));
    Ok(())
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
    /// Settings key: a core port ("http") or a service's ("mailpit.smtp").
    pub(crate) key: String,
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
    let mut ports: Vec<(String, u16)> = vec![("http".into(), p.http), ("https".into(), p.https), ("phpFpm".into(), p.php_fpm)];
    // Every service's ports, as `<service id>.<port name>`.
    ports.extend(p.services.iter().map(|(k, v)| (k.clone(), *v)));
    // Servers that aren't the primary one have their own pair of ports.
    for &server in settings.web_servers.iter().skip(1) {
        let (http, https) = settings.server_ports(server);
        let (h, s) = match server {
            config::Server::Apache => ("apacheHttp", "apacheHttps"),
            config::Server::Nginx => ("nginxHttp", "nginxHttps"),
            config::Server::Caddy => ("caddyHttp", "caddyHttps"),
            config::Server::Frankenphp => ("frankenphpHttp", "frankenphpHttps"),
        };
        ports.extend([(h.into(), http), (s.into(), https)]);
    }
    if settings.server_enabled(config::Server::Caddy) {
        ports.push(("caddyAdmin".into(), p.caddy_admin));
    }
    if settings.server_enabled(config::Server::Frankenphp) {
        ports.push(("frankenphpAdmin".into(), p.frankenphp_admin));
    }
    if settings.catch_dumps {
        ports.push(("dumps".into(), p.dumps));
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
