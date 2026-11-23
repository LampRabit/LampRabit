# Decisions: App shell, install location, logs and profiles

The user's decisions and the reasons behind them. Don't reverse one without asking the user; when the user changes one, edit it here with the date.

- **Single instance**: `tauri-plugin-single-instance` is registered first; a second launch shows the existing window.
- **Moving the LampRabit folder** (`relocate.rs`, 2026-11-01): moves `uninstall::OWNED` + `KEPT` together, all or nothing. Never store an absolute path under the root in a place that isn't regenerated on start; if one is unavoidable (Python venvs), repair it in `relocate::repair_venvs`.
- **Uninstall** (`uninstall.rs`, 2026-10-30): deletes only the root folders LampRabit creates (`OWNED`: bin, etc, logs, tmp, ssl, shims), never "everything but www/data", since the root may be a folder with other things in it. Keeps `www/` and `data/`. Also undoes the hosts block, CA trust, PATH, ngrok token and Linux sysctl file. Windows runs it from the NSIS pre-uninstall hook (`windows/hooks.nsh` → `--uninstall-cleanup`, handled in `run()` before the single-instance plugin), skipped for updates (`/UPDATE`); other OSes use Settings → Troubleshooting. A new root folder or outside change must be added there too.
- **Setup guide** (`FirstRunDialog.tsx`, 2026-11-12, reworked 2026-11-13 by the user's choice): folder (first run) → "What do you build?" (several usages, each with its recommended runtimes, shown by the Services page's categories; others added from their category; planned one component at a time by `starter_plan`, which reuses `catalog::runtime_action`) → install → import → first project (the name step was removed with Home's greeting, 2026-11-23). Settings → General reopens it (`setupGuideOpen: "guide"`) or only its import step (`"import"`).
- **Dump catcher** (2026-11-05): opt-in (`catch_dumps`), an `auto_prepend_file` that sets VarDumper's server mode for web requests only, a listener on `ports.dumps` (9912), and a PHP-unserialize decoder for VarDumper's `Data` (`dumps.rs`; arrays are `[class => position]`, checked against real output). Logs rotate by copy-then-truncate (`logs::rotate`, Settings → Logs).
- **Stack profiles**: `Settings.profiles` (name + services). The active profile is a per-device view preference (localStorage). `apply_services(start, stop)` backs profiles, selection actions and Start/Stop all.

## Winning over other stacks on Windows (the user's request, 2026-11-23)

"Make exceptions when any of other app installed, override the things to make LampRabit working." Each override asks for admin rights once (UAC), takes a second click in the UI, and is undone on uninstall (`overrides.rs`):

- A port held by another stack's **Windows service** (XAMPP/WAMP Apache, a MySQL installer's service, IIS `W3SVC` behind http.sys): "Stop the <name> service" stops it and sets it to start by hand. Killing the process isn't enough; Windows restarts it. `PortOwner.service` names it.
- **System PATH comes before the user PATH**: another install's `php`/`composer` (e.g. `C:\xampp\php`) answers first in terminals. Home's Needs attention offers "Put LampRabit first", which puts the shims folder first in the system PATH; turning "Add to PATH" off or uninstalling takes it out again.
- **Windows Security** scans every file Composer writes, so `composer install` and the autoload step stall: Settings → Troubleshooting → "Exclude LampRabit" excludes the LampRabit folder and Composer's cache (`%LOCALAPPDATA%\Composer`); a marker file (`defender-exclusion.txt` in the LampRabit folder) lets uninstall remove it.
