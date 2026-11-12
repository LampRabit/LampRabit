//! The running app's control channel, which the `lamprabit` command line and its MCP server
//! use. The app listens on 127.0.0.1 only, on a free port, and writes the port and a random
//! token to `control.json` in its config folder (readable by the user only); a request
//! without the token is refused. Each connection carries one request, a JSON line
//! `{"token","cmd","args"}`, answered by one JSON line `{"ok":true,"data":…}` or
//! `{"ok":false,"error":"…"}`. `logs.follow` instead streams `{"line":…}` lines until the
//! client hangs up.

use std::{future::Future, path::PathBuf, pin::Pin, time::Duration};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::{AppHandle, Listener, Manager};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

use crate::error::{Error, Result};

/// The app's identifier (tauri.conf.json): its config folder's name.
pub const IDENTIFIER: &str = "dev.lamprabit.desktop";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Endpoint {
    pub port: u16,
    pub token: String,
    pub pid: u32,
    pub version: String,
}

#[derive(Deserialize)]
struct Request {
    token: String,
    cmd: String,
    #[serde(default)]
    args: Value,
}

pub type Dispatch = fn(AppHandle, String, Value) -> Pin<Box<dyn Future<Output = Result<Value>> + Send>>;

/// Where the endpoint is written: the same folder Tauri's app_config_dir() gives the app, or
/// LAMPRABIT_CONTROL_FILE (the smoke test's own, so it never touches a real install's).
pub fn endpoint_path() -> Option<PathBuf> {
    if let Some(file) = std::env::var_os("LAMPRABIT_CONTROL_FILE") {
        return Some(PathBuf::from(file));
    }
    dirs::config_dir().map(|d| d.join(IDENTIFIER).join("control.json"))
}

/// Starts listening, and writes control.json.
pub async fn serve(app: AppHandle, dispatch: Dispatch) -> Result<()> {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
    let port = listener.local_addr()?.port();
    let token: String = (0..2).map(|_| format!("{:016x}", rand_u64())).collect();
    let endpoint = Endpoint { port, token: token.clone(), pid: std::process::id(), version: app.package_info().version.to_string() };
    let path = match std::env::var_os("LAMPRABIT_CONTROL_FILE") {
        Some(file) => PathBuf::from(file),
        None => app.path().app_config_dir().map_err(|e| Error::msg(e.to_string()))?.join("control.json"),
    };
    std::fs::create_dir_all(path.parent().unwrap())?;
    std::fs::write(&path, serde_json::to_vec(&endpoint).unwrap_or_default())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    tauri::async_runtime::spawn(async move {
        loop {
            let Ok((stream, peer)) = listener.accept().await else { continue };
            if !peer.ip().is_loopback() {
                continue;
            }
            let (app, token) = (app.clone(), token.clone());
            tauri::async_runtime::spawn(async move {
                let _ = handle(app, stream, &token, dispatch).await;
            });
        }
    });
    Ok(())
}

/// A random number from the OS's hasher seeds, which are random per process; enough for a
/// token that only guards a loopback port in the user's own folder.
fn rand_u64() -> u64 {
    use std::hash::{BuildHasher, Hasher};
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    h.write_u128(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos());
    h.finish()
}

async fn handle(app: AppHandle, stream: tokio::net::TcpStream, token: &str, dispatch: Dispatch) -> std::io::Result<()> {
    let (read, mut write) = stream.into_split();
    let mut line = String::new();
    // One request line, of reasonable size, within a few seconds.
    let mut reader = BufReader::new(read.take(1 << 20));
    if tokio::time::timeout(Duration::from_secs(10), reader.read_line(&mut line)).await.is_err() {
        return Ok(());
    }
    let reply = |v: Value| format!("{v}\n");
    let request: Request = match serde_json::from_str(&line) {
        Ok(r) => r,
        Err(_) => return write.write_all(reply(json!({"ok": false, "error": "Not a LampRabit request."})).as_bytes()).await,
    };
    if request.token != token {
        return write.write_all(reply(json!({"ok": false, "error": "Wrong token: control.json changed; run the command again."})).as_bytes()).await;
    }
    if request.cmd == "logs.follow" {
        return follow_logs(app, write).await;
    }
    let answer = match dispatch(app, request.cmd, request.args).await {
        Ok(data) => json!({"ok": true, "data": data}),
        Err(e) => json!({"ok": false, "error": e.to_string()}),
    };
    write.write_all(reply(answer).as_bytes()).await
}

/// Every new log line, as it's written, until the client goes away.
async fn follow_logs(app: AppHandle, mut write: tokio::net::tcp::OwnedWriteHalf) -> std::io::Result<()> {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let id = app.listen("log-line", move |event| {
        let _ = tx.send(event.payload().to_string());
    });
    let result = async {
        write.write_all(b"{\"ok\":true,\"data\":\"following\"}\n").await?;
        while let Some(payload) = rx.recv().await {
            write.write_all(format!("{{\"line\":{payload}}}\n").as_bytes()).await?;
        }
        Ok(())
    }
    .await;
    app.unlisten(id);
    result
}

// ---- The client side, for the command line and MCP. ----

fn read_endpoint() -> Option<Endpoint> {
    serde_json::from_slice(&std::fs::read(endpoint_path()?).ok()?).ok()
}

async fn connect(endpoint: &Endpoint) -> Option<tokio::net::TcpStream> {
    tokio::time::timeout(Duration::from_secs(2), tokio::net::TcpStream::connect(("127.0.0.1", endpoint.port))).await.ok()?.ok()
}

/// The running app's endpoint; with `launch`, starts LampRabit in the background (in the
/// tray, no window) when it isn't running, and waits for it.
pub async fn reach(launch: bool) -> Result<Endpoint> {
    if let Some(e) = read_endpoint() {
        if connect(&e).await.is_some() {
            return Ok(e);
        }
    }
    if !launch {
        return Err(Error::msg("LampRabit isn't running. Start it, or drop --no-launch to have the command start it."));
    }
    let exe = std::env::current_exe()?;
    let mut cmd = std::process::Command::new(exe);
    cmd.arg("--minimized").stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // Detached, so it outlives this command and its console.
        cmd.creation_flags(0x0000_0008 | 0x0000_0200);
    }
    cmd.spawn().map_err(|e| Error::msg(format!("Couldn't start LampRabit: {e}")))?;
    for _ in 0..60 {
        tokio::time::sleep(Duration::from_millis(500)).await;
        if let Some(e) = read_endpoint() {
            if connect(&e).await.is_some() {
                return Ok(e);
            }
        }
    }
    Err(Error::msg("LampRabit didn't come up within 30 seconds. Open it once to check it starts."))
}

/// Sends one request and returns its data, or the app's error.
pub async fn call(endpoint: &Endpoint, cmd: &str, args: Value) -> Result<Value> {
    let mut stream = connect(endpoint).await.ok_or_else(|| Error::msg("LampRabit stopped answering."))?;
    let request = json!({"token": endpoint.token, "cmd": cmd, "args": args});
    stream.write_all(format!("{request}\n").as_bytes()).await?;
    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line).await?;
    let reply: Value = serde_json::from_str(&line).map_err(|_| Error::msg("LampRabit sent an answer this command can't read (a different version?)."))?;
    if reply["ok"] == true {
        Ok(reply["data"].clone())
    } else {
        Err(Error::msg(reply["error"].as_str().unwrap_or("The request failed.").to_string()))
    }
}

/// Streams new log lines to `each` until it returns false or the app goes away.
pub async fn follow(endpoint: &Endpoint, mut each: impl FnMut(Value) -> bool) -> Result<()> {
    let mut stream = connect(endpoint).await.ok_or_else(|| Error::msg("LampRabit stopped answering."))?;
    stream.write_all(format!("{}\n", json!({"token": endpoint.token, "cmd": "logs.follow"})).as_bytes()).await?;
    let mut lines = BufReader::new(stream).lines();
    while let Some(line) = lines.next_line().await? {
        let Ok(v) = serde_json::from_str::<Value>(&line) else { continue };
        if v["ok"] == false {
            return Err(Error::msg(v["error"].as_str().unwrap_or("The request failed.").to_string()));
        }
        if !v["line"].is_null() && !each(v["line"].clone()) {
            break;
        }
    }
    Ok(())
}
