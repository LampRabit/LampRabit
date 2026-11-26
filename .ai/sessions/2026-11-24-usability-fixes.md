# Session 2026-11-24: usability and flow fixes

## What the user asked

Fix eleven usability issues, saved verbatim in [../plans/source-specs/2026-11-24-usability-issues.md](../plans/source-specs/2026-11-24-usability-issues.md).

## Done (uncommitted on `dev-1.0`)

- **2, 4: freeing a port.** `stop_process` retries as administrator when a plain kill is refused (UAC `Stop-Process`, `pkexec kill`, macOS `osascript … with administrator privileges`). PID 4 (http.sys, what holds Apache's port 80) is now stoppable: `net stop http /y` as administrator. One shared `FreePort` button (`HomeStatus.tsx`) is used by Needs attention, the Home tile, `ProblemPanel` and Settings → Ports (which before only showed the fix in a tooltip).
- **3: stale port error after changing the port.** `Supervisor::list` drops a port problem once its port is free or no longer the service's; Home polls services with the ports (5 s while visible).
- **5: conflicts on tiles.** The tile's pill says "Port N in use"; `conflictFor` checks every port of the service (https, `<kind>.<name>`, second servers) and counts another LampRabit service on the port as a conflict. The tile's conflict box has Stop… and Change port.
- **6: .test domains.** Manifests with `open:` (ClickHouse, MinIO, RustFS, RabbitMQ, Meilisearch) are served at `<id>.<tld>` like `ui:` services; Mailpit opens at `mail.<tld>`. localhost stays only as the fallback when the hosts file isn't updated.
- **7: pgAdmin.** Its link is `pgadmin.test`, which needs the web server. Every in-app `openUrl` now goes through `lib/open.ts`, which calls `prepare_url`: for a `.<tld>` host it starts the primary web server or reloads it when its config lacks the domain. `tool_domain_service` is now any manifest with `ui:`/`open:` (KarotX was missing). Not reproduced on a machine with pgAdmin installed.
- **8: NVM.** nvm-windows' versions are read from `root:` in its `settings.txt` (was assumed to be `NVM_HOME`). A Node.js install earlier in the system PATH than `NVM_SYMLINK` is shown in Needs attention with "Put NVM first" (`put_node_first`, UAC).
- **9: real time.** Every settled mutation invalidates all queries except slow network catalogs (`main.tsx`); queries refetch on window focus.

## Waiting for the user's answers

1 (default client per service, desktop apps), 10 (Cloudflare named tunnels on the user's own domain) and 11 (signing): filed in [../plans/backlog/usability-2026-11-24.md](../plans/backlog/usability-2026-11-24.md) with the questions.

## Later the same day

The user answered: (1) a default among the web tools per database, (10) yes plus the Cloudflare sign-in, (11) minisign against SmartScreen. Built 1 and 10 (decisions in databases.md and app-and-platform.md); explained that minisign can't affect SmartScreen and listed what can.

## Next step

The user tests on Windows (port 80 held by http.sys, a port held by an elevated process, pgAdmin from Home), then commit.

## Second list (same day, another session)

The user's second list is saved verbatim at the end of [../plans/source-specs/2026-11-24-usability-issues.md](../plans/source-specs/2026-11-24-usability-issues.md). The session first reviewed the uncommitted work above (nothing reverted; one clippy fix in `tunnels.rs`).

- **1. Dependent services**: a notice with Start on a service's page when something it needs is stopped (`NeedsRunning.tsx`), and a Needs attention item on Home while it runs without it; rule in `lib/dependencies.ts`. Installing is still Related services. Decision in services-manifests.md.
- **2. Signing**: not built. A self-signed certificate helps neither SmartScreen nor Gatekeeper; the macOS app is already ad-hoc signed. Recorded in app-and-platform.md; waiting for a choice (SignPath, Azure Trusted Signing, OV certificate; Apple Developer ID for macOS).
- **3. pgAdmin**: the cause was upstream. pgAdmin 9.19's PyPI wheel ships without `static/js/generated` (0 files; 9.18 and 9.17 have 49). New `skip:` for download sources; pgAdmin lists every PyPI release and skips 9.19 (runtimes-and-downloads.md, pitfalls.md, docs/services.md). An installed 9.19 must be removed and 9.18 installed.
- **4. Default database app**: now `Settings.db_clients` (`set_db_client`), shown on every database's and database client's page in Services, and in Settings → Databases; KarotX when nothing is chosen. Home's ⋯ menu no longer changes the default. Decision in databases.md.
- **5. Package managers**: Python templates offer pip, uv or conda; Laravel's frontend step uses the chosen Node package manager; Spring Boot (Gradle) template added. Decision in projects.md.
- Screenshot harness: `service=<id>` opens one service; scenes `services-database`, `services-client`.

Checks: `cargo test --lib` (165), `npx tsc`, `npm test` (102), `npm run screenshots` all pass.

## Next step (updated)

The user tests on Windows: remove pgAdmin 9.19 and install 9.18 from Services, then pgadmin.test; the Opens in / Default app for panels; a Flask project with uv and with conda; then commit everything on `dev-1.0`.
