mod apps;
mod cmd;
mod backups;
mod catalog;
mod comfort;
mod commands;
mod deps;
mod config;
mod control;
mod cli;
mod mcp;
mod mcp_clients;
mod configs;
mod cron;
mod diagnose;
mod doctor;
mod dumps;
mod error;
mod extensions;
mod facts;
mod git;
mod imports;
mod langver;
mod layout;
mod logs;
mod manage;
mod node;
mod php;
mod ports;
mod procgroup;
mod projectfile;
mod projects;
mod relocate;
mod runtimes;
mod services;
mod shims;
mod sites;
mod smoke;
mod ssl;
mod templategen;
mod templates;
mod tunnels;
mod uninstall;
mod version;

use std::{collections::HashSet, path::PathBuf};

use serde::Serialize;
use tauri::{AppHandle, Manager, RunEvent, State};
use tokio::sync::{Mutex, RwLock};

use cmd::*;
use config::Settings;
use error::{Error, Result};
use layout::Layout;
use runtimes::{Release, Runtime};
use services::{ServiceInfo, ServiceKind, Supervisor};

struct AppState {
    settings: RwLock<Settings>,
    settings_path: PathBuf,
    supervisor: Supervisor,
    http: reqwest::Client,
    installing: Mutex<HashSet<(Runtime, String)>>,
    /// Cached result of the (slow) trust-store lookup; reset when trust changes.
    ca_trusted: Mutex<Option<bool>>,
    tunnels: tunnels::Tunnels,
    /// Held by anything that starts, stops or reconfigures services, so rapid clicks
    /// (Start all, Stop all, Start all…) run in order and the last one wins.
    ops: Mutex<()>,
    /// Process table for the dashboard's CPU and memory readings; kept between calls
    /// because CPU usage is measured from one refresh to the next.
    processes: std::sync::Mutex<sysinfo::System>,
    /// Node apps the user started from Sites (separate from services).
    apps: apps::Apps,
}

impl AppState {
    fn new(settings: Settings, settings_path: PathBuf) -> Self {
        AppState {
            settings: RwLock::new(settings),
            settings_path,
            supervisor: Supervisor::default(),
            http: runtimes::client(),
            installing: Mutex::new(HashSet::new()),
            ca_trusted: Mutex::new(None),
            tunnels: tunnels::Tunnels::default(),
            ops: Mutex::new(()),
            processes: std::sync::Mutex::new(sysinfo::System::new()),
            apps: apps::Apps::default(),
        }
    }

    async fn snapshot(&self) -> (Settings, Layout) {
        let settings = self.settings.read().await.clone();
        let layout = Layout::new(&settings.root_dir);
        (settings, layout)
    }

    async fn update_settings(&self, f: impl FnOnce(&mut Settings)) -> Result<Settings> {
        let mut settings = self.settings.write().await;
        f(&mut settings);
        settings.save(&self.settings_path)?;
        let _ = shims::write(&settings, &Layout::new(&settings.root_dir));
        Ok(settings.clone())
    }
}

fn refresh_shims(app: &AppHandle, settings: &Settings) {
    if let Err(e) = shims::write(settings, &Layout::new(&settings.root_dir)) {
        logs::system(app, logs::Level::Warn, format!("Couldn't update command shims: {e}"));
    }
}

/// Keeps every project's domain pointed at what it's serving: the port its processes
/// listen on (a dev server from LampRabit or your own terminal), or its files when nothing
/// listens. Runs every few seconds, and straight away when an app starts or stops.
/// Processes set to start with LampRabit, then, once a minute, the ones whose schedule
/// fires (a scheduled process already running is left alone).
fn spawn_process_schedules(handle: &AppHandle) {
    let handle = handle.clone();
    tauri::async_runtime::spawn(async move {
        let state = handle.state::<AppState>();
        let (settings, layout) = state.snapshot().await;
        for (project, own) in &settings.sites {
            for p in own.processes.iter().filter(|p| p.at_launch) {
                if let Err(e) = state.apps.start_process(&handle, &settings, &layout, project, &p.name).await {
                    logs::system(&handle, logs::Level::Warn, format!("{project}/{}: couldn't start on launch: {e}", p.name));
                }
            }
        }
        loop {
            // Wake just after each minute turns, so a "0 3 * * *" job runs at 3:00, not 3:00:59.
            let now = time::OffsetDateTime::now_local().unwrap_or_else(|_| time::OffsetDateTime::now_utc());
            tokio::time::sleep(std::time::Duration::from_secs(61 - u64::from(now.second()))).await;
            let now = time::OffsetDateTime::now_local().unwrap_or_else(|_| time::OffsetDateTime::now_utc());
            let (settings, layout) = state.snapshot().await;
            for (project, own) in &settings.sites {
                for p in &own.processes {
                    let Ok(schedule) = cron::Schedule::parse(p.schedule.trim()) else { continue };
                    if schedule.matches(now)
                        && let Err(e) = state.apps.start_process(&handle, &settings, &layout, project, &p.name).await {
                            logs::system(&handle, logs::Level::Warn, format!("{project}/{}: the scheduled start failed: {e}", p.name));
                        }
                }
            }
        }
    });
}

async fn watch_ports(handle: AppHandle) {
    let mut system = sysinfo::System::new();
    // A route is dropped only after its port has been gone for two scans, so a dev server
    // restarting itself (on a config change) doesn't flip the site to its files and back.
    let mut missing: std::collections::BTreeMap<String, u8> = Default::default();
    // What the projects' version files ask for; the shims are rewritten when it changes.
    let mut versions = String::new();
    loop {
        tokio::select! {
            _ = tokio::time::sleep(std::time::Duration::from_secs(3)) => {}
            _ = ports::WAKE.notified() => {}
        }
        let state = handle.state::<AppState>();
        let (settings, layout) = state.snapshot().await;
        // Both read every project's files: off the async threads, which serve the window.
        let (now, sites) = tokio::task::block_in_place(|| (langver::signature(&layout), sites::scan(&settings, &layout)));
        if now != versions {
            if !versions.is_empty() {
                refresh_shims(&handle, &settings);
            }
            versions = now;
        }
        let projects: Vec<(String, PathBuf)> = sites.iter().map(|s| (s.name.clone(), s.path.clone())).collect();
        let Ok((found, back)) = tauri::async_runtime::spawn_blocking(move || {
            let found = ports::discover(&mut system, &projects);
            (found, system)
        })
        .await
        else {
            system = sysinfo::System::new();
            continue;
        };
        system = back;

        let statuses = state.apps.statuses().await;
        let current = ports::routes();
        let mut next = current.clone();
        let mut servers: Vec<ServiceKind> = Vec::new();
        let mut notes: Vec<String> = Vec::new();
        // Servers a newly routed domain needs; started when they aren't running, since the
        // dev server is only reachable at its domain through them.
        let mut needed: Vec<ServiceKind> = Vec::new();
        for site in &sites {
            let own = settings.sites.get(&site.name);
            let files_only = site.language != sites::Language::Php && own.and_then(|s| s.app.as_ref()).is_some_and(|a| !a.serve);
            let status = statuses.iter().find(|s| s.name == site.name);
            let managed = status.filter(|s| matches!(s.state, "starting" | "running")).and_then(|s| s.port);
            // The project's processes listen on their own ports (Vite, an API); those go to
            // the domains pointing at them, not to the main domain, unless it's pinned there.
            let pinned = own.and_then(|s| s.route_port);
            let reserved: Vec<u16> = own.map(|s| s.processes.iter().filter_map(|p| p.port).filter(|p| Some(*p) != pinned).collect()).unwrap_or_default();
            let detected: Vec<ports::Detected> = found.get(&site.name).map(|d| d.iter().filter(|d| !reserved.contains(&d.port)).cloned().collect()).unwrap_or_default();
            let detected = detected.as_slice();
            if let (Some(s), Some(first)) = (status, detected.first())
                && s.state == "starting" {
                    let port = managed.filter(|p| detected.iter().any(|d| d.port == *p)).unwrap_or(first.port);
                    state.apps.mark_ready(&handle, &site.name, port).await;
                }
            // A main domain set by hand goes where it was told (sites::scan), not where ports lead.
            if own.is_some_and(|s| s.main.is_some()) {
                next.remove(&site.name);
                missing.remove(&site.name);
                continue;
            }
            let want = ports::choose(site.language, files_only, own.and_then(|s| s.route_port), managed, detected);
            if want.as_ref() == current.get(&site.name) {
                missing.remove(&site.name);
                continue;
            }
            if want.is_none() {
                let gone = missing.entry(site.name.clone()).or_default();
                *gone += 1;
                if *gone < 2 {
                    continue;
                }
            }
            missing.remove(&site.name);
            let note = match &want {
                Some(r) => format!("{} is now served from port {}", site.domain, r.port),
                None => format!("{}'s dev server stopped; serving its files", site.domain),
            };
            logs::system(&handle, logs::Level::Info, note.clone());
            notes.push(note);
            let kind = server_kind(site.server);
            if want.is_some() && !needed.contains(&kind) {
                needed.push(kind);
            }
            match want {
                Some(r) => next.insert(site.name.clone(), r),
                None => next.remove(&site.name),
            };
            if !servers.contains(&kind) {
                servers.push(kind);
            }
        }
        // Projects that disappeared lose their routes quietly.
        next.retain(|name, _| sites.iter().any(|s| &s.name == name));
        let detected_changed = ports::set_detected(found);
        if next != current {
            ports::set_routes(next);
            let _op = state.ops.lock().await;
            for kind in servers {
                if state.supervisor.is_running(kind).await {
                    // Without dropping connections where the server can; a restart otherwise.
                    let (settings, layout) = state.snapshot().await;
                    match state.supervisor.reload(&handle, kind, &settings, &layout).await {
                        Ok(true) => continue,
                        Ok(false) => {}
                        Err(e) => {
                            logs::system(&handle, logs::Level::Error, e.to_string());
                            continue;
                        }
                    }
                    state.supervisor.stop(&handle, kind).await;
                    state.supervisor.wait_stopped(kind).await;
                    let (settings, layout) = state.snapshot().await;
                    if let Err(e) = state.supervisor.start(&handle, kind, &settings, &layout).await {
                        logs::system(&handle, logs::Level::Error, format!("{} didn't restart with the new routes: {e}", kind.label()));
                    }
                } else if needed.contains(&kind) {
                    let (settings, layout) = state.snapshot().await;
                    if let Err(e) = state.supervisor.start(&handle, kind, &settings, &layout).await {
                        let note = format!("{} didn't start, so the domain can't reach the dev server yet: {e}", kind.label());
                        logs::system(&handle, logs::Level::Error, note.clone());
                        notes.push(note);
                    }
                }
            }
            // The notes say where domains moved (shown as notifications).
            let _ = tauri::Emitter::emit(&handle, "routes-changed", &notes);
        } else if detected_changed {
            let _ = tauri::Emitter::emit(&handle, "routes-changed", Vec::<String>::new());
        }
    }
}

fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    use tauri::{
        menu::{Menu, MenuItem, PredefinedMenuItem},
        tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    };
    let menu = Menu::with_items(
        app,
        &[
            &MenuItem::with_id(app, "open", "Open LampRabit", true, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "start", "Start all", true, None::<&str>)?,
            &MenuItem::with_id(app, "stop", "Stop all", true, None::<&str>)?,
            &MenuItem::with_id(app, "www", "Open www folder", true, None::<&str>)?,
            &MenuItem::with_id(app, "terminal", "Open terminal", true, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "quit", "Quit LampRabit", true, None::<&str>)?,
        ],
    )?;
    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip("LampRabit")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            let app = app.clone();
            match event.id.as_ref() {
                "open" => show_main(&app),
                "quit" => app.exit(0),
                id => {
                    let id = id.to_string();
                    tauri::async_runtime::spawn(async move {
                        let state = app.state::<AppState>();
                        match id.as_str() {
                            "start" => {
                                let _ = start_all(app.clone(), state).await;
                            }
                            "stop" => {
                                let _ = stop_all(app.clone(), state).await;
                            }
                            "www" => {
                                let (_, layout) = state.snapshot().await;
                                use tauri_plugin_opener::OpenerExt;
                                let _ = app.opener().open_path(layout.www().to_string_lossy(), None::<&str>);
                            }
                            "terminal" => {
                                let _ = open_terminal(state, None).await;
                            }
                            _ => {}
                        }
                    });
                }
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                show_main(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

/// Project processes that exited and should run again (apps.rs decides; this has the
/// settings to start them with).
fn spawn_process_restarts(app: &AppHandle) {
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let Some(mut requests) = apps::restart_requests() else { return };
        while let Some((project, process)) = requests.recv().await {
            let state = handle.state::<AppState>();
            let (settings, layout) = state.snapshot().await;
            if let Err(e) = state.apps.start_process(&handle, &settings, &layout, &project, &process).await {
                let key = apps::process_key(&project, &process);
                state.apps.record(&handle, &layout, &key, "error", format!("Couldn't restart {process}: {e}")).await;
            }
        }
    });
}

/// `--smoke-test`: the services through the real supervisor, without a window or the
/// user's settings (smoke.rs). Exits with the test's result.
fn run_smoke_test(mut context: tauri::Context<tauri::Wry>, options: smoke::Options) {
    context.config_mut().app.windows.clear();
    let settings = smoke::settings(&options);
    // Its own control file, so the command line it tests never meets a real install's app.
    // SAFETY: nothing else runs yet, so no other thread reads the environment.
    unsafe { std::env::set_var("LAMPRABIT_CONTROL_FILE", settings.root_dir.join("control.json")) };
    if let Err(e) = Layout::new(&settings.root_dir).ensure() {
        eprintln!("smoke test: can't use {}: {e}", settings.root_dir.display());
        std::process::exit(2);
    }
    let settings_path = settings.root_dir.join("settings.toml");
    let _ = settings.save(&settings_path);
    let app = tauri::Builder::default()
        .setup(move |app| {
            let root = settings.root_dir.clone();
            app.manage(AppState::new(settings, settings_path));
            app.manage(logs::LogHub::default());
            logs::set_system_file(app.handle(), Layout::new(&root).logs().join("lampRabit.log"));
            spawn_crash_restarts(app.handle());
            spawn_process_restarts(app.handle());
            spawn_process_schedules(app.handle());
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = control::serve(handle.clone(), control_dispatch).await {
                    eprintln!("smoke test: the control channel didn't start: {e}");
                }
                let code = smoke::run(handle, options).await;
                // The services are stopped by now. AppHandle::exit loses the code on Linux
                // (GTK's loop ends with 0), and CI needs it.
                std::process::exit(code);
            });
            Ok(())
        })
        .build(context)
        .expect("error while building LampRabit");
    app.run(|_, _| {});
}

/// The release build on Windows is a GUI program with no console of its own: for the
/// command line, write to the terminal that started it (when output isn't redirected to a
/// pipe or file, which it already reaches, as MCP clients do).
fn attach_console() {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::{
            Foundation::{GENERIC_READ, GENERIC_WRITE, INVALID_HANDLE_VALUE},
            Storage::FileSystem::{CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING},
            System::Console::{AttachConsole, GetStdHandle, SetStdHandle, ATTACH_PARENT_PROCESS, STD_ERROR_HANDLE, STD_OUTPUT_HANDLE},
        };
        let missing = |h: windows_sys::Win32::Foundation::HANDLE| h.is_null() || h == INVALID_HANDLE_VALUE;
        if !missing(GetStdHandle(STD_OUTPUT_HANDLE)) {
            return;
        }
        if AttachConsole(ATTACH_PARENT_PROCESS) == 0 {
            return;
        }
        let name: Vec<u16> = "CONOUT$\0".encode_utf16().collect();
        let out = CreateFileW(name.as_ptr(), GENERIC_READ | GENERIC_WRITE, FILE_SHARE_READ | FILE_SHARE_WRITE, std::ptr::null(), OPEN_EXISTING, 0, std::ptr::null_mut());
        if !missing(out) {
            SetStdHandle(STD_OUTPUT_HANDLE, out);
            SetStdHandle(STD_ERROR_HANDLE, out);
        }
    }
}

/// `--pg-elevated <pg_ctl> <data> <port> <log>`: starts PostgreSQL through pg_ctl (which
/// drops administrator rights PostgreSQL refuses to run with), then prints the server's log
/// as it grows and exits when the server does, so the supervisor can treat this process as
/// the server: output, health, crash restarts. Stopping goes through `pg_ctl stop`.
fn pg_elevated(args: Vec<String>) -> i32 {
    use std::io::{Read, Seek, SeekFrom, Write};
    let [pg_ctl, data, port, log] = args.as_slice() else {
        eprintln!("usage: --pg-elevated <pg_ctl> <data> <port> <log>");
        return 2;
    };
    let start = std::fs::metadata(log).map(|m| m.len()).unwrap_or(0);
    let options = format!("-p {port} -c listen_addresses=127.0.0.1 -c unix_socket_directories=");
    let mut cmd = std::process::Command::new(pg_ctl);
    cmd.args(["start", "-w", "-t", "60", "-D", data, "-l", log, "-o", &options]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(procgroup::CREATE_NO_WINDOW);
    }
    let started = cmd.status();
    let mut file = std::fs::File::open(log).ok();
    let mut offset = start;
    let mut echo = |file: &mut Option<std::fs::File>| {
        if let Some(f) = file.as_mut() {
            let mut chunk = String::new();
            if f.seek(SeekFrom::Start(offset)).is_ok() && f.read_to_string(&mut chunk).is_ok() && !chunk.is_empty() {
                offset += chunk.len() as u64;
                print!("{chunk}");
                let _ = std::io::stdout().flush();
            }
        }
    };
    if !started.is_ok_and(|s| s.success()) {
        echo(&mut file);
        eprintln!("PostgreSQL didn't start (pg_ctl start failed); its log is above.");
        return 1;
    }
    let pid: Option<u32> = std::fs::read_to_string(std::path::Path::new(data).join("postmaster.pid")).ok().and_then(|t| t.lines().next()?.trim().parse().ok());
    let Some(pid) = pid else {
        echo(&mut file);
        eprintln!("PostgreSQL started but wrote no postmaster.pid.");
        return 1;
    };
    while procgroup::alive(pid) {
        if file.is_none() {
            file = std::fs::File::open(log).ok();
        }
        echo(&mut file);
        std::thread::sleep(std::time::Duration::from_millis(300));
    }
    echo(&mut file);
    0
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // The Windows uninstaller's cleanup: no window, and before the single-instance plugin,
    // which would otherwise hand the call to a running copy.
    if std::env::args().any(|a| a == "--uninstall-cleanup") {
        uninstall::cli();
        return;
    }
    // The command line (cli.rs): `lamprabit` shims call the app with --cli. It talks to the
    // running app, so it comes before the single-instance plugin too.
    if std::env::args().nth(1).as_deref() == Some("--cli") {
        attach_console();
        std::process::exit(cli::main(std::env::args().skip(2).collect()));
    }
    // PostgreSQL under an elevated LampRabit (services.rs): no window, no single instance.
    if std::env::args().nth(1).as_deref() == Some("--pg-elevated") {
        std::process::exit(pg_elevated(std::env::args().skip(2).collect()));
    }
    let context = tauri::generate_context!();
    // Also before the single-instance plugin: a LampRabit already open must not take it.
    if let Some(options) = smoke::options() {
        match options {
            Ok(options) => run_smoke_test(context, options),
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(2);
            }
        }
        return;
    }
    tauri::Builder::default()
        // Must be registered first: a second launch hands over to the running copy and exits
        // before anything else (tray, services, log files) is set up.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_main(app)))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .on_window_event(|window, event| {
            // Closing keeps LampRabit (and its services) running in the tray.
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let state = window.state::<AppState>();
                let to_tray = tauri::async_runtime::block_on(async { state.settings.read().await.close_to_tray });
                if to_tray {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .setup(|app| {
            let config_dir = app.path().app_config_dir()?;
            let settings_path = config_dir.join("settings.toml");
            // Pre-release builds used the identifier "dev.lamprabit.app"; carry their settings over.
            if !settings_path.exists()
                && let Some(old) = config_dir.parent().map(|p| p.join("dev.lamprabit.app").join("settings.toml"))
                    && old.exists() {
                        let _ = std::fs::create_dir_all(&config_dir);
                        let _ = std::fs::copy(&old, &settings_path);
                    }
            let mut warnings = Vec::new();
            let (mut settings, broken) = Settings::load_or_recover(&settings_path);
            // A first run: no settings yet, and no LampRabit folder already where they point
            // (a reinstall keeps using its folder without asking).
            let fresh = !settings_path.exists() && !["bin", "www", "data"].iter().any(|d| settings.root_dir.join(d).exists());
            app.manage(FirstRun(std::sync::atomic::AtomicBool::new(fresh)));
            if let Some(aside) = broken {
                warnings.push(format!(
                    "Your settings file was damaged, so LampRabit started with default settings. The old file was kept as {}.",
                    aside.display()
                ));
            }
            if let Err(e) = Layout::new(&settings.root_dir).ensure() {
                warnings.push(format!("LampRabit can't use its environment folder {}: {e}. Choose another one in Settings → General.", settings.root_dir.display()));
            }
            runtimes::clean_removed(&Layout::new(&settings.root_dir));
            runtimes::migrate_flat_composer(&Layout::new(&settings.root_dir));
            reconcile_versions(&mut settings);
            if tidy_extension_choices(&mut settings) {
                let _ = settings.save(&settings_path);
            }
            app.manage(AppState::new(settings, settings_path));
            app.manage(logs::LogHub::default());
            app.manage(dumps::DumpStore::default());
            spawn_dump_listener(app.handle());
            {
                let root = app.state::<AppState>().settings.try_read().map(|s| s.root_dir.clone()).unwrap_or_default();
                logs::set_system_file(app.handle(), Layout::new(root).logs().join("lampRabit.log"));
            }
            {
                // Assistants' LampRabit entries follow this build: the copy they run (Windows)
                // is refreshed, and entries pointing at another program are pointed here.
                let handle = app.handle().clone();
                tauri::async_runtime::spawn_blocking(move || {
                    if let Err(e) = mcp_clients::refresh_launcher() {
                        logs::system(&handle, logs::Level::Warn, format!("Couldn't update the copy of LampRabit that AI assistants run: {e}"));
                    }
                    for name in mcp_clients::repoint() {
                        logs::system(&handle, logs::Level::Info, format!("{name}'s LampRabit entry now runs this LampRabit; restart {name} to use it."));
                    }
                });
            }
            logs::system(app.handle(), logs::Level::Info, format!("LampRabit {} started on {}", env!("CARGO_PKG_VERSION"), std::env::consts::OS));
            for w in &warnings {
                logs::system(app.handle(), logs::Level::Warn, w.clone());
            }
            {
                let root = app.state::<AppState>().settings.try_read().map(|s| s.root_dir.clone()).unwrap_or_default();
                let leftovers = services::reap_orphans(&Layout::new(&root));
                if !leftovers.is_empty() {
                    logs::system(
                        app.handle(),
                        logs::Level::Warn,
                        format!("Stopped services left running by an earlier session: {}", leftovers.join(", ")),
                    );
                }
                // After reaping, so a Garnet left running doesn't keep its files locked.
                if runtimes::remove_garnet(&Layout::new(&root)) {
                    warnings.push("Redis is now real Redis on every platform, replacing Garnet. Install it again from Services → Redis; phpRedisAdmin and redis-cli use it automatically.".into());
                }
            }
            app.manage(StartupWarnings(std::sync::Mutex::new(warnings)));
            if let Ok(settings) = app.state::<AppState>().settings.try_read() {
                refresh_shims(app.handle(), &settings);
            }

            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    let state = handle.state::<AppState>();
                    let (settings, _) = state.snapshot().await;
                    state.supervisor.check_health(&handle, &settings, &state.http).await;
                }
            });

            spawn_crash_restarts(app.handle());
            spawn_process_restarts(app.handle());

            tauri::async_runtime::spawn(watch_ports(app.handle().clone()));

            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut tailer = logs::Tailer::default();
                let mut polls: u32 = 0;
                loop {
                    let settings = handle.state::<AppState>().settings.read().await.clone();
                    let dir = Layout::new(&settings.root_dir).logs();
                    tailer.poll(&handle, &dir);
                    // About once a minute: keep log files to their size (Settings → Logs).
                    polls = polls.wrapping_add(1);
                    if polls % 150 == 1 {
                        let mb = |n: u32| u64::from(n.max(1)) * 1024 * 1024;
                        let (max, keep, total) = (mb(settings.log_max_mb), settings.log_keep, mb(settings.log_total_mb));
                        let dir2 = dir.clone();
                        let rotated = tauri::async_runtime::spawn_blocking(move || logs::rotate(&dir2, max, keep, total)).await.unwrap_or(0);
                        if rotated > 0 {
                            logs::system(&handle, logs::Level::Info, format!("Rotated {rotated} log file(s) over {} MB", settings.log_max_mb));
                        }
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
                }
            });

            setup_tray(app)?;
            // The control channel for the `lamprabit` command line and its MCP server, and the
            // MCP server over HTTP when it's switched on.
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = control::serve(handle.clone(), control_dispatch).await {
                    logs::system(&handle, logs::Level::Warn, format!("The command line's control channel couldn't start: {e}"));
                }
                let settings = handle.state::<AppState>().snapshot().await.0;
                if settings.mcp_server {
                    let started = match mcp_token(&handle) {
                        Ok(token) => mcp::start_http(settings.ports.mcp, token).await,
                        Err(e) => Err(e),
                    };
                    if let Err(e) = started {
                        logs::system(&handle, logs::Level::Warn, format!("The MCP server didn't start: {e}"));
                    }
                }
            });
            // A window bigger than the screen (1280×860 on a 1024×768 display) would open
            // partly off it, hiding its controls: fit it to the screen's work area first.
            if let Some(w) = app.get_webview_window("main")
                && let (Ok(Some(monitor)), Ok(size)) = (w.current_monitor(), w.outer_size()) {
                    let area = monitor.work_area().size;
                    if size.width > area.width || size.height > area.height {
                        let fitted = tauri::PhysicalSize::new(size.width.min(area.width * 95 / 100), size.height.min(area.height * 95 / 100));
                        let _ = w.set_size(fitted);
                        let _ = w.center();
                    }
                }
            // Started with the OS: stay in the tray.
            if std::env::args().any(|a| a == "--minimized")
                && let Some(w) = app.get_webview_window("main") {
                    let _ = w.hide();
                }
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let state = handle.state::<AppState>();
                if state.settings.read().await.start_services_on_launch {
                    logs::system(&handle, logs::Level::Info, "Starting services on launch");
                    let _ = start_all(handle.clone(), state).await;
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            runtime_info,
            set_site_worker,
            set_site_domains,
            set_site_main,
            project_folders,
            set_site_processes,
            first_run,
            finish_first_run,
            import_sources,
            import_folder,
            import_project,
            template_from_project,
            save_template,
            list_dumps,
            set_catch_dumps,
            apply_processes,
            project_file,
            save_project_file,
            database_status,
            create_project_database,
            switch_project_database,
            mcp_status,
            set_mcp_server,
            mcp_connect,
            project_doctor,
            doctor_fix,
            git_version,
            git_status,
            git_diff,
            git_stage,
            git_discard,
            git_commit,
            git_branches,
            git_branch,
            git_abort,
            git_continue,
            git_log,
            git_commit_details,
            git_commit_file_diff,
            git_repo_settings,
            git_set_repo_settings,
            git_sync,
            git_remote,
            git_set_identity,
            git_init,
            git_clone,
            git_cancel,
            git_stashes,
            git_stash,
            git_stage_hunk,
            git_resolve,
            git_tags,
            git_tag,
            git_revert,
            git_restore_file,
            set_project_env,
            ports_listening,
            set_site_php_ini,
            rename_project,
            delete_project,
            command_history,
            process_next_runs,
            process_start,
            process_stop,
            process_restart,
            processes_all,
            restart_frankenphp_workers,
            runtime_requirements,
            get_settings,
            save_settings,
            env_paths,
            list_services,
            start_service,
            stop_service,
            start_all,
            stop_all,
            uninstall_plan,
            uninstall,
            list_templates,
            open_templates_folder,
            app_info,
            app_save,
            app_start,
            app_stop,
            app_restart,
            app_install,
            app_statuses,
            app_logs,
            app_free_port,
            project_facts,
            set_site_route,
            project_commands,
            run_project_command,
            prepare_for_update,
            project_dependencies,
            dependencies_outdated,
            dependency_plan,
            run_dependency_action,
            stop_project_command,
            save_project_commands,
            apply_services,
            service_usage,
            check_port,
            port_status,
            stop_process,
            clear_problem,
            diagnostics,
            save_text,
            read_text,
            log_history,
            clear_logs,
            restart_service,
            list_sites,
            apply_sites,
            set_site_ssl,
            set_site_php,
            set_site_server,
            php_status,
            create_project,
            php_extensions,
            set_php_extension,
            install_php_extension,
            open_terminal,
            take_startup_warnings,
            open_in_editor,
            vc_runtime_status,
            vc_runtime_install,
            runtime_updates,
            node_status,
            node_available,
            node_install,
            node_use,
            node_uninstall,
            node_enable_package_managers,
            nvm_install,
            tunnel_start,
            tunnel_stop,
            tunnel_list,
            ngrok_token_saved,
            set_ngrok_token,
            trust_ca,
            list_databases,
            list_backups,
            backup_database,
            restore_backup,
            delete_backup,
            open_compass,
            check_move_target,
            environment_in_use,
            stop_environment_holder,
            move_environment,
            ollama_elsewhere,
            set_language_version,
            open_db_client,
            runtime_catalog,
            installed_runtimes,
            install_runtime,
            starter_plan,
            remove_runtime,
            set_active_version,
        ])
        .build(context)
        .expect("error while building LampRabit")
        .run(|app, event| {
            if let RunEvent::Exit = event {
                let state = app.state::<AppState>();
                tauri::async_runtime::block_on(async {
                    state.tunnels.stop_all().await;
                    state.apps.stop_all().await;
                    state.supervisor.shutdown().await;
                });
            }
        });
}
