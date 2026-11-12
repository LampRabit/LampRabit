//! One-click setup of LampRabit's MCP server in the AI assistants people use: LampRabit
//! adds (or removes) a "lamprabit" entry in each one's MCP config, so nobody edits JSON or
//! opens a terminal. The entry runs LampRabit itself with `--cli mcp` (stdio), which every
//! client supports and which works whether the app is open or not (it starts in the tray).
//! Everything else in those files is kept, in its order, and the file is backed up first.

use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::{json, Map, Value};

use crate::error::{Error, Result};

const ENTRY: &str = "lamprabit";

#[derive(Debug, Clone, Copy, PartialEq)]
enum Shape {
    /// { "mcpServers": { name: { command, args } } }: Claude Desktop, Claude Code, Cursor, Windsurf.
    McpServers,
    /// { "servers": { name: { type: "stdio", command, args } } }: VS Code.
    Servers,
}

struct Client {
    id: &'static str,
    name: &'static str,
    shape: Shape,
    /// The config file; the client counts as installed when its folder exists.
    file: fn() -> Option<PathBuf>,
}

fn clients() -> Vec<Client> {
    vec![
        Client { id: "claude-desktop", name: "Claude Desktop", shape: Shape::McpServers, file: || dirs::config_dir().map(|d| d.join("Claude").join("claude_desktop_config.json")) },
        Client { id: "claude-code", name: "Claude Code", shape: Shape::McpServers, file: || dirs::home_dir().map(|d| d.join(".claude.json")) },
        Client { id: "cursor", name: "Cursor", shape: Shape::McpServers, file: || dirs::home_dir().map(|d| d.join(".cursor").join("mcp.json")) },
        Client { id: "vscode", name: "VS Code", shape: Shape::Servers, file: || dirs::config_dir().map(|d| d.join("Code").join("User").join("mcp.json")) },
        Client { id: "windsurf", name: "Windsurf", shape: Shape::McpServers, file: || dirs::home_dir().map(|d| d.join(".codeium").join("windsurf").join("mcp_config.json")) },
    ]
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientStatus {
    pub id: &'static str,
    pub name: &'static str,
    /// Its config folder exists (Claude Code: its config file).
    pub installed: bool,
    /// LampRabit's entry is in its config.
    pub connected: bool,
    pub path: Option<PathBuf>,
}

fn present(c: &Client, path: &Path) -> bool {
    if c.id == "claude-code" { path.is_file() } else { path.parent().is_some_and(Path::is_dir) }
}

fn read(path: &Path) -> Result<Value> {
    match std::fs::read_to_string(path) {
        Ok(text) if !text.trim().is_empty() => serde_json::from_str(&text).map_err(|e| Error::msg(format!("{} isn't valid JSON ({e}); fix or remove it first.", path.display()))),
        _ => Ok(json!({})),
    }
}

fn servers<'a>(config: &'a mut Value, shape: Shape) -> Result<&'a mut Map<String, Value>> {
    let key = if shape == Shape::Servers { "servers" } else { "mcpServers" };
    let root = config.as_object_mut().ok_or_else(|| Error::msg("The config isn't a JSON object."))?;
    root.entry(key).or_insert_with(|| json!({})).as_object_mut().ok_or_else(|| Error::msg(format!("\"{key}\" in the config isn't an object.")))
}

pub fn status() -> Vec<ClientStatus> {
    clients()
        .into_iter()
        .map(|c| {
            let path = (c.file)();
            let installed = path.as_deref().is_some_and(|p| present(&c, p));
            let connected = path.as_deref().and_then(|p| read(p).ok()).is_some_and(|mut v| servers(&mut v, c.shape).is_ok_and(|s| s.contains_key(ENTRY)));
            ClientStatus { id: c.id, name: c.name, installed, connected, path }
        })
        .collect()
}

/// What the entry runs: LampRabit itself, as `lamprabit mcp`.
fn entry(shape: Shape) -> Value {
    let exe = std::env::current_exe().map(|p| p.to_string_lossy().into_owned()).unwrap_or_else(|_| "lamprabit".into());
    match shape {
        Shape::Servers => json!({"type": "stdio", "command": exe, "args": ["--cli", "mcp"]}),
        Shape::McpServers => json!({"command": exe, "args": ["--cli", "mcp"]}),
    }
}

/// Adds (or, with `on` false, removes) LampRabit's entry in a client's MCP config.
pub fn set(id: &str, on: bool) -> Result<String> {
    let client = clients().into_iter().find(|c| c.id == id).ok_or_else(|| Error::msg(format!("Unknown assistant {id}.")))?;
    let path = (client.file)().ok_or_else(|| Error::msg("Couldn't find your home folder."))?;
    if on && !present(&client, &path) {
        return Err(Error::msg(format!("{} doesn't seem to be installed (no {}). Install it, open it once, then connect.", client.name, path.parent().unwrap_or(&path).display())));
    }
    let mut config = read(&path)?;
    if path.is_file() {
        // One backup, of the file as it was before LampRabit first touched it.
        let backup = path.with_extension("json.lamprabit-backup");
        if !backup.exists() {
            std::fs::copy(&path, &backup)?;
        }
    }
    let list = servers(&mut config, client.shape)?;
    if on {
        list.insert(ENTRY.to_string(), entry(client.shape));
    } else {
        list.remove(ENTRY);
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // Written beside it, then moved over it, so a crash never leaves half a file.
    let temp = path.with_extension("json.lamprabit-tmp");
    std::fs::write(&temp, serde_json::to_string_pretty(&config)? + "\n")?;
    std::fs::rename(&temp, &path)?;
    Ok(if on { format!("{} can use LampRabit now; restart {} if it was open.", client.name, client.name) } else { format!("Removed LampRabit from {}.", client.name) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_other_entries_and_their_order() {
        let mut config: Value = serde_json::from_str(r#"{"theme":"dark","mcpServers":{"github":{"command":"gh"}},"zz":1}"#).unwrap();
        servers(&mut config, Shape::McpServers).unwrap().insert(ENTRY.into(), entry(Shape::McpServers));
        let text = serde_json::to_string(&config).unwrap();
        assert!(text.starts_with(r#"{"theme":"dark","mcpServers":{"github":{"command":"gh"},"lamprabit":"#), "{text}");
        assert!(text.ends_with(r#","zz":1}"#));
        let mut vs: Value = json!({});
        servers(&mut vs, Shape::Servers).unwrap().insert(ENTRY.into(), entry(Shape::Servers));
        assert_eq!(vs["servers"]["lamprabit"]["type"], "stdio");
        assert_eq!(vs["servers"]["lamprabit"]["args"], json!(["--cli", "mcp"]));
    }
}
