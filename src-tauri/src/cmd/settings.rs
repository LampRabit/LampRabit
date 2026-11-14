//! Settings: reading and saving them, their checks, and the first run.

use crate::*;

/// Drops "disabled" entries for extensions that are off by default anyway (older builds
/// stored them) and versions left with no choices.
pub(crate) fn tidy_extension_choices(settings: &mut Settings) -> bool {
    let before = settings.php_extensions.clone();
    for choices in settings.php_extensions.values_mut() {
        choices.disabled.retain(|name| extensions::DEFAULTS.contains(&name.as_str()));
    }
    settings.php_extensions.retain(|_, c| !c.enabled.is_empty() || !c.disabled.is_empty());
    settings.php_extensions != before
}

/// Drops active versions whose files are gone (e.g. deleted by hand).
pub(crate) fn reconcile_versions(settings: &mut Settings) {
    let layout = Layout::new(&settings.root_dir);
    for runtime in Runtime::ALL {
        let installed = runtimes::installed(&layout, runtime);
        if settings.versions.get(runtime.id()).is_none_or(|v| !installed.contains(v)) {
            settings.versions.set(runtime.id(), installed.into_iter().next());
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EnvPaths {
    pub(crate) root: PathBuf,
    pub(crate) www: PathBuf,
    pub(crate) logs: PathBuf,
    pub(crate) settings_file: PathBuf,
    /// "windows", "macos" or "linux", so the UI can hide OS-specific options.
    pub(crate) os: &'static str,
}

#[tauri::command]
pub(crate) async fn get_settings(state: State<'_, AppState>) -> Result<Settings> {
    Ok(state.settings.read().await.clone())
}

#[tauri::command]
pub(crate) async fn save_settings(app: AppHandle, state: State<'_, AppState>, mut settings: Settings) -> Result<Settings> {
    let old = state.settings.read().await.clone();
    check_values(&mut settings)?;
    check_ports(&settings)?;
    let layout = Layout::new(&settings.root_dir);
    check_root(&layout)?;
    // Running services belong to the old folder; stop them before switching.
    if state.settings.read().await.root_dir != settings.root_dir {
        let _op = state.ops.lock().await;
        let running: Vec<ServiceKind> = state.supervisor.pids().await.into_values().collect();
        if !running.is_empty() {
            logs::system(&app, logs::Level::Info, "Environment folder changed; stopping services from the old folder");
            for &kind in &running {
                state.supervisor.stop(&app, kind).await;
            }
            for kind in running {
                state.supervisor.wait_stopped(kind).await;
            }
        }
    }
    reconcile_versions(&mut settings);
    let was_on_path = state.settings.read().await.add_to_path;
    if settings.add_to_path != was_on_path {
        shims::set_on_path(&layout, settings.add_to_path)?;
        let what = if settings.add_to_path { "added to" } else { "removed from" };
        logs::system(&app, logs::Level::Info, format!("{} {what} your PATH", shims::dir(&layout).display()));
    }
    settings.save(&state.settings_path)?;
    *state.settings.write().await = settings.clone();
    refresh_shims(&app, &settings);
    *state.ca_trusted.lock().await = None;
    logs::set_system_file(&app, layout.logs().join("lampRabit.log"));
    logs::system(&app, logs::Level::Info, "Settings saved");
    for kind in ServiceKind::WEB {
        if !kind.enabled(&settings) && state.supervisor.is_running(kind).await {
            logs::system(&app, logs::Level::Info, format!("{} is switched off in Settings; stopping it", kind.label()));
            state.supervisor.stop(&app, kind).await;
        }
    }
    // Running services pick up what changed for them (ports, the primary server, the
    // domain suffix…) straight away; services in the old folder were stopped above.
    if old.root_dir == settings.root_dir {
        let old_layout = Layout::new(&old.root_dir);
        let mut restart = Vec::new();
        for kind in ServiceKind::all() {
            if kind.enabled(&settings)
                && state.supervisor.is_running(kind).await
                && service_fingerprint(kind, &old, &old_layout) != service_fingerprint(kind, &settings, &layout)
            {
                state.supervisor.stop(&app, kind).await;
                restart.push(kind);
            }
        }
        for &kind in &restart {
            state.supervisor.wait_stopped(kind).await;
        }
        for kind in restart {
            if let Err(e) = state.supervisor.start(&app, kind, &settings, &layout).await {
                logs::system(&app, logs::Level::Error, format!("{} didn't restart with the new settings: {e}", kind.label()));
            }
        }
    }
    Ok(settings)
}

/// What a settings change can alter in a running service: its ports and, for web servers,
/// everything their configs are built from. A different fingerprint means a restart.
pub(crate) fn service_fingerprint(kind: ServiceKind, settings: &Settings, layout: &Layout) -> String {
    let ports = services::required_ports(kind, settings, layout);
    if kind.web_server().is_some() {
        format!(
            "{ports:?} {:?} {:?} {} {} {} {}",
            settings.web_servers, settings.ports, settings.tld, settings.https_by_default, settings.php_workers, settings.route_mail
        )
    } else if matches!(kind, ServiceKind::Rustfs | ServiceKind::Minio) {
        format!("{ports:?} {:?}", settings.storage_dir(layout, kind.id()))
    } else {
        format!("{ports:?}")
    }
}

/// Domain endings that exist on the internet: using one would send real websites
/// (google.com, …) to this machine. .dev and .app also force HTTPS in browsers.
pub(crate) const PUBLIC_TLDS: &[&str] = &[
    "com", "net", "org", "io", "co", "dev", "app", "ai", "me", "info", "biz", "us", "uk", "in", "de", "fr", "eu",
    "xyz", "site", "online", "tech", "store", "cloud", "page", "gov", "edu", "mil",
];

/// Tidies and checks values the UI can't fully constrain. Rejects rather than guesses
/// where a guess could surprise someone.
pub(crate) fn check_values(settings: &mut Settings) -> Result<()> {
    let tld = settings.tld.trim().trim_matches('.').to_lowercase();
    let valid_label = |l: &str| !l.is_empty() && l.len() <= 63 && !l.starts_with('-') && !l.ends_with('-') && l.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
    if tld.is_empty() || !tld.split('.').all(valid_label) {
        return Err(Error::msg("The domain suffix can use letters, numbers and dashes, for example \"test\" or \"localhost\"."));
    }
    let last = tld.rsplit('.').next().unwrap_or_default();
    if PUBLIC_TLDS.contains(&last) {
        return Err(Error::msg(format!(
            ".{tld} is a real domain ending on the internet, so it would hide real websites. Use \"test\" or \"localhost\"."
        )));
    }
    settings.tld = tld;
    settings.normalize();
    if !(1..=32).contains(&settings.php_workers) {
        return Err(Error::msg("PHP workers must be between 1 and 32."));
    }
    let p = &settings.ports;
    for (port, name) in [
        (p.http, "HTTP"), (p.https, "HTTPS"), (p.nginx_http, "Nginx HTTP"), (p.nginx_https, "Nginx HTTPS"),
        (p.apache_http, "Apache HTTP"), (p.apache_https, "Apache HTTPS"), (p.caddy_http, "Caddy HTTP"), (p.caddy_https, "Caddy HTTPS"),
        (p.caddy_admin, "Caddy admin"), (p.frankenphp_http, "FrankenPHP HTTP"), (p.frankenphp_https, "FrankenPHP HTTPS"),
        (p.frankenphp_admin, "FrankenPHP admin"), (p.tomcat_http, "Tomcat HTTP"), (p.tomcat_shutdown, "Tomcat shutdown"), (p.mysql, "MariaDB"), (p.mail_smtp, "Mailpit SMTP"), (p.mail_ui, "Mailpit inbox"), (p.redis, "Redis"),
        (p.memcached, "Memcached"), (p.php_fpm, "PHP FastCGI"), (p.postgres, "PostgreSQL"), (p.pgadmin, "pgAdmin"), (p.mongodb, "MongoDB"), (p.clickhouse_http, "ClickHouse HTTP"), (p.clickhouse_tcp, "ClickHouse native"), (p.neo4j_http, "Neo4j Browser"), (p.neo4j_bolt, "Neo4j Bolt"), (p.meilisearch, "Meilisearch"), (p.typesense, "Typesense"), (p.rustfs, "RustFS"), (p.rustfs_console, "RustFS console"), (p.ollama, "Ollama"), (p.minio, "MinIO"), (p.minio_console, "MinIO console"), (p.elasticsearch, "Elasticsearch"), (p.elasticsearch_transport, "Elasticsearch transport"), (p.opensearch, "OpenSearch"), (p.opensearch_transport, "OpenSearch transport"), (p.n8n, "n8n"), (p.rabbitmq, "RabbitMQ"), (p.rabbitmq_management, "RabbitMQ console"),
    ] {
        if port == 0 {
            return Err(Error::msg(format!("Choose a port for {name} (1–65535).")));
        }
    }
    if p.php_fpm > 65535 - 99 {
        return Err(Error::msg("PHP FastCGI needs 100 ports from its starting port; choose a starting port below 65437."));
    }
    Ok(())
}

/// The environment folder must be creatable, writable and outside system folders.
pub(crate) fn check_root(layout: &Layout) -> Result<()> {
    let root = layout.root();
    if !root.is_absolute() {
        return Err(Error::msg("Choose a full path for the environment folder, for example C:\\lampRabit."));
    }
    #[cfg(windows)]
    {
        let lower = root.to_string_lossy().to_lowercase();
        let system: Vec<String> = ["SystemRoot", "ProgramFiles", "ProgramFiles(x86)", "ProgramData"]
            .iter()
            .filter_map(|v| std::env::var(v).ok())
            .map(|v| v.to_lowercase())
            .collect();
        if system.iter().any(|s| lower == *s || lower.starts_with(&format!("{s}\\"))) {
            return Err(Error::msg(
                "The environment folder can't be inside Windows or Program Files: those need administrator rights. Use something like C:\\lampRabit.",
            ));
        }
    }
    layout.ensure().map_err(|e| Error::msg(format!("Couldn't create {}: {e}", root.display())))?;
    // Some locations accept folder creation but not writes (or redirect them); check for real.
    let probe = layout.tmp().join(".lamprabit-write-test");
    std::fs::write(&probe, b"ok")
        .and_then(|_| std::fs::read(&probe))
        .map_err(|e| Error::msg(format!("LampRabit can't write to {}: {e}", root.display())))?;
    let _ = std::fs::remove_file(probe);
    Ok(())
}

/// Rejects settings where two services would listen on the same port.
pub(crate) fn check_ports(settings: &Settings) -> Result<()> {
    use config::Server;
    let p = &settings.ports;
    let mut used: Vec<(u16, String)> = vec![
        (p.mysql, "MariaDB".into()),
        (p.mail_smtp, "Mailpit SMTP".into()),
        (p.mail_ui, "Mailpit inbox".into()),
        (p.redis, "Redis".into()),
        (p.postgres, "PostgreSQL".into()),
        (p.pgadmin, "pgAdmin".into()),
        (p.mongodb, "MongoDB".into()),
        (p.clickhouse_http, "ClickHouse HTTP".into()),
        (p.clickhouse_tcp, "ClickHouse native".into()),
        (p.neo4j_http, "Neo4j Browser".into()),
        (p.neo4j_bolt, "Neo4j Bolt".into()),
        (p.meilisearch, "Meilisearch".into()),
        (p.typesense, "Typesense".into()),
        (p.rustfs, "RustFS".into()),
        (p.rustfs_console, "RustFS console".into()),
        (p.ollama, "Ollama".into()),
        (p.minio, "MinIO".into()),
        (p.minio_console, "MinIO console".into()),
        (p.elasticsearch, "Elasticsearch".into()),
        (p.elasticsearch_transport, "Elasticsearch transport".into()),
        (p.opensearch, "OpenSearch".into()),
        (p.opensearch_transport, "OpenSearch transport".into()),
        (p.n8n, "n8n".into()),
        (p.rabbitmq, "RabbitMQ".into()),
        (p.rabbitmq_management, "RabbitMQ console".into()),
        (p.memcached, "Memcached".into()),
    ];
    for server in Server::ALL {
        if settings.server_enabled(server) {
            let (http, https) = settings.server_ports(server);
            let name = server.label();
            used.push((http, format!("{name} HTTP")));
            used.push((https, format!("{name} HTTPS")));
        }
    }
    if settings.server_enabled(Server::Caddy) {
        used.push((p.caddy_admin, "Caddy admin".into()));
    }
    if settings.server_enabled(Server::Frankenphp) {
        used.push((p.frankenphp_admin, "FrankenPHP admin".into()));
    }
    if settings.versions.get("tomcat").is_some() {
        used.push((p.tomcat_http, "Tomcat HTTP".into()));
        used.push((p.tomcat_shutdown, "Tomcat shutdown".into()));
    }
    let php_block = p.php_fpm..p.php_fpm.saturating_add(100);
    for (i, (port, a)) in used.iter().enumerate() {
        if let Some((_, b)) = used[i + 1..].iter().find(|(q, _)| q == port) {
            return Err(Error::msg(format!("{a} and {b} are both set to port {port}. Give each its own port.")));
        }
        if php_block.contains(port) {
            return Err(Error::msg(format!(
                "{a} (port {port}) overlaps the PHP FastCGI ports ({}-{}). Move one of them.",
                php_block.start,
                php_block.end - 1
            )));
        }
    }
    Ok(())
}

/// Set on the first run, until the LampRabit folder's place is confirmed.
pub(crate) struct FirstRun(pub(crate) std::sync::atomic::AtomicBool);

/// Whether this is the first run, when the app asks where the LampRabit folder goes.
#[tauri::command]
pub(crate) fn first_run(flag: State<'_, FirstRun>) -> bool {
    flag.0.load(std::sync::atomic::Ordering::Relaxed)
}

/// The LampRabit folder's place is settled: the settings are written, so later runs
/// don't ask again.
#[tauri::command]
pub(crate) async fn finish_first_run(app: AppHandle, state: State<'_, AppState>, flag: State<'_, FirstRun>) -> Result<()> {
    flag.0.store(false, std::sync::atomic::Ordering::Relaxed);
    let settings = state.update_settings(|_| {}).await?;
    logs::system(&app, logs::Level::Info, format!("LampRabit's folder is {}", settings.root_dir.display()));
    Ok(())
}

#[tauri::command]
pub(crate) async fn env_paths(state: State<'_, AppState>) -> Result<EnvPaths> {
    let (_, layout) = state.snapshot().await;
    Ok(EnvPaths {
        root: layout.root().to_path_buf(),
        www: layout.www(),
        logs: layout.logs(),
        settings_file: state.settings_path.clone(),
        os: std::env::consts::OS,
    })
}

/// Problems found while starting up, shown once the window is ready.
pub(crate) struct StartupWarnings(pub(crate) std::sync::Mutex<Vec<String>>);

#[tauri::command]
pub(crate) fn take_startup_warnings(warnings: State<'_, StartupWarnings>) -> Vec<String> {
    std::mem::take(&mut *warnings.0.lock().unwrap())
}
