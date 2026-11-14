//! A project's dev server, processes, commands and dependencies.

use crate::*;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppInfo {
    /// The saved setup, or what package.json suggests when there's none yet (port 0: picked
    /// when it starts).
    pub(crate) config: config::NodeApp,
    pub(crate) configured: bool,
    pub(crate) scripts: std::collections::BTreeMap<String, String>,
    pub(crate) detected_package_manager: config::PackageManager,
    /// Versions installed with NVM, newest first.
    pub(crate) node_versions: Vec<String>,
    /// What .nvmrc / .node-version asks for.
    pub(crate) project_node: Option<String>,
    pub(crate) framework: Option<&'static str>,
    pub(crate) status: apps::AppStatus,
}

/// How a Node project runs, for its panel on the Sites page.
#[tauri::command]
pub(crate) async fn app_info(state: State<'_, AppState>, name: String) -> Result<AppInfo> {
    let (settings, layout) = state.snapshot().await;
    let dir = project_dir(&layout, &name)?;
    let saved = settings.sites.get(&name).and_then(|s| s.app.clone());
    Ok(AppInfo {
        configured: saved.is_some(),
        config: saved.unwrap_or_else(|| apps::suggest(&dir, 0)),
        scripts: apps::scripts(&dir),
        detected_package_manager: apps::detect_package_manager(&dir),
        node_versions: node::installed_versions(),
        project_node: node::project_version(&dir),
        framework: apps::framework(&dir),
        status: state.apps.status(&name).await,
    })
}

/// Saves how a Node project runs. Serving it (or its port) changing rewrites the web
/// server's sites, which a running web server picks up with a restart.
#[tauri::command]
pub(crate) async fn app_save(app: AppHandle, state: State<'_, AppState>, name: String, config: config::NodeApp) -> Result<()> {
    let (settings, layout) = state.snapshot().await;
    project_dir(&layout, &name)?;
    // Port 0 means "pick one when it starts".
    if let Some((other, _)) = settings.sites.iter().find(|(n, s)| config.port > 0 && **n != name && s.app.as_ref().is_some_and(|a| a.port == config.port)) {
        return Err(Error::msg(format!("Port {} is already {other}'s. Choose another one.", config.port)));
    }
    let before = settings.sites.get(&name).and_then(|s| s.app.clone());
    let routing = |a: &Option<config::NodeApp>| a.as_ref().map(|a| (a.serve, a.port));
    let changed = routing(&before) != routing(&Some(config.clone()));
    state.update_settings(|s| s.sites.entry(name.clone()).or_default().app = Some(config)).await?;
    if changed {
        restart_web_servers(&app, &state).await?;
    }
    Ok(())
}

/// Gets a project ready to start: a free port (the saved one while it's still free) and
/// the web server for its domain running. The domain switches to the app once its port
/// listens (see `watch_ports`). Returns a note when the web server couldn't start; the app
/// still runs on its port then.
pub(crate) async fn prepare_app(app: &AppHandle, state: &AppState, name: &str) -> Result<Option<String>> {
    let (settings, layout) = state.snapshot().await;
    let dir = project_dir(&layout, name)?;
    let saved = settings.sites.get(name).and_then(|s| s.app.clone());
    let mut config = saved.clone().unwrap_or_else(|| apps::suggest(&dir, 0));
    let taken = settings.sites.iter().any(|(n, s)| n != name && s.app.as_ref().is_some_and(|a| a.port == config.port));
    if config.port == 0 || taken || std::net::TcpListener::bind(("127.0.0.1", config.port)).is_err() {
        config.port = apps::free_port(&settings);
    }
    if saved.as_ref() != Some(&config) {
        let to_save = config.clone();
        state.update_settings(|s| s.sites.entry(name.to_string()).or_default().app = Some(to_save)).await?;
    }
    if !config.serve || matches!(sites::language(&dir), sites::Language::Php | sites::Language::Static) {
        return Ok(None);
    }
    let kind = server_kind(sites::site_server(&settings, name));
    let _op = state.ops.lock().await;
    if state.supervisor.is_running(kind).await {
        return Ok(None);
    }
    // The domain needs the web server; it starts with the site (and its certificate).
    let (settings, layout) = state.snapshot().await;
    Ok(state
        .supervisor
        .start(app, kind, &settings, &layout)
        .await
        .err()
        .map(|e| format!("{} didn't start, so {} isn't reachable at its domain yet: {e}", kind.label(), name)))
}

pub(crate) fn server_kind(server: config::Server) -> ServiceKind {
    ServiceKind::for_server(server)
}

/// Starts a project's dev server and serves its domain through it.
#[tauri::command]
pub(crate) async fn app_start(app: AppHandle, state: State<'_, AppState>, name: String) -> Result<apps::AppStatus> {
    let status = state.apps.status(&name).await;
    if matches!(status.state, "starting" | "running") {
        return Ok(status);
    }
    let note = prepare_app(&app, &state, &name).await?;
    let (settings, layout) = state.snapshot().await;
    let status = state.apps.start(&app, &settings, &layout, &name).await?;
    if let Some(note) = note {
        state.apps.record(&app, &layout, &name, "error", note).await;
    }
    // Its processes that run with it; one failing doesn't stop the app (its log says why).
    let _ = state.apps.start_with_app(&app, &settings, &layout, &name).await;
    Ok(status)
}

#[tauri::command]
pub(crate) async fn app_stop(app: AppHandle, state: State<'_, AppState>, name: String) -> Result<()> {
    state.apps.stop(&app, &name).await;
    let (settings, _) = state.snapshot().await;
    state.apps.stop_processes(&app, &settings, &name, true).await;
    Ok(())
}

/// Saves a project's processes. Removed ones are stopped; a changed one keeps running
/// with its old command until it's restarted.
#[tauri::command]
pub(crate) async fn set_site_processes(app: AppHandle, state: State<'_, AppState>, name: String, processes: Vec<config::ProjectProcess>) -> Result<()> {
    let _op = state.ops.lock().await;
    let (settings, layout) = state.snapshot().await;
    project_dir(&layout, &name)?;
    let mut names = HashSet::new();
    let app_port = settings.sites.get(&name).and_then(|s| s.app.as_ref()).map(|a| a.port);
    let mut ports = HashSet::new();
    for p in &processes {
        if p.name.is_empty() || !p.name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') || p.name.starts_with('-') {
            return Err(Error::msg(format!("\"{}\" can't be a process name: use lowercase letters, digits and dashes, like queue or vite.", p.name)));
        }
        if !names.insert(p.name.clone()) {
            return Err(Error::msg(format!("Two processes are called {}.", p.name)));
        }
        if p.command.trim().is_empty() {
            return Err(Error::msg(format!("Enter the command {} runs.", p.name)));
        }
        let plain = std::path::Path::new(&p.cwd).components().all(|c| matches!(c, std::path::Component::Normal(_)));
        if !plain || !layout.www().join(&name).join(&p.cwd).is_dir() {
            return Err(Error::msg(format!("{}: \"{}\" isn't a folder in {name}.", p.name, p.cwd)));
        }
        if !p.schedule.trim().is_empty() {
            cron::Schedule::parse(p.schedule.trim()).map_err(|e| Error::msg(format!("{}: {e}", p.name)))?;
        }
        if let Some(port) = p.port
            && (port < 1024 || Some(port) == app_port || !ports.insert(port)) {
                return Err(Error::msg(format!("{}: port {port} is taken (below 1024, the app's, or another process's). Pick another one.", p.name)));
            }
    }
    // A domain can't point at a process that's going away, or that no longer has a port.
    if let Some(own) = settings.sites.get(&name) {
        let main = own.main.iter().map(|t| (name.clone(), t));
        for (host, target) in own.domains.iter().map(|d| (d.host.clone(), &d.target)).chain(main) {
            if let config::DomainTarget::Process { name: target } = target
                && !processes.iter().any(|p| &p.name == target && p.port.is_some()) {
                    return Err(Error::msg(format!("{host} points to the process {target}. Point it somewhere else first.")));
                }
        }
    }
    let removed: Vec<String> = settings.sites.get(&name).map(|s| s.processes.iter().filter(|old| !names.contains(&old.name)).map(|old| old.name.clone()).collect()).unwrap_or_default();
    for process in &removed {
        state.apps.stop(&app, &apps::process_key(&name, process)).await;
    }
    let ports_changed = settings.sites.get(&name).is_some_and(|s| s.processes.iter().map(|p| (&p.name, p.port)).ne(processes.iter().map(|p| (&p.name, p.port))));
    state.update_settings(|s| s.sites.entry(name.clone()).or_default().processes = processes).await?;
    // Domains that point at a process follow its port.
    let to_process = |t: &config::DomainTarget| matches!(t, config::DomainTarget::Process { .. });
    if ports_changed && settings.sites.get(&name).is_some_and(|s| s.domains.iter().any(|d| to_process(&d.target)) || s.main.as_ref().is_some_and(to_process)) {
        restart_web_servers(&app, &state).await?;
    }
    ports::poke();
    Ok(())
}

#[tauri::command]
pub(crate) async fn process_start(app: AppHandle, state: State<'_, AppState>, name: String, process: String) -> Result<apps::AppStatus> {
    let (settings, layout) = state.snapshot().await;
    state.apps.start_process(&app, &settings, &layout, &name, &process).await
}

#[tauri::command]
pub(crate) async fn process_stop(app: AppHandle, state: State<'_, AppState>, name: String, process: String) -> Result<()> {
    state.apps.stop(&app, &apps::process_key(&name, &process)).await;
    Ok(())
}

#[tauri::command]
pub(crate) async fn process_restart(app: AppHandle, state: State<'_, AppState>, name: String, process: String) -> Result<apps::AppStatus> {
    state.apps.stop(&app, &apps::process_key(&name, &process)).await;
    let (settings, layout) = state.snapshot().await;
    state.apps.start_process(&app, &settings, &layout, &name, &process).await
}

/// Starts (or stops) all of a project's processes, whether or not they run with the app.
#[tauri::command]
pub(crate) async fn processes_all(app: AppHandle, state: State<'_, AppState>, name: String, start: bool) -> Result<()> {
    let (settings, layout) = state.snapshot().await;
    if !start {
        state.apps.stop_processes(&app, &settings, &name, false).await;
        return Ok(());
    }
    let all: Vec<String> = settings.sites.get(&name).map(|s| s.processes.iter().map(|p| p.name.clone()).collect()).unwrap_or_default();
    let mut errors = Vec::new();
    for process in all {
        if let Err(e) = state.apps.start_process(&app, &settings, &layout, &name, &process).await {
            errors.push(format!("{process}: {e}"));
        }
    }
    if errors.is_empty() { Ok(()) } else { Err(Error::msg(errors.join("\n"))) }
}

#[tauri::command]
pub(crate) async fn app_restart(app: AppHandle, state: State<'_, AppState>, name: String) -> Result<apps::AppStatus> {
    state.apps.stop(&app, &name).await;
    app_start(app, state, name).await
}

/// Installs a project's dependencies with its package manager.
#[tauri::command]
pub(crate) async fn app_install(app: AppHandle, state: State<'_, AppState>, name: String) -> Result<()> {
    let (settings, layout) = state.snapshot().await;
    project_dir(&layout, &name)?;
    state.apps.install(&app, &settings, &layout, &name).await
}

#[tauri::command]
pub(crate) async fn app_statuses(state: State<'_, AppState>) -> Result<Vec<apps::AppStatus>> {
    Ok(state.apps.statuses().await)
}

#[tauri::command]
pub(crate) async fn app_logs(state: State<'_, AppState>, name: String) -> Result<Vec<String>> {
    let (_, layout) = state.snapshot().await;
    Ok(state.apps.logs(&layout, &name).await)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProjectCommands {
    pub(crate) commands: Vec<commands::Command>,
    /// Sources that couldn't be listed, with why.
    pub(crate) problems: Vec<String>,
    pub(crate) sources: Vec<commands::Source>,
}

/// Every command a project can run: yours, its template's, and those discovered from
/// package.json, composer.json and the framework CLIs.
#[tauri::command]
pub(crate) async fn project_commands(state: State<'_, AppState>, name: String) -> Result<ProjectCommands> {
    let (settings, layout) = state.snapshot().await;
    let dir = project_dir(&layout, &name)?;
    let site = settings.sites.get(&name).cloned().unwrap_or_default();
    let template = site.template.as_deref().and_then(|t| templates::find(&layout, t.split('@').next().unwrap_or(t)).ok()).map(|(t, _)| t);
    let declared = template.as_ref().and_then(|t| t.commands.clone()).unwrap_or_default();
    let sources = declared.discover.clone().unwrap_or_else(|| commands::detect(&dir));
    let pm = site.app.as_ref().and_then(|a| a.package_manager).unwrap_or_else(|| apps::detect_package_manager(&dir));
    let path = apps::Apps::command_path(&settings, &layout, &name);
    let (discovered, problems) = commands::discover(&dir, &sources, &path, pm).await;

    // Template commands may use the project's names and package manager.
    let url = sites::url_for(&settings, &name).unwrap_or_default().trim_end_matches('/').to_string();
    let mut values = std::collections::BTreeMap::from([
        ("project".to_string(), name.clone()),
        ("domain".into(), url.split("://").nth(1).unwrap_or_default().to_string()),
        ("url".into(), url),
    ]);
    if let Ok(pmc) = apps::pm_commands(pm, &path) {
        values.extend([
            ("pm".to_string(), pmc.id.to_string()),
            ("pm.run".into(), pmc.run.clone()),
            ("pm.args".into(), pmc.args.into()),
            ("pm.exec".into(), pmc.exec.clone()),
            ("pm.install".into(), pmc.install.clone()),
            ("pm.add".into(), pmc.add.clone()),
        ]);
    }
    let values = templates::Values(values);
    let mut list: Vec<commands::Command> = site
        .commands
        .iter()
        .map(|c| commands::Command { group: "Yours".into(), name: c.name.clone(), command: c.run.clone(), description: None, origin: "yours" })
        .collect();
    if let Some(t) = &template {
        list.extend(declared.list.iter().map(|c| commands::Command {
            group: t.name.clone(),
            name: c.name.clone(),
            command: values.render(&c.run),
            description: c.description.clone(),
            origin: "template",
        }));
    }
    list.extend(discovered);
    Ok(ProjectCommands { commands: list, problems, sources })
}

/// Runs a command in a project's folder; its output goes to the project's log.
#[tauri::command]
pub(crate) async fn run_project_command(app: AppHandle, state: State<'_, AppState>, name: String, label: String, command: String) -> Result<()> {
    let (settings, layout) = state.snapshot().await;
    project_dir(&layout, &name)?;
    if command.trim().is_empty() {
        return Err(Error::msg("Enter a command to run."));
    }
    state.apps.run_command(&app, &settings, &layout, &name, &label, &command).await
}

/// A project's dependencies for each package manager it uses, read from its files.
#[tauri::command]
pub(crate) async fn project_dependencies(state: State<'_, AppState>, name: String) -> Result<Vec<deps::Ecosystem>> {
    let (settings, layout) = state.snapshot().await;
    let dir = project_dir(&layout, &name)?;
    let path = apps::Apps::command_path(&settings, &layout, &name);
    tokio::task::spawn_blocking(move || deps::list(&dir, &path)).await.map_err(|e| Error::msg(e.to_string()))
}

/// The packages with newer releases than the installed ones, for one package manager.
#[tauri::command]
pub(crate) async fn dependencies_outdated(state: State<'_, AppState>, name: String, manager: deps::Manager) -> Result<Vec<deps::Outdated>> {
    let (settings, layout) = state.snapshot().await;
    let dir = project_dir(&layout, &name)?;
    let path = apps::Apps::command_path(&settings, &layout, &name);
    let env = shims::tool_env(&settings, &layout);
    deps::outdated(&state.http, &dir, &path, &env, manager).await
}

/// What a dependency action would run, shown for confirmation before it does.
#[tauri::command]
pub(crate) async fn dependency_plan(state: State<'_, AppState>, name: String, request: deps::Request) -> Result<deps::Plan> {
    let (settings, layout) = state.snapshot().await;
    let dir = project_dir(&layout, &name)?;
    let path = apps::Apps::command_path(&settings, &layout, &name);
    deps::plan(&dir, &path, &request)
}

/// Runs a confirmed dependency action through the project's command runner (one at a
/// time, output in the Commands tab and the Dependencies tab's output).
#[tauri::command]
pub(crate) async fn run_dependency_action(app: AppHandle, state: State<'_, AppState>, name: String, request: deps::Request) -> Result<()> {
    let (settings, layout) = state.snapshot().await;
    let dir = project_dir(&layout, &name)?;
    let path = apps::Apps::command_path(&settings, &layout, &name);
    let plan = deps::plan(&dir, &path, &request)?;
    if let Some(task) = state.apps.status(&name).await.task {
        return Err(Error::msg(format!("\"{task}\" is still running for {name}. Wait for it or stop it first.")));
    }
    deps::apply_edit(&dir, &request)?;
    state.apps.run_command(&app, &settings, &layout, &name, &plan.label, &plan.command).await
}

/// The project's recent command runs, newest first.
#[tauri::command]
pub(crate) async fn command_history(state: State<'_, AppState>, name: String) -> Result<Vec<apps::CommandRun>> {
    let (_, layout) = state.snapshot().await;
    Ok(apps::history(&layout, &name))
}

/// When each of the project's scheduled processes runs next (Unix ms).
#[tauri::command]
pub(crate) async fn process_next_runs(state: State<'_, AppState>, name: String) -> Result<std::collections::BTreeMap<String, u64>> {
    let (settings, _) = state.snapshot().await;
    let now = time::OffsetDateTime::now_local().unwrap_or_else(|_| time::OffsetDateTime::now_utc());
    Ok(settings
        .sites
        .get(&name)
        .map(|own| {
            own.processes
                .iter()
                .filter_map(|p| {
                    let next = cron::Schedule::parse(p.schedule.trim()).ok()?.next(now)?;
                    Some((p.name.clone(), u64::try_from(next.unix_timestamp_nanos() / 1_000_000).ok()?))
                })
                .collect()
        })
        .unwrap_or_default())
}

#[tauri::command]
pub(crate) async fn stop_project_command(state: State<'_, AppState>, name: String) -> Result<()> {
    state.apps.stop_command(&name).await;
    Ok(())
}

/// Saves the commands the user added to a project.
#[tauri::command]
pub(crate) async fn save_project_commands(state: State<'_, AppState>, name: String, commands: Vec<commands::UserCommand>) -> Result<()> {
    let (_, layout) = state.snapshot().await;
    project_dir(&layout, &name)?;
    let cleaned: Vec<commands::UserCommand> = commands.into_iter().filter(|c| !c.name.trim().is_empty() && !c.run.trim().is_empty()).collect();
    state.update_settings(|s| s.sites.entry(name.clone()).or_default().commands = cleaned).await?;
    Ok(())
}

/// A free port from 3000 up that no other project uses.
#[tauri::command]
pub(crate) async fn app_free_port(state: State<'_, AppState>) -> Result<u16> {
    Ok(apps::free_port(&*state.settings.read().await))
}
