# Current status

Last updated 2026-11-26. Keep this short: where things stand and what comes next. History goes in [history.md](history.md).

## Released

- **0.6.0** is the current stable release, marked Latest on GitHub on 2026-11-14 (tag `v0.6.0` on `main`, PR #23). macOS is supported from this version.
- Remaining macOS checks by hand: Redis, Memcached, MySQL, folder names with spaces, CA removal, no leftover processes after quitting.

## In progress: phase 27, pluggable services (branch `dev-1.0`)

Plan: [../plans/active/phase-27-pluggable-services.md](../plans/active/phase-27-pluggable-services.md). Decisions: [../context/decisions/services-manifests.md](../context/decisions/services-manifests.md).

| Step | State |
|---|---|
| A. Engine and 24 manifests | done, committed in `f18e816` |
| B1. Downloads from YAML (17 of 24 services) | done, committed in `f18e816` |
| B2. `settings:` options in the Services page | done, committed in `f18e816` |
| B3. Services without an enum variant (`ServiceKind::Plugin`, `Ports.extra`) | done, commit `981d67d` |
| C. Local services from a file, `lamprabit service check/try` | done, commit `e2700d6` |
| D. Registry repo, signed index, marketplace | **later** (the user, 2026-11-21) |
| KarotX bundled as LampRabit's own database client (`connects`) | done 2026-11-21; services file (every local database) 2026-11-22; repo and v0.1.0 release public, installs checked 2026-11-22 |
| E1–E6: every service (and KarotX) defined only by YAML; only the site engine (`engine:`) stays Rust | done, commits `469dca5`, `6d37942`, `d272dba`, `508bdb0` |
| E7a keywords/docs/config files, E7b backup hooks (+ smoke round trip, MySQL restore fix) | done, `1325049`, `0785179` |
| E7c `kind: cli`/`tool`, `commands` (shims), `source.prefix`, `{{root}}`; Bun, Deno, uv, mongosh, Adminer moved to YAML | done 2026-11-26 |
| E7 later: move Composer, SQLite, Go, Maven, Gradle, Java, Python, Git, conda, Erlang, Compass, Mongotools, phpMyAdmin, phpRedisAdmin, phpPgAdmin (vendor indexes or source zips without checksums: need new source types) | open |
| `scope: project`, connection adapters | **with** Firebase/PocketBase and service binding (the user, 2026-11-26: "start the other things one by one") |
| Add/Remove a service in the running app (add, install, remove version, remove service) | done: the user tested it 2026-11-26, works; New Project's auto-install also tested and works |
| macOS through CI (runs when `dev-1.0` is pushed) | **open: waiting for the user's OK to push** |

## Next release: 0.8 (not yet)

The user, 2026-11-24: 0.8 ships from `dev-1.0` with a lot more features, **only once they're all built and tested**. Commits carry no AI attribution.

Done on `dev-1.0` since phase 27: UI polish and Home's status strip + Needs attention (2026-11-23), fixes from the owner's notes (selection Start/Stop, phpMyAdmin after install, other stacks' Windows services and PATH, Windows Security exclusion), **phase 28 command palette**, complete (2026-11-24, [plan](../plans/active/phase-28-command-palette.md)). Usability fixes from the user's two 2026-11-24 lists (ports, admin kill, .test domains, NVM, live refresh, Cloudflare on your domain, default database app on Services, dependent services, pgAdmin 9.19 skipped, pip/uv/conda and Gradle projects), uncommitted: [session](../sessions/2026-11-24-usability-fixes.md); code signing stays in the backlog (the user, 2026-11-26; [backlog](../plans/backlog/usability-2026-11-24.md)). The owner's 12-item list of 2026-11-25 (title bar, Laravel serve commands, links start their service, Git branch pull/push and tags, Python/templates, templates from CLI/MCP, Node on a fresh PATH, Home layouts), uncommitted: [session](../sessions/2026-11-25-owner-list.md). 2026-11-26: compact tiles with detail, menus that no dimmed card can fade, a Git Stashes tab, PostgreSQL extensions per database, uncommitted: [session](../sessions/2026-11-26-tiles-stashes-pg-extensions.md). Everything since 0.6.0 is listed in [docs/since-0.6.md](../../docs/since-0.6.md).

## Next after phase 27

The user's feature context of 2026-11-21 was filed into the backlog with feasibility checked against the code. The order, confirmed by the user on 2026-11-21, is in the [roadmap](../plans/roadmap.md) under "Next": command palette → project model (endpoints, kinds) → Tools tab + dynamic dashboard → deep link and branch-aware snapshots → .NET → Flutter/Mobile → KarotX, Docker, plugin SDK. Linked projects from any folder/drive are approved (unordered).

## 2026-11-21

The AI context was reorganised into `.ai/`; the developer feed and Playground ideas were dropped; the 2026-11-21 feature context was filed (decisions: [project-model](../context/decisions/project-model.md), [ux-principles](../context/decisions/ux-principles.md), [product](../context/decisions/product.md)).
