# LampRabit features

Status labels: **Available** (in the app today) and **Coming soon** (planned, not shipped). Platform notes appear where a feature isn't on every OS; the in-app Services catalog is the final word on what can be installed on each OS and processor (see also the table in [installation.md](installation.md)).

LampRabit runs on **Windows, macOS (Apple silicon and Intel) and Linux**.

---

## Highlights (for the homepage)

- **Every folder is a site.** Put a project in `www/` and open `https://project.test`. No virtual-host files, no hosts-file editing.
- **Real HTTPS, locally.** LampRabit creates its own certificate authority and trusted certificates for every site, sub-domains included.
- **Every language, several versions.** PHP 8.1 to 8.5, Node.js, Python, Java, Go and Rust side by side; each project uses the version it asks for.
- **Five web and app servers.** Apache, Nginx, Caddy, FrankenPHP and Tomcat, side by side, chosen per site.
- **Any database.** MariaDB, MySQL, PostgreSQL, MongoDB, SQLite, ClickHouse and Neo4j, with backups and the admin tools that go with them.
- **Projects, handled.** Create one from a template, see its health, Git, commands, dependencies, domains and processes in one place.
- **Tells you what's wrong.** A project doctor with one-click fixes, health checks, port-conflict detection that names the program in the way, and a diagnostics report.
- **Command line and AI.** `php`, `composer`, `mysql` and friends on your PATH; the `lamprabit` CLI and an MCP server for AI assistants.

---

## 1. Services

### A catalog of services (Available)
The Services page groups everything LampRabit can install: web and app servers, languages, package managers, databases, search, storage, queues, AI and automation, caches, developer tools and sharing. **Essentials** shows what most projects need; **Everything** shows the rest. Each entry says where the download comes from, what it needs (Tomcat needs Java, n8n needs Node.js 24+, pgAdmin needs Python) and installs those first with your OK, and suggests what goes well with it (phpMyAdmin with MariaDB, Compass with MongoDB). Where something can't run on your computer, it says why instead of hiding it.

### Service management (Available)
Start, stop and restart services one by one, as a selection, or all at once. Each shows its status, version, port, process ID, uptime and live CPU and memory use, and warns before it starts if another program holds its port. Services that crash can restart by themselves (Settings → Startup).

**Start all** starts what's installed except the heavy services (Elasticsearch, OpenSearch, ClickHouse, Neo4j, Ollama, n8n and RabbitMQ), which start from their own card or a stack profile.

### Stack profiles (Available)
Save named sets of services, such as "Laravel" (Apache, MariaDB, Redis, Mailpit) or "Spring" (Tomcat, PostgreSQL). Starting a profile starts its services (and the project processes it lists) and stops the rest: switching between projects is one click or Shift+S. Profiles export to and import from JSON, with the downloads the other computer needs.

### One-click runtimes and versions (Available)
Install, switch and remove versions from the Services page. Downloads come from official sources wherever they exist and are checked against their published checksums before use. On Linux and macOS, where nobody publishes portable builds, Apache, Nginx, PHP, Redis, Memcached and MariaDB (macOS) come from LampRabit's own builds, made in the open on GitHub Actions. Switching a version restarts the services that use it, and newer patch releases show up as one-click updates installed next to the old version.

### Connection details (Available)
Every service shows how your app connects to it, as a URL (`mariadb://root@127.0.0.1:3306`) or ready-to-paste `.env` lines (`DB_HOST`, `DB_PORT`, `REDIS_HOST`, `MAIL_PORT`…). "Copy everything as .env" copies them all.

### Health checks and problems explained (Available)
Running services are probed every few seconds; a service that runs but doesn't answer is flagged instead of looking fine. Web server configurations are tested before every start, so a broken config is caught with the file and line. When a service fails, its card explains why in plain language and suggests a fix: a port in use, a missing Visual C++ runtime, an invalid config line, database files from a newer version.

### Port conflict detection (Available)
If a port is taken, LampRabit names the program holding it (for example `node.exe, PID 6096`), explains the usual suspects (IIS, Skype, another XAMPP or Laragon, Docker), and offers to change the port or stop that program.

### Keyboard first (Available)
Press `?` for every shortcut. Number keys switch pages; J/K move between services, Space selects, Enter starts or stops. Everything is reachable with Tab, with visible focus.

---

## 2. Web and app servers

### Apache, Nginx, Caddy and FrankenPHP (Available)
Turn on any of them; the first is the primary server (ports 80 and 443, `localhost` and the tools) and each other one gets its own ports. Each site picks its server. Nginx and Caddy route unknown URLs to `index.php`, so Laravel, Symfony and WordPress pretty URLs just work; Apache honours `.htaccess`. Configs are tested before a reload, and servers reload without dropping connections where they can.

FrankenPHP runs PHP itself (its own PHP, not LampRabit's versions) and can keep a Laravel Octane or plain PHP app in memory with **worker mode**, per site. Platform note: FrankenPHP publishes no build for Intel Macs.

### Tomcat (Available)
Java web apps (a WAR or an exploded `WEB-INF` folder) are served by Tomcat 9, 10.1 or 11 on the active Java, behind your web server at their own `.test` domain.

---

## 3. Languages

### PHP, side by side (Available)
Install PHP 8.1 to 8.5 together; each site uses the default version or its own, and in a terminal `php` and `composer` inside a project use that project's version. Each version gets its own `php.ini` with developer-friendly defaults (errors shown, 512 MB memory, 128 MB uploads, OPcache on), and each site can add its own php.ini lines and turn Xdebug on or off, without touching the others.

### PHP extension manager (Available on Windows)
Switch extensions on or off per PHP version and add extras with one click: Xdebug, Redis, Memcached, APCu, igbinary, MongoDB, PCOV, Imagick and YAML, each shown with the version available for that exact PHP release. On Linux and macOS, LampRabit's PHP builds come with the common extensions (Redis, Memcached and igbinary included) compiled in.

### Node.js through NVM (Available)
LampRabit manages Node.js through NVM: an existing NVM is reused, otherwise it's installed when you ask. Install, switch and remove Node versions from Services; projects get the version their `.nvmrc` asks for without changing NVM's default.

### Python, Java, Go and Rust (Available)
Python (python-build-standalone), Java (Eclipse Temurin), Go (go.dev) and Rust (through rustup, kept inside LampRabit's folder) install like any other runtime. Projects get the version their `.python-version`, `.java-version`/`.sdkmanrc`, `go.mod` or `rust-toolchain.toml` asks for, and the Sites page shows it.

### Package managers (Available)
Composer, Bun, Deno, uv, conda (Miniforge), Maven and Gradle are runtimes; npm comes with Node.js, pip with Python and Cargo with Rust. pnpm and Yarn are turned on through corepack from the Node.js card. Everything LampRabit runs uses your system's certificates, so npm works behind company proxies that re-sign HTTPS (with Node.js 22.19 / 24.6 or newer).

---

## 4. Projects

### Automatic sites (Available)
Every folder in `www/` becomes a site at `<folder>.test` (the suffix is configurable, for example `.localhost`). Sub-domains work too, and a project with a `public/` folder (Laravel, Symfony) is served from it. LampRabit recognises PHP, Node.js, Python, Java, Go, Rust and static projects and their frameworks.

### Local HTTPS with a trusted certificate (Available)
LampRabit creates a local certificate authority once; trust it with one click. Every site gets a certificate for its domain and sub-domains, renewed automatically. HTTPS is on by default and can be turned off per site.

### Hosts file managed for you (Available)
When the web server starts, LampRabit adds your sites to the system hosts file, asking for approval only when the list changed. Only LampRabit's own marked section is touched, and a backup of the previous file is kept. With the `.localhost` suffix no hosts file is needed.

### Create a project (Available)
A guided **New project** dialog: name it (with a live preview of its URL), pick a template, choose versions, the web server and HTTPS, and optionally create a database on the engine you choose. Requirements are checked before anything is created; if a step fails, nothing is left behind.

### Project templates (Available)
- **PHP:** Blank PHP, Laravel, Symfony, WordPress, CakePHP, CodeIgniter 4, Drupal, Statamic and Magento, with their database, mail and URL settings filled in.
- **JavaScript:** Vite (React, Vue, Svelte, Solid, Preact, Lit, Qwik, vanilla), Next.js, Nuxt, Astro, Express and NestJS, asking the questions their own CLIs ask.
- **Python, Go, Rust and Java:** Django, Flask, FastAPI, Go (net/http), Gin, Axum, Spring Boot and a plain Java web app for Tomcat.
- **Static site**, and every template can start a Git repository.

Templates are YAML. Write your own with the **template editor** (a guided form for every language, or the YAML itself), or save an existing project as a template. LampRabit asks before a template you added runs a risky step. See [the template reference](../templates.md).

### Import from other tools (Available)
Bring projects over from Laragon, XAMPP, AMPPS, MAMP, Herd, Valet, FlyEnv, EnvKit, WampServer, ServBay, Local or any folder. Projects are copied into `www/` (the originals are never moved or changed); the database names are listed with how to export them.

### The project card (Available)
Open a project to see everything about it, in tabs:

- **Overview:** framework and version, language and version, where its domain points, ports, dependencies, `.env` settings (never keys or passwords), its database and Git state, and its **health**.
- **Git:** changes with diffs, staging (whole files or single hunks), commits, history, branches, merges and rebases with conflict resolution, stash, tags, remotes, and fetch/pull/push with progress. It uses your own Git, so your credentials and SSH keys apply; on Windows LampRabit can install Git for you. See [Git workflow](../git.md).
- **Commands:** everything the project offers (package.json, composer.json, artisan, Symfony console, Drush, deno.json tasks, pyproject.toml scripts, Maven, Gradle, Cargo, Go…) plus your own, with live output and a history of runs. **Processes** keep commands running (queue workers, Vite, an API) with restart policies, start with the project or with LampRabit, or run on a cron schedule.
- **Dependencies:** each package manager's packages (Composer, npm, pnpm, Yarn, Bun, Deno, uv, Poetry, pip, Cargo, Go, Maven, Gradle) with what the project asks for and what's installed. Check for updates, install, add, update, upgrade or remove: each change shows its command first and runs only when you confirm.
- **Domains:** extra domains pointing at a folder, a port or one of the project's processes.
- **Settings:** PHP version, web server, HTTPS, php.ini lines and Xdebug, and `lamprabit.toml`, the project's setup saved in its own folder.

### Project doctor (Available)
Checks a project against your environment: PHP version and extensions against composer.json, missing or outdated dependencies, a missing `.env`, the database (installed, running, created), Laravel's `APP_URL`, the hosts file, the certificate and Git's identity. Each issue says why it matters, and fixes are previewed before they run. See [Project doctor](../project-doctor.md).

### The domain follows your dev server (Available)
LampRabit watches which ports each project's processes listen on, whether it started them or you did in a terminal, and points the project's domain and certificate at that port within seconds. When the dev server stops, the domain goes back to the project's files.

### Share a site on the internet (Available)
Put any site or local port on a public URL, for webhooks, a client or a phone: Cloudflare quick tunnels (no account) or ngrok (your token stays in the system keychain).

---

## 5. Databases

### MariaDB and MySQL (Available)
Run them side by side (MySQL on port 3307) and choose the default: it's what `mysql` and `mysqldump` use and where new projects create their database. MySQL keeps one data folder per release line, so switching is always safe; moving MariaDB to an older version is blocked to protect your data. Platform note: Oracle's macOS builds of MySQL need a recent macOS.

### PostgreSQL, MongoDB, SQLite, ClickHouse and Neo4j (Available)
PostgreSQL 14 to 18 (with a `root` superuser so framework defaults work), MongoDB with mongosh and its database tools, the sqlite3 shell, ClickHouse with its web console, and Neo4j Community with Neo4j Browser. Platform note: ClickHouse has no Windows build.

### Backups (Available)
Back up and restore MariaDB, MySQL, PostgreSQL, MongoDB and SQLite databases from their details, into the `backups/` folder (kept when LampRabit is uninstalled), restoring into any database name.

### Database tools (Available)
phpMyAdmin (MariaDB, MySQL), Adminer (MariaDB, MySQL, PostgreSQL), phpPgAdmin and pgAdmin (PostgreSQL), phpRedisAdmin (Redis) and MongoDB Compass, each set up for LampRabit's servers. Local `root` logs in without a password.

### Caches (Available)
Real Redis on every platform (with `redis-cli`) and Memcached, with health checks and logs like every other service.

---

## 6. More services

- **Search (Available):** Meilisearch, Typesense, Elasticsearch and OpenSearch. Typesense has no Windows build; OpenSearch has no macOS build.
- **Object storage (Available):** RustFS and MinIO, both S3-compatible, with their consoles; their storage folder can be anywhere. RustFS has no Intel Mac build; MinIO is its last release, labelled as no longer maintained upstream.
- **Queues (Available):** RabbitMQ with its management console (Erlang is installed on Windows; elsewhere it uses your system's Erlang).
- **AI and automation (Available):** Ollama for local models (an Ollama you installed yourself is used as is) and n8n for workflows.

---

## 7. Email testing

### Mailpit (Available)
Catches every email your apps send and shows it in a web inbox (`mail.test` or `http://localhost:8025`). PHP's `mail()` is routed to it for you; frameworks that use SMTP need `MAIL_HOST=127.0.0.1` and `MAIL_PORT=1025`.

---

## 8. Command line and AI assistants

### LampRabit's commands on your PATH (Available)
`php`, `composer`, `mysql`, `psql`, `redis-cli`, `python`, `go`, `cargo`, `bun`, `uv` and the others work in any terminal, always running the active (or the project's) version with LampRabit's settings. On by default; on macOS and Linux it's a marked block in your shell profile, removed cleanly when you switch it off. **Open terminal** (title bar, tray, project card) opens one with everything ready.

### The `lamprabit` CLI (Available)
Status, services, profiles, projects, the doctor, logs (with `--follow`), templates and Git from a terminal or a script, with JSON output and clear exit codes. It starts LampRabit in the background if needed. See [CLI and MCP](../cli.md).

### MCP server for AI assistants (Available)
Settings → AI assistants connects Claude Desktop, Claude Code, Codex, Cursor, VS Code, GitHub Copilot CLI, Windsurf, Gemini CLI, Antigravity, LM Studio, Cline, Roo Code, Kiro and opencode with one click, or serves MCP over HTTP on 127.0.0.1 with a token. Assistants can read status, logs and the doctor, and (marked as changes) start services or projects and apply fixes. Settings and credentials are never exposed.

---

## 9. Logs and troubleshooting

### Logs (Available)
Services, projects (their dev servers, processes and commands) and LampRabit's own activity in one place, with each service's timestamps read correctly. Pick sources, search (regex, case, exclude), filter by severity, stream and time range, fold repeats, see only problems, pause, copy and export. Old log files rotate (Settings → Logs).

### Dump catcher (Available)
Turn it on and Laravel's and Symfony's `dump()` and `dd()` output from web requests shows in Logs instead of in the page.

### Diagnostics report (Available)
One click produces a report with versions, service health, who is using each port, config tests, hosts and certificate status and recent errors, with your user name hidden, ready for a bug report.

---

## 10. The app

### Setup guide (Available)
The first run asks where LampRabit's folder goes, then "What do you build?" (PHP, Laravel, WordPress, JavaScript, Python, Java, Go and Rust…) and installs what that needs, offers to import projects from another tool, and creates a first project. Reopen it from Settings → General.

### Your folder, anywhere (Available)
Everything lives in one folder (`C:\lampRabit` or `~/lampRabit` by default). Move it later from Settings: LampRabit checks the target, stops what's using the folder, moves it and repairs paths.

### Interface (Available)
Home dashboard with live status, projects and quick links; light, dark and system themes; a tab bar you can dock to any edge; motion you control (system, full, reduced or off). LampRabit lives in the system tray, can start when you sign in and start your services, and keeps them running when you close the window (optional).

### App updates (Available)
LampRabit checks for a new version when it starts (optional) and installs it after you confirm: services stop cleanly first, updates are signed, and your folder, projects and databases are untouched.

### Uninstall that keeps your work (Available)
Removing LampRabit deletes only what it created (runtimes, configs, logs, certificates) and undoes its system changes (hosts entries, certificate trust, PATH), keeping `www/`, `data/` and `backups/`.

### Installers (Available)
Windows (`.exe`, `.msi`), macOS for Apple silicon and Intel (`.dmg`) and Linux (`.AppImage`, `.deb`) from GitHub Releases.

### Coming soon
- **Code-signed installers** for Windows and macOS (today both show a one-time warning on first launch; the installation guide explains it).

---

## Comparison (for a table on the Features page)

| | LampRabit | XAMPP | Laragon |
|---|---|---|---|
| Automatic `project.test` domains | Yes | No (manual vhosts) | Yes |
| Trusted local HTTPS | Yes, per site | Self-signed, manual | Yes |
| Several PHP versions at once, per site | Yes | No | Switch globally |
| Web servers | Apache, Nginx, Caddy, FrankenPHP, Tomcat, side by side | Apache | Apache or Nginx, one at a time |
| Databases | MariaDB, MySQL, PostgreSQL, MongoDB, SQLite, ClickHouse, Neo4j | MariaDB | MySQL or MariaDB, more through add-ons |
| Languages, several versions each | PHP, Node.js, Python, Java, Go, Rust | PHP, Perl | PHP, with others through add-ons |
| One-click version downloads | Yes | Reinstall | Manual download |
| Project health checks with fixes | Yes | No | No |
| Git and dependency panels per project | Yes | No | No |
| CLI and MCP server for AI assistants | Yes | No | No |
| Port conflict explanation | Names the program and suggests fixes | Error message | Error message |
| Mail catcher | Mailpit | Mercury | Mailpit option |
| Runs without administrator rights | Yes (asks only for hosts and certificates) | Often needs admin | Usually run as admin |
| Windows, macOS, Linux | All three | All three | Windows only |
| Open source | Yes (MIT) | Yes | Source available |

Keep the comparison factual and update it if competitors change; avoid disparaging language.
