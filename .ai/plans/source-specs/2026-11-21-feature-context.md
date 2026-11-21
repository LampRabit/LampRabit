# LampRabit — Feature Planning Context

> Hand-off file for Claude Code. Living document: each feature section holds the decisions made so far,
> the open questions, and an implementation outline. **Before writing code, read the repo to confirm the
> actual stack (Electron vs Tauri), service/process model, vhost generator, and SSL/CA code, and adapt
> this plan to what exists. Do not invent parallel systems where one already exists.**

## Product snapshot (v0.6.0, from the website, Nov 2026)

LampRabit: "Local development for every project, without the setup." MIT, free, Windows/macOS/Linux.
Website: https://lamprabit.shivam-vastav33.workers.dev/

**Already shipped (do NOT rebuild; extend):**
- Runtimes: PHP 8.1–8.5, Node.js, Python, Go, Rust, Java; multiple versions side by side; per-project
  versions from `composer.json`, `.nvmrc`, `.python-version`, `go.mod`, `rust-toolchain.toml`,
  `.java-version`/`.sdkmanrc`.
- Servers: Apache, Nginx, Caddy, FrankenPHP, Tomcat.
- Databases: MariaDB, MySQL, PostgreSQL, MongoDB, SQLite, ClickHouse, Neo4j. Services: Redis,
  Memcached, Meilisearch, Elasticsearch, OpenSearch, Typesense, MinIO, RustFS, RabbitMQ, Ollama, Mailpit.
- Package managers: Composer, npm, pnpm, Yarn, Bun, Deno, uv, pip, Poetry, conda, Cargo, Go modules,
  Maven, Gradle; Dependencies tab with outdated updates.
- 24 project templates + template editor; auto project detection with framework identification.
- **Project doctor** (runtime, extensions, deps, `.env`, DB, hosts, cert) with previewed fixes;
  service health checks.
- Built-in Git client (diffs, hunk staging, branches, merge/rebase with conflicts, stash, tags).
- `.test` domains + subdomains (or `.localhost`), trusted HTTPS via local CA, auto-renew; dev
  servers detected by listening port and mapped to a domain; extra domains → folder/port/process.
- **Share a site** via Cloudflare Tunnel (no account) or ngrok.
- **Port conflicts** tool with one-step fix.
- Background processes with restart policies or cron schedule.
- **`lamprabit.toml`** (server, HTTPS, PHP settings, subdomains, processes), committed and applied
  by teammates. **Stack profiles** exported/imported as JSON.
- Import from 11 tools (Laragon, XAMPP, AMPPS, MAMP, Herd, Valet, FlyEnv, EnvKit, WampServer,
  ServBay, Local). Connection details as `.env`.
- DB backup/restore (MariaDB, MySQL, Postgres, MongoDB, SQLite); Mailpit inbox.
- Live logs with filters; plain-language failure explanations; diagnostics report.
- Built-in MCP server (one-click setup for 14 assistants); `lamprabit` CLI with JSON output,
  exit codes, `--dry-run`.
- Tools on PATH at each project's version (shims).
- Clean uninstall reversing system changes; movable storage folder.
- Keyboard-first: shortcuts for every action, `?` lists them.
- Also (from owner): per-project cron, per-service CPU/RAM, folder-import onboarding, DB snapshots.

Queued: Tools tab, JFF tab, KarotX (Go single-binary DB client as a LampRabit service),
WarenX (Chrome extension exposing DOM/console/network to MCP).

Considering: LAN phone testing, profiler view, SSH/deploy workflow builder, community plugin SDK.

---

## Core architecture: Project → Processes → Endpoints (web is optional)

### Decision (2026-11-21)
LampRabit is not web-only. Users build web apps, APIs, workers, bots, CLIs, AI/ML scripts, etc.
Web exposure is a **capability a project may have**, not the type of the project.

Rule: **no `if (project.type === 'web')` branching across the codebase.** Features attach to
capabilities, not project types.

### Model
```
Project
 ├─ runtime(s): php 8.3 | node 22 | dotnet 8 | python 3.12 | go ...   (version manager)
 ├─ env: vars, .env file, secrets
 ├─ processes[]: name, command, cwd, env, autostart, restart policy
 │    └─ endpoints[] (optional): port, protocol (http | tcp), expose: { vhost?, ssl?, tunnel? }
 ├─ services[]: shared deps it uses (mysql, postgres, redis, mailpit, ...)
 └─ schedules[]: cron jobs (already implemented)
```

- A Laravel site = 1 process (php-fpm or `artisan serve`) with 1 http endpoint (vhost + SSL).
- An ASP.NET app = 1 process (`dotnet watch run`) with 1 http endpoint.
- A Discord bot / queue worker / Python script = 1 process, **0 endpoints**.
- A full-stack app = several processes (api, frontend dev server, worker), some with endpoints.

### What every project gets (regardless of web)
Runtime version pinning, env management, start/stop/restart, logs, CPU/RAM metrics, cron, DB
services and snapshots, CLI + MCP control.

### What only endpoints get
vHost, SSL, tunnel/Share, QR / LAN phone testing, "Open in browser".
UI: these controls render per endpoint. A project with no endpoints simply doesn't show them.

### Manifest
- **Use the existing `lamprabit.toml`** (already holds server, HTTPS, PHP settings, subdomains,
  processes). Extend it with `kinds`, per-process `endpoints`, and runtimes — do not add a second file.
  Background processes with restart policies already exist; this model formalises them.
- **Detectors** (composer.json, package.json, *.csproj, pyproject.toml, go.mod, ...) generate a
  default manifest on import. Users never have to write it by hand; the UI edits it.
- Projects without a manifest still work from LampRabit's internal store (current behaviour);
  migration should wrap existing site config into this model, not break it.

### Project kinds (UX layer on top of the model) — decided 2026-11-21
Purpose: keep the UI simple for users by hiding irrelevant panels, without reintroducing
type-based branching in the core.

- Kinds (multi-select): **Web app, API/Backend, Background service/worker, CLI tool,
  Package/Library, Desktop app, Mobile app, Serverless, Other**. Stored in project settings / manifest as `kinds: []`.
- **Auto-detect and pre-select** on create/clone/import; user confirms or edits. Ask explicitly only
  when detection is unsure. Editable later in Project Settings.
  - Detection hints: `Microsoft.NET.Sdk.Web` / Laravel / Next.js / Django → Web; `bin` field in
    package.json, `[project.scripts]`, `console_scripts`, Go `main` + no HTTP → CLI;
    library-only (no entry point, publishable) → Package; Electron/Tauri/WPF/WinUI/MAUI → Desktop;
    `Microsoft.NET.Sdk.Worker`, Celery, queue workers → Background service.
- **Kinds are presets, not logic.** Selecting a kind scaffolds config; it does not gate features:
  - Web / API → adds an http endpoint (vhost + SSL; tunnel available).
  - Background service → process with autostart + restart-on-crash.
  - CLI → "Run with args" panel, optional "add to PATH" shim.
  - Package → build/test/pack/publish scripts panel; local link (`npm link`, `dotnet pack` to a
    local feed, `pip install -e`).
  - Desktop app → run/build/package panel, no endpoint.
- **Single source of truth:** web panels (vhost, SSL, tunnel, QR) render when an **endpoint exists**,
  not when the `Web` kind is ticked. Kinds only decide defaults and which panels are shown first.
  - Kind `Web` but no endpoint → show "Add endpoint" prompt.
  - Endpoint exists but kind `Web` unticked → still show web panels (and suggest ticking the kind).
- Core code must never read `kinds` for behaviour; only UI layout/scaffolding reads it.

### Guardrails (avoid scope creep)
- Not a Docker replacement or a generic process manager (pm2, supervisord). Identity stays
  "local dev environment": runtimes + services + projects.
- Web stays the polished default path; non-web is "same project, no endpoint".
- Keep the per-language part small: a detector + a runtime installer per language. Everything else
  (process runner, logs, metrics, endpoints, tunnels) is shared code.

---

## UX principles: "fit the elephant in a box" (applies to EVERY feature) — decided 2026-11-21

Problem: every new feature makes LampRabit harder for new users ("Where's Gradle?", "How do I
connect my phone?", "Where's php.ini?"). Power must stay invisible until needed.

### 1. Fixed mental model: five homes, every feature lives in exactly one
> **Decision (owner, 2026-11-21):** no separate Runtimes tab. Runtimes stay inside **Services**,
> as in the current app.

- **Projects** — your code: run, endpoints, env, logs, cron, project-specific tools.
- **Services** — runtimes (PHP, Node, .NET, Dart/Flutter, Python, Go, Java/JDK... versions + their
  config files like php.ini) **and** background services (MySQL, Postgres, Redis, Mailpit,
  KarotX, Firebase emulators, PocketBase...). Keep the existing Services layout; new runtimes
  (.NET, Dart/Flutter, JDK pinning) are added there.
- **Devices** — phones, emulators, simulators (Mobile Dev Hub).
- **Tools** — standalone utilities (Tools tab).
- **Remote** — servers, SSH/SFTP/FTP, tunnels to servers, deployed app logs/health (Features 13, 15, 16).
New features must declare which home they belong to. No new top-level tabs without strong reason.

### 2. Command palette (Ctrl/Cmd+K) — the #1 discoverability feature
- Searches everything: actions ("restart nginx", "connect phone", "share site"), settings, config
  files ("php.ini", "my.cnf", "nginx.conf", "gradle.properties"), projects, services, docs.
- Synonym/alias table so user words match: "phone", "mobile", "android", "adb" → Devices;
  "gradle", "jdk", "java_home" → JDK runtime / doctor; "ssl", "https", "certificate" → endpoint SSL.
- Every feature MUST register its actions + keywords with the palette (via a registry, so the
  plugin SDK can use it too).
- Same registry powers the CLI (`lamprabit <action>`) and MCP tools — one action list, three surfaces.

### 3. "Where is it?" is always one click
- Every managed config/binary has: **Open in editor · Reveal in folder · Copy path**.
- Per runtime/service: a "Config files" list (php.ini for the active PHP version, my.cnf,
  vhost files, gradle.properties, .env...). Show which file is actually in effect.
- Edits to config offer "restart service to apply".

### 4. Progressive disclosure
- Project view shows only panels relevant to its kinds/endpoints (see Project kinds).
- Heavy capabilities are **opt-in modules**: Mobile Dev Hub, .NET, Flutter, tunnels download their
  binaries on first use ("Add mobile development" button), keeping the default install lean.
- Simple view by default; "Advanced" sections collapsed, never removed.

### 5. Contextual nudges (come to the user, not the other way round)
- Event-driven suggestions: phone plugged in → "Android phone detected. Connect it to <project>?";
  `.csproj` imported but no SDK → "Install .NET 8?"; port conflict → "Port 80 used by Skype/IIS.
  Fix?"; build fails with JDK error → link to doctor fix.
- Every error message = what happened + why + one-click fix or doc link. No raw stack traces as
  the only output (keep them under "Details").
- Rate-limit nudges, allow "don't show again".

### 6. Guided first-runs
- Per-feature first-run checklist/empty state (e.g. Devices empty → "Connect a phone in 3 steps").
- Global onboarding stays short; feature onboarding happens when the feature is first opened.

### 7. Docs as part of the product
- Every panel has a "?" linking to its docs page.
- Write docs as task pages titled the way users search: "Where is php.ini in LampRabit",
  "Connect Android phone to local backend", "Fix JAVA_HOME / Gradle JDK error". Doubles as
  SEO/AEO content for the website.
- Palette results include doc pages.

### 8. Learn where users get lost (opt-in only)
- Open source → telemetry strictly opt-in, anonymous, documented. Most useful signal: palette
  searches with no result → tells you what's missing or misnamed.

### Definition of done for any new feature
1. Has one home. 2. Registered in palette with keywords (and CLI/MCP where sensible).
3. Config files reachable via Open/Reveal/Copy path. 4. Errors have fix or doc link.
5. Has empty state / first-run hint. 6. Has a task-titled docs page.

---

## Feature 1: ASP.NET Core support (SDK management + vHost + SSL)

### Goal
Treat a .NET web project like any other LampRabit site: detect it, run it, give it a pretty local
domain with trusted HTTPS, and show it in the dashboard with logs and resource usage.

### Competitive note
ServBay already supports .NET (SDK 2.0–10.0, plus Mono/XSP for legacy ASP.NET Framework) on macOS
and Windows. Laragon, XAMPP, MAMP, Herd do not. So LampRabit is **not first**; the differentiators
must be: free and open source, Windows-first polish, and deeper .NET workflow integration
(launchSettings, dotnet watch, user-secrets, EF Core migrations, SQL Server).

### Scope
- **In (v1):** ASP.NET Core / modern .NET (6, 8, 9, 10+). Cross-platform, runs on Kestrel.
- **Out (v1):** Classic ASP.NET Framework 4.x (System.Web). Needs IIS/IIS Express on Windows or Mono
  elsewhere. Revisit later as an optional Windows-only "IIS Express runner".

### Architecture (decided direction)
TLS terminates at LampRabit's existing reverse proxy. Kestrel runs **plain HTTP on a local port**.

```
browser ── https://myapp.test ──▶ LampRabit proxy (Nginx/Apache/Caddy, LampRabit CA cert)
                                        │  proxy_pass http://127.0.0.1:<assigned port>
                                        ▼
                                   Kestrel (dotnet watch / dotnet run / published DLL)
```

Why: one CA, one cert flow for all stacks; avoids fighting `dotnet dev-certs`; tunnels and vhosts
work the same for .NET as for PHP/Node.

### Implementation outline
1. **SDK manager**
   - Detect installed SDKs: `dotnet --list-sdks`, `dotnet --list-runtimes`.
   - Install portable SDKs into LampRabit's bin dir using Microsoft's `dotnet-install` script
     (`dotnet-install.ps1` / `.sh`, `-Channel 8.0` / `-Version x.y.z`, `-InstallDir`).
   - Version source of truth: `releases-index.json` from Microsoft's .NET release metadata.
   - Respect per-project `global.json` (pin SDK); set `DOTNET_ROOT` and PATH per process.
2. **Project detection** (onboarding import + manual add)
   - `*.csproj` with `Sdk="Microsoft.NET.Sdk.Web"` → web project. `*.sln` → let user pick the
     startup project.
   - Read `Properties/launchSettings.json` for profiles, env vars, `ASPNETCORE_ENVIRONMENT`.
   - Read `<TargetFramework>` to warn if the required SDK/runtime is missing (offer install).
3. **Process runner** (reuse existing service/process model)
   - Modes: `dotnet watch run` (dev, hot reload), `dotnet run`, `dotnet <App>.dll` (published).
   - Env: `ASPNETCORE_URLS=http://127.0.0.1:<port>`, `ASPNETCORE_ENVIRONMENT=Development`,
     `DOTNET_WATCH_SUPPRESS_LAUNCH_BROWSER=1`, plus launchSettings profile env vars.
   - Port allocation from LampRabit's port pool; detect readiness by polling the port or parsing
     `Now listening on:` from stdout.
   - Stream stdout/stderr into the log viewer; hook into per-service CPU/RAM metrics.
   - Clean shutdown: send Ctrl+C/SIGINT first (dotnet watch spawns child processes), then kill tree.
4. **vHost + SSL**
   - Generate reverse-proxy vhost for `<name>.test` (or configured TLD) → `127.0.0.1:<port>`.
   - Include WebSocket upgrade headers (SignalR, Blazor Server, hot reload need them).
   - Forward `X-Forwarded-For`, `X-Forwarded-Proto`, `X-Forwarded-Host`.
   - Issue cert from existing LampRabit CA.
5. **Proxy-awareness helper** (important; most common breakage)
   - Behind a proxy, ASP.NET sees `http` and may redirect-loop with `UseHttpsRedirection`, or build
     wrong OAuth callback URLs.
   - Fix in-app: `app.UseForwardedHeaders()` with `ForwardedHeaders.XForwardedFor | XForwardedProto`
     and clear `KnownNetworks`/`KnownProxies` for local dev, OR set env
     `ASPNETCORE_FORWARDEDHEADERS_ENABLED=true` (zero code change; prefer this as default).
   - UI: show a one-click "copy snippet" and a warning if redirect loops are detected.
6. **Databases for .NET devs** (follow-up)
   - SQL Server: Windows → detect/start SQL Server LocalDB (`sqllocaldb`); other OS → SQL Server
     container if Docker present. Postgres/MySQL already in LampRabit work as-is.
   - Show connection strings in the format `appsettings.json` expects.
7. **Nice-to-haves (differentiators)**
   - EF Core panel: list/apply migrations (`dotnet ef migrations list`, `dotnet ef database update`).
   - User-secrets viewer (`dotnet user-secrets list`).
   - `dotnet new` templates in the "new project" wizard.
   - Expose run/stop/logs through the existing MCP connectors and CLI (`lamprabit dotnet ...`).

### Open questions
- Which reverse proxy does LampRabit use per platform today? (Plan assumes one already exists.)
- Bundle SDKs or download on demand? (Recommend on demand; SDKs are ~200 MB each.)
- Default local TLD (`.test`?). Note: .NET 10 adds `.localhost` TLD support in Kestrel/dev-certs.

---

## Feature 2: Public tunnels (Cloudflare Tunnel + ngrok) — for ALL stacks

> **STATUS: ALREADY SHIPPED** as "Share a site". Treat this section as a checklist of possible
> improvements only (host-header rewrite, base-URL warnings, QR, share-link password/expiry,
> attach to endpoints of non-PHP projects incl. .NET/Dart). Audit the existing code before changing.

### Decision
Tunnels are a **site-level feature, not .NET-specific**. Any site (PHP, Node, .NET, static) gets a
"Share" button. Ship it alongside .NET support so .NET launch has it, but build it generic.

### Providers
1. **Cloudflare quick tunnel** (default; no account needed)
   - Manage the `cloudflared` binary (download per OS/arch from GitHub releases, verify checksum).
   - `cloudflared tunnel --url https://myapp.test --http-host-header myapp.test --no-tls-verify`
     or point straight at the backend port.
   - Parse the `https://<random>.trycloudflare.com` URL from stderr and show/copy it.
   - Optional later: named tunnels with the user's Cloudflare account and a stable hostname.
2. **ngrok** (requires user's authtoken)
   - Manage `ngrok` binary; store authtoken via OS keychain, not plain config.
   - `ngrok http https://myapp.test --host-header=rewrite` (or `--url` for reserved domains).
   - Read public URL from ngrok's local API `http://127.0.0.1:4040/api/tunnels`.

### Gotchas to handle
- **Host header rewrite** is mandatory, or the proxy serves the wrong vhost / default page.
- Apps that hardcode their base URL (WordPress `siteurl`, Laravel `APP_URL`, ASP.NET OAuth
  callbacks) break through a tunnel; show a warning + per-framework hint.
- Forwarded headers: the app sees the tunnel's https; same `UseForwardedHeaders` fix as above.
- Show tunnel status, URL, QR code (ties in with planned LAN phone testing), and one-click stop.
- Kill tunnel processes on app exit and on site stop.
- Expose via CLI (`lamprabit share <site> --provider cloudflare|ngrok`) and MCP.

---

## Feature 3: Dart & Flutter support (planned after the Tools tab)

### Positioning
Flutter devs live in VS Code / Android Studio; LampRabit must **not** try to be an IDE or debugger.
Its value is the environment around the app: SDK versions, devices/emulators, backend services,
and getting a phone/emulator to talk to the local backend.

### Runtime / SDK management
- Flutter SDK bundles Dart. Also support standalone Dart SDK for backend/CLI-only projects.
- Per-project versions: respect FVM if present (`.fvmrc`, `.fvm/`), and `environment.sdk` /
  `environment.flutter` constraints in `pubspec.yaml`. Prefer integrating with FVM over inventing a
  parallel version scheme.
- Flutter SDK is git-based and large; install on demand into LampRabit's bin dir, or detect existing.
- **Doctor panel:** run `flutter doctor -v` (or `--machine` where available), render
  pass/warn/fail with fix hints. High value, low cost.
- Android SDK / Xcode: **detect, don't bundle** (multi-GB). Link to install; maybe wrap
  `sdkmanager` later.

### Detection → kinds (uses the Project kinds presets)
- `pubspec.yaml` with `flutter:` dependency → Flutter app. Platform folders decide kinds:
  `web/` → Web (endpoint), `windows/` `macos/` `linux/` → Desktop app, `android/` `ios/` → Mobile.
  (Add **Mobile app** to the kinds list.)
- `pubspec.yaml` without Flutter:
  - depends on `shelf`, `dart_frog`, `serverpod` → API/Backend (endpoint)
  - has `executables:` or `bin/` entry → CLI tool
  - otherwise → Package/Library
- Serverpod projects need Postgres + Redis → offer to attach LampRabit's existing services and
  write the connection config.

### Processes
- Flutter web: `flutter run -d web-server --web-port <port> --web-hostname 127.0.0.1` → http
  endpoint → vhost/SSL/tunnel work automatically.
- Mobile/desktop: `flutter run -d <deviceId>`. Device picker from `flutter devices --machine`;
  emulator list/launch via `flutter emulators` / `flutter emulators --launch <id>`.
- Hot reload / hot restart buttons: `flutter run` reads `r` / `R` from stdin → the process runner
  must keep stdin open and expose "send key". (Alternative: `--machine` JSON-RPC mode for a richer
  integration later.)
- Dart backend: `dart run bin/server.dart`, `dart_frog dev`, or Serverpod's run command, with port
  env injected.
- Common actions panel: `pub get`, `pub upgrade`, `build_runner build/watch`, `flutter build <target>`,
  `dart test`, `flutter test`.

### Killer feature: phone ↔ local backend
- Problem: Android emulator reaches the host at `10.0.2.2`; a physical phone can't resolve
  `myapi.test` and can't trust the LampRabit CA.
- LampRabit can show, per backend endpoint, the right URL for each target: desktop
  (`https://myapi.test`), Android emulator (`http://10.0.2.2:<port>`), iOS simulator
  (`http://127.0.0.1:<port>`), physical device on LAN (`http://<lan-ip>:<port>` + QR), anywhere
  (tunnel URL, trusted HTTPS).
- Optional: generate a `--dart-define=API_BASE_URL=...` for the chosen target.
- Ties directly into the planned **LAN phone testing** feature; build them together.

### Out of scope
Debugger, widget inspector, code editing, app store publishing.

---

## Feature 4: Mobile Dev Hub (shared by Flutter + React Native/Expo)

### Goal
Make "create app → fix my environment → run it on my real phone talking to my local backend"
painless. Flutter-specific parts stay in Feature 3; this section is the framework-agnostic layer.

### 1. Create apps — wrap official CLIs, never maintain own templates
- Flutter: `flutter create`
- Expo: `npx create-expo-app@latest`
- Bare React Native: `npx @react-native-community/cli init`
- After scaffold: auto-import as a project, detect kinds, attach backend services if asked.

### 2. Doctorize — one unified health panel
- Sources: `flutter doctor -v`, `npx expo-doctor`, `npx react-native doctor`, plus LampRabit's own
  checks.
- Own checks (the common real breakages): JDK version vs Gradle/AGP requirement, `JAVA_HOME`,
  `ANDROID_HOME`/`ANDROID_SDK_ROOT`, platform-tools on PATH, required SDK platforms/build-tools,
  `adb` version conflicts (multiple adb binaries), Node version, CocoaPods/Xcode (macOS only).
- Each failure → explanation + one-click fix where safe (set env var, install JDK, run sdkmanager).
- **JDK manager** (new runtime in the version manager): per-project JDK pinning. JDK mismatch is one
  of the most common React Native Android build failures.
- Android SDK/emulator: detect existing install; offer `sdkmanager` / `avdmanager` wrappers.
  Don't bundle (multi-GB).

### 3. Connect your phone
Android (main focus):
- USB: list via `adb devices -l`; show model, Android version, authorization state.
- Wireless: Android 11+ wireless debugging pairing — QR code / pairing code flow
  (`adb pair host:port code`, then `adb connect`), discovered via `adb mdns services`.
- **`adb reverse` automation (core feature):** for every endpoint of the project and its backend
  (Metro 8081, Flutter/Dart backend, API ports), run `adb reverse tcp:<port> tcp:<port>` on each
  connected device and re-apply on reconnect. Phone then reaches `http://localhost:<port>` exactly
  like the desktop — no LAN IP, no firewall, no DNS, no cert trust issues.
- Wi-Fi without adb: LAN IP + QR (ties into LAN phone testing); warn about Windows firewall and
  offer to add the rule.
- Anywhere / HTTPS: tunnel URL (Feature 2). Note: Android 7+ apps don't trust user-installed CAs by
  default, so local `.test` HTTPS on a phone needs `network_security_config` — prefer adb reverse
  (http localhost) or tunnel (publicly trusted) instead.
- Device panel extras: install APK (drag & drop), logcat viewer filtered by app package, screenshot,
  screen mirroring via scrcpy (optional external binary).

iOS:
- Simulator: works through localhost directly. Physical devices require macOS + Xcode; scope to
  detection + guidance in v1.

### 4. Run
- Expo: `npx expo start` (Metro endpoint), dev builds `npx expo run:android` / `run:ios`.
- Bare RN: Metro `npx react-native start` + `npx react-native run-android`.
- Flutter: see Feature 3.
- One "Run on device" button: pick device → apply adb reverse → start dev server → launch.

### Out of scope
IDE features, debugger, building/signing for store release, cloud builds (EAS).

---

## Feature 5: Pluggable services via YAML manifests — decided 2026-11-21

### Format note
Project config is already TOML (`lamprabit.toml`). Consider **TOML for service manifests too** for
consistency (one format for users and contributors). The draft below is YAML only because it was
written first; the schema is format-agnostic.

### Verdict
Worth it, **if** it stays declarative and built-in services use it too. Precedents: Scoop manifests,
Homebrew formulae, DDEV add-ons, docker-compose.

### Why
- New services/versions added by writing a file, not app code → faster releases, community PRs.
- One generic service engine (download → verify → init → start → health → stop) instead of
  per-service code.
- Natural foundation for the community plugin SDK.
- Each manifest also feeds the UX layer: palette keywords, config files ("where is it"), docs link,
  connection strings for projects.

### Rules
1. **Dogfood first:** convert built-in services (MySQL/MariaDB, Postgres, Redis, Mailpit, KarotX)
   to manifests before opening it to the community. If a built-in can't be expressed, fix the schema.
2. **Declarative, not a language:** no loops/conditionals in YAML. Only variable substitution
   (`${port}`, `${dataDir}`, `${version}`, `${os}`, `${arch}`) and per-OS overrides.
   Escape hatch: named lifecycle hooks pointing to a script shipped with the manifest — sparingly.
3. **Schema-validated:** JSON Schema with `schemaVersion`; validate on load, show clear errors in UI.
   Publish the schema so editors autocomplete.
4. **Security (manifests download binaries and run commands):**
   - SHA-256 checksum required for every download.
   - Official registry = a git repo; changes via reviewed PRs (Scoop "bucket" model).
   - Third-party/local manifests: show an install consent screen listing download URLs, commands,
     ports, and paths it touches. Mark as "unverified".
   - No arbitrary shell strings by default; commands as argv arrays.
5. **Rollout:** v1 = schema + built-ins as manifests + "load from local folder".
   v2 = official registry + in-app browse/install. Marketplace/ratings later, if ever.

### Draft schema (illustrative; refine against real built-ins)
```yaml
schemaVersion: 1
id: redis
name: Redis
category: cache            # database | cache | mail | queue | storage | search | other
description: In-memory key-value store
homepage: https://redis.io
keywords: [redis, cache, key value, queue]   # command palette synonyms

versions:
  - version: "7.4.1"
    downloads:
      windows-x64: { url: "https://...", sha256: "...", extract: zip, binDir: "." }
      linux-x64:   { url: "https://...", sha256: "...", extract: tar.gz, binDir: "bin" }
      darwin-arm64:{ url: "https://...", sha256: "...", extract: tar.gz, binDir: "bin" }

ports:
  - name: main
    default: 6379
    protocol: tcp

paths:
  dataDir: ${serviceHome}/data
  logFile: ${serviceHome}/logs/redis.log

configFiles:                       # shown in "Config files" with Open/Reveal/Copy path
  - path: ${serviceHome}/redis.conf
    template: templates/redis.conf.tpl   # rendered once on init, then user-owned
    restartOnChange: true

lifecycle:
  init:  []                        # one-time setup (e.g. initdb, mysqld --initialize)
  start: ["${bin}/redis-server", "${serviceHome}/redis.conf", "--port", "${port.main}"]
  stop:  { signal: SIGTERM, timeoutSec: 10 }   # or a command
  health:
    tcp: ${port.main}              # or http: url, or command: [...]
    intervalSec: 5

snapshot:                          # hooks into existing DB snapshots
  backup:  ["${bin}/redis-cli", "-p", "${port.main}", "SAVE"]
  files:   ["${paths.dataDir}/dump.rdb"]

connection:                        # offered to projects (env injection, copy buttons)
  env:
    REDIS_HOST: 127.0.0.1
    REDIS_PORT: ${port.main}
  url: redis://127.0.0.1:${port.main}

docs: https://<lamprabit-docs>/services/redis
```

### Open questions
- Where does the existing service code live, and how much of it can become the generic engine?
- Same manifest approach for runtimes (PHP, Node, .NET, JDK)? Probably later; runtimes have more
  special cases (extensions, ini files, shims). Do services first, learn, then decide.

---

## Feature 6: Tools tab — curated list — drafted 2026-11-21

### Positioning
Basic converters/formatters are commodity (DevToys, IT-Tools, CyberChef, countless websites).
Include them for completeness, but the Tools tab is only "worthy" because of **context-aware tools**
that know LampRabit's projects, endpoints, services, devices and files — things a website can't do.
Everything works offline.

### Tier 1 — Context-aware (the differentiators, build first)
| Tool | What it does | Ties into |
|---|---|---|
| **Webhook catcher & replay** | Local URL that captures incoming requests (headers, body); inspect, edit, replay to a project endpoint. Expose via tunnel for Stripe/GitHub/Razorpay webhooks. | Endpoints, tunnels |
| **Port inspector** | Who is using port 80/443/3306/8081? Show process + "kill" / "change LampRabit port". | Services, nudges |
| **Disk reclaimer** | Find and clean `node_modules`, `vendor`, `bin/obj`, `.dart_tool`, `build/`, Gradle/npm/pip/NuGet caches, unused SDK versions, old snapshots. "Reclaim 23 GB". | Projects, services (runtimes) |
| **Mock API server** | Serve an OpenAPI spec or JSON file as a fake API with latency/error toggles. | Endpoints (it's a process with an endpoint) |
| **HTTP client (lite)** | Send requests with project endpoints + env vars prefilled; save per project. Not a Postman clone. | Endpoints, env |
| **.env manager** | Compare `.env` vs `.env.example` (missing/extra keys), switch env profiles, mask secrets, warn if `.env` is tracked by git. | Projects |
| **Hosts file editor** | View/edit hosts safely; LampRabit-managed entries locked and labelled. | vHosts |
| **SSL / cert tool** | Inspect any cert chain (local or remote), generate certs for custom domains, export LampRabit CA for devices/other browsers. | SSL |
| **Fake data seeder** | Generate realistic fake rows (names, emails, addresses, Indian locale too) and insert into a chosen DB service table. | Services, KarotX |
| **App icon & favicon generator** | One image → all Android/iOS/Flutter/RN icon sizes + web favicons/manifest, written into the project. | Mobile Dev Hub, projects |
| **Network diagnostics** | Resolve `.test`, check endpoint reachable from LAN/tunnel, firewall rule check, LAN IP + QR. | Devices, tunnels |
| **Quick load test** | Hit an endpoint with N requests / concurrency, show latency percentiles. | Endpoints |
| **Responsive preview** | Open an endpoint in several viewport sizes side by side. | Endpoints |

### Tier 2 — Developer utilities (expected, cheap)
- **JWT** decode + verify (pick secret from project `.env`)
- **Hash & secrets:** bcrypt/argon2 hash (for seeding admin users), random secret/API key, UUID/ULID/nanoid
- **Regex tester** (with flavour: JS, PHP/PCRE, .NET, Dart)
- **Cron builder** with human-readable preview → "add to project scheduler"
- **Timestamp converter** (Unix ↔ human, timezones)
- **Formatters/validators:** JSON, YAML, XML, SQL, TOML
- **Converters:** JSON ↔ YAML ↔ CSV ↔ TOML, JSON → TypeScript/C#/Dart/PHP classes
- **Encoders:** Base64 (text + image/data URI), URL, HTML entities
- **Text diff** (and file diff)
- **Case/slug converter**, lorem ipsum, string escape
- **Markdown preview**
- **Clipboard tools:** history (opt-in, local only, excludes password managers), smart detect
  ("this looks like a JWT → decode?"). Privacy: off by default, clearly explained.

### Tier 3 — Design tools (as planned)
- Color picker (screen eyedropper), converter (HEX/RGB/HSL/OKLCH), **contrast checker (WCAG)**
- **Logo → theme:** extract palette from a logo → export as CSS variables, Tailwind config,
  Flutter `ThemeData`, MUI theme
- Palette generator (shades/tints scale)
- Font preview & pairing (local + Google Fonts), CSS `@font-face` snippet
- Gradient, box-shadow, border-radius generators
- Image optimizer/converter (PNG/JPG → WebP/AVIF, resize) and SVG optimizer

### Skip (not worth it)
Full Postman clone, image editor, AI chat inside Tools, anything requiring an online service,
tools that duplicate KarotX (DB client) or Mailpit.

### Tool framework requirements
- Each tool = a module with id, name, home (`Tools`), keywords, input/output → auto-registered in the
  **command palette, CLI and MCP** (an AI agent can call "generate bcrypt hash" or "seed fake data").
- Favourites / pinned tools; recent tools.
- Context entry points: tools also appear inside relevant panels (e.g. "Mock API" in a project's
  endpoints, "Port inspector" from a port-conflict error).
- Lazy-load tool code so the Tools tab doesn't bloat startup.
- Later: "send output to another tool" chaining (CyberChef-style); tools via the plugin SDK.

---

## Feature 7: Dynamic dashboard — drafted 2026-11-21

### Definition of "dynamic"
Three meanings, in priority order:
1. **Adaptive (build first):** the dashboard decides what to show from current state — what's
   running, broken, connected, and what the user works on. Zero configuration.
2. **Live:** everything updates in real time via push (IPC/event bus), not per-widget polling.
3. **Customizable (second):** pin, hide, reorder widgets. Pins override the adaptive order.
   No free-form grid builder in v1 — most users never configure dashboards.

Goal: in 3 seconds the user knows **what's running, what's broken, and what to do next**.

### Layout (top to bottom)
1. **Status strip** — services up/down with health dots, active tunnels, connected devices,
   overall "All good" / "3 issues".
2. **Needs attention** (only shown when non-empty) — computed issues, each with a one-click fix:
   crashed service, port conflict, doctor failure, missing SDK for an imported project, failed cron
   run, cert near expiry, disk reclaimable > threshold, LampRabit/SDK update available.
3. **Projects** — recent + running first. Card content adapts to kinds/endpoints:
   - Web/API: URL, Open, Share (tunnel), status, last request time.
   - Mobile: target device, Run on device, hot reload.
   - Background service: up/down, restarts, last log line.
   - CLI/Package: Run / Test / Build buttons.
4. **Contextual cards** (appear only when relevant): phone connected → "Run <project> on Pixel 7";
   tunnel active → URL + QR + Stop; webhooks caught; new mails in Mailpit; next scheduled job;
   active database snapshots.
5. **Resources** — CPU/RAM per service/project with sparklines (existing metrics).
6. **Quick actions + pinned tools** — ranked by the user's own usage frequency (from the action
   registry), with a "Ctrl+K" hint.

### First-run state
Until the first project exists, the dashboard is an onboarding checklist: import/create a project,
start a service, open a site with SSL, (optional) connect a phone.

### Architecture
- **Widget registry** (same pattern as the action/palette registry):
  ```ts
  interface DashboardWidget {
    id: string;
    home: 'projects' | 'services' | 'devices' | 'tools' | 'remote';
    size: 'strip' | 'small' | 'medium' | 'wide';
    relevance(state: AppState): number | null; // null = hide; higher = show earlier
    subscribe: string[];                         // state/event keys it re-renders on
    render(...): ...;
  }
  ```
- Order = user pins first, then by `relevance` score. Hidden widgets stay hidden.
- Single app state store fed by an event bus (service status, process exits, device connect,
  tunnel up, cron results, metrics). Widgets subscribe; no widget polls on its own.
- Plugins (later, via SDK) can register widgets.
- Persist per-user layout (pins/hidden/order) locally.

### Avoid
Charts for the sake of charts, noisy notifications, a blank "add widget" canvas, showing panels for
features the user hasn't enabled.

---

## Gap analysis (revised against v0.6.0 website) — 2026-11-21

Already covered by LampRabit (removed from gap list): Python + uv + Ollama, PATH shims, clean
uninstall, tunnels, port conflicts, stack profiles, project doctor, health checks, templates,
import from other tools, DB backup/restore, keyboard shortcuts.

### Genuine gaps (not on the website)
1. **New runtimes:** .NET (Feature 1), Dart/Flutter (Feature 3), Mobile Dev Hub incl. React Native/
   Expo, adb reverse, JDK pinning in doctor (Feature 4). These are the biggest differentiators left.
2. ~~Clone from git URL~~ **already built** (per owner). Remaining idea: an **"Open in LampRabit"
   README badge / deep link** (`lamprabit://clone?repo=...`) that triggers the existing clone flow.
3. **Branch-aware DB snapshots:** auto-backup on git branch switch, restore on switch back. Built on
   existing backup/restore + built-in Git client. Strong differentiator.
4. **Command palette (Ctrl/Cmd+K) + shared action registry** (keyboard shortcuts exist; a searchable
   palette with synonyms/config-file search is the discoverability layer). Register CLI + MCP actions
   from the same registry if not already.
5. **Pluggable service manifests** (Feature 5) — if services are currently hard-coded.
6. **Tools tab** (Feature 6) and **dynamic dashboard** (Feature 7).
7. **Docker interop:** import services from `docker-compose.yml`, show containers as services.
8. **Share-link hardening:** optional password + auto-expiry on shared URLs; "exposed" indicator.
9. **Debugging toggles:** Xdebug per project, Node `--inspect`, generate VS Code `launch.json`.

### Unknown — verify in repo before planning
- Update mechanism: auto-update with rollback + config migration/backup?
- Security defaults: services bound to 127.0.0.1? DB root passwords generated/stored in keychain?
- Non-web project kinds in UI (processes exist; are endpoints/kinds explicit?).
- i18n and accessibility status; LampRabit's own startup time / idle RAM.

### Recommended order
1. Command palette + action registry (foundation for discoverability of everything below)
2. Service manifests (if hard-coded today)
3. Tools tab (Tier 1) + dynamic dashboard
4. "Open in LampRabit" badge; branch-aware snapshots
5. .NET → Dart/Flutter → Mobile Dev Hub (React Native/Expo)
6. KarotX, WarenX, Docker interop, plugin SDK

---

## Feature batch 2 (proposed by owner 2026-11-21): feasibility + implementation

Home: **Remote** (servers, SSH/FTP, deployed apps) — one of the five homes (Projects, Services,
Devices, Tools, Remote). Features 13–15 live there.

### Feature 8: Projects from any folder/drive + safe recovery — FEASIBLE, high value
- Project registry stores absolute path + a **stable project identity** (id in `lamprabit.toml`,
  fallback: git remote URL + root commit hash, fallback: folder fingerprint).
- Watch each project root (OS file watcher: ReadDirectoryChangesW / FSEvents / inotify via
  chokidar or Rust `notify`). Network drives/shares: watchers unreliable → poll every N seconds.
- Detect volume removal events (USB/external drive unplugged) separately from deletion.
- On project disappearing:
  1. Mark project **Offline** (not deleted). Keep all config in LampRabit's own store.
  2. Gracefully stop its processes/terminals (SIGTERM → timeout → kill tree), pause its cron jobs,
     close tunnels.
  3. vHost stays registered but serves a LampRabit "Project offline" page instead of a proxy error.
  4. Shared services (MySQL, Redis) keep running; project DBs are untouched.
  5. Notify: "MyApp went offline (drive E: removed)". Dashboard "Needs attention" card.
- On reappearing (drive reconnected): auto-restore → re-validate (doctor quick check) → restart
  processes that were running before (store a "was running" snapshot).
- Moved/renamed: if a folder with the same identity appears elsewhere (scan on import, or user
  picks), offer **Relink** instead of re-import.
- Edge cases: Windows file locks (running processes hold cwd; stop before move), macOS permission
  prompts for external volumes/Documents, WSL paths (`\\wsl$\...`), symlinks, case-insensitive paths.
- Note: different from shipped "Your folder, anywhere" (moves LampRabit's own storage).

### Feature 9: One-click service binding to projects — FEASIBLE, very high value
"Add Redis / Mail / Storage / Search / Queue to this project" from the project page.
- Flow: pick service → ensure installed + running → create per-project resource → write config →
  show diff preview → apply → optional "test connection".
- **Per-project isolation (create resources, not just point at the service):**
  DB → new database + user; Redis → dedicated DB index or key prefix; MinIO/RustFS → bucket +
  access key/secret scoped to it; RabbitMQ → vhost + user; Meilisearch → index prefix/API key;
  Mailpit → shared inbox, tag by project (e.g. via SMTP username or header) for filtering.
- **Framework adapters** (data-driven, ideally inside service manifests as `adapters`):
  - Laravel `.env`: `REDIS_HOST/PORT`, `CACHE_STORE=redis`, `QUEUE_CONNECTION=redis`,
    `MAIL_MAILER=smtp MAIL_HOST=127.0.0.1 MAIL_PORT=1025`, `FILESYSTEM_DISK=s3`
    `AWS_ENDPOINT` + `AWS_USE_PATH_STYLE_ENDPOINT=true`, `SCOUT_DRIVER=meilisearch`.
  - Node/Next.js `.env`: `REDIS_URL`, `SMTP_HOST/PORT`, `S3_ENDPOINT`, `S3_FORCE_PATH_STYLE`.
  - Django/FastAPI `.env` (+ snippet for settings.py if not env-driven).
  - Spring Boot `application-local.properties`; .NET `appsettings.Development.json` or user-secrets;
    Rails `.env`/`config/*.yml` snippet; Go/Rust `.env`.
- Rules: edit `.env`/dev-only config files only; **never silently edit source code** (show a
  copyable snippet instead); diff preview + backup (same as doctor fixes); mark lines
  `# managed by LampRabit`; record the binding in `lamprabit.toml` so teammates get the same setup;
  "Unbind" reverses it.
- Optionally offer the matching package install (`predis`/`phpredis`, `ioredis`, `league/flysystem-aws-s3-v3`)
  via the existing Dependencies tab (command previewed).

### Feature 10: Local BaaS (Firebase Emulator Suite + PocketBase) + payments sandbox — CONFIRMED by owner

Do NOT build a Firebase clone. Ship **both** official Firebase Emulator Suite and PocketBase.

#### Key design point: project-scoped services
Unlike MySQL/Redis (one shared instance), both are **per-project**: each project has its own
emulator config/data or `pb_data`. Add `scope: shared | project` to service manifests (Feature 5):
the binary/CLI is installed once and versioned globally; each project runs its own instance as a
process with endpoints (Project → Processes → Endpoints model).

#### 10a. Firebase Local Emulator Suite
- Requirements: Node.js 16+, Java JDK 11+, Firebase CLI 8.14.0+ (`firebase-tools`). LampRabit already
  manages Node and Java → install `firebase-tools` into LampRabit's tool dir; doctor checks versions.
- Detection: `firebase.json` in project → offer "Run Firebase emulators". No `firebase.json` →
  offer setup (`firebase init emulators`, wizard picks emulators and downloads binaries).
- Emulators: Auth, Firestore, Realtime Database, Storage, Functions, Hosting, Pub/Sub, Eventarc,
  Emulator UI. Default ports: UI 4000, Hub 4400, Auth 9099, Firestore 8080, Database 9000,
  Storage 9199, Functions 5001, Hosting 5000, Pub/Sub 8085.
  - **Port conflicts:** 8080 and 5000 clash often (other dev servers; macOS AirPlay uses 5000).
    LampRabit's port allocator proposes free ports and writes them to `firebase.json` `emulators`
    block — with diff preview.
- Run: `firebase emulators:start --project <id> --import <dir> --export-on-exit` so data survives
  restarts (store in project `.firebase-emulator-data/` or LampRabit data dir; user choice; add to
  `.gitignore` suggestion).
  - Offer **demo project IDs** (`demo-<name>`) for fully offline use without a Google account.
  - Warn on multiple project IDs; respect `singleProjectMode` in `firebase.json`.
- Endpoints: Emulator UI (http, gets `firebase.<project>.test` vhost + SSL), each emulator port
  listed; Hosting emulator can be shared via tunnel.
- Java heap: expose a setting that maps to `JAVA_TOOL_OPTIONS` (e.g. `-Xmx4g`) for Firestore/
  Database/Storage heap errors; detect "Java heap space" in logs → suggest fix.
- Service binding (Feature 9): write env vars for SDKs: `FIRESTORE_EMULATOR_HOST`,
  `FIREBASE_AUTH_EMULATOR_HOST`, `FIREBASE_DATABASE_EMULATOR_HOST`,
  `FIREBASE_STORAGE_EMULATOR_HOST`, `PUBSUB_EMULATOR_HOST`, `GCLOUD_PROJECT`; show client SDK
  snippets (`connectFirestoreEmulator`, `connectAuthEmulator`, Flutter `useFirestoreEmulator`) —
  snippet only, never edit source silently.
- Mobile Dev Hub tie-in: Android emulator must use `10.0.2.2`, physical phones need adb reverse
  of each emulator port (Feature 4 handles it automatically).
- Logs: each emulator's output into the log viewer with source tags.

#### 10b. PocketBase
- Single Go binary (auth, SQLite DB, realtime, file storage, admin UI). Manage versions globally;
  checksum-verified download per OS/arch.
- Per project: `pocketbase serve --http=127.0.0.1:<port> --dir=<project>/pb_data`
  (+ `--migrationsDir`, `--hooksDir` if present). Detect existing `pb_data/`, `pb_migrations/`,
  `pb_hooks/` → auto-offer.
- Endpoint gets vhost + SSL (`pb.<project>.test`); admin UI at `/_/`; first-run creates superuser
  (generated password stored in OS keychain, shown once).
- Backups: hook into existing backup/restore (PocketBase uses SQLite → snapshot `pb_data`).
- Binding: write `POCKETBASE_URL` (and framework-specific keys) to `.env`.

#### 10c. Payments sandbox
- **Stripe:** manage Stripe CLI binary; per-project panel: login, `stripe listen --forward-to
  <project endpoint>/<webhook path>` (no tunnel needed), capture `whsec_...` and write
  `STRIPE_WEBHOOK_SECRET` to `.env`, one-click `stripe trigger <event>`, live event log.
  Offline: **stripe-mock** (official mock API server) as a service.
- Providers without a CLI (Razorpay, PayPal webhooks): webhook catcher + tunnel (Tools #35) with
  saved sample payloads.
- Later: Supabase/Appwrite local (need Docker) only when Docker is present.

### Feature 11: WebSockets over SSL + tunnels — FEASIBLE, mostly configuration
- Cloudflare Tunnel and ngrok already pass WebSockets; the work is in LampRabit's proxy configs:
  - Nginx: `proxy_http_version 1.1`, `Upgrade $http_upgrade`, `Connection $connection_upgrade`
    (map), raise `proxy_read_timeout` (default 60s closes idle sockets).
  - Apache: `mod_proxy_wstunnel` or `ProxyPass ... upgrade=websocket`.
  - Caddy: works automatically. FrankenPHP: verify.
- Dedicated WS servers (Laravel Reverb, Soketi, Socket.IO, Centrifugo) as a process endpoint →
  give it its own subdomain (`ws.myapp.test`) with SSL, and share via tunnel.
- **Vite/webpack HMR behind HTTPS domains:** detect and suggest `server.hmr` config
  (`host`, `protocol: 'wss'`, `clientPort: 443`) — common breakage.
- Write correct client env (`VITE_REVERB_HOST/PORT/SCHEME`, `PUSHER_*`) via Feature 9 adapters.
- Add automated test: after vhost generation, perform a WS upgrade against the endpoint.

### Feature 12: WebSocket tester tool + WS doctor — FEASIBLE, low effort
- Tester: connect `ws://`/`wss://`, custom headers, subprotocols, send text/JSON/binary, message
  log with timestamps, saved messages, auto-reconnect, latency/ping.
- Protocol modes: raw WebSocket, **Socket.IO/Engine.IO** (not raw WS — a common confusion),
  **Pusher protocol** (Reverb/Soketi: subscribe to public/private/presence channels with auth endpoint).
- Doctor checks (in order, stop at first failure with explanation + fix): DNS → TCP → TLS cert
  valid/trusted → HTTP upgrade returns **101** (vs 400/404/426) → proxy upgrade headers present →
  origin/CORS rejection → idle timeout disconnect → mixed content (ws:// from https page) →
  reachable through tunnel/LAN.
- Prefill endpoints from projects; register in palette/CLI/MCP.

### Feature 13: SSH + SFTP/FTP toolkit (Remote home) — FEASIBLE, large; win by integration
Libraries: Node `ssh2` + `basic-ftp`, or Rust `russh` + `suppaftp` (match app stack); xterm.js terminal.
- Connections: read `~/.ssh/config` (hosts, ProxyJump), use ssh-agent / Pageant / OpenSSH agent on
  Windows; passwords/keys passphrases in OS keychain; **strict host key verification**
  (known_hosts, show fingerprint on first connect, never auto-accept silently).
- Terminal tabs; SFTP file browser (drag-drop upload/download, edit remote file in local editor
  with save-back, permissions); FTP/FTPS when SFTP isn't available; transfer queue with resume.
- **Tunnels:** local forward (remote MySQL 3306 → localhost:13306), remote forward, dynamic SOCKS;
  saved tunnels that auto-start with a project; show them as connectable in KarotX and in
  Feature 9 bindings.
- **Killer integrations:**
  - "Pull production/staging DB to local": SSH → `mysqldump`/`pg_dump` streamed → restore into a
    local DB via existing backup/restore (with "anonymize emails" option later).
  - "Deploy folder": one-way sync (rsync over SSH when available, SFTP fallback) with dry-run diff.
  - Remote log tail (feeds Feature 15).
- Don't chase Termius/MobaXterm/WinSCP feature parity; the value is the project/DB integration.

### Feature 14: One-click Docker config builder (tool) — FEASIBLE, strong because LampRabit knows the project
Precedent: `docker init` (generic). LampRabit's advantage: it knows exact runtime versions, PHP
extensions, framework, bound services and their versions, background processes and cron jobs.
- Generates (preview + diff, never overwrite without consent): multi-stage `Dockerfile`,
  `docker-compose.yml` (app + bound services with matching image versions + volumes +
  healthchecks), `.dockerignore`, `.env.docker.example`.
- Per-framework templates: Laravel (php-fpm + nginx, or FrankenPHP; queue worker + scheduler as
  separate services), Next.js (standalone output), Node API, Django/FastAPI (gunicorn/uvicorn),
  Spring Boot (layered jar), Go/Rust (static binary → distroless/scratch), .NET (sdk → aspnet).
- Background processes → separate compose services; cron → scheduler container (e.g. supercronic).
- Production defaults: pinned image tags, non-root user, no secrets baked in, healthchecks,
  small final images.
- If Docker is installed: "Build & run locally" to validate, show errors via plain-language explainer.
- Templates stored as data (contributable like service manifests). Output is a starting point; say so.

### Feature 15: Deployed app health, errors, live logs (Remote home) — LOCAL-ONLY (decided by owner)

> **Decision (owner, 2026-11-21):** monitoring runs only while LampRabit is running. When the
> machine sleeps/is off, nothing is monitored — accepted and stated clearly in the UI.
> **Gap backfill:** on wake/app start, pull logs for the missed period over SSH
> (`journalctl --since "<last seen>"`, `docker logs --since`, file tail from saved offset) and
> mark the gap on the timeline ("Not monitored 23:10–07:45, logs backfilled"). Health pings
> can't be backfilled; show them as a gap.

Full monitoring (24/7 uptime, error tracking, alerting) needs an always-on server — a different
product (Sentry, Better Stack, Uptime Kuma, Grafana) with hosting costs, which breaks "local, free".
Build what fits a desktop app:
- **Live remote logs over SSH** (via Feature 13): `tail -f` files, `journalctl -fu`, `docker logs -f`,
  `pm2 logs`; same log viewer, filters and plain-language explanations as local logs.
- **Server snapshot:** CPU/RAM/disk, `docker ps`/`stats`, systemd service status, open ports.
- **Health pings while LampRabit is open:** HTTP checks on deployed URLs with response time; clearly
  labelled "only while LampRabit is running".
- **Integrations, not reinvention:** read status from Uptime Kuma / Sentry / GlitchTip APIs if the
  user has them; offer to generate a self-hosted Uptime Kuma or GlitchTip deployment via Feature 14.
- Optional much later: tiny open-source agent the user installs on their server — only if demand is clear.

---

### Feature 16: Remote development + real-time collaboration — drafted 2026-11-21

Owner's pain points: autosave, broken connections, several people working at once, network
constraints. Goal: conflict-free collaboration for developers.

#### Honest framing
- Real-time co-editing is a crowded, hard category: VS Code Live Share, JetBrains Code With Me,
  Zed (CRDT multiplayer built in), Replit. **LampRabit is not an editor** — building one loses.
- CRDTs (Yjs, Automerge, Loro) guarantee every copy **converges**, not that the merged code is
  **correct**. Two people editing the same function still produce broken code. "Conflict-free"
  must come mostly from **awareness + resilient sync**, not magic merging.
- Build in three layers; ship layer 1 and 2 first.

#### Layer 1: Resilient remote dev (single user) — FEASIBLE, solves autosave/disconnect pain
- **Local-first:** edit a local copy in any editor; LampRabit syncs to the remote server in the
  background. Disconnection never blocks typing or loses saves.
- Sync engine: integrate **Mutagen** (open-source bidirectional file sync over SSH with conflict
  handling, used by DDEV/Docker Desktop) or build on Feature 13's SSH layer. Prefer integrating.
- Offline queue: changes queue up while disconnected; auto-reconnect with backoff; resume
  without full rescan. Status chip: Synced / Syncing 12 files / Offline (queued 5) / Conflict.
- Conflicts (file changed on both sides): 3-way merge (`git merge-file` style) using last synced
  version as base; unresolved → side-by-side resolver; never silently overwrite.
- Ignore rules: respect `.gitignore` + defaults (`node_modules`, `vendor`, build dirs); run
  dependency installs on the remote instead of syncing them.
- Remote processes: run/restart the remote app, stream its logs (Feature 15), forward its ports
  locally via SSH tunnels (Feature 13) so `localhost:3000` works.

#### Layer 2: Shared session + presence (team) — FEASIBLE, LampRabit's unique angle
Share the **environment**, not just text:
- "Start session" → teammate joins via invite link and gets (permission-scoped):
  the running app URL (via tunnel), live logs, read-only or interactive terminal, forwarded DB/
  service ports, project doctor output, `lamprabit.toml`/stack profile to reproduce locally.
- **Presence & soft locks (prevents most conflicts cheaply):** show who has which file open/dirty
  ("Alex is editing app/Http/UserController.php"), warn before you edit the same file, optional
  soft lock. Works with any editor via file watching; richer with an editor extension.
- Git-aware: show teammates' current branch and uncommitted files; suggest branch-per-person for
  bigger changes.
- Transport: peer-to-peer over Cloudflare Tunnel / LAN; end-to-end encrypted session; tokens with
  expiry; host can revoke anyone instantly; audit log of who did what.

#### Layer 3: Real-time co-editing — EXPERIMENT ONLY, later
- If pursued: VS Code (and later JetBrains) extension using **Yjs** for shared text + cursors;
  LampRabit acts as the local sync host/relay. Do not build a LampRabit editor.
- Needs: relay for NAT traversal (tunnel-exposed LampRabit or WebRTC with STUN; TURN servers cost
  money), E2E encryption, persistence of CRDT docs, mapping editor buffers ↔ files on disk.
- Gate on clear demand after Layers 1–2 ship. Evaluate whether integrating Live Share/Zed is enough.

#### Recommended scope
Layer 1 (resilient sync) → Layer 2 (presence + shared environment) → Layer 3 only if users ask.

---

## Feature 17: Serverless local development — drafted 2026-11-21 (exploratory)

### What it is (context for implementers)
Code is written as small **functions** triggered by events (HTTP request, queue message, file upload,
cron). The cloud runs them on demand and scales automatically; no server to manage. Platforms:
AWS Lambda, Cloudflare Workers, Vercel/Netlify Functions, Firebase/Google Cloud Functions,
Azure Functions, Supabase Edge Functions (Deno).
Languages: JS/TS everywhere; Python, Java, .NET, Go, Ruby on AWS/Azure/GCP; Rust/Go via custom
runtimes or WASM; Workers mostly JS/TS/WASM. LampRabit already manages most of these runtimes.

### How people develop locally today (each platform has its own emulator CLI)
| Platform | Detect by | Local dev command | Notes |
|---|---|---|---|
| AWS SAM | `template.yaml` (Transform: AWS::Serverless) | `sam local start-api`, `sam local invoke` | Needs Docker |
| Serverless Framework | `serverless.yml` | `serverless offline` (plugin) | Node-based emulation |
| SST | `sst.config.ts` | `sst dev` | Proxies to real AWS (needs account) |
| LocalStack | used alongside above | runs AWS service emulators | Needs Docker |
| Cloudflare Workers | `wrangler.toml` / `wrangler.jsonc` | `wrangler dev` | Local workerd runtime; KV/D1/R2 simulated locally |
| Vercel | `vercel.json`, `api/` | `vercel dev` | |
| Netlify | `netlify.toml` | `netlify dev` | |
| Firebase Functions | `firebase.json` functions | Firebase emulators (Feature 10) | Already planned |
| Azure Functions | `host.json` | `func start` (Core Tools) | Storage emulator: Azurite |
| Supabase Edge | `supabase/functions/` | `supabase functions serve` | Deno; Docker |
| GCP Functions | `functions-framework` dep | Functions Framework | |

Common pains: one CLI per platform; heavy Docker requirements; non-HTTP triggers (queue, storage,
cron events) are hard to simulate; emulators differ from real cloud; secrets/env juggling;
many functions on random ports; scattered logs.

### What LampRabit adds (don't replace platform CLIs; orchestrate them)
1. **Detection → kind "Serverless"** (add to Project kinds) with platform identified from table.
2. **Runs the right dev command** with pinned runtime versions; manage the platform CLI binaries
   (wrangler, sam, func, netlify, vercel, supabase) like other tools; doctor checks
   (Docker needed? CLI version? credentials only when required).
3. **Endpoint per function app** → vhost + SSL (`api.<project>.test`) + tunnel for webhooks.
4. **Local stand-ins via Service binding (Feature 9):** S3 → RustFS/MinIO, queues → RabbitMQ or
   LocalStack SQS, KV/cache → Redis, DB → Postgres/MySQL, Azure storage → **Azurite** (add as service
   manifest), AWS services → **LocalStack** (service, only when Docker present).
5. **Event trigger panel** (like Stripe trigger): fire sample events at local functions — HTTP,
   S3 object created, SQS/queue message, cron/scheduled, Pub/Sub — from saved payload templates;
   `sam local invoke -e event.json` style under the hood.
6. **Secrets/env:** map `.dev.vars` (Wrangler), `local.settings.json` (Azure), `env.json` (SAM),
   `.env` into one env editor; keep secrets out of git.
7. **Unified logs** for all functions with function name tags; show invocation duration.

### Out of scope
Deploying to the cloud (users keep their platform CLI/CI), billing, IaC editing.

### Priority
Medium. Start with the no-Docker, most popular: Cloudflare Workers (`wrangler dev`), Vercel/Netlify
dev, Firebase Functions (already via Feature 10), Azure `func start` + Azurite. Then SAM/LocalStack
when Docker interop exists.

---

## Tools tab: final build list (small → big) — 2026-11-21

> Master list for the Tools tab (home: Tools). Supersedes the ordering in Feature 6; details for each
> tool are in Feature 6, 12, 13 and 14. Every tool: offline, registered in command palette + CLI + MCP,
> lazy-loaded, follows UX "definition of done".
> Size: **S** = hours to 1 day · **M** = a few days · **L** = 1–3 weeks · **XL** = multi-week.

### S — Small utilities
1. Base64 encode/decode (text, image ↔ data URI)
2. URL encode/decode, HTML entities, string escape/unescape
3. JSON / YAML / XML / TOML / SQL formatter + validator
4. Converters: JSON ↔ YAML ↔ CSV ↔ TOML
5. Timestamp converter (Unix ↔ human, timezones)
6. UUID / ULID / nanoid generator
7. Hash generator (MD5, SHA-1/256/512)
8. bcrypt / argon2 hash + verify (for seeding admin users)
9. Random secret / API key / password generator
10. Case & slug converter, lorem ipsum
11. Text diff
12. Markdown preview
13. JWT decoder
14. Regex tester (JS, PCRE/PHP, .NET, Dart flavours)
15. Cron expression builder → "add to project scheduler"
16. QR code generator (URLs, LAN links)
17. Color converter (HEX/RGB/HSL/OKLCH) + WCAG contrast checker
18. Gradient / box-shadow / border-radius generators

### M — Medium, context-aware
19. JWT verify using secret from project `.env`
20. JSON → TypeScript / C# / Dart / PHP / Go classes
21. Screen color picker (eyedropper) + palette generator (shades/tints)
22. Logo → theme (export CSS vars, Tailwind, Flutter ThemeData, MUI)
23. Font preview & pairing + `@font-face` snippet
24. Image optimizer/converter (WebP/AVIF, resize) + SVG optimizer
25. App icon & favicon generator (Android/iOS/Flutter/RN sizes + web manifest, written into project)
26. Clipboard history + smart detect (opt-in, local only, excludes password managers)
27. Hosts file editor (LampRabit entries locked/labelled)
28. .env manager (diff vs `.env.example`, profiles, mask secrets, warn if tracked by git)
29. SSL / certificate inspector + export LampRabit CA for devices
30. Network diagnostics (resolve `.test`, LAN/tunnel reachability, firewall check, LAN IP + QR)
31. Responsive preview (endpoint in multiple viewports)
32. Quick load test (N requests, concurrency, latency percentiles)
33. WebSocket tester — raw WS (connect, headers, send/receive, log)

### L — Large, differentiators
34. WebSocket tester advanced: Socket.IO + Pusher/Reverb modes + **WS doctor** (Feature 12)
35. Webhook catcher & replay (local URL + tunnel, inspect, edit, replay to project)
36. Mock API server (OpenAPI/JSON → endpoint, latency/error toggles)
37. HTTP client lite (project endpoints + env prefilled, saved per project)
38. Fake data seeder (realistic rows incl. Indian locale → insert into DB service)
39. Disk reclaimer (node_modules, vendor, bin/obj, .dart_tool, build caches, Gradle/npm/pip/NuGet
    caches, unused SDKs, old backups)
40. Docker config builder (Feature 14)

### XL — Extra large
41. SSH + SFTP/FTP toolkit: terminal, file browser, SSH tunnels, pull remote DB to local, deploy
    sync (Feature 13; may graduate to the Remote home)

### Not in Tools (already shipped or lives elsewhere)
Port conflicts (shipped), DB client (KarotX), mail inbox (Mailpit, shipped), Stripe/Firebase
(service panels, Feature 10), remote logs (Feature 15).

---

## Changelog
- 2026-11-21: Created. Added ASP.NET Core support plan and generic tunnels plan.
- 2026-11-21: Added core architecture: Project → Processes → Endpoints; web exposure is optional
  and capability-based. .NET support and tunnels should be built on this model (a .NET worker
  service = process with no endpoint; tunnels attach to endpoints, not sites).
- 2026-11-21: Added multi-select Project kinds as auto-detected UX presets; endpoints remain the
  single source of truth for web features.
- 2026-11-21: Added Dart & Flutter support plan (after Tools tab). Added "Mobile app" kind.
- 2026-11-21: Added Mobile Dev Hub (Flutter + React Native/Expo): create via official CLIs,
  unified doctor + JDK manager, phone connection with adb reverse automation, run-on-device.
- 2026-11-21: Added UX principles (five homes, command palette + shared action registry,
  "where is it" actions, progressive disclosure/opt-in modules, contextual nudges, docs-as-product,
  definition of done). Applies to all features.
- 2026-11-21: Added pluggable YAML service manifests (declarative, schema-validated, checksummed,
  built-ins dogfood first, registry via reviewed git repo).
- 2026-11-21: Added Tools tab curated list (Tier 1 context-aware, Tier 2 utilities, Tier 3 design,
  skip list, tool framework requirements).
- 2026-11-21: Added dynamic dashboard plan (adaptive first, live, then customizable; widget
  registry with relevance scoring).
- 2026-11-21: Added gap analysis (clone-to-running, Python/Ollama, shims, security defaults, clean
  uninstall, update/rollback, Docker interop, branch-aware snapshots, stacks) + recommended order.
- 2026-11-21: Corrected against the v0.6.0 website: added full shipped-feature snapshot; tunnels
  marked shipped; manifest moved to existing lamprabit.toml; TOML suggested for service manifests;
  gap analysis rewritten to genuine gaps only.
- 2026-11-21: Clone from git marked as already built. Added feature batch 2 (Features 8–15): projects
  anywhere + recovery, service binding, local BaaS + payments sandbox, WebSockets SSL/tunnels, WS
  tester + doctor, SSH/SFTP/FTP toolkit, Docker config builder, remote logs/health. Proposed 6th
  home "Remote".
- 2026-11-21: Feature 15 confirmed local-only (no monitoring while asleep/off) + log backfill on wake.
  Added Feature 16: remote dev + collaboration in 3 layers (resilient local-first sync, shared
  session + presence, optional Yjs co-editing experiment).
- 2026-11-21: Added final Tools tab build list (41 tools, sized S → XL).

- 2026-11-21: Feature 10 expanded: Firebase Local Emulator Suite (requirements, ports/conflicts,
  data persistence, demo projects, env binding, mobile tie-in) + PocketBase, both project-scoped
  services (`scope: shared | project` in manifests). Owner confirmed both.

- 2026-11-21: Added Feature 17: serverless local development (platform detection table, orchestrate
  platform CLIs, local stand-in services incl. Azurite/LocalStack, event trigger panel). Added
  "Serverless" kind.
- 2026-11-21: Homes finalised by owner: Projects, Services (runtimes stay inside Services as today),
  Devices, Tools, Remote. No separate Runtimes tab.
