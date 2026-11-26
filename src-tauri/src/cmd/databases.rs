//! Databases: lists, backups, a project's database and the database tools.

use crate::*;

/// Sets the app a database opens in (`tool: None` goes back to the default, KarotX). Applies
/// at once, like the other choices on the Services page.
#[tauri::command]
pub(crate) async fn set_db_client(state: State<'_, AppState>, database: String, tool: Option<String>) -> Result<()> {
    for id in std::iter::once(&database).chain(tool.as_ref()) {
        if runtimes::Runtime::from_id(id).is_none() {
            return Err(Error::msg(format!("{id} isn't something LampRabit installs.")));
        }
    }
    state
        .update_settings(|s| match tool {
            Some(t) => drop(s.db_clients.insert(database, t)),
            None => drop(s.db_clients.remove(&database)),
        })
        .await?;
    Ok(())
}

/// The service of a database engine, which has to be running for its tools to connect.
pub(crate) async fn running_database(state: &AppState, engine: templates::DbEngine) -> Result<()> {
    let kind = match engine {
        templates::DbEngine::Mariadb => ServiceKind::Mariadb,
        templates::DbEngine::Mysql => ServiceKind::Mysql,
        templates::DbEngine::Postgres => ServiceKind::Postgres,
        templates::DbEngine::Mongodb => ServiceKind::Mongodb,
        // Files in the projects: nothing has to run.
        templates::DbEngine::Sqlite => return Ok(()),
    };
    if state.supervisor.is_running(kind).await {
        Ok(())
    } else {
        Err(Error::msg(format!("Start {} first: backups are made and restored through the running server.", engine.label())))
    }
}

#[tauri::command]
pub(crate) async fn list_databases(state: State<'_, AppState>, engine: templates::DbEngine) -> Result<Vec<String>> {
    running_database(&state, engine).await?;
    let (settings, layout) = state.snapshot().await;
    backups::databases(&settings, &layout, engine).await
}

#[tauri::command]
pub(crate) async fn list_backups(state: State<'_, AppState>) -> Result<Vec<backups::Backup>> {
    Ok(backups::list(&state.snapshot().await.1))
}

#[tauri::command]
pub(crate) async fn backup_database(app: AppHandle, state: State<'_, AppState>, engine: templates::DbEngine, database: String) -> Result<backups::Backup> {
    running_database(&state, engine).await?;
    let (settings, layout) = state.snapshot().await;
    let backup = backups::create(&settings, &layout, engine, &database).await?;
    logs::system(&app, logs::Level::Info, format!("Backed up {} database {database} to {}", engine.label(), backup.path.display()));
    Ok(backup)
}

#[tauri::command]
pub(crate) async fn restore_backup(app: AppHandle, state: State<'_, AppState>, engine: templates::DbEngine, path: PathBuf, database: String) -> Result<()> {
    running_database(&state, engine).await?;
    let (settings, layout) = state.snapshot().await;
    logs::system(&app, logs::Level::Info, format!("Restoring {} into {} database {database}", path.display(), engine.label()));
    backups::restore(&settings, &layout, engine, &path, &database).await?;
    logs::system(&app, logs::Level::Info, format!("Restored {database}"));
    Ok(())
}

/// Where an Ollama installed outside LampRabit is, if there is one (its manifest's `external`).
#[tauri::command]
pub(crate) async fn ollama_elsewhere(state: State<'_, AppState>) -> Result<Option<PathBuf>> {
    let (settings, layout) = state.snapshot().await;
    Ok(services::external_program(ServiceKind::Ollama, &settings, &layout))
}

/// Opens MongoDB Compass connected to LampRabit's MongoDB.
#[tauri::command]
pub(crate) async fn open_compass(state: State<'_, AppState>) -> Result<()> {
    let (settings, layout) = state.snapshot().await;
    let version = settings.versions.get("compass").ok_or_else(|| Error::msg("Install MongoDB Compass first (Services)."))?;
    let home = layout.bin("compass", version);
    let program = runtimes::compass_program(&home).ok_or_else(|| Error::msg("Compass's install looks incomplete. Remove it on the Services page and install it again."))?;
    let uri = format!("mongodb://127.0.0.1:{}", settings.ports.service("mongodb.mongodb"));
    let mut cmd = if cfg!(target_os = "macos") {
        let mut c = std::process::Command::new("open");
        c.arg("-a").arg(&program).arg("--args").arg(&uri);
        c
    } else {
        let mut c = std::process::Command::new(&program);
        c.arg(&uri);
        c
    };
    cmd.stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
    cmd.spawn().map_err(|e| Error::msg(format!("Couldn't open Compass: {e}")))?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn delete_backup(state: State<'_, AppState>, path: PathBuf) -> Result<()> {
    backups::delete(&state.snapshot().await.1, &path)
}

/// The PostgreSQL extensions one database has, and the ones it could have.
#[tauri::command]
pub(crate) async fn pg_extensions(state: State<'_, AppState>, database: String) -> Result<Vec<manage::PgExtension>> {
    running_database(&state, templates::DbEngine::Postgres).await?;
    let (settings, layout) = state.snapshot().await;
    manage::pg_extensions(&settings, &layout, &database).await
}

#[tauri::command]
pub(crate) async fn set_pg_extension(state: State<'_, AppState>, database: String, name: String, action: manage::PgExtensionAction) -> Result<()> {
    running_database(&state, templates::DbEngine::Postgres).await?;
    let (settings, layout) = state.snapshot().await;
    manage::set_pg_extension(&settings, &layout, &database, &name, action).await
}

/// Whether the project's database exists, and its server runs.
#[tauri::command]
pub(crate) async fn database_status(state: State<'_, AppState>, name: String) -> Result<Option<manage::DbStatus>> {
    let (settings, layout) = state.snapshot().await;
    let dir = project_dir(&layout, &name)?;
    let Some(database) = sites::scan(&settings, &layout).into_iter().find(|s| s.name == name).and_then(|s| s.database) else { return Ok(None) };
    let Some(engine) = manage::engine(&settings, &dir) else { return Ok(None) };
    let running = match engine.service() {
        Some(kind) => state.supervisor.is_running(kind).await,
        None => true,
    };
    Ok(manage::status(&settings, &layout, &dir, &database, running).await)
}

/// Creates the database the project's config names.
#[tauri::command]
pub(crate) async fn create_project_database(app: AppHandle, state: State<'_, AppState>, name: String) -> Result<()> {
    let (settings, layout) = state.snapshot().await;
    let dir = project_dir(&layout, &name)?;
    let database = sites::scan(&settings, &layout).into_iter().find(|s| s.name == name).and_then(|s| s.database).ok_or_else(|| Error::msg(format!("{name} doesn't name a database in its config.")))?;
    let engine = manage::engine(&settings, &dir).ok_or_else(|| Error::msg("Install MariaDB, MySQL or PostgreSQL first (Services)."))?;
    if let Some(kind) = engine.service()
        && !state.supervisor.is_running(kind).await {
            return Err(Error::msg(format!("Start {} first, then create the database.", engine.label())));
        }
    manage::create(&settings, &layout, &dir, engine, &database).await?;
    logs::system(&app, logs::Level::Info, format!("Created database {database} in {} for {name}", engine.label()));
    Ok(())
}

/// Points a project at the other MySQL-family server (its .env's DB_PORT).
#[tauri::command]
pub(crate) async fn switch_project_database(state: State<'_, AppState>, name: String, engine: manage::Engine) -> Result<()> {
    let (settings, layout) = state.snapshot().await;
    manage::switch_database(&settings, &project_dir(&layout, &name)?, engine)
}

/// Opens a database's command-line client in a new terminal window (the default
/// database when none is named).
#[tauri::command]
pub(crate) async fn open_db_client(state: State<'_, AppState>, database: Option<config::Database>) -> Result<()> {
    let (settings, layout) = state.snapshot().await;
    let db = database.or(settings.database()).ok_or_else(|| Error::msg("Install MariaDB or MySQL first (Services)."))?;
    let home = configs::database_home(&settings, &layout, db)?;
    let client = configs::database_client(&settings, &layout, db)?;
    let defaults = format!("--defaults-file={}", configs::write_client_config(&settings, &layout, db)?.display());
    if cfg!(windows) {
        std::process::Command::new("cmd")
            .args(["/C", "start", db.label()])
            .arg(&client)
            .arg(defaults.as_str())
            .current_dir(&home)
            .spawn()?;
        Ok(())
    } else if cfg!(target_os = "macos") {
        Err(Error::msg(format!("Opening a terminal is not supported on this OS yet. Run: {} {defaults}", client.display())))
    } else {
        #[cfg(not(windows))]
        comfort::run_in_terminal(&home, &[client.to_string_lossy().into_owned(), defaults])?;
        Ok(())
    }
}
