# Project model: endpoints and kinds

Home: Projects. Decisions: [../../context/decisions/project-model.md](../../context/decisions/project-model.md). Source: [spec](../source-specs/2026-11-21-feature-context.md) "Core architecture". Status: **partly built**. Size: L.

This is the foundation for .NET, Flutter, mobile, serverless and service binding, so it comes before them.

## Already in LampRabit

- Processes per project (command, port, `at_launch`, crontab `schedule`, restart, crash toasts), in the Commands tab and `lamprabit.toml`.
- Domains and subdomains pointing at files, a port or a process; live port discovery routes a domain to whatever the project listens on (`ports.rs`).
- `sites::Language` (php, node, static, java…) and `ServeMode` decide serving; the doctor and Overview use the language.
- Gap: there is no explicit "endpoint" object or "kinds"; web panels show for every project; projects are only the folders in `www/` (see [projects-anywhere.md](projects-anywhere.md)).

## Feasibility

Feasible, mostly a refactor of existing pieces: an endpoint is today's "process with a port + a domain pointing at it". Risk is migration: existing `lamprabit.toml` files and site settings must keep working unchanged.

## Plan

1. Model `endpoints[]` on processes (port, http|tcp, expose vhost/ssl/tunnel); map current domain/port settings onto it with no file changes needed.
2. Render vhost, SSL, Share, "Open" per endpoint; hide them for projects without one.
3. Detectors set `kinds` (heuristics in the spec: `Microsoft.NET.Sdk.Web`, `bin` in package.json, `[project.scripts]`, Electron/Tauri → Desktop, Celery/queue workers → worker…), confirmed in the create/import wizard and editable in Settings.
4. Kind presets: Web/API add an endpoint, worker gets autostart + restart, CLI gets "Run with args" + optional shim, Package gets build/test/pack/publish + local link, Desktop gets run/build/package.
5. Extend `lamprabit.toml` with `kinds`, endpoints and runtimes (still applied only from Settings).

## Open questions

- Which existing panels move per endpoint first (domains, HTTPS, Share)?
