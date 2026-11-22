# Current status

Last updated 2026-11-21. Keep this short: where things stand and what comes next. History goes in [history.md](history.md).

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
| KarotX bundled as LampRabit's own database client (`connects`) | done 2026-11-21; services file (every local database) 2026-11-22. Repo is public, but **the v0.1.0 release is still a draft**: publish it before anyone can install KarotX |
| E1–E6: every service (and KarotX) defined only by YAML; only the site engine (`engine:`) stays Rust | done, commits `469dca5`, `6d37942`, `d272dba`, `508bdb0` |
| E7a keywords/docs/config files, E7b backup hooks (+ smoke round trip, MySQL restore fix) | done, `1325049`, `0785179` |
| **E7 rest: `scope: project`, connection adapters, `kind: tool`/`cli` (PHP tools, CLIs are runtimes)** | **open: waiting for the user's call** (scope/adapters have no consumer until Firebase/PocketBase and service binding) |
| Leftovers: clicking Add/Remove in the running app; macOS (CI runs when the branch is pushed) | **open** |

## Next after phase 27

The user's feature context of 2026-11-21 was filed into the backlog with feasibility checked against the code. The order, confirmed by the user on 2026-11-21, is in the [roadmap](../plans/roadmap.md) under "Next": command palette → project model (endpoints, kinds) → Tools tab + dynamic dashboard → deep link and branch-aware snapshots → .NET → Flutter/Mobile → KarotX, Docker, plugin SDK. Linked projects from any folder/drive are approved (unordered).

## 2026-11-21

The AI context was reorganised into `.ai/`; the developer feed and Playground ideas were dropped; the 2026-11-21 feature context was filed (decisions: [project-model](../context/decisions/project-model.md), [ux-principles](../context/decisions/ux-principles.md), [product](../context/decisions/product.md)).
