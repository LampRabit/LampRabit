# LampRabit AI context: index

Everything AI agents need for this project lives in `.ai/`. Start here: find the row for what you need and open only that file. Rules for keeping this folder up to date are in [instructions/agent-rules.md](instructions/agent-rules.md). **Read those first.**

## Start of every session

1. [instructions/agent-rules.md](instructions/agent-rules.md): how to read, record and organise context (required)
2. [progress/status.md](progress/status.md): where the work stands and what's next
3. The decision file for the area you're about to touch (below)

## Find it here

| I need… | File |
|---|---|
| What LampRabit is, the code layout, key modules | [context/project-overview.md](context/project-overview.md) |
| How the maintainer likes to work, plan, review and release | [context/working-with-the-user.md](context/working-with-the-user.md) |
| Where each runtime is downloaded from | [context/download-sources.md](context/download-sources.md) |
| Code style, UI tokens, keyboard, dialogs | [instructions/conventions.md](instructions/conventions.md) |
| Which checks to run, screenshots, Linux testing in WSL | [instructions/checks.md](instructions/checks.md) |
| Traps already hit (sysinfo threads, CRLF, Vitest in Git Bash…) | [instructions/pitfalls.md](instructions/pitfalls.md) |
| Current state, the active branch, the next step | [progress/status.md](progress/status.md) |
| Timeline of releases and milestones | [progress/history.md](progress/history.md) |
| What was asked in a past session | [sessions/README.md](sessions/README.md) |

### Decisions by area (`context/decisions/`)

Read the one for the code you're changing. Don't reverse a decision without asking the user.

| Area | File | Covers |
|---|---|---|
| Runtimes and downloads | [runtimes-and-downloads.md](context/decisions/runtimes-and-downloads.md) | Own CI builds for Linux/macOS, Composer, Redis, Memcached, phpRedisAdmin, adding a runtime, catalog cache, macOS via CI |
| Databases | [databases.md](context/decisions/databases.md) | MySQL next to MariaDB, default database, PostgreSQL, MongoDB, backups, PostgreSQL elevated on Windows |
| Web servers | [web-servers.md](context/decisions/web-servers.md) | Several web servers (Apache, Nginx, Caddy, FrankenPHP, Tomcat), ports below 1024 on Linux |
| Projects | [projects.md](context/decisions/projects.md) | Templates, serving and port routing, Sites page, Commands and processes, `lamprabit.toml`, template editor, imports, Dependencies tab |
| App and platform | [app-and-platform.md](context/decisions/app-and-platform.md) | Single instance, moving the folder, uninstall, setup guide, dump catcher, stack profiles |
| Integrations | [integrations.md](context/decisions/integrations.md) | Git, project doctor, CLI, MCP and AI assistants |
| Pluggable services | [services-manifests.md](context/decisions/services-manifests.md) | YAML manifests, LampRabit/Services registry, signing, marketplace (phase 27) |
| Product direction | [product.md](context/decisions/product.md) | Roadmap order, what's deliberately not built, dropped ideas, homes, local-only monitoring, YAML stays |
| Project model | [project-model.md](context/decisions/project-model.md) | Not web-only: Project → Processes → Endpoints, kinds as UI presets, no type branching, guardrails |
| UX principles | [ux-principles.md](context/decisions/ux-principles.md) | Five homes (no Runtimes tab), command palette, "where is it", progressive disclosure, nudges, definition of done for every feature |

### Plans (`plans/`)

| What | File |
|---|---|
| Roadmap: every phase with its status and product-level decisions | [plans/roadmap.md](plans/roadmap.md) |
| **Active:** phase 27, pluggable services (branch `dev-1.0`) | [plans/active/phase-27-pluggable-services.md](plans/active/phase-27-pluggable-services.md) |
| **Active:** phase 28, command palette (branch `dev-1.0`) | [plans/active/phase-28-command-palette.md](plans/active/phase-28-command-palette.md) |
| Completed phases (6, 7, 9–26), one file each | [plans/completed/](plans/completed/) |
| **Order** of the backlog (confirmed 2026-11-21) | [plans/roadmap.md](plans/roadmap.md) "Next" |

Backlog (`plans/backlog/`), each with what exists today, feasibility checked against the code, plan and open questions:

| Feature | File | State |
|---|---|---|
| Project model: endpoints and kinds | [project-model.md](plans/backlog/project-model.md) | partly built |
| Tools tab: build order (41 tools, S→XL) | [tools-tab-build-list.md](plans/backlog/tools-tab-build-list.md) | not started |
| Tools tab: research and per-tool detail | [tools-tab.md](plans/backlog/tools-tab.md) | research |
| Dynamic dashboard (Home redesign) | [dynamic-dashboard.md](plans/backlog/dynamic-dashboard.md) | redesign |
| ASP.NET Core / .NET | [dotnet.md](plans/backlog/dotnet.md) | not started |
| Mobile Dev Hub, Dart/Flutter, phone ↔ local backend, LAN testing | [mobile-dev-hub.md](plans/backlog/mobile-dev-hub.md) | not started |
| One-click service binding to projects | [service-binding.md](plans/backlog/service-binding.md) | partly built |
| Local BaaS (Firebase emulators, PocketBase), Stripe | [local-baas-payments.md](plans/backlog/local-baas-payments.md) | not started, confirmed |
| Projects from any folder/drive, offline recovery | [projects-anywhere.md](plans/backlog/projects-anywhere.md) | approved |
| WebSockets: proxy, tester, WS doctor | [websockets.md](plans/backlog/websockets.md) | proxy mostly shipped |
| Share a site: QR, expiry, password, base-URL hints | [share-improvements.md](plans/backlog/share-improvements.md) | shipped; follow-ups |
| Remote: SSH/SFTP/FTP, deployed app logs and health | [remote.md](plans/backlog/remote.md) | not started |
| Docker config builder and compose import | [docker.md](plans/backlog/docker.md) | not started |
| Remote development and collaboration | [collaboration.md](plans/backlog/collaboration.md) | idea |
| Serverless local development | [serverless.md](plans/backlog/serverless.md) | exploratory |
| Small ideas (deep link, branch-aware snapshots, debug toggles), KarotX, WarenX, unknowns | [small-ideas.md](plans/backlog/small-ideas.md) | ideas |
| Product improvements, add-ons, accessibility (2026-11-14 notes) | [product-improvements.md](plans/backlog/product-improvements.md) | ideas |

Source specs (kept verbatim; the decisions and plans above are taken from them):

| Spec | File |
|---|---|
| Product spec exported from another LLM (2026-10-29) | [2026-10-29-llm-product-spec.md](plans/source-specs/2026-10-29-llm-product-spec.md) |
| Feature planning context (2026-11-21): project model, UX principles, Features 1–17, Tools list | [2026-11-21-feature-context.md](plans/source-specs/2026-11-21-feature-context.md) |

### Tools

| What | File |
|---|---|
| Export the user's prompts from Claude Code transcripts | [tools/export-prompts.py](tools/export-prompts.py) |

## Not in here

User-facing documentation stays where it was: [README.md](../README.md), [CHANGELOG.md](../CHANGELOG.md), [CONTRIBUTING.md](../CONTRIBUTING.md), [RELEASING.md](../RELEASING.md) (releases, Linux/macOS runtime builds), [SECURITY.md](../SECURITY.md) and [docs/](../docs/) (CLI, Git, doctor, services format, templates, website pages).

## Layout

```
.ai/
  README.md            this index
  instructions/        rules for agents: agent-rules, conventions, checks, pitfalls
  context/             what is true now: overview, the user's way of working, download sources
    decisions/         the user's decisions, one file per area
  plans/               roadmap.md, active/, completed/, backlog/, source-specs/
  progress/            status.md (now), history.md (timeline)
  sessions/            one file per session: verbatim prompts and handoff notes
  tools/               scripts that maintain this folder
```
