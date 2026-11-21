# Phase 20: headless core and the `lamprabit` CLI

Status: **done**. Listed in the [roadmap](../roadmap.md).

**Done (2026-11-12):** the running app's control channel (`control.rs`: 127.0.0.1, a token in `control.json`, one JSON line per request, `logs.follow` streams) and `lamprabit` (`cli.rs`, the app run with `--cli` through a shim; Windows attaches to the calling terminal). Commands as listed below plus `doctor --fix/--fix-all`, `project create`, `templates`, `git`; `--json`, `--yes`, `--dry-run`, `--no-launch`; exit codes 0/1/2/3/4. It starts LampRabit in the background when it isn't running. Reference in docs/cli.md. The smoke test drives the channel on every OS.

- One backend process owns the services. The app can run without a window (tray only, `--background`); the window and the CLI are both clients of it, so they always show the same state.
- `lamprabit` (installed as a shim like `php`) connects over a local socket / named pipe with a token in the app data folder. If no backend is running, it starts one in the background. Opening the app later attaches to the same backend (the single-instance plugin already routes a second launch).
- Commands: `status`, `service start|stop|restart`, `project list|start|stop|open`, `runtime list|install|use|remove`, `logs [--follow] [--source]`, `doctor`, `profile`, `config`. `--json` everywhere, stable exit codes, `--yes`/`--dry-run` for destructive actions.
- Commands reuse the Tauri command code paths (no second implementation): split the bodies in `lib.rs` into functions both call.
