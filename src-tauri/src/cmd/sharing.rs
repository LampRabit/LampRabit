//! Public tunnels (ngrok, Cloudflare).

use crate::*;

/// Shares a site (by folder name) or a local port on a public URL.
#[tauri::command]
pub(crate) async fn tunnel_start(
    app: AppHandle,
    state: State<'_, AppState>,
    provider: tunnels::Provider,
    site: Option<String>,
    port: Option<u16>,
) -> Result<tunnels::Tunnel> {
    let (settings, layout) = state.snapshot().await;
    let site = match site {
        Some(name) => Some(
            sites::served(&settings, &layout)
                .into_iter()
                .find(|s| s.name == name)
                .ok_or_else(|| Error::msg(format!("No site called {name}")))?,
        ),
        None => None,
    };
    if let Some(s) = &site {
        let kind = server_kind(s.server);
        if !state.supervisor.is_running(kind).await {
            return Err(Error::msg(format!("Start {} first; the tunnel forwards to it.", kind.label())));
        }
    }
    state.tunnels.start(&app, &settings, &layout, provider, site.as_ref(), port).await
}

#[tauri::command]
pub(crate) async fn tunnel_stop(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<()> {
    state.tunnels.stop(&app, &id).await;
    Ok(())
}

#[tauri::command]
pub(crate) async fn tunnel_list(state: State<'_, AppState>) -> Result<Vec<tunnels::Tunnel>> {
    Ok(state.tunnels.list().await)
}

/// Whether Cloudflare is connected (`cloudflared tunnel login` done) for sharing on your domain.
#[tauri::command]
pub(crate) fn cloudflare_connected() -> bool {
    tunnels::cloudflare_connected()
}

/// Connects Cloudflare through the browser; returns once the sign-in is finished.
#[tauri::command]
pub(crate) async fn cloudflare_login(app: AppHandle, state: State<'_, AppState>) -> Result<()> {
    let (settings, layout) = state.snapshot().await;
    tunnels::cloudflare_login(&app, &settings, &layout).await
}

#[tauri::command]
pub(crate) fn ngrok_token_saved() -> bool {
    tunnels::ngrok_token().is_some()
}

/// Saves the Ngrok authtoken in the system keychain (None removes it).
#[tauri::command]
pub(crate) fn set_ngrok_token(app: AppHandle, token: Option<String>) -> Result<()> {
    tunnels::set_ngrok_token(token.as_deref())?;
    logs::system(&app, logs::Level::Info, if token.is_some() { "Ngrok authtoken saved to the keychain" } else { "Ngrok authtoken removed" });
    Ok(())
}
