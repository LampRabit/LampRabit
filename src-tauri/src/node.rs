//! Node.js versions through NVM: nvm-windows on Windows, nvm (nvm-sh) on macOS and Linux.
//! NVM is only installed when the user asks; an existing installation is used as is.

use std::{path::PathBuf, process::Stdio};

use serde::Serialize;
use tauri::AppHandle;
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::{
    error::{Error, Result},
    layout::Layout,
    logs::{self, Level},
    procgroup,
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeStatus {
    /// "nvm-windows" or "nvm"; None when NVM isn't installed.
    pub manager: Option<&'static str>,
    pub manager_version: Option<String>,
    pub current: Option<String>,
    pub installed: Vec<String>,
    /// Windows: switching versions needs admin rights unless Developer Mode is on.
    pub needs_elevation: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeRelease {
    pub version: String,
    pub major: u32,
    /// LTS codename ("Jod"), None for Current releases.
    pub lts: Option<String>,
}

/// An environment variable from the process, or (Windows) from the registry, where a
/// fresh NVM install writes it without the running app seeing it.
fn env_var(name: &str) -> Option<String> {
    if let Ok(v) = std::env::var(name) {
        if !v.is_empty() {
            return Some(v);
        }
    }
    #[cfg(windows)]
    for key in [r"HKCU\Environment", r"HKLM\SYSTEM\CurrentControlSet\Control\Session Manager\Environment"] {
        let mut cmd = std::process::Command::new("reg");
        cmd.args(["query", key, "/v", name]);
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x0800_0000);
        }
        if let Ok(out) = cmd.output() {
            let text = String::from_utf8_lossy(&out.stdout);
            if let Some(line) = text.lines().find(|l| l.trim_start().starts_with(name)) {
                let value = line.split("REG_").nth(1).and_then(|r| r.split_once(char::is_whitespace)).map(|(_, v)| v.trim().to_string());
                if let Some(v) = value.filter(|v| !v.is_empty()) {
                    return Some(expand(&v));
                }
            }
        }
    }
    None
}

/// Expands %VAR% references (REG_EXPAND_SZ values).
#[cfg(windows)]
fn expand(value: &str) -> String {
    let mut out = String::new();
    let mut rest = value;
    while let Some(start) = rest.find('%') {
        let Some(len) = rest[start + 1..].find('%') else { break };
        out.push_str(&rest[..start]);
        let name = &rest[start + 1..start + 1 + len];
        out.push_str(&std::env::var(name).unwrap_or_else(|_| format!("%{name}%")));
        rest = &rest[start + len + 2..];
    }
    out.push_str(rest);
    out
}

fn nvm_windows() -> Option<(PathBuf, String)> {
    let home = env_var("NVM_HOME")?;
    let exe = PathBuf::from(&home).join("nvm.exe");
    exe.exists().then(|| (exe, env_var("NVM_SYMLINK").unwrap_or_default()))
}

/// NVM (nvm-windows, or nvm-sh elsewhere) is there to install Node versions with.
pub fn nvm_installed() -> bool {
    if cfg!(windows) { nvm_windows().is_some() } else { nvm_sh_dir().is_some() }
}

fn nvm_sh_dir() -> Option<PathBuf> {
    let dir = env_var("NVM_DIR").map(PathBuf::from).or_else(|| dirs::home_dir().map(|h| h.join(".nvm")))?;
    dir.join("nvm.sh").exists().then_some(dir)
}

fn developer_mode() -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let out = std::process::Command::new("reg")
            .args(["query", r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\AppModelUnlock", "/v", "AllowDevelopmentWithoutDevLicense"])
            .creation_flags(0x0800_0000)
            .output();
        return out.is_ok_and(|o| String::from_utf8_lossy(&o.stdout).contains("0x1"));
    }
    #[allow(unreachable_code)]
    false
}

/// Runs an NVM command, mirroring its output to the System log. Returns stdout.
async fn nvm(app: &AppHandle, args: &[&str]) -> Result<String> {
    let mut cmd = if cfg!(windows) {
        let (exe, symlink) = nvm_windows().ok_or_else(|| Error::msg("NVM isn't installed"))?;
        let mut c = tokio::process::Command::new(&exe);
        c.args(args)
            .env("NVM_HOME", exe.parent().unwrap())
            .env("NVM_SYMLINK", symlink);
        c
    } else {
        let dir = nvm_sh_dir().ok_or_else(|| Error::msg("NVM isn't installed"))?;
        let quoted: Vec<String> = args.iter().map(|a| format!("'{}'", a.replace('\'', ""))).collect();
        let mut c = tokio::process::Command::new("bash");
        c.args(["-c", &format!("export NVM_DIR='{}'; . \"$NVM_DIR/nvm.sh\"; nvm {}", dir.display(), quoted.join(" "))]);
        c
    };
    cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    procgroup::prepare(&mut cmd);
    let mut child = cmd.spawn().map_err(|e| Error::msg(format!("Couldn't run nvm: {e}")))?;
    let mut out = String::new();
    let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        if !line.trim().is_empty() {
            logs::push(app, "system", "output", format!("[nvm] {line}"));
        }
        out.push_str(&line);
        out.push('\n');
    }
    let mut err = String::new();
    let mut elines = BufReader::new(child.stderr.take().unwrap()).lines();
    while let Ok(Some(line)) = elines.next_line().await {
        err.push_str(&line);
        err.push('\n');
    }
    let status = child.wait().await?;
    if !status.success() {
        let msg = if err.trim().is_empty() { out.trim() } else { err.trim() };
        return Err(Error::msg(format!("nvm {} failed: {msg}", args.join(" "))));
    }
    Ok(out)
}

/// Version numbers in nvm's list output (either flavour), without the "v".
fn parse_versions(text: &str) -> (Vec<String>, Option<String>) {
    let mut versions = Vec::new();
    let mut current = None;
    for line in text.lines() {
        // nvm-sh alias lines ("default -> 20 (-> v20.1.0)") aren't installations.
        if line.contains("default") || line.contains("node ->") || line.contains("stable") || line.contains("lts/") {
            continue;
        }
        let marked = line.contains('*') || line.trim_start().starts_with("->");
        let Some(v) = line
            .split(|c: char| c.is_whitespace() || c == '(')
            .map(|t| t.trim_start_matches('v'))
            .find(|t| t.split('.').count() == 3 && t.split('.').all(|p| p.parse::<u32>().is_ok()))
        else {
            continue;
        };
        if marked {
            current = Some(v.to_string());
        }
        if !versions.iter().any(|x| x == v) {
            versions.push(v.to_string());
        }
    }
    (versions, current)
}

pub async fn status(app: &AppHandle) -> NodeStatus {
    let manager = if cfg!(windows) {
        nvm_windows().map(|_| "nvm-windows")
    } else {
        nvm_sh_dir().map(|_| "nvm")
    };
    let Some(manager) = manager else {
        return NodeStatus { manager: None, manager_version: None, current: None, installed: vec![], needs_elevation: false };
    };
    let manager_version = nvm(app, &[if cfg!(windows) { "version" } else { "--version" }]).await.ok().map(|v| v.trim().to_string());
    let list = nvm(app, &if cfg!(windows) { vec!["list"] } else { vec!["ls", "--no-colors"] }).await.unwrap_or_default();
    let (installed, mut current) = parse_versions(&list);
    if current.is_none() {
        current = nvm(app, &["current"]).await.ok().map(|c| c.trim().trim_start_matches('v').to_string()).filter(|c| c.contains('.'));
    }
    NodeStatus {
        manager: Some(manager),
        manager_version,
        current,
        installed,
        needs_elevation: cfg!(windows) && !developer_mode(),
    }
}

/// The newest release of each recent Node major, LTS first, from nodejs.org.
pub async fn available(client: &reqwest::Client) -> Result<Vec<NodeRelease>> {
    let index: Vec<serde_json::Value> = client
        .get("https://nodejs.org/dist/index.json")
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| Error::msg(format!("Couldn't load Node versions: {e}")))?
        .json()
        .await
        .map_err(|e| Error::msg(format!("Couldn't load Node versions: {e}")))?;
    let mut out: Vec<NodeRelease> = Vec::new();
    for r in index {
        let Some(v) = r["version"].as_str() else { continue };
        let v = v.trim_start_matches('v');
        let Some(major) = v.split('.').next().and_then(|m| m.parse().ok()) else { continue };
        if major < 18 || out.iter().any(|x| x.major == major) {
            continue;
        }
        out.push(NodeRelease { version: v.to_string(), major, lts: r["lts"].as_str().map(str::to_string) });
        if out.len() == 6 {
            break;
        }
    }
    Ok(out)
}

pub async fn install_version(app: &AppHandle, version: &str) -> Result<()> {
    logs::system(app, Level::Info, format!("Installing Node {version} with NVM"));
    nvm(app, &["install", version]).await?;
    logs::system(app, Level::Info, format!("Installed Node {version}"));
    Ok(())
}

/// Makes a version the default. nvm-windows switches the global symlink; nvm-sh sets
/// the default alias for new shells.
pub async fn use_version(app: &AppHandle, version: &str) -> Result<()> {
    if cfg!(windows) {
        nvm(app, &["use", version]).await?;
    } else {
        nvm(app, &["alias", "default", version]).await?;
    }
    logs::system(app, Level::Info, format!("Node {version} is now the default"));
    Ok(())
}

pub async fn uninstall_version(app: &AppHandle, version: &str) -> Result<()> {
    nvm(app, &["uninstall", version]).await?;
    logs::system(app, Level::Info, format!("Removed Node {version}"));
    Ok(())
}

/// Installs NVM with its official installer (Windows: elevated nvm-setup.exe; elsewhere
/// the nvm-sh install script, which adds itself to your shell profile).
pub async fn install_nvm(app: &AppHandle, client: &reqwest::Client, layout: &Layout) -> Result<()> {
    let err = |e: reqwest::Error| Error::msg(format!("Download failed: {e}"));
    if cfg!(windows) {
        let release: serde_json::Value = client
            .get("https://api.github.com/repos/coreybutler/nvm-windows/releases/latest")
            .header("Accept", "application/vnd.github+json")
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(err)?
            .json()
            .await
            .map_err(err)?;
        let arch = if cfg!(target_arch = "aarch64") { "arm64" } else { "x64" };
        let asset = release["assets"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|a| a["name"].as_str().is_some_and(|n| n.ends_with(&format!("{arch}-setup.exe")) || n == "nvm-setup.exe"))
            .ok_or_else(|| Error::msg("Couldn't find the nvm-windows installer"))?;
        let bytes = client.get(asset["browser_download_url"].as_str().unwrap_or_default()).send().await.and_then(|r| r.error_for_status()).map_err(err)?.bytes().await.map_err(err)?;
        if let Some(expected) = asset["digest"].as_str().and_then(|d| d.strip_prefix("sha256:")) {
            use sha2::Digest;
            let actual: String = sha2::Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect();
            if actual != expected {
                return Err(Error::msg("The nvm-windows installer failed its checksum check."));
            }
        }
        std::fs::create_dir_all(layout.tmp())?;
        let setup = layout.tmp().join("nvm-setup.exe");
        std::fs::write(&setup, &bytes)?;
        logs::system(app, Level::Info, "Running the nvm-windows installer (needs administrator approval)");
        let launcher = format!(
            "try {{ $p = Start-Process -FilePath '{}' -ArgumentList '/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART','/SP-' -Verb RunAs -Wait -PassThru; exit $p.ExitCode }} catch {{ exit 1223 }}",
            setup.display().to_string().replace('\'', "''")
        );
        let mut cmd = tokio::process::Command::new("powershell");
        cmd.args(["-NoProfile", "-NonInteractive", "-Command", &launcher]);
        procgroup::prepare(&mut cmd);
        let status = cmd.status().await?;
        let _ = std::fs::remove_file(&setup);
        match status.code() {
            Some(0) => {}
            Some(1223) => return Err(Error::msg("Installing NVM was cancelled.")),
            code => return Err(Error::msg(format!("The NVM installer exited with code {}.", code.unwrap_or(-1)))),
        }
    } else {
        let release: serde_json::Value = client
            .get("https://api.github.com/repos/nvm-sh/nvm/releases/latest")
            .header("Accept", "application/vnd.github+json")
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(err)?
            .json()
            .await
            .map_err(err)?;
        let tag = release["tag_name"].as_str().ok_or_else(|| Error::msg("Couldn't find the latest nvm release"))?;
        let script = client
            .get(format!("https://raw.githubusercontent.com/nvm-sh/nvm/{tag}/install.sh"))
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(err)?
            .text()
            .await
            .map_err(err)?;
        std::fs::create_dir_all(layout.tmp())?;
        let path = layout.tmp().join("nvm-install.sh");
        std::fs::write(&path, script)?;
        let status = tokio::process::Command::new("bash").arg(&path).status().await?;
        let _ = std::fs::remove_file(&path);
        if !status.success() {
            return Err(Error::msg("The nvm install script failed; see the Logs page."));
        }
    }
    logs::system(app, Level::Info, "NVM installed");
    Ok(())
}

/// Where NVM keeps installed versions: nvm-windows' NVM_HOME\v<version>, nvm-sh's
/// $NVM_DIR/versions/node/v<version>.
fn versions_dir() -> Option<PathBuf> {
    if cfg!(windows) {
        nvm_windows().and_then(|(exe, _)| exe.parent().map(PathBuf::from))
    } else {
        nvm_sh_dir().map(|d| d.join("versions").join("node"))
    }
}

/// The folder with a version's `node` and `npm`.
fn bin_of(version: &str) -> Option<PathBuf> {
    let dir = versions_dir()?.join(format!("v{version}"));
    Some(if cfg!(windows) { dir } else { dir.join("bin") })
}

/// Versions installed with NVM, newest first, read from disk (no nvm process, so it's
/// cheap enough to call on every app start).
pub fn installed_versions() -> Vec<String> {
    let mut out: Vec<String> = std::fs::read_dir(versions_dir().unwrap_or_default())
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| e.file_name().to_str()?.strip_prefix('v').map(str::to_string))
        .filter(|v| v.split('.').count() == 3 && v.split('.').all(|p| p.parse::<u32>().is_ok()))
        .collect();
    out.sort_by_key(|v| std::cmp::Reverse(v.split('.').filter_map(|p| p.parse::<u32>().ok()).collect::<Vec<_>>()));
    out
}

/// The installed version a spec asks for: "22", "22.1", "v22.1.0"; "node", "lts/*" and
/// LTS names pick the newest installed one.
pub fn resolve(spec: &str, installed: &[String]) -> Option<String> {
    let spec = spec.trim().trim_start_matches('v');
    if spec == "node" || spec == "stable" || spec.starts_with("lts") {
        return installed.first().cloned();
    }
    installed.iter().find(|v| *v == spec || v.starts_with(&format!("{spec}."))).cloned()
}

/// NVM's default: nvm-windows' active version (its symlink), nvm-sh's `default` alias.
fn default_version(installed: &[String]) -> Option<String> {
    if cfg!(windows) {
        let target = std::fs::read_link(env_var("NVM_SYMLINK")?).ok()?;
        let name = target.file_name()?.to_str()?.trim_start_matches('v').to_string();
        installed.contains(&name).then_some(name)
    } else {
        let alias = std::fs::read_to_string(nvm_sh_dir()?.join("alias").join("default")).ok()?;
        resolve(alias.trim(), installed)
    }
}

/// The Node version and folder a project's processes use: the one chosen for it, else
/// its .nvmrc, else NVM's default. Running from that folder means each project gets its
/// own version without switching NVM's global one.
pub fn for_project(chosen: Option<&str>, project: &std::path::Path) -> Result<(String, PathBuf)> {
    let installed = installed_versions();
    if installed.is_empty() {
        return Err(Error::msg("No Node.js versions are installed. Install one from Services → Node.js."));
    }
    let wanted = chosen.map(str::to_string).or_else(|| project_version(project));
    let version = match &wanted {
        Some(spec) => resolve(spec, &installed).ok_or_else(|| {
            Error::msg(format!("This project wants Node {spec}, which isn't installed. Install it from Services → Node.js, or pick another version."))
        })?,
        None => default_version(&installed).unwrap_or_else(|| installed[0].clone()),
    };
    let bin = bin_of(&version).ok_or_else(|| Error::msg("NVM isn't installed. Install it from Services → Node.js."))?;
    Ok((version, bin))
}

/// The Node version a project asks for in .nvmrc / .node-version, if any.
pub fn project_version(dir: &std::path::Path) -> Option<String> {
    [".nvmrc", ".node-version"]
        .iter()
        .find_map(|f| std::fs::read_to_string(dir.join(f)).ok())
        .map(|v| v.trim().trim_start_matches('v').to_string())
        .filter(|v| !v.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_nvmrc_specs() {
        let installed = vec!["24.22.1".to_string(), "22.23.4".to_string(), "22.1.0".to_string()];
        assert_eq!(resolve("22", &installed).as_deref(), Some("22.23.4"));
        assert_eq!(resolve("v22.1", &installed).as_deref(), Some("22.1.0"));
        assert_eq!(resolve("lts/jod", &installed).as_deref(), Some("24.22.1"));
        assert_eq!(resolve("2", &installed), None, "a major must match exactly, not as a prefix");
        assert_eq!(resolve("20", &installed), None);
    }

    #[test]
    fn parses_nvm_windows_list() {
        let text = "\n  * 26.9.0 (Currently using 64-bit executable)\n    24.13.1\n";
        let (v, c) = parse_versions(text);
        assert_eq!(v, vec!["26.9.0", "24.13.1"]);
        assert_eq!(c.as_deref(), Some("26.9.0"));
    }

    #[test]
    fn parses_nvm_sh_list() {
        let text = "       v18.20.4\n->     v20.17.0\ndefault -> 20 (-> v20.17.0)\nnode -> stable (-> v20.17.0) (default)\nlts/* -> lts/jod (-> N/A)\n";
        let (v, c) = parse_versions(text);
        assert_eq!(v, vec!["18.20.4", "20.17.0"]);
        assert_eq!(c.as_deref(), Some("20.17.0"));
    }
}
