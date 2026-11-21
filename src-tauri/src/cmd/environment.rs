//! The LampRabit folder: importing projects, moving it, uninstalling, diagnostics, terminal and editor.

use crate::*;

/// Other local environments found on this computer (Laragon, XAMPP, Herd…), with their projects.
#[tauri::command]
pub(crate) async fn import_sources(state: State<'_, AppState>) -> Result<Vec<imports::Source>> {
    let layout = state.snapshot().await.1;
    tauri::async_runtime::spawn_blocking(move || imports::detect(&layout)).await.map_err(|e| Error::msg(e.to_string()))
}

/// The projects in a folder the user picked, for tools LampRabit doesn't look for.
#[tauri::command]
pub(crate) async fn import_folder(state: State<'_, AppState>, path: PathBuf) -> Result<imports::Source> {
    let layout = state.snapshot().await.1;
    tauri::async_runtime::spawn_blocking(move || imports::from_folder(&path, &layout)).await.map_err(|e| Error::msg(e.to_string()))?
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImportProgress {
    pub(crate) name: String,
    pub(crate) files: u64,
}

/// Copies another tool's project into www/<name> (the original stays), then serves it.
#[tauri::command]
pub(crate) async fn import_project(app: AppHandle, state: State<'_, AppState>, source: PathBuf, name: String) -> Result<imports::Copied> {
    use tauri::Emitter;
    let layout = state.snapshot().await.1;
    let name = name.trim().to_string();
    let events = app.clone();
    let (from, label) = (source.clone(), name.clone());
    let copied = tauri::async_runtime::spawn_blocking(move || imports::copy_project(&from, &label, &layout, |files| drop(events.emit("import-progress", ImportProgress { name: label.clone(), files }))))
        .await
        .map_err(|e| Error::msg(e.to_string()))??;
    logs::system(&app, logs::Level::Info, format!("Imported {} into www/{name} ({} files)", source.display(), copied.files));
    let _op = state.ops.lock().await;
    sync_sites(&app, &state).await?;
    Ok(copied)
}

/// Whether a folder can hold the LampRabit folder, and what moving there involves.
#[tauri::command]
pub(crate) async fn check_move_target(state: State<'_, AppState>, target: PathBuf) -> Result<relocate::MoveCheck> {
    let root = state.snapshot().await.1.root().to_path_buf();
    tauri::async_runtime::spawn_blocking(move || relocate::check(&root, &target)).await.map_err(|e| Error::msg(e.to_string()))
}

/// Programs other than LampRabit that keep the folder busy.
#[tauri::command]
pub(crate) async fn environment_in_use(state: State<'_, AppState>) -> Result<Vec<relocate::Holder>> {
    let root = state.snapshot().await.1.root().to_path_buf();
    Ok(relocate::holders(&root))
}

#[tauri::command]
pub(crate) async fn stop_environment_holder(app: AppHandle, state: State<'_, AppState>, pid: u32) -> Result<()> {
    let root = state.snapshot().await.1.root().to_path_buf();
    relocate::stop_holder(&root, pid)?;
    logs::system(&app, logs::Level::Info, format!("Stopped process {pid}, which was using the LampRabit folder"));
    Ok(())
}

/// Moves the LampRabit folder to `target`: stops everything that runs from it, moves it
/// (all or nothing), repairs what records absolute paths, and switches the settings,
/// shims and PATH over. Services aren't restarted; the user starts what they need.
#[tauri::command]
pub(crate) async fn move_environment(app: AppHandle, state: State<'_, AppState>, target: PathBuf) -> Result<Settings> {
    let _op = state.ops.lock().await;
    let (settings, layout) = state.snapshot().await;
    let old = layout.root().to_path_buf();
    let check = {
        let (old, target) = (old.clone(), target.clone());
        tauri::async_runtime::spawn_blocking(move || relocate::check(&old, &target)).await.map_err(|e| Error::msg(e.to_string()))?
    };
    if !check.problems.is_empty() {
        return Err(Error::msg(check.problems.join(" ")));
    }
    logs::system(&app, logs::Level::Info, format!("Moving the LampRabit folder from {} to {}", old.display(), target.display()));
    // Everything LampRabit runs from the folder stops first.
    let running: Vec<ServiceKind> = state.supervisor.pids().await.into_values().collect();
    for &kind in &running {
        state.supervisor.stop(&app, kind).await;
    }
    for kind in running {
        state.supervisor.wait_stopped(kind).await;
    }
    state.apps.stop_all().await;
    state.tunnels.stop_all().await;
    state.supervisor.php.stop_all(&app).await;
    let busy = relocate::holders(&old);
    if !busy.is_empty() {
        let names: Vec<String> = busy.iter().map(|h| format!("{} (PID {})", h.name, h.pid)).collect();
        return Err(Error::msg(format!("These programs still use the folder: {}. Stop them and try again.", names.join(", "))));
    }
    if settings.add_to_path {
        let _ = shims::set_on_path(&layout, false);
    }
    let (from, to, same) = (old.clone(), target.clone(), check.same_drive);
    let app2 = app.clone();
    let moved = tauri::async_runtime::spawn_blocking(move || relocate::move_root(&app2, &from, &to, same)).await.map_err(|e| Error::msg(e.to_string()))?;
    if let Err(e) = moved {
        if settings.add_to_path {
            let _ = shims::set_on_path(&layout, true);
        }
        logs::system(&app, logs::Level::Error, e.to_string());
        return Err(e);
    }
    relocate::repair_venvs(&old, &target);
    let settings = state.update_settings(|s| s.root_dir = target.clone()).await?;
    let layout = Layout::new(&settings.root_dir);
    if settings.add_to_path
        && let Err(e) = shims::set_on_path(&layout, true) {
            logs::system(&app, logs::Level::Warn, format!("Couldn't put the new shims folder on PATH: {e}"));
        }
    *state.ca_trusted.lock().await = None;
    logs::set_system_file(&app, layout.logs().join("lampRabit.log"));
    logs::system(&app, logs::Level::Info, format!("The LampRabit folder is now {}", target.display()));
    Ok(settings)
}

/// What removing LampRabit deletes and keeps, for Settings to show before asking.
#[tauri::command]
pub(crate) async fn uninstall_plan(app: AppHandle, state: State<'_, AppState>) -> Result<uninstall::Plan> {
    let (_, layout) = state.snapshot().await;
    let config_dir = app.path().app_config_dir().map_err(|e| Error::msg(e.to_string()))?;
    Ok(uninstall::plan(&layout, &config_dir))
}

/// Removes LampRabit's files and outside changes, keeping www/ and data/. The app is left
/// without settings or runtimes, so Settings quits it afterwards.
#[tauri::command]
pub(crate) async fn uninstall(app: AppHandle, state: State<'_, AppState>) -> Result<uninstall::Report> {
    let _op = state.ops.lock().await;
    state.tunnels.stop_all().await;
    // A clean stop, so the databases kept in data/ shut down properly.
    for kind in ServiceKind::all() {
        state.supervisor.stop(&app, kind).await;
    }
    for kind in ServiceKind::all() {
        state.supervisor.wait_stopped(kind).await;
    }
    let (_, layout) = state.snapshot().await;
    let config_dir = app.path().app_config_dir().map_err(|e| Error::msg(e.to_string()))?;
    Ok(uninstall::run(&layout, &config_dir).await)
}

#[tauri::command]
pub(crate) async fn open_terminal(state: State<'_, AppState>, dir: Option<PathBuf>) -> Result<()> {
    let (settings, layout) = state.snapshot().await;
    comfort::open_terminal(&settings, &layout, &dir.unwrap_or_else(|| layout.www()))
}

#[tauri::command]
pub(crate) async fn open_in_editor(state: State<'_, AppState>, dir: PathBuf) -> Result<()> {
    let (settings, _) = state.snapshot().await;
    comfort::open_in_editor(&settings, &dir).await
}

#[tauri::command]
pub(crate) fn vc_runtime_status() -> comfort::VcRuntime {
    comfort::vc_runtime()
}

#[tauri::command]
pub(crate) async fn vc_runtime_install(app: AppHandle, state: State<'_, AppState>) -> Result<()> {
    let (_, layout) = state.snapshot().await;
    logs::system(&app, logs::Level::Info, "Installing the Microsoft Visual C++ runtime");
    comfort::install_vc_runtime(&state.http, &layout).await?;
    logs::system(&app, logs::Level::Info, "Microsoft Visual C++ runtime installed");
    Ok(())
}

#[tauri::command]
pub(crate) fn save_text(path: PathBuf, contents: String) -> Result<()> {
    std::fs::write(path, contents)?;
    Ok(())
}

/// A small text file the user picked (an exported profile list); refuses large files.
#[tauri::command]
pub(crate) fn read_text(path: PathBuf) -> Result<String> {
    if std::fs::metadata(&path)?.len() > 1024 * 1024 {
        return Err(Error::msg("That file is over 1 MB; pick the file LampRabit exported."));
    }
    Ok(std::fs::read_to_string(path)?)
}

/// A Markdown report of everything useful for troubleshooting.
#[tauri::command]
pub(crate) async fn diagnostics(state: State<'_, AppState>, redact: bool) -> Result<String> {
    use std::fmt::Write;
    let (settings, layout) = state.snapshot().await;
    let mut r = String::new();
    let now = time::OffsetDateTime::now_local().unwrap_or_else(|_| time::OffsetDateTime::now_utc());

    let _ = writeln!(r, "# LampRabit diagnostics\n");
    let generated = now
        .format(time::macros::format_description!("[year]-[month]-[day] [hour]:[minute]:[second]"))
        .unwrap_or_default();
    let _ = writeln!(r, "Generated {generated}\n");
    let _ = writeln!(r, "## System\n");
    let _ = writeln!(r, "- LampRabit {}", env!("CARGO_PKG_VERSION"));
    let _ = writeln!(r, "- OS: {} {} ({})", std::env::consts::OS, os_version().await, std::env::consts::ARCH);
    if cfg!(windows) {
        let sys = std::path::PathBuf::from(std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into())).join("System32");
        for dll in ["vcruntime140.dll", "vcruntime140_1.dll", "msvcp140.dll"] {
            let found = if sys.join(dll).exists() { "found" } else { "MISSING (install the VC++ 2015–2022 Redistributable)" };
            let _ = writeln!(r, "- {dll}: {found}");
        }
    }

    let _ = writeln!(r, "\n## Environment\n");
    let _ = writeln!(r, "- Root: {}", layout.root().display());
    let _ = writeln!(r, "- Domain suffix: .{}", settings.tld);
    let servers: Vec<&str> = settings.web_servers.iter().map(|s| s.label()).collect();
    let _ = writeln!(r, "- Web servers: {}", servers.join(", "));
    let _ = writeln!(r, "- HTTPS by default: {}", settings.https_by_default);
    let _ = writeln!(r, "- Manage hosts file: {}", settings.manage_hosts);
    let _ = writeln!(r, "- PHP mail() to Mailpit: {}", settings.route_mail);

    let _ = writeln!(r, "\n## Runtimes\n\n| Runtime | Active | Installed |\n|---|---|---|");
    for runtime in Runtime::all() {
        let active = settings.versions.get(runtime.id()).cloned().unwrap_or_else(|| "—".into());
        let installed = runtimes::installed(&layout, runtime).join(", ");
        let _ = writeln!(r, "| {} | {active} | {} |", runtime.id(), if installed.is_empty() { "—".into() } else { installed });
    }

    let _ = writeln!(r, "\n## Services\n\n| Service | State | PID | Port | Health | Last problem |\n|---|---|---|---|---|---|");
    for s in state.supervisor.list(&settings, &layout).await {
        let health = s.health.as_ref().map_or("—".to_string(), |h| match &h.detail {
            Some(d) => format!("{} ({d})", h.status),
            None => h.status.to_string(),
        });
        let problem = s.problem.as_ref().map_or("—".to_string(), |p| format!("{}: {}", p.title, p.detail));
        let _ = writeln!(
            r,
            "| {} {} | {:?} | {} | {} | {health} | {} |",
            s.label,
            s.version.as_deref().unwrap_or("(not installed)"),
            s.state,
            s.pid.map_or("—".into(), |p| p.to_string()),
            s.port,
            problem.replace('|', "\\|"),
        );
    }

    let _ = writeln!(r, "\n## Ports\n\n| Setting | Port | Status |\n|---|---|---|");
    for p in port_status(state.clone()).await? {
        let status = if p.free {
            "free".to_string()
        } else if let Some(svc) = p.used_by_service {
            format!("in use by LampRabit ({svc})")
        } else if let Some(o) = &p.owner {
            format!("in use by {} (PID {})", o.name, o.pid)
        } else {
            "in use (owner unknown)".into()
        };
        let _ = writeln!(r, "| {} | {} | {status} |", p.key, p.port);
    }

    let _ = writeln!(r, "\n## PHP workers\n");
    let pools = state.supervisor.php.status().await;
    if pools.is_empty() {
        let _ = writeln!(r, "None running (they start with a web server).");
    }
    for p in pools {
        let _ = writeln!(r, "- PHP {}: {}/{} workers alive, ports {}-{}", p.version, p.alive, p.workers, p.port, p.port + p.workers - 1);
    }

    for kind in ServiceKind::WEB {
        let _ = writeln!(r, "\n## {} configuration test\n", kind.label());
        let _ = match services::config_test(kind, &settings, &layout).await {
            None => writeln!(r, "Not run (not installed or never started)."),
            Some(Ok(())) => writeln!(r, "Syntax OK"),
            Some(Err(e)) => writeln!(r, "```text\n{e}\n```"),
        };
    }

    let served = sites::served(&settings, &layout);
    let _ = writeln!(r, "\n## Sites and HTTPS\n");
    let _ = writeln!(r, "- Sites: {}", served.iter().map(|s| s.domain.as_str()).collect::<Vec<_>>().join(", "));
    let _ = writeln!(r, "- Hosts file in sync: {}", !sites::needs_hosts_file(&settings) || sites::hosts_in_sync(&served));
    let _ = writeln!(r, "- Local CA: {}", if ssl::ca_thumbprint(&layout).is_some() {
        if ssl::ca_trusted(&layout).await { "created, trusted" } else { "created, NOT trusted" }
    } else {
        "not created"
    });

    let _ = writeln!(r, "\n## Recent warnings and errors\n");
    for file in ["lampRabit.log", "apache-error.log", "mariadb-error.log", "php-error.log", "mailpit-output.log"] {
        let tail = diagnose::tail(&layout.logs().join(file), 64 * 1024);
        let lines: Vec<&str> = tail
            .lines()
            .filter(|l| {
                let l = l.to_lowercase();
                ["error", "warn", "fatal", "crit", "emerg"].iter().any(|k| l.contains(k))
            })
            .collect();
        let start = lines.len().saturating_sub(15);
        if !lines.is_empty() {
            let _ = writeln!(r, "### {file}\n\n```text\n{}\n```\n", lines[start..].join("\n"));
        }
    }

    if redact {
        if let Some(home) = dirs::home_dir() {
            let home = home.to_string_lossy().to_string();
            r = r.replace(&home, "~").replace(&home.replace('\\', "/"), "~");
        }
        for var in ["USERNAME", "USER"] {
            if let Some(user) = std::env::var_os(var).map(|u| u.to_string_lossy().to_string()).filter(|u| u.len() > 2) {
                r = r.replace(&user, "<user>");
            }
        }
    }
    Ok(r)
}

pub(crate) async fn os_version() -> String {
    let (program, args): (&str, &[&str]) = if cfg!(windows) {
        ("cmd", &["/C", "ver"])
    } else if cfg!(target_os = "macos") {
        ("sw_vers", &["-productVersion"])
    } else {
        ("uname", &["-r"])
    };
    let mut cmd = tokio::process::Command::new(program);
    cmd.args(args);
    procgroup::prepare(&mut cmd);
    cmd.output().await.map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default()
}
