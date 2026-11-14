//! Logs and the dump catcher.

use crate::*;

#[tauri::command]
pub(crate) async fn log_history(app: AppHandle, state: State<'_, AppState>) -> Result<logs::History> {
    let (_, layout) = state.snapshot().await;
    Ok(logs::history(&app, &layout.logs()))
}

/// The dumps caught this session, oldest first.
#[tauri::command]
pub(crate) fn list_dumps(app: AppHandle) -> Vec<dumps::Dump> {
    app.state::<dumps::DumpStore>().all()
}

/// Turns the dump catcher on or off; PHP restarts with the web servers to pick it up.
#[tauri::command]
pub(crate) async fn set_catch_dumps(app: AppHandle, state: State<'_, AppState>, enabled: bool) -> Result<()> {
    let _op = state.ops.lock().await;
    state.update_settings(|s| s.catch_dumps = enabled).await?;
    logs::system(&app, logs::Level::Info, if enabled { "Catching dump() and dd() in Logs" } else { "dump() and dd() show in the page again" });
    restart_web_servers(&app, &state).await
}

/// Keeps the dump listener in step with the settings: listening on `ports.dumps` while
/// "Catch dumps" is on, closed otherwise.
pub(crate) fn spawn_dump_listener(handle: &AppHandle) {
    let handle = handle.clone();
    tauri::async_runtime::spawn(async move {
        let mut current: Option<(u16, tauri::async_runtime::JoinHandle<()>)> = None;
        loop {
            let (settings, layout) = handle.state::<AppState>().snapshot().await;
            let wanted = settings.catch_dumps.then_some(settings.ports.dumps);
            let finished = current.as_ref().is_some_and(|(_, task)| task.inner().is_finished());
            if current.as_ref().map(|(p, _)| *p) != wanted || finished {
                if let Some((_, task)) = current.take() {
                    task.abort();
                }
                if let Some(port) = wanted {
                    let (app, www) = (handle.clone(), layout.www());
                    let task = tauri::async_runtime::spawn(async move {
                        if let Err(e) = dumps::serve(app.clone(), port, www).await {
                            logs::system(&app, logs::Level::Warn, format!("Can't catch dumps on port {port}: {e}. Change the dumps port in Settings → Ports."));
                        }
                    });
                    current = Some((port, task));
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(if finished { 30 } else { 2 })).await;
        }
    });
}

/// Empties the log files of one source ("apache", "dumps", "project:shop"), or all of them.
#[tauri::command]
pub(crate) async fn clear_logs(app: AppHandle, state: State<'_, AppState>, source: Option<String>) -> Result<()> {
    let (_, layout) = state.snapshot().await;
    let failed = logs::clear(&app, &layout.logs(), source.as_deref());
    if source.is_none() || source.as_deref() == Some("dumps") {
        app.state::<dumps::DumpStore>().clear();
    }
    logs::system(&app, logs::Level::Info, match &source {
        Some(s) => format!("Logs of {} cleared", s.strip_prefix("project:").unwrap_or(s)),
        None => "Logs cleared".to_string(),
    });
    if failed.is_empty() {
        Ok(())
    } else {
        Err(Error::msg(format!("Could not clear {} (in use by a running service?)", failed.join(", "))))
    }
}
