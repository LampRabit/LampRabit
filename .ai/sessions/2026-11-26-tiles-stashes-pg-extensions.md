# 2026-11-26: compact tiles, Git stashes, PostgreSQL extensions, opaque menus

## What the user asked

With two screenshots of Home's compact tiles and an open ⋮ menu:

1. "these compact mode, looks like a piece of shit. add a little details."
2. "Add option to manage the git stashes."
3. "postgres contains extensions so if possible to add extensions option if yes, add it also check for other services if they have extensions options."
4. "trasnparent menu looks shitter, make it proper white so background doesn't interupt."

"Do that then we start our left things." (Next: the open items in [status](../progress/status.md).)

## Done (uncommitted, on `dev-1.0`)

- **Menus** (`components/Menu.tsx`): the list is drawn in a portal on `<body>` (fixed, z-50), placed under the button or above it when there's no room, and closes on scroll/resize. The cause: the MariaDB tile was dimmed (`opacity-75`, not in the active profile) and the menu inherited it. Labels no longer wrap. Pitfall recorded.
- **Compact tiles** (`pages/Overview.tsx`): always show the status dot and word plus the port, a line of detail (CPU and RAM while running, else the version, else what the service is), Start/Stop with their labels, the open button while running, and copy.
- **Git stashes**: a **Stashes** tab (count badge) in the project's Git view. Stash with a name, choosing all changes and new files / tracked files only / staged only (`git stash push --staged`, git 2.35+); a list with the branch each was made on; the selected stash's files (new files from its third parent marked U) and each file's diff; Restore (pop), Apply and keep, New branch (`git stash branch`), Delete (confirmed), Clear all (confirmed). The Changes view keeps the Stash button and shows the newest stash with Restore and "Manage stashes". Backend: `git::stash_files`, `git::stash_file_diff`, `StashAction::Branch`/`Clear`, `Save.staged`; commands `git_stash_files`, `git_stash_file_diff`. Covered by `works_on_a_real_repository`.
- **PostgreSQL extensions** (`components/PgExtensionsPanel.tsx` on PostgreSQL's Services page, while it runs): pick a database, see its extensions (version, schema, Update when the server has a newer one), switch popular ones on/off, search or show all. Enable = `CREATE EXTENSION IF NOT EXISTS … CASCADE`, off = `DROP EXTENSION` (no cascade, PostgreSQL's refusal names what depends on it), update = `ALTER EXTENSION … UPDATE`. plpgsql shows "Built in". `pg_stat_statements`, `auto_explain`, `pg_prewarm` are marked as needing `shared_preload_libraries`. Backend: `manage::pg_extensions`/`set_pg_extension`, commands `pg_extensions`/`set_pg_extension`. The installed 19.0 Windows build ships 50 extensions; it doesn't include uuid-ossp, PostGIS or pgvector (the popular list only shows what the server has).
- **Other services' extensions, checked:** PHP already has its extensions dialog. MariaDB/MySQL have plugins, not extensions, and nothing a project usually needs. Redis modules (RedisJSON, RediSearch) aren't in the builds LampRabit installs (not checked further). MongoDB, Memcached and Mailpit have none. So PostgreSQL is the only one that got a panel.
- Screenshot harness: `pg=1` adds a running PostgreSQL; new shots `home-compact-light`, `git-stashes`, `services-postgres`; `height=` sets the window height.

Checks: `tsc`, `npm test` (102), `cargo test --lib manage:: git::` pass. Not tried against a running PostgreSQL (it was stopped); the SQL is the standard catalog query.

## Later the same day

The user: "commit current changes, keep the code signin as backlog and start the other things one by one. start by 27 phase leftovers. let me know if something needed from myside." Then: "Whie creating new project if a dependancy is missing, instead auto-install it instead if not installed."

- Committed the three sessions' work (`29b678c` code, `cde75bc` docs and `.ai/`). `Scrumpad.MD` (the user's scratch file) left uncommitted.
- Code signing stays in the backlog.
- Phase 27 E7c done, commit `2e8db7f`: `kind: cli`/`tool`, `commands`, `source.prefix`, `{{root}}`; Bun, Deno, uv, mongosh, Adminer in YAML (details in the phase file).
- Add/Remove in the running app: computer use isn't available in this session, so it's waiting on the user (a test file was prepared; the CLI's `service check` passes on it). macOS CI needs `dev-1.0` pushed: asked the user.
- New Project installs missing dependencies (decision in projects.md), `lib/projectNeeds.ts` with tests, `project_requirements` command.

## Next

Phase 27: the user's Add/Remove click test and the push for macOS CI; then E7's remaining runtimes (new source types). Then the roadmap's next item, the project model (endpoints and kinds).
