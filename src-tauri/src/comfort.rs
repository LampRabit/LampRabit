//! Everyday conveniences: terminal and editor launchers, and the Microsoft Visual C++
//! runtime that Apache and PHP need on Windows.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::{
    config::Settings,
    error::{Error, Result},
    layout::Layout,
    procgroup, shims,
};

/// Opens a terminal in `dir` with LampRabit's command shims (php, composer, mariadb…) first on PATH.
pub fn open_terminal(settings: &Settings, layout: &Layout, dir: &Path) -> Result<()> {
    let shims = shims::write(settings, layout)?;
    let existing = std::env::var_os("PATH").unwrap_or_default();
    let path = std::env::join_paths(std::iter::once(shims).chain(std::env::split_paths(&existing)))
        .map_err(|e| Error::msg(e.to_string()))?;
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        // `start` gives the terminal its own console (and its own input; a cmd spawned
        // directly would inherit LampRabit's empty stdin and exit at once). The first quoted
        // argument is the window title; unquoted, `start` would launch it as a program.
        std::process::Command::new("cmd")
            .raw_arg(r#"/C start "LampRabit" cmd /K echo LampRabit terminal: php, composer and mariadb use your active versions."#)
            .current_dir(dir)
            .env("PATH", path)
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()?;
        return Ok(());
    }
    #[cfg(not(windows))]
    {
        if cfg!(target_os = "macos") {
            // Keep paths out of AppleScript source, then quote them for the shell that
            // Terminal starts. Project folders and the environment root can contain quotes.
            let script = r#"
                on run argv
                    set projectDir to item 1 of argv
                    set lampRabitPath to item 2 of argv
                    set shellCommand to "cd " & quoted form of projectDir & "; export PATH=" & quoted form of lampRabitPath
                    tell application "Terminal" to do script shellCommand
                    tell application "Terminal" to activate
                end run
            "#;
            std::process::Command::new("osascript")
                .args(["-e", script])
                .arg(dir)
                .arg(path)
                .spawn()?;
            return Ok(());
        }
        // GNOME's terminals start shells from a background server that never sees our
        // environment, so PATH goes in through `env` as part of the command.
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
        run_in_terminal(dir, &["env".to_string(), format!("PATH={}", path.to_string_lossy()), shell])
    }
}

/// Runs `command` in a new terminal window (Linux and other Unix desktops), trying the
/// usual terminal emulators in turn.
#[cfg(not(windows))]
pub fn run_in_terminal(dir: &Path, command: &[String]) -> Result<()> {
    let terminals: [(&str, &[&str]); 9] = [
        ("x-terminal-emulator", &["-e"]),
        ("ptyxis", &["--new-window", "--"]),
        ("gnome-terminal", &["--"]),
        ("kgx", &["--"]),
        ("konsole", &["-e"]),
        ("xfce4-terminal", &["-x"]),
        ("alacritty", &["-e"]),
        ("kitty", &[]),
        ("xterm", &["-e"]),
    ];
    for (term, run) in terminals {
        if std::process::Command::new(term).args(run).args(command).current_dir(dir).spawn().is_ok() {
            return Ok(());
        }
    }
    Err(Error::msg("No terminal emulator found. Install one such as gnome-terminal, konsole or xterm."))
}

/// Opens a folder in the configured editor command (VS Code's `code` by default).
pub async fn open_in_editor(settings: &Settings, dir: &Path) -> Result<()> {
    let editor = settings.editor.trim();
    if editor.is_empty() {
        return Err(Error::msg("Set an editor command in Settings → General."));
    }
    let mut cmd = if cfg!(windows) {
        // Editor launchers are usually .cmd shims (code.cmd), which need cmd to run.
        let mut c = tokio::process::Command::new("cmd");
        c.args(["/C", editor]).arg(dir);
        c
    } else {
        let mut c = tokio::process::Command::new(editor);
        c.arg(dir);
        c
    };
    procgroup::prepare(&mut cmd);
    let out = cmd.output().await.map_err(|_| Error::msg(format!("Couldn't run \"{editor}\". Check Settings → General → Editor.")))?;
    if !out.status.success() {
        return Err(Error::msg(format!(
            "\"{editor}\" didn't start. Make sure it's installed and on your PATH (VS Code: Command Palette → \"Shell Command: Install 'code' command\"), or change it in Settings → General."
        )));
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VcRuntime {
    /// Only Windows needs it.
    pub applicable: bool,
    pub installed: bool,
    pub version: Option<String>,
}

pub fn vc_runtime() -> VcRuntime {
    if !cfg!(windows) {
        return VcRuntime { applicable: false, installed: true, version: None };
    }
    let sys = PathBuf::from(std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into())).join("System32");
    let files = ["vcruntime140.dll", "vcruntime140_1.dll", "msvcp140.dll"].iter().all(|f| sys.join(f).exists());
    #[allow(unused_mut)]
    let mut version = None;
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        if let Ok(out) = std::process::Command::new("reg")
            .args(["query", r"HKLM\SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\X64", "/v", "Version"])
            .creation_flags(0x0800_0000)
            .output()
        {
            version = String::from_utf8_lossy(&out.stdout)
                .lines()
                .find(|l| l.contains("Version"))
                .and_then(|l| l.split_whitespace().last())
                .map(|v| v.trim_start_matches('v').to_string());
        }
    }
    VcRuntime { applicable: true, installed: files, version }
}

/// Downloads Microsoft's Visual C++ redistributable and installs it (one admin prompt).
pub async fn install_vc_runtime(client: &reqwest::Client, layout: &Layout) -> Result<()> {
    let err = |e: reqwest::Error| Error::msg(format!("Download failed: {e}"));
    let bytes = client
        .get("https://aka.ms/vc14/vc_redist.x64.exe")
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(err)?
        .bytes()
        .await
        .map_err(err)?;
    std::fs::create_dir_all(layout.tmp())?;
    let setup = layout.tmp().join("vc_redist.x64.exe");
    std::fs::write(&setup, &bytes)?;
    // Microsoft signs the installer; Windows verifies that signature when elevating.
    let launcher = format!(
        "try {{ $p = Start-Process -FilePath '{}' -ArgumentList '/install','/quiet','/norestart' -Verb RunAs -Wait -PassThru; exit $p.ExitCode }} catch {{ exit 1223 }}",
        setup.display().to_string().replace('\'', "''")
    );
    let mut cmd = tokio::process::Command::new("powershell");
    cmd.args(["-NoProfile", "-NonInteractive", "-Command", &launcher]);
    procgroup::prepare(&mut cmd);
    let status = cmd.status().await?;
    let _ = std::fs::remove_file(&setup);
    match status.code() {
        // 3010: installed, restart recommended; 1638: a newer version is already there.
        Some(0) | Some(3010) | Some(1638) => Ok(()),
        Some(1223) => Err(Error::msg("Installing the Visual C++ runtime was cancelled.")),
        code => Err(Error::msg(format!("The Visual C++ installer exited with code {}.", code.unwrap_or(-1)))),
    }
}
