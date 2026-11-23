//! Making LampRabit win over other local stacks on Windows (XAMPP, WAMP, Laragon, IIS, a
//! MySQL installer's service) and over Windows Security's scanning. Each fix asks for admin
//! rights once, through a UAC prompt, and changes only what it names:
//! - a Windows service holding a port LampRabit needs is stopped and set to start by hand;
//! - LampRabit's command folder goes first in the system PATH, which Windows reads before
//!   the user PATH (so XAMPP's php no longer answers `php` in a terminal);
//! - the LampRabit folder and Composer's cache are excluded from Windows Security scans,
//!   which otherwise scan every file `composer install` and its autoload step write.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::{
    error::{Error, Result},
    layout::Layout,
};

/// A command in the system PATH that answers before LampRabit's own.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Shadow {
    pub command: String,
    pub path: String,
}

/// The Windows service a process runs as (`Apache2.4`, `MySQL80`), if any. Port 80 held by
/// Windows itself (PID 4, http.sys) belongs to IIS when its web service runs.
pub async fn service_for(pid: u32) -> Option<String> {
    if !cfg!(windows) {
        return None;
    }
    if pid == 4 {
        let out = output("sc", &["query", "W3SVC"]).await?;
        return out.contains("RUNNING").then(|| "W3SVC".to_string());
    }
    let filter = format!("PID eq {pid}");
    let list = output("tasklist", &["/svc", "/FI", &filter, "/FO", "CSV", "/NH"]).await?;
    // "httpd.exe","1234","Apache2.4" (several services are comma-separated in the last field).
    let services = list.split('"').nth(5)?.trim();
    let first = services.split(',').next()?.trim();
    (!first.is_empty() && first != "N/A").then(|| first.to_string())
}

/// Stops a Windows service and sets it to start by hand, so it doesn't take the port back
/// at the next boot.
pub async fn stop_service(layout: &Layout, name: &str) -> Result<()> {
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || " ._-$".contains(c)) {
        return Err(Error::msg(format!("\"{name}\" isn't a Windows service name")));
    }
    let n = quote(name);
    elevated(layout, &format!("Stop-Service -Name '{n}' -Force; Set-Service -Name '{n}' -StartupType Manual"), &format!("Stopping the {name} service")).await
}

/// LampRabit's commands found later than another install's in the system PATH.
pub fn shadows(shims: &Path) -> Vec<Shadow> {
    let Some(path) = system_path() else { return Vec::new() };
    let entries: Vec<PathBuf> = path.split(';').filter(|p| !p.trim().is_empty()).map(|p| PathBuf::from(expand(p.trim()))).collect();
    let ours = |p: &Path| same_dir(p, shims);
    let mut out = Vec::new();
    let Ok(listing) = std::fs::read_dir(shims) else { return out };
    let mut commands: Vec<String> = listing.flatten().filter_map(|e| e.path().file_stem().map(|s| s.to_string_lossy().to_lowercase())).collect();
    commands.sort();
    commands.dedup();
    for command in commands {
        for dir in &entries {
            if ours(dir) {
                break;
            }
            if let Some(hit) = ["exe", "bat", "cmd", "com"].iter().map(|ext| dir.join(format!("{command}.{ext}"))).find(|p| p.is_file()) {
                out.push(Shadow { command: command.clone(), path: hit.to_string_lossy().into_owned() });
                break;
            }
        }
    }
    out
}

/// Puts LampRabit's command folder first in the system PATH, or takes it out. Entries like
/// %SystemRoot% stay unexpanded.
pub async fn system_path_first(layout: &Layout, shims: &Path, on: bool) -> Result<()> {
    let in_system = system_path().is_some_and(|p| p.split(';').any(|e| same_dir(Path::new(&expand(e.trim())), shims)));
    if !on && !in_system {
        return Ok(());
    }
    let dir = quote(&shims.to_string_lossy());
    let add = if on { format!("$parts = @('{dir}') + $parts; ") } else { String::new() };
    let script = format!(
        "$k = Get-Item 'HKLM:\\SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment'; \
         $p = $k.GetValue('Path', '', 'DoNotExpandEnvironmentNames'); \
         $parts = @($p -split ';' | Where-Object {{ $_ -and ($_.TrimEnd('\\') -ne '{dir}') }}); {add}\
         Set-ItemProperty -LiteralPath $k.PSPath -Name Path -Value ($parts -join ';') -Type ExpandString"
    );
    elevated(layout, &script, "Changing the system PATH").await?;
    broadcast();
    Ok(())
}

/// Excludes the LampRabit folder and Composer's cache from Windows Security scans.
pub async fn exclude_from_defender(layout: &Layout) -> Result<Vec<PathBuf>> {
    let mut paths = vec![layout.root().to_path_buf()];
    paths.extend(dirs::data_local_dir().map(|d| d.join("Composer")));
    let list = paths.iter().map(|p| format!("'{}'", quote(&p.to_string_lossy()))).collect::<Vec<_>>().join(",");
    elevated(layout, &format!("Add-MpPreference -ExclusionPath {list}"), "Adding the Windows Security exclusion").await?;
    // Remembered so uninstalling takes it away again (Windows hides exclusions from non-admins).
    let _ = std::fs::write(marker(layout), list);
    Ok(paths)
}

/// Takes the exclusion away again, when LampRabit added one.
pub async fn remove_defender_exclusion(layout: &Layout) -> Result<()> {
    let Ok(list) = std::fs::read_to_string(marker(layout)) else { return Ok(()) };
    elevated(layout, &format!("Remove-MpPreference -ExclusionPath {list}"), "Removing the Windows Security exclusion").await?;
    let _ = std::fs::remove_file(marker(layout));
    Ok(())
}

fn marker(layout: &Layout) -> PathBuf {
    layout.root().join("defender-exclusion.txt")
}

fn quote(s: &str) -> String {
    s.replace('\'', "''")
}

fn same_dir(a: &Path, b: &Path) -> bool {
    let norm = |p: &Path| p.to_string_lossy().trim_end_matches(['\\', '/']).to_lowercase();
    norm(a) == norm(b)
}

/// %VAR% → its value, for reading PATH entries.
fn expand(entry: &str) -> String {
    let mut out = String::new();
    let mut rest = entry;
    while let Some(start) = rest.find('%') {
        let Some(len) = rest[start + 1..].find('%') else { break };
        let name = &rest[start + 1..start + 1 + len];
        out.push_str(&rest[..start]);
        out.push_str(&std::env::var(name).unwrap_or_else(|_| format!("%{name}%")));
        rest = &rest[start + len + 2..];
    }
    out + rest
}

#[cfg(windows)]
fn system_path() -> Option<String> {
    use winreg::{enums::*, RegKey};
    let env = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey_with_flags(r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment", KEY_READ).ok()?;
    let raw = env.get_raw_value("Path").ok()?;
    let wide: Vec<u16> = raw.bytes.as_chunks::<2>().0.iter().map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
    Some(String::from_utf16_lossy(&wide).trim_end_matches('\0').to_string())
}

#[cfg(not(windows))]
fn system_path() -> Option<String> {
    None
}

#[cfg(windows)]
fn broadcast() {
    use windows_sys::Win32::UI::WindowsAndMessaging::{SendMessageTimeoutW, HWND_BROADCAST, SMTO_ABORTIFHUNG, WM_SETTINGCHANGE};
    let name: Vec<u16> = "Environment".encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        SendMessageTimeoutW(HWND_BROADCAST, WM_SETTINGCHANGE, 0, name.as_ptr() as isize, SMTO_ABORTIFHUNG, 2000, std::ptr::null_mut());
    }
}

#[cfg(not(windows))]
fn broadcast() {}

async fn output(program: &str, args: &[&str]) -> Option<String> {
    let mut cmd = tokio::process::Command::new(program);
    cmd.args(args);
    crate::procgroup::prepare(&mut cmd);
    let out = tokio::time::timeout(std::time::Duration::from_secs(5), cmd.output()).await.ok()?.ok()?;
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Runs a PowerShell script as administrator (one UAC prompt); its error message, if any,
/// comes back through a file, since an elevated process's output can't be read.
async fn elevated(layout: &Layout, script: &str, what: &str) -> Result<()> {
    if !cfg!(windows) {
        return Err(Error::msg(format!("{what} is only needed on Windows")));
    }
    use base64::Engine;
    std::fs::create_dir_all(layout.tmp())?;
    let error_file = layout.tmp().join("elevated.error");
    let _ = std::fs::remove_file(&error_file);
    let wrapped = format!(
        "$ErrorActionPreference = 'Stop'; try {{ {script}; exit 0 }} catch {{ Set-Content -LiteralPath '{}' -Value $_.Exception.Message; exit 2 }}",
        quote(&error_file.to_string_lossy())
    );
    let utf16: Vec<u8> = wrapped.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let encoded = base64::engine::general_purpose::STANDARD.encode(utf16);
    let launcher = format!("try {{ $p = Start-Process powershell -Verb RunAs -Wait -PassThru -WindowStyle Hidden -ArgumentList '-NoProfile','-EncodedCommand','{encoded}'; exit $p.ExitCode }} catch {{ exit 1 }}");
    let mut cmd = tokio::process::Command::new("powershell");
    cmd.args(["-NoProfile", "-NonInteractive", "-Command", &launcher]);
    crate::procgroup::prepare(&mut cmd);
    let status = cmd.status().await?;
    match status.code() {
        Some(0) => Ok(()),
        Some(1) => Err(Error::msg(format!("{what} was cancelled: Windows asks for administrator approval for it."))),
        _ => {
            let message = std::fs::read_to_string(&error_file).unwrap_or_default();
            Err(Error::msg(format!("{what} failed: {}", message.trim())))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_path_entries_and_compares_folders_loosely() {
        // SAFETY: tests in this module don't read this variable concurrently.
        unsafe { std::env::set_var("LR_TEST_ROOT", r"C:\Tools") };
        assert_eq!(expand(r"%LR_TEST_ROOT%\php"), r"C:\Tools\php");
        assert_eq!(expand(r"%LR_NOT_SET%\x"), r"%LR_NOT_SET%\x");
        assert_eq!(expand("plain"), "plain");
        assert!(same_dir(Path::new(r"C:\lampRabit\bin\shims\"), Path::new(r"c:\LampRabit\bin\shims")));
        assert_eq!(quote("it's"), "it''s");
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn refuses_service_names_that_could_inject_script() {
        let layout = Layout::new(std::env::temp_dir().join("lr-overrides-test"));
        assert!(stop_service(&layout, "Apache2.4'; Remove-Item C:\\ -Recurse; '").await.unwrap_err().to_string().contains("isn't a Windows service name"));
    }
}

