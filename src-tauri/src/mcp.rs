//! `lamprabit mcp`: an MCP server on stdin/stdout (phase 21), so AI assistants can see and
//! drive the local stack: projects, services and their health, logs, the doctor, Git
//! status, runtimes. It's a thin layer over the app's control channel (control.rs), started
//! in the background when it isn't running.
//!
//! Tools that only read are annotated `readOnlyHint`; the ones that change something are not,
//! and stopping or fixing is also `destructiveHint`, so clients ask before calling them.
//! Settings aren't exposed at all, and nothing here returns a password or token: services
//! have none locally, and Git remotes come back with credentials stripped.

use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::control;

const PROTOCOL: &str = "2025-06-18";

struct Tool {
    name: &'static str,
    description: &'static str,
    /// The control request, and how the tool's arguments map onto it.
    cmd: &'static str,
    schema: Value,
    read_only: bool,
    destructive: bool,
}

fn tools() -> Vec<Tool> {
    let none = || json!({"type": "object", "properties": {}});
    let project = || json!({"type": "object", "properties": {"project": {"type": "string", "description": "The project's folder name in www (see list_projects)"}}, "required": ["project"]});
    let service = || json!({"type": "object", "properties": {"service": {"type": "string", "description": "A service id such as apache, nginx, mariadb, mysql, postgres, redis, mailpit (see list_services)"}}, "required": ["service"]});
    let t = |name, description, cmd, schema, read_only, destructive| Tool { name, description, cmd, schema, read_only, destructive };
    vec![
        t("status", "LampRabit's version, folder, the installed services with their state and port, and the number of projects.", "status", none(), true, false),
        t("list_services", "Every service LampRabit can run: installed or not, state (running, stopped, error), port, version, health and any problem (such as a port taken by another program).", "services", none(), true, false),
        t("list_projects", "The projects in www: name, local URL (https://<name>.test), folder, language, framework, PHP version, web server, Git branch, database name, whether its dev server runs.", "projects", none(), true, false),
        t(
            "read_logs",
            "Recent log lines from LampRabit's services and projects, oldest first. Filter by source (a service id like apache or mysql, 'php', 'system', or 'project:<name>') and by text.",
            "logs",
            json!({"type": "object", "properties": {
                "source": {"type": "string", "description": "A service id, php, system, or project:<name>"},
                "lines": {"type": "integer", "description": "How many of the newest lines (default 100, at most 1000)"},
                "contains": {"type": "string", "description": "Only lines containing this text (case-insensitive)"}
            }}),
            true,
            false,
        ),
        t("project_doctor", "What stands between a project and working (PHP version, extensions, dependencies, .env, database, domain, certificate), most serious first, each with an id and, when LampRabit can fix it, what the fix does.", "doctor", project(), true, false),
        t("git_status", "A project's Git branch, upstream with ahead/behind counts, changed files and remotes (credentials stripped).", "git.status", project(), true, false),
        t("list_runtimes", "Installed runtimes (PHP, Node via NVM, MariaDB, MySQL, PostgreSQL, Redis, Java…) with the version in use and every installed version.", "runtimes", none(), true, false),
        t("list_templates", "Project templates (Laravel, Symfony, WordPress, Vite, Next.js, Django…) with their ids, for create_project.", "templates", none(), true, false),
        t("start_service", "Start a service (and what it needs, like PHP for a web server).", "service.start", service(), false, false),
        t("stop_service", "Stop a service. Sites or projects using it stop working until it starts again.", "service.stop", service(), false, true),
        t("restart_service", "Restart a service, for example after changing its config.", "service.restart", service(), false, false),
        t("start_project", "Start a project's dev server (its serve command), routed to its .test domain.", "project.start", project(), false, false),
        t("stop_project", "Stop a project's dev server.", "project.stop", project(), false, true),
        t(
            "fix_project_issue",
            "Apply one project_doctor fix by its issue id (for example run composer install, start the database, switch PHP). It runs only while the problem is still there.",
            "doctor.fix",
            json!({"type": "object", "properties": {
                "project": {"type": "string"},
                "id": {"type": "string", "description": "The issue id from project_doctor"}
            }, "required": ["project", "id"]}),
            false,
            true,
        ),
        t(
            "create_project",
            "Create a project in www from a template (see list_templates); it's served at https://<name>.test. Templates that ship with LampRabit only run their own steps; a user template's risky steps are refused here.",
            "project.create",
            json!({"type": "object", "properties": {
                "name": {"type": "string", "description": "Folder name: letters, digits, dashes"},
                "template": {"type": "string", "description": "Template id"},
                "php": {"type": "string", "description": "PHP version for the site (default: the active one)"},
                "database": {"type": "string", "description": "Database to create for it (omit for none)"}
            }, "required": ["name", "template"]}),
            false,
            false,
        ),
        t(
            "install_runtime",
            "Download and use a runtime version (checksum checked), such as php 8.4, mariadb, redis. Without a version, the LTS or newest.",
            "runtime.install",
            json!({"type": "object", "properties": {
                "runtime": {"type": "string", "description": "Runtime id (see list_runtimes)"},
                "version": {"type": "string", "description": "A version, or a line like 8.4"}
            }, "required": ["runtime"]}),
            false,
            false,
        ),
    ]
}

/// The control request for a tool call: the tool's argument names mapped onto the request's.
fn request(tool: &Tool, args: &Value) -> Value {
    match tool.cmd {
        "service.start" | "service.stop" | "service.restart" => json!({"name": args["service"]}),
        "doctor" | "git.status" | "project.start" | "project.stop" => json!({"name": args["project"]}),
        "doctor.fix" => json!({"name": args["project"], "id": args["id"]}),
        "runtime.install" => json!({"name": args["runtime"], "version": args["version"]}),
        "logs" => json!({"source": args["source"], "lines": args["lines"].as_u64().unwrap_or(100).min(1000)}),
        "project.create" => json!({
            "name": args["name"], "template": args["template"], "php": args["php"], "server": null, "ssl": true,
            "database": args["database"], "variables": {}, "optional": {},
        }),
        _ => json!({}),
    }
}

async fn call_tool(launch: bool, name: &str, args: &Value) -> Value {
    let Some(tool) = tools().into_iter().find(|t| t.name == name) else {
        return json!({"content": [{"type": "text", "text": format!("Unknown tool {name}.")}], "isError": true});
    };
    let result = async {
        let endpoint = control::reach(launch).await?;
        control::call(&endpoint, tool.cmd, request(&tool, args)).await
    }
    .await;
    match result {
        Ok(mut data) => {
            if tool.cmd == "logs" {
                if let Some(needle) = args["contains"].as_str().map(str::to_lowercase) {
                    if let Some(lines) = data.as_array_mut() {
                        lines.retain(|l| l["line"].as_str().is_some_and(|t| t.to_lowercase().contains(&needle)));
                    }
                }
            }
            let text = if data.is_null() { "Done.".to_string() } else { serde_json::to_string_pretty(&data).unwrap_or_default() };
            json!({"content": [{"type": "text", "text": text}], "structuredContent": if data.is_object() { data } else { json!({"result": data}) }})
        }
        Err(e) => json!({"content": [{"type": "text", "text": e.to_string()}], "isError": true}),
    }
}

/// Answers one JSON-RPC message; None for notifications.
pub(crate) async fn answer(launch: bool, message: &Value) -> Option<Value> {
    let id = message.get("id").cloned();
    let method = message["method"].as_str().unwrap_or("");
    let result = match method {
        "initialize" => Ok(json!({
            "protocolVersion": message["params"]["protocolVersion"].as_str().unwrap_or(PROTOCOL),
            "capabilities": {"tools": {"listChanged": false}},
            "serverInfo": {"name": "lamprabit", "title": "LampRabit", "version": env!("CARGO_PKG_VERSION")},
            "instructions": "LampRabit runs a local web development stack: web servers, PHP and other languages, databases, caches, and projects served at https://<name>.test. Start with status or list_projects; project_doctor explains what's wrong with a project and how to fix it."
        })),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({"tools": tools().iter().map(|t| json!({
            "name": t.name,
            "description": t.description,
            "inputSchema": t.schema,
            "annotations": {"readOnlyHint": t.read_only, "destructiveHint": t.destructive, "openWorldHint": false},
        })).collect::<Vec<_>>()})),
        "tools/call" => Ok(call_tool(launch, message["params"]["name"].as_str().unwrap_or(""), &message["params"]["arguments"]).await),
        _ if id.is_none() => return None, // notifications (initialized, cancelled…) need no answer
        _ => Err(json!({"code": -32601, "message": format!("Method not found: {method}")})),
    };
    let id = id?;
    Some(match result {
        Ok(r) => json!({"jsonrpc": "2.0", "id": id, "result": r}),
        Err(e) => json!({"jsonrpc": "2.0", "id": id, "error": e}),
    })
}

/// Serves MCP over stdio until stdin closes; returns the exit code.
pub async fn serve(launch: bool) -> i32 {
    let mut lines = BufReader::new(tokio::io::stdin()).lines();
    let mut out = tokio::io::stdout();
    while let Ok(Some(line)) = lines.next_line().await {
        if line.trim().is_empty() {
            continue;
        }
        let reply = match serde_json::from_str::<Value>(&line) {
            Ok(message) => answer(launch, &message).await,
            Err(_) => Some(json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32700, "message": "Parse error"}})),
        };
        if let Some(reply) = reply {
            if out.write_all(format!("{reply}\n").as_bytes()).await.is_err() || out.flush().await.is_err() {
                break;
            }
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn speaks_mcp() {
        let init = answer(false, &json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-06-18"}})).await.unwrap();
        assert_eq!(init["result"]["protocolVersion"], "2025-06-18");
        assert_eq!(init["result"]["serverInfo"]["name"], "lamprabit");
        assert!(answer(false, &json!({"jsonrpc": "2.0", "method": "notifications/initialized"})).await.is_none());
        let list = answer(false, &json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"})).await.unwrap();
        let tools = list["result"]["tools"].as_array().unwrap();
        let stop = tools.iter().find(|t| t["name"] == "stop_service").unwrap();
        assert_eq!(stop["annotations"]["readOnlyHint"], false);
        assert_eq!(stop["annotations"]["destructiveHint"], true);
        assert!(tools.iter().find(|t| t["name"] == "read_logs").unwrap()["annotations"]["readOnlyHint"] == true);
        // No tool exposes the settings.
        assert!(tools.iter().all(|t| t["name"] != "config"));
        let unknown = answer(false, &json!({"jsonrpc": "2.0", "id": 3, "method": "resources/list"})).await.unwrap();
        assert_eq!(unknown["error"]["code"], -32601);
    }

    /// The HTTP transport, over a real socket: answers with the token, refuses without it or
    /// from a web page, and takes notifications with 202.
    #[tokio::test]
    async fn serves_over_http() {
        use tokio::io::AsyncReadExt;
        let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
        start_http(port, "secret-token".into()).await.unwrap();
        async fn post(port: u16, body: &str, auth: Option<&str>, origin: Option<&str>) -> (u16, String) {
            let mut s = tokio::net::TcpStream::connect(("127.0.0.1", port)).await.unwrap();
            let crlf = "\r\n";
            let mut req = format!("POST /mcp HTTP/1.1{crlf}Host: 127.0.0.1{crlf}Content-Type: application/json{crlf}Content-Length: {}{crlf}", body.len());
            if let Some(a) = auth {
                req.push_str(&format!("Authorization: Bearer {a}{crlf}"));
            }
            if let Some(o) = origin {
                req.push_str(&format!("Origin: {o}{crlf}"));
            }
            req.push_str(crlf);
            req.push_str(body);
            s.write_all(req.as_bytes()).await.unwrap();
            let mut out = String::new();
            s.read_to_string(&mut out).await.unwrap();
            let status = out.split_whitespace().nth(1).and_then(|c| c.parse().ok()).unwrap_or(0);
            let body = out.split("\r\n\r\n").nth(1).unwrap_or_default().to_string();
            (status, body)
        }
        let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18"}}"#;
        let (status, body) = post(port, init, Some("secret-token"), None).await;
        assert_eq!(status, 200);
        assert!(body.contains("\"lamprabit\""), "{body}");
        let (status, body) = post(port, r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#, Some("secret-token"), Some("http://localhost:5173")).await;
        assert_eq!(status, 200);
        assert!(body.contains("read_logs"));
        assert_eq!(post(port, r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#, Some("secret-token"), None).await.0, 202);
        assert_eq!(post(port, init, None, None).await.0, 401);
        assert_eq!(post(port, init, Some("wrong"), None).await.0, 401);
        assert_eq!(post(port, init, Some("secret-token"), Some("https://evil.example")).await.0, 403);
        // A browser gets a page saying what this is, with nothing private on it.
        let mut s = tokio::net::TcpStream::connect(("127.0.0.1", port)).await.unwrap();
        s.write_all(b"GET /mcp HTTP/1.1\r\nHost: 127.0.0.1\r\nAccept: text/html,*/*\r\n\r\n").await.unwrap();
        let mut page = String::new();
        s.read_to_string(&mut page).await.unwrap();
        assert!(page.starts_with("HTTP/1.1 200") && page.contains("Settings → AI assistants") && !page.contains("secret-token"));
        assert!(http_running());
        stop_http();
    }

    #[test]
    fn maps_tool_arguments() {
        let tools = tools();
        let fix = tools.iter().find(|t| t.name == "fix_project_issue").unwrap();
        assert_eq!(request(fix, &json!({"project": "shop", "id": "env-missing"})), json!({"name": "shop", "id": "env-missing"}));
        let logs = tools.iter().find(|t| t.name == "read_logs").unwrap();
        assert_eq!(request(logs, &json!({"lines": 5000}))["lines"], 1000);
    }
}

// ---- The same server over HTTP, run by the app itself (Settings → AI assistants). ----

/// The running HTTP server, so it can be stopped from Settings.
static HTTP: std::sync::Mutex<Option<tauri::async_runtime::JoinHandle<()>>> = std::sync::Mutex::new(None);

pub fn http_running() -> bool {
    HTTP.lock().ok().is_some_and(|h| h.as_ref().is_some_and(|t| !t.inner().is_finished()))
}

pub fn stop_http() {
    if let Some(task) = HTTP.lock().ok().and_then(|mut h| h.take()) {
        task.abort();
    }
}

/// MCP's Streamable HTTP transport, answering with plain JSON: POST /mcp with a JSON-RPC
/// message, the token as a Bearer header. Only 127.0.0.1, and a browser page from another
/// site is refused (its Origin header), so a web page can't reach it by rebinding a domain.
pub async fn start_http(port: u16, token: String) -> crate::error::Result<()> {
    stop_http();
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port))
        .await
        .map_err(|e| crate::error::Error::msg(format!("Port {port} is taken ({e}); pick another for the MCP server.")))?;
    let task = tauri::async_runtime::spawn(async move {
        loop {
            let Ok((stream, peer)) = listener.accept().await else { continue };
            if !peer.ip().is_loopback() {
                continue;
            }
            let token = token.clone();
            tauri::async_runtime::spawn(async move {
                let _ = http_request(stream, &token).await;
            });
        }
    });
    if let Ok(mut h) = HTTP.lock() {
        *h = Some(task);
    }
    Ok(())
}

async fn http_request(mut stream: tokio::net::TcpStream, token: &str) -> std::io::Result<()> {
    use tokio::io::AsyncReadExt;
    // Headers, then a body of Content-Length bytes; both bounded.
    let mut buf = Vec::new();
    let mut chunk = [0u8; 8192];
    let head_end = loop {
        let n = tokio::time::timeout(std::time::Duration::from_secs(10), stream.read(&mut chunk)).await.map_err(|_| std::io::ErrorKind::TimedOut)??;
        if n == 0 {
            return Ok(());
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break i + 4;
        }
        if buf.len() > 64 * 1024 {
            return respond(&mut stream, 431, "", "").await;
        }
    };
    let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
    let mut lines = head.lines();
    let request_line = lines.next().unwrap_or_default();
    let mut parts = request_line.split_whitespace();
    let (method, path) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
    let header = |name: &str| lines.clone().find_map(|l| l.split_once(':').filter(|(k, _)| k.trim().eq_ignore_ascii_case(name)).map(|(_, v)| v.trim().to_string()));
    // Someone opened the address in a browser: say what it is instead of a bare refusal.
    // The page holds nothing private; assistants still need the token.
    if method == "GET" && header("accept").is_some_and(|a| a.contains("text/html")) {
        return respond(&mut stream, 200, "text/html; charset=utf-8", BROWSER_PAGE).await;
    }
    if path.split('?').next() != Some("/mcp") {
        return respond(&mut stream, 404, "text/plain", "LampRabit's MCP server is at /mcp").await;
    }
    if let Some(origin) = header("origin") {
        let local = ["http://localhost", "http://127.0.0.1", "https://localhost", "https://127.0.0.1"].iter().any(|o| origin == *o || origin.starts_with(&format!("{o}:")));
        if !local {
            return respond(&mut stream, 403, "text/plain", "Requests from web pages aren't allowed").await;
        }
    }
    if header("authorization").as_deref() != Some(&format!("Bearer {token}")) {
        return respond(&mut stream, 401, "text/plain", "Missing or wrong token: copy the config from LampRabit's Settings").await;
    }
    if method != "POST" {
        // No server-initiated stream: everything is a reply to a POST.
        return respond(&mut stream, 405, "text/plain", "Use POST").await;
    }
    let length: usize = header("content-length").and_then(|v| v.parse().ok()).unwrap_or(0);
    if length > 4 * 1024 * 1024 {
        return respond(&mut stream, 413, "", "").await;
    }
    let mut body = buf[head_end..].to_vec();
    while body.len() < length {
        let n = stream.read(&mut chunk).await?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..n]);
    }
    let Ok(message) = serde_json::from_slice::<Value>(&body) else {
        let error = json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32700, "message": "Parse error"}});
        return respond(&mut stream, 400, "application/json", &error.to_string()).await;
    };
    // The app is running (it's serving this), so tools never launch it.
    match answer(false, &message).await {
        Some(reply) => respond(&mut stream, 200, "application/json", &reply.to_string()).await,
        None => respond(&mut stream, 202, "", "").await,
    }
}

async fn respond(stream: &mut tokio::net::TcpStream, status: u16, kind: &str, body: &str) -> std::io::Result<()> {
    let reason = match status {
        200 => "OK",
        202 => "Accepted",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        413 => "Payload Too Large",
        _ => "Request Header Fields Too Large",
    };
    let kind = if kind.is_empty() { String::new() } else { format!("Content-Type: {kind}\r\n") };
    let allow = if status == 405 { "Allow: POST\r\n" } else { "" };
    let response = format!("HTTP/1.1 {status} {reason}\r\n{kind}{allow}Content-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
    stream.write_all(response.as_bytes()).await?;
    stream.flush().await
}

/// What a browser sees at the MCP address: it's for AI assistants, and how to connect one.
const BROWSER_PAGE: &str = r#"<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>LampRabit MCP server</title>
<style>:root{color-scheme:light dark}body{font:15px/1.55 system-ui,sans-serif;max-width:40rem;margin:4rem auto;padding:0 1.25rem;color:#1b2028;background:#f7f8fa}@media(prefers-color-scheme:dark){body{color:#e7ecf1;background:#13171d}code{background:#232a34}}h1{font-size:1.4rem;margin:0 0 .25rem}.ok{color:#178a5f;font-weight:600}code{background:#eceff3;padding:.1rem .35rem;border-radius:.3rem}li{margin:.35rem 0}</style></head>
<body><h1>LampRabit's MCP server</h1><p class="ok">● Running</p>
<p>This address is for AI assistants (Claude, Cursor, VS Code…), not for a browser: they connect here with a token, and LampRabit refuses requests without it.</p>
<p>To connect an assistant, open LampRabit → <b>Settings → AI assistants</b> and either:</p>
<ul><li>press <b>Connect</b> next to your assistant, or</li><li>copy the <code>claude mcp add …</code> command or the JSON config shown under <b>Run the MCP server in LampRabit</b>; they include the token.</li></ul>
</body></html>"#;
