//! The command line's control channel and the MCP server.

use crate::*;

/// The control channel's requests (control.rs), for the `lamprabit` command line and its
/// MCP server. Each goes through the app's own command for it, so it runs the same checks
/// and logs the same lines as the buttons do.
pub(crate) fn control_dispatch(app: AppHandle, cmd: String, args: serde_json::Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<serde_json::Value>> + Send>> {
    Box::pin(async move { control_command(app, &cmd, args).await })
}

pub(crate) async fn control_command(app: AppHandle, cmd: &str, args: serde_json::Value) -> Result<serde_json::Value> {
    use serde_json::{json, to_value, Value};
    let state = app.state::<AppState>();
    let text = |key: &str| -> Result<String> { args[key].as_str().map(str::to_string).ok_or_else(|| Error::msg(format!("Missing \"{key}\"."))) };
    let kind = |key: &str| -> Result<ServiceKind> {
        let id = text(key)?;
        serde_json::from_value(Value::String(id.clone())).map_err(|_| Error::msg(format!("There's no service called {id}. `lamprabit services` lists them.")))
    };
    let runtime = |key: &str| -> Result<Runtime> {
        let id = text(key)?;
        Runtime::from_id(&id).ok_or_else(|| Error::msg(format!("There's no runtime called {id}. `lamprabit runtimes` lists them.")))
    };
    let ok = |v: Result<serde_json::Value>| v;
    let done = || Ok(json!(null));
    match cmd {
        "version" => Ok(json!(app.package_info().version.to_string())),
        "status" => {
            let services = list_services(state.clone()).await?;
            let sites = list_sites(state.clone()).await?;
            Ok(json!({
                "version": app.package_info().version.to_string(),
                "root": state.snapshot().await.1.root(),
                "services": services.iter().filter(|s| s.installed).map(|s| json!({"id": s.kind, "label": s.label, "state": s.state, "port": s.port, "version": s.version, "pid": s.pid})).collect::<Vec<_>>(),
                "projects": sites.sites.len(),
                "hostsOk": sites.hosts_ok,
                "caTrusted": sites.ca_trusted,
            }))
        }
        "services" => Ok(to_value(list_services(state.clone()).await?)?),
        "service.start" => start_service(app.clone(), state.clone(), kind("name")?).await.and_then(|_| done()),
        "service.stop" => stop_service(app.clone(), state.clone(), kind("name")?).await.and_then(|_| done()),
        "service.restart" => restart_service(app.clone(), state.clone(), kind("name")?).await.and_then(|_| done()),
        "start_all" => Ok(to_value(start_all(app.clone(), state.clone()).await?)?),
        "stop_all" => stop_all(app.clone(), state.clone()).await.and_then(|_| done()),
        "profiles" => Ok(to_value(state.snapshot().await.0.profiles)?),
        "profile.start" => {
            let name = text("name")?;
            let profile = state.snapshot().await.0.profiles.into_iter().find(|p| p.name.eq_ignore_ascii_case(&name)).ok_or_else(|| Error::msg(format!("There's no profile called {name}.")))?;
            Ok(to_value(apply_services(app.clone(), state.clone(), profile.services, Vec::new()).await?)?)
        }
        "projects" => {
            let sites = list_sites(state.clone()).await?;
            let statuses = state.apps.statuses().await;
            Ok(json!(sites
                .sites
                .iter()
                .map(|s| json!({
                    "name": s.name, "domain": s.domain, "url": s.url, "path": s.path, "language": s.language, "framework": s.framework,
                    "php": s.php, "server": s.server, "gitBranch": s.git_branch, "database": s.database,
                    "running": statuses.iter().any(|a| a.name == s.name && (a.state == "running" || a.state == "starting")),
                }))
                .collect::<Vec<_>>()))
        }
        "project.start" => Ok(to_value(app_start(app.clone(), state.clone(), text("name")?).await?)?),
        "project.stop" => app_stop(app.clone(), state.clone(), text("name")?).await.and_then(|_| done()),
        "project.open" => {
            use tauri_plugin_opener::OpenerExt;
            let name = text("name")?;
            let site = list_sites(state.clone()).await?.sites.into_iter().find(|s| s.name.eq_ignore_ascii_case(&name)).ok_or_else(|| Error::msg(format!("There's no project called {name}.")))?;
            app.opener().open_url(site.url.clone(), None::<&str>).map_err(|e| Error::msg(e.to_string()))?;
            Ok(json!(site.url))
        }
        "project.create" => {
            // A user template's risky steps need the wizard's explicit approval, never this.
            let mut request: projects::NewProject = serde_json::from_value(args.clone()).map_err(|e| Error::msg(format!("Not a project request: {e}")))?;
            request.approve_risky = false;
            Ok(to_value(create_project(app.clone(), state.clone(), request).await?)?)
        }
        "templates" => Ok(to_value(list_templates(state.clone()).await?)?),
        "runtimes" => Ok(to_value(installed_runtimes(state.clone()).await?)?),
        "runtime.available" => Ok(to_value(runtime_catalog(state.clone(), runtime("name")?, None).await?)?),
        "runtime.install" => {
            let rt = runtime("name")?;
            let wanted = args["version"].as_str().map(str::to_string);
            let catalog = runtime_catalog(state.clone(), rt, None).await?;
            // The version asked for (exact, or the newest of a line like "8.4"), else LTS, else the newest.
            let release = match &wanted {
                Some(v) => catalog.iter().find(|r| &r.version == v).or_else(|| catalog.iter().find(|r| r.version.starts_with(&format!("{v}.")))),
                None => catalog.iter().find(|r| r.label.as_deref() == Some("LTS")).or(catalog.first()),
            }
            .cloned()
            .ok_or_else(|| Error::msg(format!("No {} {} download for this system. `lamprabit runtimes --available {}` lists them.", rt.id(), wanted.unwrap_or_default(), rt.id())))?;
            let version = release.version.clone();
            install_runtime(app.clone(), state.clone(), release).await?;
            set_active_version(app.clone(), state.clone(), rt, version.clone()).await?;
            Ok(json!(version))
        }
        "runtime.use" => set_active_version(app.clone(), state.clone(), runtime("name")?, text("version")?).await.and_then(|_| done()),
        "runtime.remove" => remove_runtime(app.clone(), state.clone(), runtime("name")?, text("version")?).await.and_then(|_| done()),
        "logs" => {
            let history = log_history(app.clone(), state.clone()).await?;
            let source = args["source"].as_str().map(str::to_ascii_lowercase);
            let lines = args["lines"].as_u64().unwrap_or(100) as usize;
            let picked: Vec<&logs::LogLine> = history
                .lines
                .iter()
                .filter(|l| match &source {
                    None => true,
                    Some(s) if s.starts_with("project:") => l.service == "project" && l.project.as_deref().is_some_and(|p| p.eq_ignore_ascii_case(&s[8..])),
                    Some(s) => l.service == s,
                })
                .collect();
            Ok(to_value(&picked[picked.len().saturating_sub(lines)..])?)
        }
        "doctor" => Ok(to_value(project_doctor(state.clone(), text("name")?).await?)?),
        "doctor.fix" => ok(doctor_fix(app.clone(), state.clone(), text("name")?, text("id")?).await.map(|label| json!(label))),
        "config" => Ok(to_value(get_settings(state.clone()).await?)?),
        "git.status" => Ok(to_value(git_status(state.clone(), text("name")?).await?)?),
        other => Err(Error::msg(format!("Unknown request {other} (this LampRabit may be older than the command line).")))
    }
}

/// The token clients send to LampRabit's MCP server over HTTP, made once and kept in the
/// app's config folder (readable by the user only).
pub(crate) fn mcp_token(app: &AppHandle) -> Result<String> {
    let path = app.path().app_config_dir().map_err(|e| Error::msg(e.to_string()))?.join("mcp-token");
    if let Ok(t) = std::fs::read_to_string(&path)
        && t.trim().len() >= 32 {
            return Ok(t.trim().to_string());
        }
    let token = control::random_token()?;
    std::fs::create_dir_all(path.parent().unwrap())?;
    std::fs::write(&path, &token)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(token)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct McpStatus {
    /// The HTTP server runs (Settings → AI assistants).
    pub(crate) running: bool,
    pub(crate) url: String,
    pub(crate) token: String,
    /// What LampRabit adds to an assistant's config: its own program with these arguments.
    pub(crate) command: String,
    pub(crate) args: Vec<&'static str>,
    pub(crate) clients: Vec<mcp_clients::ClientStatus>,
}

#[tauri::command]
pub(crate) async fn mcp_status(app: AppHandle, state: State<'_, AppState>) -> Result<McpStatus> {
    let (settings, _) = state.snapshot().await;
    Ok(McpStatus {
        running: mcp::http_running(),
        url: format!("http://127.0.0.1:{}/mcp", settings.ports.mcp),
        token: mcp_token(&app)?,
        command: mcp_clients::program(),
        args: vec!["--cli", "mcp"],
        clients: mcp_clients::status(),
    })
}

/// Switches LampRabit's own MCP server (HTTP, on ports.mcp) on or off, and remembers it.
#[tauri::command]
pub(crate) async fn set_mcp_server(app: AppHandle, state: State<'_, AppState>, enabled: bool) -> Result<()> {
    if enabled {
        let port = state.snapshot().await.0.ports.mcp;
        mcp::start_http(port, mcp_token(&app)?).await?;
        logs::system(&app, logs::Level::Info, format!("MCP server on http://127.0.0.1:{port}/mcp"));
    } else {
        mcp::stop_http();
        logs::system(&app, logs::Level::Info, "MCP server stopped");
    }
    state.update_settings(|s| s.mcp_server = enabled).await?;
    Ok(())
}

/// Adds LampRabit to (or removes it from) an AI assistant's MCP config.
#[tauri::command]
pub(crate) fn mcp_connect(app: AppHandle, client: String, on: bool) -> Result<String> {
    let note = mcp_clients::set(&client, on)?;
    logs::system(&app, logs::Level::Info, note.clone());
    Ok(note)
}
