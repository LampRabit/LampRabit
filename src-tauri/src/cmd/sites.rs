//! Sites: the list, applying configs, per-site settings and domains.

use crate::*;

/// Sets the version a Python or Java project uses, by writing its version file (None
/// removes it, back to the active version); its shims follow straight away.
#[tauri::command]
pub(crate) async fn set_language_version(app: AppHandle, state: State<'_, AppState>, name: String, version: Option<String>) -> Result<()> {
    let (settings, layout) = state.snapshot().await;
    let dir = layout.www().join(&name);
    let runtime = match sites::language(&dir) {
        sites::Language::Python => "python",
        sites::Language::Java => "java",
        sites::Language::Go => "go",
        sites::Language::Rust => "rust",
        _ => return Err(Error::msg("This project's language has no version to choose.")),
    };
    langver::set(&dir, runtime, version.as_deref())?;
    refresh_shims(&app, &settings);
    Ok(())
}

/// Makes a new project's domain work right away: its certificate is issued, the local CA
/// trusted and the hosts file updated now, so the admin prompts come while the user is
/// watching the wizard instead of later, when a web server happens to start. A cancelled
/// prompt doesn't fail the project; the Sites page offers the same fixes.
pub(crate) async fn prepare_new_site(app: &AppHandle, state: &AppState, name: &str) {
    let (settings, layout) = state.snapshot().await;
    let served = sites::served(&settings, &layout);
    if let Some(site) = served.iter().find(|s| s.name == name && s.ssl) {
        match ssl::ensure_ca(&layout).and_then(|(ca, _)| ssl::ensure_site_cert(&layout, &ca, &site.domain)) {
            Ok(_) if !ssl::ca_trusted(&layout).await => {
                *state.ca_trusted.lock().await = None;
                if let Err(e) = ssl::trust_ca(&layout).await {
                    logs::system(app, logs::Level::Warn, format!("{e} Browsers will warn about {} until LampRabit's certificate authority is trusted (Sites page).", site.domain));
                }
            }
            Ok(_) => {}
            Err(e) => logs::system(app, logs::Level::Warn, format!("Couldn't create a certificate for {}: {e}", site.domain)),
        }
    }
    if settings.manage_hosts && sites::needs_hosts_file(&settings) && !sites::hosts_in_sync(&served) {
        match sites::update_hosts(&layout, &served).await {
            Ok(()) => logs::system(app, logs::Level::Info, "Hosts file updated"),
            Err(e) => logs::system(app, logs::Level::Warn, format!("{e} The new domain may not resolve until the hosts file is updated (Sites page).")),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SitesInfo {
    pub(crate) sites: Vec<sites::Site>,
    /// Installed developer tools and their domains.
    pub(crate) tools: Vec<sites::Site>,
    pub(crate) tld: String,
    /// Apache is running with a different site list than what is in www/ now.
    pub(crate) needs_apply: bool,
    /// The hosts file is in sync, or does not need to be (`.localhost`, or management off).
    pub(crate) hosts_ok: bool,
    pub(crate) manage_hosts: bool,
    /// None while no site uses HTTPS.
    pub(crate) ca_trusted: Option<bool>,
    pub(crate) ca_path: PathBuf,
    /// Folders in www/ that can't be served, with the reason.
    pub(crate) unserved: Vec<sites::Unserved>,
    /// Tool domains (adminer.test…) that a folder of the same name has taken.
    pub(crate) shadowed_tools: Vec<String>,
}

/// Every folder in www/ is a site, most recently modified first.
#[tauri::command]
pub(crate) async fn list_sites(state: State<'_, AppState>) -> Result<SitesInfo> {
    let (settings, layout) = state.snapshot().await;
    let served = sites::served(&settings, &layout);
    let plan = php::plan(&settings, &layout, &served);
    let mut needs_apply = false;
    for kind in ServiceKind::WEB {
        if state.supervisor.is_running(kind).await {
            needs_apply |= sites::config_stale(&settings, &layout, &served, &plan, kind.web_server().unwrap());
        }
    }
    let hosts_ok = !sites::needs_hosts_file(&settings) || sites::hosts_in_sync(&served);
    let (tools, sites): (Vec<_>, Vec<_>) = served.into_iter().partition(|s| s.tool.is_some());
    // A project's subdomains are served as copies of it (for configs, certificates and the
    // hosts file); the app shows them on their project, not as projects of their own.
    let sites: Vec<_> = sites.into_iter().filter(|s| !sites::is_extra(s)).collect();
    let ca_trusted = if sites.iter().any(|s| s.ssl) && ssl::ca_thumbprint(&layout).is_some() {
        let mut cached = state.ca_trusted.lock().await;
        if cached.is_none() {
            *cached = Some(ssl::ca_trusted(&layout).await);
        }
        *cached
    } else if sites.iter().any(|s| s.ssl) {
        Some(false)
    } else {
        None
    };
    Ok(SitesInfo {
        sites,
        tools,
        tld: settings.tld.clone(),
        needs_apply,
        hosts_ok,
        manage_hosts: settings.manage_hosts,
        ca_trusted,
        ca_path: ssl::ca_cert_path(&layout),
        unserved: sites::unserved(&settings, &layout),
        shadowed_tools: sites::shadowed_tools(&settings, &layout),
    })
}

/// Makes Apache and the hosts file match www/: restarts Apache if it is running
/// (which regenerates vhosts and certificates), otherwise just syncs the hosts file.
#[tauri::command]
pub(crate) async fn apply_sites(app: AppHandle, state: State<'_, AppState>) -> Result<()> {
    let _op = state.ops.lock().await;
    logs::system(&app, logs::Level::Info, "Applying site changes");
    sync_sites(&app, &state).await
}

/// Restarts the running web servers (vhosts, certificates) or, with none running, syncs
/// the hosts file; for changes to the list of projects.
pub(crate) async fn sync_sites(app: &AppHandle, state: &AppState) -> Result<()> {
    let (settings, layout) = state.snapshot().await;
    let app = app.clone();
    let mut any_running = false;
    for kind in ServiceKind::WEB {
        any_running |= state.supervisor.is_running(kind).await;
    }
    if any_running {
        restart_web_servers(&app, state).await
    } else {
        let sites = sites::served(&settings, &layout);
        if sites::needs_hosts_file(&settings) && !sites::hosts_in_sync(&sites) {
            sites::update_hosts(&layout, &sites).await?;
            logs::system(&app, logs::Level::Info, "Hosts file updated");
        }
        Ok(())
    }
}

#[tauri::command]
pub(crate) async fn set_site_ssl(app: AppHandle, state: State<'_, AppState>, name: String, ssl: bool) -> Result<()> {
    let _op = state.ops.lock().await;
    state.update_settings(|s| s.sites.entry(name.clone()).or_default().ssl = Some(ssl)).await?;
    logs::system(&app, logs::Level::Info, format!("HTTPS {} for {name}", if ssl { "enabled" } else { "disabled" }));
    restart_web_servers(&app, &state).await
}

/// Sets a site's PHP version (None = follow the active version).
#[tauri::command]
pub(crate) async fn set_site_php(app: AppHandle, state: State<'_, AppState>, name: String, version: Option<String>) -> Result<()> {
    let _op = state.ops.lock().await;
    let (_, layout) = state.snapshot().await;
    if let Some(v) = &version
        && !runtimes::installed(&layout, Runtime::Php).contains(v) {
            return Err(Error::msg(format!("PHP {v} is not installed")));
        }
    let label = version.clone().unwrap_or_else(|| "the active version".into());
    state.update_settings(|s| s.sites.entry(name.clone()).or_default().php = version).await?;
    logs::system(&app, logs::Level::Info, format!("{name} now uses PHP {label}"));
    restart_web_servers(&app, &state).await
}

/// Turns FrankenPHP's worker mode on or off for a site.
#[tauri::command]
pub(crate) async fn set_site_worker(app: AppHandle, state: State<'_, AppState>, name: String, enabled: bool) -> Result<()> {
    let _op = state.ops.lock().await;
    state.update_settings(|s| s.sites.entry(name.clone()).or_default().worker = enabled).await?;
    logs::system(&app, logs::Level::Info, format!("Worker mode {} for {name}", if enabled { "on" } else { "off" }));
    restart_web_servers(&app, &state).await
}

/// Reloads the code of FrankenPHP's workers (they keep the app in memory between requests)
/// through its admin API, without stopping the server.
#[tauri::command]
pub(crate) async fn restart_frankenphp_workers(app: AppHandle, state: State<'_, AppState>) -> Result<()> {
    if !state.supervisor.is_running(ServiceKind::Frankenphp).await {
        return Err(Error::msg("FrankenPHP isn't running; its workers load the current code when it starts."));
    }
    let (settings, _) = state.snapshot().await;
    let url = format!("http://127.0.0.1:{}/frankenphp/workers/restart", settings.ports.frankenphp_admin);
    let response = state.http.post(&url).send().await.map_err(|e| Error::msg(format!("Couldn't reach FrankenPHP's admin API: {e}")))?;
    if !response.status().is_success() {
        return Err(Error::msg(format!("FrankenPHP didn't restart its workers ({}). Restart FrankenPHP on Home instead.", response.status())));
    }
    logs::system(&app, logs::Level::Info, "FrankenPHP's workers restarted");
    Ok(())
}

/// Picks the web server for a site when both are enabled.
#[tauri::command]
pub(crate) async fn set_site_server(app: AppHandle, state: State<'_, AppState>, name: String, server: Option<config::Server>) -> Result<()> {
    let _op = state.ops.lock().await;
    state.update_settings(|s| s.sites.entry(name.clone()).or_default().server = server).await?;
    logs::system(&app, logs::Level::Info, format!("{name} is now served by {}", server.map_or("the default server", |s| s.label())));
    restart_web_servers(&app, &state).await?;
    // The site now needs its new server; start it when a web server was serving sites.
    let (settings, layout) = state.snapshot().await;
    let kind = server_kind(sites::site_server(&settings, &name));
    let mut others_running = false;
    for other in ServiceKind::WEB.into_iter().filter(|&k| k != kind) {
        others_running |= state.supervisor.is_running(other).await;
    }
    if !state.supervisor.is_running(kind).await && others_running {
        state.supervisor.start(&app, kind, &settings, &layout).await?;
    }
    Ok(())
}

/// A project folder in www/ by name; names with path separators are refused.
pub(crate) fn project_dir(layout: &Layout, name: &str) -> Result<PathBuf> {
    let dir = layout.www().join(name);
    if name.is_empty() || name.contains(['/', '\\']) || name.starts_with('.') || !dir.is_dir() {
        return Err(Error::msg(format!("There's no project called {name} in www.")));
    }
    Ok(dir)
}

/// Picks the port a project's domain goes to when it listens on several; None goes back
/// to choosing automatically.
#[tauri::command]
pub(crate) async fn set_site_route(state: State<'_, AppState>, name: String, port: Option<u16>) -> Result<()> {
    let (_, layout) = state.snapshot().await;
    project_dir(&layout, &name)?;
    state.update_settings(|s| s.sites.entry(name.clone()).or_default().route_port = port).await?;
    ports::poke();
    Ok(())
}

/// Saves a project's extra domains and serves them (new certificates and hosts entries
/// come with the web server's restart).
#[tauri::command]
pub(crate) async fn set_site_domains(app: AppHandle, state: State<'_, AppState>, name: String, domains: Vec<config::ExtraDomain>) -> Result<()> {
    let _op = state.ops.lock().await;
    let (settings, layout) = state.snapshot().await;
    project_dir(&layout, &name)?;
    let domains: Vec<config::ExtraDomain> = domains
        .into_iter()
        .map(|d| config::ExtraDomain { host: d.host.trim().trim_end_matches('.').to_lowercase(), target: d.target })
        .collect();
    sites::check_domains(&settings, &layout, &name, &domains)?;
    let count = domains.len();
    state.update_settings(|s| s.sites.entry(name.clone()).or_default().domains = domains).await?;
    logs::system(&app, logs::Level::Info, format!("{name} has {count} extra domain{}", if count == 1 { "" } else { "s" }));
    restart_web_servers(&app, &state).await
}

/// Sets where a project's main domain goes: by hand (its files or a folder, a port, a
/// process), or None to follow what's running.
#[tauri::command]
pub(crate) async fn set_site_main(app: AppHandle, state: State<'_, AppState>, name: String, target: Option<config::DomainTarget>) -> Result<()> {
    let _op = state.ops.lock().await;
    let (settings, layout) = state.snapshot().await;
    project_dir(&layout, &name)?;
    let target = target.filter(|t| *t != config::DomainTarget::Main);
    if let Some(t) = &target {
        let host = sites::url_for(&settings, &name).unwrap_or_else(|| name.clone());
        sites::check_target(&settings, &layout, &name, &host, t)?;
    }
    state.update_settings(|s| s.sites.entry(name.clone()).or_default().main = target).await?;
    ports::poke();
    restart_web_servers(&app, &state).await
}

/// A project's folders, for the domain and process folder pickers.
#[tauri::command]
pub(crate) async fn project_folders(state: State<'_, AppState>, name: String) -> Result<Vec<String>> {
    let (_, layout) = state.snapshot().await;
    let dir = project_dir(&layout, &name)?;
    Ok(sites::project_folders(&dir))
}

/// Which of the given ports something listens on.
#[tauri::command]
pub(crate) async fn ports_listening(ports: Vec<u16>) -> Result<Vec<u16>> {
    tauri::async_runtime::spawn_blocking(move || manage::listening(&ports)).await.map_err(|e| Error::msg(e.to_string()))
}

/// A site's own php.ini settings and Xdebug choice; PHP restarts with the web servers.
#[tauri::command]
pub(crate) async fn set_site_php_ini(app: AppHandle, state: State<'_, AppState>, name: String, values: std::collections::BTreeMap<String, String>, xdebug: bool) -> Result<()> {
    let _op = state.ops.lock().await;
    let (_, layout) = state.snapshot().await;
    project_dir(&layout, &name)?;
    for (key, value) in &values {
        if !configs::site_ini_ok(key, value) {
            return Err(Error::msg(format!("{key} = {value} isn't valid: use a size like 512M (or -1), whole seconds, or On/Off.")));
        }
    }
    state
        .update_settings(|s| {
            let own = s.sites.entry(name.clone()).or_default();
            own.php_ini = values;
            own.xdebug = xdebug;
        })
        .await?;
    logs::system(&app, logs::Level::Info, format!("Saved {name}'s PHP settings"));
    restart_web_servers(&app, &state).await
}

/// Adds the local CA to the user's trust store (the OS asks for confirmation).
#[tauri::command]
pub(crate) async fn trust_ca(app: AppHandle, state: State<'_, AppState>) -> Result<Option<String>> {
    let (_, layout) = state.snapshot().await;
    *state.ca_trusted.lock().await = None;
    match ssl::trust_ca(&layout).await {
        Ok(note) => {
            logs::system(&app, logs::Level::Info, "Local certificate authority trusted");
            if let Some(note) = &note {
                logs::system(&app, logs::Level::Warn, note.clone());
            }
            Ok(note)
        }
        Err(e) => {
            logs::system(&app, logs::Level::Warn, e.to_string());
            Err(e)
        }
    }
}
