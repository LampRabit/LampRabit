//! One-click setup of LampRabit's MCP server in the AI assistants people use: LampRabit
//! adds (or removes) a "lamprabit" entry in each one's MCP config, so nobody edits JSON or
//! opens a terminal. The entry runs LampRabit itself with `--cli mcp` (stdio), which every
//! client supports and which works whether the app is open or not (it starts in the tray).
//! Everything else in those files is kept, in its order, and the file is backed up first.
//! Apps that can't run a local MCP server are listed too, with the reason, so people aren't
//! left wondering why theirs is missing.

use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::{json, Map, Value};

use crate::error::{Error, Result};

const ENTRY: &str = "lamprabit";
const ARGS: [&str; 2] = ["--cli", "mcp"];

#[derive(Debug, Clone, Copy, PartialEq)]
enum Shape {
    /// { "mcpServers": { name: { command, args } } }: Claude, Cursor, Windsurf, LM Studio, Gemini…
    McpServers,
    /// { "servers": { name: { type: "stdio", command, args } } }: VS Code.
    Servers,
    /// { "mcpServers": { name: { type: "stdio", command, args, tools: ["*"] } } }: GitHub Copilot CLI.
    CopilotCli,
    /// { "mcp": { name: { type: "local", command: [program, args…], enabled } } }: opencode.
    OpenCode,
    /// `[mcp_servers.<name>]` with command and args in config.toml: Codex (app, CLI, IDE extension).
    CodexToml,
}

/// When an assistant counts as installed.
#[derive(Debug, Clone, Copy)]
enum Presence {
    /// Its config file exists (it shares a folder with something else).
    File,
    /// The folder that many levels above the config file exists (1: the file's own folder).
    Folder(usize),
}

struct Client {
    id: &'static str,
    name: &'static str,
    shape: Shape,
    presence: Presence,
    /// The config file.
    file: fn() -> Option<PathBuf>,
    /// Shown under the name: what else the entry covers.
    note: Option<&'static str>,
}

fn home(parts: &[&str]) -> Option<PathBuf> {
    dirs::home_dir().map(|d| parts.iter().fold(d, |p, s| p.join(s)))
}

fn config(parts: &[&str]) -> Option<PathBuf> {
    dirs::config_dir().map(|d| parts.iter().fold(d, |p, s| p.join(s)))
}

fn clients() -> Vec<Client> {
    use Presence::*;
    use Shape::*;
    let c = |id, name, shape, presence, file, note| Client { id, name, shape, presence, file, note };
    vec![
        c("claude-desktop", "Claude Desktop", McpServers, Folder(1), claude_desktop_file, None),
        c("claude-code", "Claude Code", McpServers, File, || home(&[".claude.json"]), None),
        c("codex", "Codex", CodexToml, Folder(1), || home(&[".codex", "config.toml"]), Some("The Codex app, CLI and IDE extension")),
        c("cursor", "Cursor", McpServers, Folder(1), || home(&[".cursor", "mcp.json"]), None),
        c("vscode", "VS Code", Servers, Folder(1), || config(&["Code", "User", "mcp.json"]), Some("Copilot's agent mode")),
        c("copilot-cli", "GitHub Copilot CLI", CopilotCli, Folder(1), || home(&[".copilot", "mcp-config.json"]), None),
        c("windsurf", "Windsurf", McpServers, Folder(1), || home(&[".codeium", "windsurf", "mcp_config.json"]), None),
        c("gemini-cli", "Gemini CLI", McpServers, File, || home(&[".gemini", "settings.json"]), None),
        c("antigravity", "Antigravity", McpServers, Folder(1), || home(&[".gemini", "antigravity", "mcp_config.json"]), None),
        c("lm-studio", "LM Studio", McpServers, Folder(1), lm_studio_file, Some("Local models, Ollama's included")),
        c("cline", "Cline", McpServers, Folder(2), || config(&["Code", "User", "globalStorage", "saoudrizwan.claude-dev", "settings", "cline_mcp_settings.json"]), Some("In VS Code")),
        c("roo-code", "Roo Code", McpServers, Folder(2), || config(&["Code", "User", "globalStorage", "rooveterinaryinc.roo-cline", "settings", "mcp_settings.json"]), Some("In VS Code")),
        c("kiro", "Kiro", McpServers, Folder(2), || home(&[".kiro", "settings", "mcp.json"]), None),
        c("opencode", "opencode", OpenCode, Folder(1), || home(&[".config", "opencode", "opencode.json"]), None),
    ]
}

/// Apps people ask about that can't start a local MCP server, and what to do instead.
const UNSUPPORTED: [(&str, &str, &str); 2] = [
    ("chatgpt", "ChatGPT", "ChatGPT's connectors reach only MCP servers on the internet, and LampRabit's stays on this computer. Codex, OpenAI's coding app, runs local ones: connect Codex."),
    ("ollama", "Ollama", "Ollama runs models but has no MCP client of its own. Use its models from an app that has one, such as LM Studio, Cline or opencode, and connect that app."),
];

/// Claude Desktop's config. The Microsoft Store (MSIX) build on Windows keeps it in
/// `%LOCALAPPDATA%\Claude-Data\<package family>\` and never reads the classic
/// `%APPDATA%\Claude\` file, so an entry written there silently doesn't load. When that
/// folder exists it wins (the one whose config changed last, if there are several).
fn claude_desktop_file() -> Option<PathBuf> {
    const NAME: &str = "claude_desktop_config.json";
    #[cfg(windows)]
    if let Some(data) = dirs::data_local_dir().map(|d| d.join("Claude-Data")) {
        let store = std::fs::read_dir(&data)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().to_ascii_lowercase().starts_with("anthropicpbc.claude_") && e.path().is_dir())
            .map(|e| e.path())
            .max_by_key(|dir| std::fs::metadata(dir.join(NAME)).and_then(|m| m.modified()).ok());
        if let Some(dir) = store {
            return Some(dir.join(NAME));
        }
    }
    config(&["Claude", NAME])
}

/// LM Studio's mcp.json is documented in ~/.lmstudio, but some macOS versions keep it in
/// ~/.cache/lm-studio; whichever folder exists is the one it reads.
fn lm_studio_file() -> Option<PathBuf> {
    let documented = home(&[".lmstudio", "mcp.json"])?;
    if cfg!(target_os = "macos") && !documented.parent().is_some_and(Path::is_dir) {
        if let Some(cache) = home(&[".cache", "lm-studio", "mcp.json"]).filter(|p| p.parent().is_some_and(Path::is_dir)) {
            return Some(cache);
        }
    }
    Some(documented)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientStatus {
    pub id: &'static str,
    pub name: &'static str,
    /// It seems installed (its config folder or file exists).
    pub installed: bool,
    /// LampRabit's entry is in its config.
    pub connected: bool,
    /// The entry runs another program than this LampRabit's (an old copy, a moved app).
    pub stale: bool,
    pub path: Option<PathBuf>,
    pub note: Option<&'static str>,
    /// Set when it can't run LampRabit's server at all: why, and what to use instead.
    pub unsupported: Option<&'static str>,
}

fn present(c: &Client, path: &Path) -> bool {
    match c.presence {
        Presence::File => path.is_file(),
        Presence::Folder(up) => path.ancestors().nth(up).is_some_and(Path::is_dir),
    }
}

/// A config file, as JSON or TOML depending on the client.
enum Doc {
    Json(Value),
    Toml(toml_edit::DocumentMut),
}

fn read(path: &Path, shape: Shape) -> Result<Doc> {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let invalid = |e: &dyn std::fmt::Display| Error::msg(format!("{} isn't valid ({e}); fix or remove it first.", path.display()));
    if shape == Shape::CodexToml {
        return text.parse::<toml_edit::DocumentMut>().map(Doc::Toml).map_err(|e| invalid(&e));
    }
    if text.trim().is_empty() {
        return Ok(Doc::Json(json!({})));
    }
    serde_json::from_str(&text).map(Doc::Json).map_err(|e| invalid(&e))
}

fn servers(config: &mut Value, shape: Shape) -> Result<&mut Map<String, Value>> {
    let key = match shape {
        Shape::Servers => "servers",
        Shape::OpenCode => "mcp",
        _ => "mcpServers",
    };
    let root = config.as_object_mut().ok_or_else(|| Error::msg("The config isn't a JSON object."))?;
    root.entry(key).or_insert_with(|| json!({})).as_object_mut().ok_or_else(|| Error::msg(format!("\"{key}\" in the config isn't an object.")))
}

fn has_entry(doc: &mut Doc, shape: Shape) -> bool {
    match doc {
        Doc::Json(v) => servers(v, shape).is_ok_and(|s| s.contains_key(ENTRY)),
        Doc::Toml(t) => t.get("mcp_servers").and_then(|s| s.get(ENTRY)).is_some(),
    }
}

pub fn status() -> Vec<ClientStatus> {
    let want = program();
    let mut list: Vec<ClientStatus> = clients()
        .into_iter()
        .map(|c| {
            let path = (c.file)();
            let installed = path.as_deref().is_some_and(|p| present(&c, p));
            let mut doc = if installed { path.as_deref().and_then(|p| read(p, c.shape).ok()) } else { None };
            let connected = doc.as_mut().is_some_and(|d| has_entry(d, c.shape));
            let stale = connected && doc.as_mut().and_then(|d| entry_command(d, c.shape)).is_some_and(|have| have != want);
            ClientStatus { id: c.id, name: c.name, installed, connected, stale, path, note: c.note, unsupported: None }
        })
        .collect();
    list.extend(UNSUPPORTED.iter().map(|&(id, name, why)| ClientStatus { id, name, installed: false, connected: false, stale: false, path: None, note: None, unsupported: Some(why) }));
    list
}

/// The app's identifier (tauri.conf.json), for its folder.
const IDENTIFIER: &str = "dev.lamprabit.desktop";

/// Where Windows keeps the copy of LampRabit that assistants run. An assistant keeps its MCP
/// server running for as long as it's open, and Windows won't replace a running program, so
/// pointing assistants at the app itself blocked updates (and rebuilds). The copy is
/// refreshed when LampRabit starts, set aside first when it's in use.
pub fn launcher_dir() -> Option<PathBuf> {
    cfg!(windows).then(|| dirs::data_local_dir().map(|d| d.join(IDENTIFIER).join("mcp"))).flatten()
}

fn launcher() -> Option<PathBuf> {
    launcher_dir().map(|d| d.join("lamprabit.exe"))
}

/// Run as the copy, the real app it was made from (written beside it), to start when
/// LampRabit isn't running; otherwise this program.
pub fn app_exe() -> std::io::Result<PathBuf> {
    let me = std::env::current_exe()?;
    if launcher().is_some_and(|l| same_file(&l, &me)) {
        if let Some(app) = launcher_dir().and_then(|d| std::fs::read_to_string(d.join("app.txt")).ok()).map(|t| PathBuf::from(t.trim())).filter(|p| p.is_file()) {
            return Ok(app);
        }
    }
    Ok(me)
}

fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

/// What assistants' entries run: the copy on Windows; the AppImage (not its mount, which
/// moves every launch) on Linux; the program itself elsewhere, since Unix replaces a running
/// program without trouble.
pub fn program() -> String {
    if let Some(l) = launcher() {
        return l.to_string_lossy().into_owned();
    }
    if let Some(image) = std::env::var_os("APPIMAGE").filter(|v| !v.is_empty()) {
        return PathBuf::from(image).to_string_lossy().into_owned();
    }
    std::env::current_exe().map(|p| p.to_string_lossy().into_owned()).unwrap_or_else(|_| "lamprabit".into())
}

/// Brings the copy up to date with this build (Windows), when LampRabit starts. Copies only
/// when the build changed; a copy that's running is renamed aside (Windows allows that) and
/// deleted on a later start, once nothing runs it.
pub fn refresh_launcher() -> std::io::Result<()> {
    let (Some(dir), Some(target)) = (launcher_dir(), launcher()) else { return Ok(()) };
    let me = std::env::current_exe()?;
    if same_file(&me, &target) {
        return Ok(());
    }
    std::fs::create_dir_all(&dir)?;
    for old in std::fs::read_dir(&dir)?.flatten().filter(|e| e.file_name().to_string_lossy().starts_with("lamprabit.exe.old-")) {
        let _ = std::fs::remove_file(old.path());
    }
    let stamp = |p: &Path| std::fs::metadata(p).ok().map(|m| (m.len(), m.modified().ok()));
    let source = format!("{}\n{:?}\n", me.display(), stamp(&me));
    let current = std::fs::read_to_string(dir.join("source.txt")).unwrap_or_default();
    if current != source || !target.is_file() {
        if target.exists() && std::fs::remove_file(&target).is_err() {
            let aside = dir.join(format!("lamprabit.exe.old-{}", std::process::id()));
            std::fs::rename(&target, aside)?;
        }
        // Copied beside it and moved into place, so an assistant never starts half a file.
        let temp = dir.join("lamprabit.exe.tmp");
        std::fs::copy(&me, &temp)?;
        std::fs::rename(&temp, &target)?;
        std::fs::write(dir.join("source.txt"), source)?;
    }
    std::fs::write(dir.join("app.txt"), me.to_string_lossy().as_bytes())
}

/// The program a client's LampRabit entry runs, if it has one.
fn entry_command(doc: &mut Doc, shape: Shape) -> Option<String> {
    let text = |v: &Value| v.as_str().map(str::to_string);
    match doc {
        Doc::Json(v) => {
            let e = servers(v, shape).ok()?.get(ENTRY)?;
            if shape == Shape::OpenCode { e["command"].get(0).and_then(text) } else { text(&e["command"]) }
        }
        Doc::Toml(t) => t.get("mcp_servers")?.get(ENTRY)?.get("command")?.as_str().map(str::to_string),
    }
}

/// Points every LampRabit entry that runs another program (an older copy, a moved or
/// reinstalled app) at this one, at startup. Returns the assistants it changed.
pub fn repoint() -> Vec<&'static str> {
    let want = program();
    let mut changed = Vec::new();
    for c in clients() {
        let Some(path) = (c.file)() else { continue };
        let Ok(mut doc) = read(&path, c.shape) else { continue };
        if entry_command(&mut doc, c.shape).is_some_and(|have| have != want) && set(c.id, true).is_ok() {
            changed.push(c.name);
        }
    }
    changed
}

/// Takes LampRabit out of every assistant (uninstall). Returns what couldn't be changed.
pub fn remove_all() -> Vec<String> {
    let mut problems = Vec::new();
    for c in clients() {
        let Some(path) = (c.file)() else { continue };
        let has = read(&path, c.shape).is_ok_and(|mut d| has_entry(&mut d, c.shape));
        if has {
            if let Err(e) = set(c.id, false) {
                problems.push(format!("Couldn't take LampRabit out of {} ({e}); remove its \"lamprabit\" entry from {} by hand.", c.name, path.display()));
            }
        }
    }
    problems
}

/// What the entry runs: LampRabit itself, as `lamprabit mcp`, in the client's notation.
fn entry(shape: Shape) -> Value {
    let exe = program();
    match shape {
        Shape::Servers => json!({"type": "stdio", "command": exe, "args": ARGS}),
        Shape::CopilotCli => json!({"type": "stdio", "command": exe, "args": ARGS, "tools": ["*"]}),
        Shape::OpenCode => json!({"type": "local", "command": [exe, ARGS[0], ARGS[1]], "enabled": true}),
        Shape::McpServers | Shape::CodexToml => json!({"command": exe, "args": ARGS}),
    }
}

/// Adds or removes the entry in Codex's config.toml, leaving the rest as written.
fn set_toml(doc: &mut toml_edit::DocumentMut, on: bool) -> Result<()> {
    use toml_edit::{value, Array, Item, Table};
    if !on {
        if let Some(list) = doc.get_mut("mcp_servers").and_then(Item::as_table_like_mut) {
            list.remove(ENTRY);
        }
        return Ok(());
    }
    let list = doc.entry("mcp_servers").or_insert_with(|| {
        // A header-less parent, so the file gains only [mcp_servers.lamprabit].
        let mut t = Table::new();
        t.set_implicit(true);
        Item::Table(t)
    });
    let list = list.as_table_like_mut().ok_or_else(|| Error::msg("mcp_servers in Codex's config.toml isn't a table."))?;
    let mut server = Table::new();
    server.insert("command", value(program()));
    server.insert("args", value(ARGS.into_iter().collect::<Array>()));
    list.insert(ENTRY, Item::Table(server));
    Ok(())
}

/// `config.json` → `config.json.<suffix>`, beside it.
fn beside(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".{suffix}"));
    path.with_file_name(name)
}

/// Adds (or, with `on` false, removes) LampRabit's entry in a client's MCP config.
pub fn set(id: &str, on: bool) -> Result<String> {
    if let Some((_, name, why)) = UNSUPPORTED.iter().find(|u| u.0 == id) {
        return Err(Error::msg(format!("{name} can't be connected. {why}")));
    }
    let client = clients().into_iter().find(|c| c.id == id).ok_or_else(|| Error::msg(format!("Unknown assistant {id}.")))?;
    let path = (client.file)().ok_or_else(|| Error::msg("Couldn't find your home folder."))?;
    if on && !present(&client, &path) {
        return Err(Error::msg(format!("{} doesn't seem to be installed (no {}). Install it, open it once, then connect.", client.name, path.parent().unwrap_or(&path).display())));
    }
    if on && launcher().is_some_and(|l| !l.is_file()) {
        refresh_launcher().map_err(|e| Error::msg(format!("Couldn't prepare LampRabit's copy for assistants ({e}). Check there's free disk space, then connect again.")))?;
    }
    let mut doc = read(&path, client.shape)?;
    if path.is_file() {
        // One backup, of the file as it was before LampRabit first touched it.
        let backup = beside(&path, "lamprabit-backup");
        if !backup.exists() {
            std::fs::copy(&path, &backup)?;
        }
    }
    let text = match &mut doc {
        Doc::Json(config) => {
            let list = servers(config, client.shape)?;
            if on {
                list.insert(ENTRY.to_string(), entry(client.shape));
            } else {
                list.remove(ENTRY);
            }
            serde_json::to_string_pretty(config)? + "\n"
        }
        Doc::Toml(config) => {
            set_toml(config, on)?;
            config.to_string()
        }
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // Written beside it, then moved over it, so a crash never leaves half a file.
    let temp = beside(&path, "lamprabit-tmp");
    std::fs::write(&temp, text)?;
    std::fs::rename(&temp, &path)?;
    // Assistants read the file only at startup, and some (Claude Desktop) keep running in the
    // tray when their window closes, so "restart" has to mean quitting it completely.
    Ok(if on { format!("{} can use LampRabit now. If it's open, quit it completely (from the tray too) and reopen it.", client.name) } else { format!("Removed LampRabit from {}.", client.name) })
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

    #[test]
    fn other_shapes() {
        let mut oc: Value = json!({"$schema": "https://opencode.ai/config.json"});
        servers(&mut oc, Shape::OpenCode).unwrap().insert(ENTRY.into(), entry(Shape::OpenCode));
        assert_eq!(oc["mcp"]["lamprabit"]["type"], "local");
        assert_eq!(oc["mcp"]["lamprabit"]["command"].as_array().unwrap().len(), 3);
        assert_eq!(entry(Shape::CopilotCli)["tools"], json!(["*"]));
    }

    #[test]
    fn codex_toml_keeps_the_rest() {
        let original = "# my settings\nmodel = \"gpt-5\"\n\n[mcp_servers.other]\ncommand = 'C:\\tools\\x.exe'\nargs = []\n\n[windows]\nsandbox = true\n";
        let mut doc: toml_edit::DocumentMut = original.parse().unwrap();
        set_toml(&mut doc, true).unwrap();
        let text = doc.to_string();
        assert!(text.starts_with("# my settings\nmodel = \"gpt-5\"\n\n[mcp_servers.other]\ncommand = 'C:\\tools\\x.exe'\n"), "{text}");
        assert!(text.contains("[mcp_servers.lamprabit]\ncommand = "), "{text}");
        assert!(text.contains("args = [\"--cli\", \"mcp\"]"), "{text}");
        assert!(has_entry(&mut Doc::Toml(text.parse().unwrap()), Shape::CodexToml));
        set_toml(&mut doc, false).unwrap();
        assert_eq!(doc.to_string(), original);

        // An empty file gains just the one table, no bare [mcp_servers] header.
        let mut empty = toml_edit::DocumentMut::new();
        set_toml(&mut empty, true).unwrap();
        assert!(empty.to_string().starts_with("[mcp_servers.lamprabit]\n"), "{empty}");
    }

    #[test]
    fn reads_the_entry_program() {
        let mut json = Doc::Json(json!({"mcpServers": {"lamprabit": {"command": "C:/old/lamprabit.exe", "args": ["--cli", "mcp"]}}}));
        assert_eq!(entry_command(&mut json, Shape::McpServers).as_deref(), Some("C:/old/lamprabit.exe"));
        let mut oc = Doc::Json(json!({"mcp": {"lamprabit": {"type": "local", "command": ["/opt/lamprabit", "--cli", "mcp"]}}}));
        assert_eq!(entry_command(&mut oc, Shape::OpenCode).as_deref(), Some("/opt/lamprabit"));
        let mut toml = Doc::Toml("[mcp_servers.lamprabit]\ncommand = 'C:\\x\\lamprabit.exe'\n".parse().unwrap());
        assert_eq!(entry_command(&mut toml, Shape::CodexToml).as_deref(), Some(r"C:\x\lamprabit.exe"));
        assert_eq!(entry_command(&mut Doc::Json(json!({})), Shape::McpServers), None);
    }

    #[test]
    fn backups_keep_the_old_names() {
        assert_eq!(beside(Path::new("a/claude_desktop_config.json"), "lamprabit-backup"), Path::new("a/claude_desktop_config.json.lamprabit-backup"));
        assert_eq!(beside(Path::new("a/config.toml"), "lamprabit-tmp"), Path::new("a/config.toml.lamprabit-tmp"));
    }
}
