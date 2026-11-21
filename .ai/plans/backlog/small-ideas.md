# Small ideas and side projects

Ideas too small for their own file. Source: [spec](../source-specs/2026-11-21-feature-context.md) "Gap analysis" and "Product snapshot". Each line says what exists and how feasible it is (checked 2026-11-21).

## Ideas

- **"Open in LampRabit" badge / deep link** (`lamprabit://clone?repo=…`) that starts the existing Clone flow (Git tab, clones into `www/`). Feasible with `tauri-plugin-deep-link`; validate the URL with the same rules as `git.rs` (no option-like names, no `ext::`). S–M.
- **Branch-aware database snapshots:** back up on git branch switch, restore on switching back. Built on existing backups/restore (`backups.rs`) and the Git tab. Feasible; opt-in per project, since large databases make it slow and costly in disk space. Only catches switches made through LampRabit unless a git hook is installed (ask first). M.
- **Debugging toggles:** Xdebug per site already exists (`SiteSettings.xdebug`). Add Node `--inspect` as a process option and generate a VS Code `launch.json` (PHP Xdebug, Node attach). S.
- **Per-service config files list** with Open/Reveal/Copy path: part of [command-palette.md](command-palette.md).

## Side projects (the user's, separate repositories)

- **KarotX:** the user's Go single-binary database client, to ship as a LampRabit service. Fits phase 27 once local/registry manifests work (steps C/D); it then replaces "database client" in the Tools tab.
- **WarenX:** a Chrome extension exposing DOM, console and network to MCP. Separate product; LampRabit's MCP server could later proxy its tools.

## Verify before planning (unknowns from the spec)

- Update rollback and settings migration: the updater exists (GitHub releases); rollback doesn't.
- Security defaults: services bind 127.0.0.1 (yes); database root passwords: local root without password today, by design for local dev.
- i18n: none. Accessibility: contrast/focus work was done in Nov 2026; no audit yet.
- LampRabit's own startup time and idle RAM: never measured.

## Considering (from the website, not planned)

LAN phone testing (now in [mobile-dev-hub.md](mobile-dev-hub.md)), profiler view, SSH/deploy workflow builder (see [remote.md](remote.md) and the roadmap's "Later"), community plugin SDK ("tools and plugins", after phase 27).
