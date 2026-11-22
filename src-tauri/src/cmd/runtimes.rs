//! Runtimes: the catalog, installs, versions, PHP extensions and Node.js.

use crate::*;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InstalledRuntime {
    pub(crate) runtime: Runtime,
    pub(crate) active: Option<String>,
    pub(crate) versions: Vec<String>,
}

#[tauri::command]
pub(crate) async fn node_status(app: AppHandle) -> Result<node::NodeStatus> {
    Ok(node::status(&app).await)
}

#[tauri::command]
pub(crate) async fn node_available(state: State<'_, AppState>) -> Result<Vec<node::NodeRelease>> {
    node::available(&state.http).await
}

#[tauri::command]
pub(crate) async fn node_install(app: AppHandle, version: String) -> Result<()> {
    node::install_version(&app, &version).await
}

#[tauri::command]
pub(crate) async fn node_use(app: AppHandle, version: String) -> Result<()> {
    node::use_version(&app, &version).await
}

#[tauri::command]
pub(crate) async fn node_enable_package_managers(app: AppHandle, state: State<'_, AppState>) -> Result<()> {
    let (_, layout) = state.snapshot().await;
    node::enable_package_managers(&app, &layout).await
}

#[tauri::command]
pub(crate) async fn node_uninstall(app: AppHandle, version: String) -> Result<()> {
    node::uninstall_version(&app, &version).await
}

#[tauri::command]
pub(crate) async fn nvm_install(app: AppHandle, state: State<'_, AppState>) -> Result<()> {
    let (_, layout) = state.snapshot().await;
    node::install_nvm(&app, &state.http, &layout).await
}

#[tauri::command]
pub(crate) async fn runtime_updates(state: State<'_, AppState>) -> Result<Vec<runtimes::UpdateNotice>> {
    let (_, layout) = state.snapshot().await;
    Ok(runtimes::updates(&state.http, &layout).await)
}

#[tauri::command]
pub(crate) async fn php_extensions(state: State<'_, AppState>, version: String) -> Result<Vec<extensions::ExtInfo>> {
    let (settings, layout) = state.snapshot().await;
    extensions::list(&state.http, &settings, &layout, &version).await
}

/// Switches an extension on or off for one PHP version and restarts PHP to apply it.
#[tauri::command]
pub(crate) async fn set_php_extension(app: AppHandle, state: State<'_, AppState>, version: String, name: String, enabled: bool) -> Result<()> {
    let _op = state.ops.lock().await;
    state
        .update_settings(|s| {
            let choices = s.php_extensions.entry(version.clone()).or_default();
            if enabled {
                choices.disabled.remove(&name);
                if !extensions::DEFAULTS.contains(&name.as_str()) {
                    choices.enabled.insert(name.clone());
                }
            } else {
                choices.enabled.remove(&name);
                if extensions::DEFAULTS.contains(&name.as_str()) {
                    choices.disabled.insert(name.clone());
                }
            }
            // Only differences from the defaults are stored; back to defaults means no entry.
            if choices.enabled.is_empty() && choices.disabled.is_empty() {
                s.php_extensions.remove(&version);
            }
        })
        .await?;
    logs::system(&app, logs::Level::Info, format!("PHP {version}: {name} {}", if enabled { "enabled" } else { "disabled" }));
    restart_web_servers(&app, &state).await
}

/// Downloads an extra extension for one PHP version and enables it.
#[tauri::command]
pub(crate) async fn install_php_extension(app: AppHandle, state: State<'_, AppState>, version: String, name: String) -> Result<()> {
    let (_, layout) = state.snapshot().await;
    logs::system(&app, logs::Level::Info, format!("Installing {name} for PHP {version}"));
    let ext_version = extensions::install(&state.http, &layout, &version, &name).await?;
    logs::system(&app, logs::Level::Info, format!("Installed {name} {ext_version} for PHP {version}"));
    set_php_extension(app, state, version, name, true).await
}

#[tauri::command]
pub(crate) async fn php_status(state: State<'_, AppState>) -> Result<Vec<php::PoolStatus>> {
    Ok(state.supervisor.php.status().await)
}

/// Every runtime with its group, description, source, platform gaps and companions.
#[tauri::command]
pub(crate) fn runtime_info() -> catalog::Catalog {
    catalog::catalog()
}

/// What a runtime version needs, and how each need gets met (see `catalog::requirements`).
#[tauri::command]
pub(crate) async fn runtime_requirements(app: AppHandle, state: State<'_, AppState>, runtime: Runtime, version: Option<String>) -> Result<Vec<catalog::Requirement>> {
    let (settings, layout) = state.snapshot().await;
    catalog::requirements(&app, &state.http, &settings, &layout, runtime, version).await
}

/// What installing a starter stack would do, for the first-run wizard.
#[tauri::command]
pub(crate) async fn starter_plan(state: State<'_, AppState>, runtimes: Vec<String>) -> Result<Vec<catalog::StarterItem>> {
    let (settings, layout) = state.snapshot().await;
    Ok(catalog::starter(&state.http, &settings, &layout, &runtimes).await)
}

#[tauri::command]
pub(crate) async fn runtime_catalog(state: State<'_, AppState>, runtime: Runtime, fresh: Option<bool>) -> Result<Vec<Release>> {
    runtimes::available(&state.http, runtime, fresh.unwrap_or(false)).await
}

#[tauri::command]
pub(crate) async fn installed_runtimes(state: State<'_, AppState>) -> Result<Vec<InstalledRuntime>> {
    let (settings, layout) = state.snapshot().await;
    Ok(Runtime::all()
        .into_iter()
        .map(|runtime| InstalledRuntime {
            runtime,
            active: settings.versions.get(runtime.id()).cloned(),
            versions: runtimes::installed(&layout, runtime),
        })
        .collect())
}

#[tauri::command]
pub(crate) async fn install_runtime(app: AppHandle, state: State<'_, AppState>, release: Release) -> Result<()> {
    let key = (release.runtime, release.version.clone());
    if !state.installing.lock().await.insert(key.clone()) {
        return Err(Error::msg(format!("{} {} is already being installed", release.runtime.id(), release.version)));
    }
    let (_, layout) = state.snapshot().await;
    let label = format!("{} {}", release.runtime.id(), release.version);
    logs::system(&app, logs::Level::Info, format!("Installing {label} from {}", release.url));
    let result = async {
        runtimes::install(&app, &state.http, &layout, &release).await?;
        // Some runtimes need a setup step after the download; the install isn't finished
        // (and the version isn't used) until it is.
        let setup = match release.runtime {
            Runtime::Rust => {
                logs::system(&app, logs::Level::Info, format!("Installing Rust {} with rustup", release.version));
                runtimes::setup_rust(&layout, &release.version).await
            }
            Runtime::Conda => {
                logs::system(&app, logs::Level::Info, "Running the Miniforge installer (a few minutes)");
                runtimes::setup_conda(&layout, &release.version).await
            }
            Runtime::Phpredisadmin => {
                logs::system(&app, logs::Level::Info, "Adding Predis, phpRedisAdmin's Redis client");
                runtimes::add_predis(&state.http, &layout, &release.version).await
            }
            // A service's own install steps (its manifest's `install`).
            _ => match manifest::get(release.runtime.id()).filter(|m| m.install.is_some()) {
                Some(m) => {
                    logs::system(&app, logs::Level::Info, format!("Setting up {} (a few minutes)", m.name));
                    let settings = state.snapshot().await.0;
                    match services::need_values(m, &settings, &layout, Some(&release.version)) {
                        Ok(needs) => crate::setup::install(m, &settings, &layout, &release.version, &needs).await,
                        Err(e) => {
                            let _ = std::fs::remove_dir_all(layout.bin(release.runtime.id(), &release.version));
                            Err(e)
                        }
                    }
                }
                None => Ok(()),
            },
        };
        if runtimes::has_setup(release.runtime) {
            let stage = if setup.is_ok() { "done" } else { "error" };
            let _ = tauri::Emitter::emit(&app, "install-progress", runtimes::InstallProgress { runtime: release.runtime, version: release.version.clone(), stage, downloaded: 0, total: None });
        }
        setup
    }
    .await;
    state.installing.lock().await.remove(&key);
    match &result {
        Ok(()) => logs::system(&app, logs::Level::Info, format!("Installed {label}")),
        Err(e) => logs::system(&app, logs::Level::Error, format!("Installing {label} failed: {e}")),
    }
    result?;

    // A service that isn't built in gets its ports now, clear of every other service's.
    if let Runtime::Plugin(i) = release.runtime {
        state.update_settings(|s| manifest::extra(i).assign_ports(s, services::port_free)).await?;
    }
    // The first installed version of a runtime becomes the active one.
    let mut activated = false;
    state
        .update_settings(|s| {
            if s.versions.get(release.runtime.id()).is_none() {
                s.versions.set(release.runtime.id(), Some(release.version.clone()));
                activated = true;
            }
        })
        .await?;
    // A new tool brings a domain and, for Mailpit, PHP mail settings: the primary web
    // server picks both up on restart.
    if activated && is_tool(release.runtime) {
        let primary = server_kind(state.snapshot().await.0.primary_server());
        restart_if_running(&app, &state, primary).await?;
    }
    Ok(())
}

pub(crate) fn is_tool(runtime: Runtime) -> bool {
    // The PHP tools, and services with a web interface (their manifest's `ui`).
    matches!(runtime, Runtime::Adminer | Runtime::Phpmyadmin | Runtime::Phpredisadmin | Runtime::Phppgadmin) || manifest::get(runtime.id()).is_some_and(|m| m.ui.is_some())
}

#[tauri::command]
pub(crate) async fn remove_runtime(app: AppHandle, state: State<'_, AppState>, runtime: Runtime, version: String) -> Result<()> {
    let (settings, layout) = state.snapshot().await;
    let was_active = settings.versions.get(runtime.id()) == Some(&version);
    // Web tools are just files served by Apache, which restarts below instead of blocking.
    if was_active && !matches!(runtime, Runtime::Adminer | Runtime::Phpmyadmin | Runtime::Phpredisadmin | Runtime::Phppgadmin) {
        for kind in dependents(runtime) {
            if state.supervisor.is_running(kind).await {
                return Err(Error::msg(format!("Stop {} before removing the version it is using", kind.label())));
            }
        }
    }
    // A site can run a PHP version other than the default; its workers lock the files.
    if runtime == Runtime::Php && state.supervisor.php.status().await.iter().any(|p| p.version == version && p.alive > 0) {
        let users: Vec<String> =
            settings.sites.iter().filter(|(_, s)| s.php.as_deref() == Some(version.as_str())).map(|(n, _)| n.clone()).collect();
        return Err(Error::msg(format!(
            "PHP {version} is running for {}. Switch {} to another version or stop the web server first.",
            if users.is_empty() { "a site".to_string() } else { users.join(", ") },
            if users.len() == 1 { "that site" } else { "those sites" },
        )));
    }
    runtimes::remove(&layout, runtime, &version)?;
    logs::system(&app, logs::Level::Info, format!("Removed {} {version}", runtime.id()));
    state
        .update_settings(|s| {
            reconcile_versions(s);
            // Sites pinned to the removed PHP go back to the default version.
            if runtime == Runtime::Php {
                for site in s.sites.values_mut().filter(|site| site.php.as_deref() == Some(version.as_str())) {
                    site.php = None;
                }
            }
        })
        .await?;
    if was_active && is_tool(runtime) {
        restart_if_running(&app, &state, server_kind(settings.primary_server())).await?;
    }
    Ok(())
}

/// Switches the active version, restarting services that were using the old one.
#[tauri::command]
pub(crate) async fn set_active_version(
    app: AppHandle,
    state: State<'_, AppState>,
    runtime: Runtime,
    version: String,
) -> Result<()> {
    let _op = state.ops.lock().await;
    let (_, layout) = state.snapshot().await;
    if !runtimes::installed(&layout, runtime).contains(&version) {
        return Err(Error::msg(format!("{} {version} is not installed", runtime.id())));
    }

    let mut restart = Vec::new();
    for kind in dependents(runtime) {
        if state.supervisor.is_running(kind).await {
            state.supervisor.stop(&app, kind).await;
            restart.push(kind);
        }
    }
    for &kind in &restart {
        state.supervisor.wait_stopped(kind).await;
    }

    logs::system(&app, logs::Level::Info, format!("Switching {} to {version}", runtime.id()));
    let settings = state.update_settings(|s| s.versions.set(runtime.id(), Some(version))).await?;
    let layout = Layout::new(&settings.root_dir);
    for kind in restart {
        state.supervisor.start(&app, kind, &settings, &layout).await?;
    }
    Ok(())
}
