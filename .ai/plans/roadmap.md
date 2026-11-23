# LampRabit — roadmap

The phase list and the product-level decisions. Each phase's details are in its own file: [active/](active/) for work in progress, [completed/](completed/) for finished phases. Download sources are in [../context/download-sources.md](../context/download-sources.md).

A Laragon/XAMPP-style local dev environment built with Tauri v2 (Rust) and React.

## Decisions

| Topic | Decision |
|---|---|
| Platforms | Windows, macOS and Linux from day one; OS-specific code is isolated behind `cfg` |
| Binaries | Downloaded on demand from a per-OS version manifest, sha256-verified |
| MVP stack | Apache + PHP + MariaDB (MySQL added in phase 9) |
| PHP integration | Windows: thread-safe PHP loaded into Apache as `mod_php`. macOS/Linux: `php-fpm` + `mod_proxy_fcgi` |
| Binary sources | Windows: Apache Lounge, nginx.org, windows.php.net, mariadb.org, MySQL CDN. Linux: LampRabit's own CI builds of Apache, Nginx and PHP (`build/linux/`), official MariaDB and MySQL tarballs. macOS: LampRabit CI builds for Apache, Nginx, PHP, Redis, Memcached and MariaDB; MySQL downloads from Oracle CDN |
| Frontend | React 19, TypeScript, Vite, Tailwind v4, TanStack Query, Zustand |

## Environment layout (default `C:\lampRabit` / `~/lampRabit`)

```
bin/<runtime>/<version>/   etc/<runtime>/   www/   data/mariadb/   logs/   tmp/   ssl/
```

## Phases

- [x] **0. Scaffold**: Tauri v2 + React + TS + Tailwind
- [x] **1. Service supervisor**: start/stop/status, port-conflict check, log streaming, cleanup on exit
- [x] **2. Runtime manager**: manifest, download with progress, verify, extract, switch active version
- [x] **3. Config templating**: httpd.conf / php.ini / my.ini / php-fpm.conf from templates; MariaDB data-dir init
- [x] **4. Sites**: `www/foo` → `foo.test` (+ sub-domains, `public/` detection), hosts file kept in sync when Apache starts (one elevation prompt, managed block only), local CA + per-site HTTPS with a global default, System log stream persisted to `logs/lampRabit.log`, log history read from disk with clear, OS-specific settings hidden elsewhere
- [x] **5. Developer utilities**: Adminer / phpMyAdmin at `adminer.test`; Mailpit as a managed service (SMTP 1025, UI 8025) with PHP `mail()` routed to it (Windows: `SMTP`/`smtp_port`; macOS/Linux: `sendmail_path = mailpit sendmail`) and a panel explaining SMTP vs `mail()` settings for frameworks
- [x] [**6. Feature set**](completed/phase-06-feature-set.md)
- [x] [**7. Comfort and PHP tooling**](completed/phase-07-comfort-php-tooling.md)
- [x] **8. Release**: installers (NSIS/MSI, dmg, AppImage/deb), signed auto-updates, CI on every push, tag-triggered draft releases (see [RELEASING.md](../../RELEASING.md))
- [x] **Polish**: animated logo (ears, eyes, nose every few minutes), page/dialog/list motion, Motion setting (System / Full / Reduced / Off)
- [x] [**9. Linux parity and developer dashboard**](completed/phase-09-linux-parity-dashboard.md)
- [x] [**10. macOS parity, stability on every OS, safe uninstall**](completed/phase-10-macos-parity-uninstall.md) (2026-11-14: checked by hand on a Mac, compatibility matrix published; macOS is supported from 0.6.0, no longer a preview)
- [x] [**11. Service foundation and web servers: Caddy, FrankenPHP, Tomcat (with Java)**](completed/phase-11-service-foundation-web-servers.md) (2026-11-05: runtime needs and platform gaps from `catalog.rs`; the frontend's runtime and service labels load from it)
- [x] [**12. Projects: several domains, processes, Node.js serving**](completed/phase-12-projects-domains-processes.md) (2026-11-05: stack profiles list project processes; `lamprabit.toml` saved and applied from a project's Settings)
- [x] [**13. Logs redesign and dump catcher**](completed/phase-13-logs-dump-catcher.md) (2026-11-05: dump catcher, source sidebar, virtualised list, line details, per-source clear, rotation)
- [x] [**14. Movable tab bar**](completed/phase-14-movable-tab-bar.md) (2026-11-12: docks to any edge in the layout, drag with animation, Settings → Appearance)
- [x] [**15. More databases: PostgreSQL, MongoDB, SQLite, ClickHouse, Neo4j, backups**](completed/phase-15-more-databases.md) (2026-11-12: implemented; see phase details)
- [x] [**16. More services: Meilisearch, Typesense, RustFS, MinIO, OpenSearch/Elasticsearch, RabbitMQ, Ollama, n8n**](completed/phase-16-more-services.md)
- [x] [**17. Templates and more frameworks**](completed/phase-17-templates-frameworks.md) (2026-11-05: `processes:` and `domains:` in templates, a template editor, Save as a template)
- [x] [**18. Onboarding**](completed/phase-18-onboarding.md) (2026-11-12: first-run starter setup and reopen from Settings)
- [x] [**19. Project doctor**](completed/phase-19-project-doctor.md)
- [x] [**20. Headless core and the `lamprabit` CLI**](completed/phase-20-cli.md)
- [x] [**21. MCP server**](completed/phase-21-mcp-server.md)
- [x] [**22. Git**](completed/phase-22-git.md) (2026-11-12: project Git workflow and Git runtime)
- [x] [**23. Dependency manager**](completed/phase-23-dependency-manager.md) (part of 25; 2026-11-14: the project card's Dependencies tab)
- [x] [**24. Languages: Python, Go, Rust, Java**](completed/phase-24-languages.md) (2026-11-12: runtime, project and template support)
- [x] [**25. Package managers: npm, pnpm, Yarn, Bun, Deno, pip, conda, uv, Maven, Gradle, Cargo**](completed/phase-25-package-managers.md) (2026-11-14)
- [x] [**26. Movable environment folder**](completed/phase-26-movable-folder.md) (2026-11-05: the first run asks where the folder goes)
- [ ] [**27. Pluggable services: every service a YAML manifest, community registry and marketplace**](active/phase-27-pluggable-services.md) (in progress on branch `dev-1.0`, started 2026-11-15)
- [ ] **Later**: Git workflow builder, deployment checklists

**Order (decided 2026-11-01):** services before everything else: 11 → 15 → 16 → 24 → 25 → 22, then the rest. The numbers stay so older references keep working.

## Next (from the 2026-11-21 spec, order confirmed by the user on 2026-11-21)

The spec's recommended order, checked against the code. Each item links to its backlog plan; an item becomes a numbered phase (and moves to `active/`) when the user starts it.

1. [Command palette + action registry](active/phase-28-command-palette.md): phase 28, complete 2026-11-24
2. Service manifests: phase 27, in progress
3. [Project model: endpoints and kinds](backlog/project-model.md): foundation for everything non-web (partly built)
4. [Tools tab](backlog/tools-tab-build-list.md) and [dynamic dashboard](backlog/dynamic-dashboard.md)
5. ["Open in LampRabit" deep link, branch-aware snapshots](backlog/small-ideas.md)
6. [.NET](backlog/dotnet.md) → [Dart/Flutter and Mobile Dev Hub](backlog/mobile-dev-hub.md)
7. KarotX ([small-ideas.md](backlog/small-ideas.md)), [Docker](backlog/docker.md), plugin SDK

Not ordered yet: [service binding](backlog/service-binding.md), [local BaaS and payments](backlog/local-baas-payments.md), [linked projects from any folder or drive](backlog/projects-anywhere.md) (approved), [WebSockets](backlog/websockets.md) (proxy mostly shipped), [Share improvements](backlog/share-improvements.md), [Remote: SSH and deployed apps](backlog/remote.md), [collaboration](backlog/collaboration.md), [serverless](backlog/serverless.md).

## Product evolution (decided 2026-10-29)

Phases 10 and up come from an exported product spec ([source-specs/2026-10-29-llm-product-spec.md](source-specs/2026-10-29-llm-product-spec.md)), checked against the code. About a third of it already existed: the supervisor, Apache/Nginx, PHP per project, extensions, MariaDB/MySQL, Redis, profiles and the dashboard. The decisions below are the maintainer's answers to the points where the spec was unclear or contradicted earlier decisions.

| Topic | Decision |
|---|---|
| Order | Stable on all three OSes first (macOS builds), then all services (web servers, databases, extra services), then project tooling; CLI/MCP and Git/dependency panels come last |
| FrankenPHP and Caddy | Brought back (reverses 7.3): both become web servers next to Apache and Nginx |
| In-app database manager | Not now. Adminer, phpMyAdmin and phpRedisAdmin stay the database UIs; Redis stays on phpRedisAdmin |
| "DD and dump catcher" | PHP `dump()` / `dd()` output is caught and shown in the Logs tab, which is redesigned with better filters and options |
| Uninstall | Keeps `www/` (projects) and `data/` (databases) and removes everything else LampRabit created |
| Docking | No dockable panels; only the tab bar can be moved to the bottom, top, left or right |
| CLI | Works whether or not the app is open. The CLI and the window show exactly the same state, because both talk to one backend process |
| `lamprabit.toml` in projects | Only if a feature needs it (phase 12 decides) |
| Reference | FlyEnv (github.com/xpf0000/FlyEnv) covers many of the same services; check how it gets binaries and configures each service before designing one here. Check its licence before copying any code or templates |
