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

    #[test]
    fn maps_tool_arguments() {
        let tools = tools();
        let fix = tools.iter().find(|t| t.name == "fix_project_issue").unwrap();
        assert_eq!(request(fix, &json!({"project": "shop", "id": "env-missing"})), json!({"name": "shop", "id": "env-missing"}));
        let logs = tools.iter().find(|t| t.name == "read_logs").unwrap();
        assert_eq!(request(logs, &json!({"lines": 5000}))["lines"], 1000);
    }
}
