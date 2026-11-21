# Decisions: Product direction

The user's decisions and the reasons behind them. Don't reverse one without asking the user; when the user changes one, edit it here with the date.

- **Product roadmap** (the user's choices, 2026-10-29): phases 10+ in the roadmap (`.ai/plans/roadmap.md`), in order (macOS parity first, all services before CLI/MCP and Git). No in-app database browser for now (Adminer/phpMyAdmin/phpRedisAdmin stay); uninstall keeps only `www/` and `data/`; the tab bar is movable but there are no dockable panels; FrankenPHP and Caddy are back.
- **Dropped ideas** (2026-11-21, the user's choice): the developer feed (Hacker News/RSS reader) and the "just for fun" Playground. Don't propose them again.
- **Homes and guardrails** (2026-11-21, the user's choices): five homes (Projects, Services, Devices, Tools, Remote), runtimes stay inside Services, no Runtimes tab; LampRabit stays a local dev environment, not a Docker replacement or generic process manager. Details: [ux-principles.md](ux-principles.md), [project-model.md](project-model.md).
- **Remote monitoring is local-only** (2026-11-21): deployed-app health and logs are watched only while LampRabit runs; nothing runs while the machine sleeps or is off, and the UI says so. On wake, logs for the missed period are backfilled over SSH and the gap is marked; health pings can't be backfilled. No always-on server or hosted service.
- **Local BaaS** (2026-11-21, confirmed): ship both the official Firebase Local Emulator Suite and PocketBase, as project-scoped services. Never build a Firebase clone.
- **Service manifests stay YAML** (decided 2026-11-15 and already built): the 2026-11-21 spec suggests TOML for consistency with `lamprabit.toml`, but that predates knowing YAML was in place. Confirmed again by the user on 2026-11-21: keep YAML.
