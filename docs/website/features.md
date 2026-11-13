# LampRabit features

Status labels: **Available** (in the app today) and **Coming soon** (planned, not shipped). Platform notes appear where a feature isn't on every OS; confirm runtime coverage against the current in-app catalog.

---

## Highlights (for the homepage)

- **Every folder is a site.** Put a project in `www/` and open `https://project.test`. No virtual-host files, no hosts-file editing.
- **Real HTTPS, locally.** LampRabit creates its own certificate authority and trusted certificates for every site, sub-domains included.
- **Any PHP version, per project.** Run PHP 8.1 to 8.5 side by side; each site picks its own.
- **Several web and app servers.** Apache, Nginx, Caddy, FrankenPHP and Tomcat, subject to platform/runtime requirements.
- **Command line included.** `php`, `composer` and the MariaDB tools on your PATH, following each site's PHP version.
- **Batteries included.** phpMyAdmin, Adminer and Mailpit, installed with one click.
- **Tells you what's wrong.** Health checks, port-conflict detection that names the program in the way, and a diagnostics report.
- **Project Health and Git.** Check project requirements, preview fixes, review changes and handle everyday Git operations from the project view.
- **CLI and MCP.** Control the same backend from `lamprabit` or supported AI clients.

## Recent capabilities

LampRabit can manage projects beyond PHP and Node.js. Python, Go, Rust and Java runtimes are available where supported; project templates include frameworks such as Django, Flask, FastAPI, Gin, Axum and Spring Boot. Per-project language version files are respected for supported languages.

Projects can define additional domains and long-running processes such as workers and development servers. Processes can have environment variables, ports and restart policies. Project settings can be stored in `lamprabit.toml`; templates can define domains and processes too.

The database and service catalog includes PostgreSQL, MongoDB, SQLite, ClickHouse and Neo4j, plus search, object storage, queues and local AI/automation services. Installers and dependencies vary by platform; the app explains requirements and unavailable runtimes.

The environment folder is selected during first-run onboarding and can be moved later from Settings. Onboarding can install a PHP, Laravel, WordPress, Node.js, full-stack or custom starter. Stack profiles can be exported and imported.

---

## 1. Services

### Service management (Available)
Start, stop and restart Apache, Nginx, MariaDB, MySQL and Mailpit individually, as a selection, or all at once. Each service shows its status, version, port, process ID, uptime and live CPU and memory use, and warns before you start it if another program holds its port.

### Stack profiles (Available)
Save named sets of services, such as "Laravel" (Apache, MariaDB, Redis, Mailpit) or "WordPress" (Apache, MySQL). Starting a profile starts its services and stops the rest, so switching between projects is one click or Shift+S.

### phpRedisAdmin (Available)
Browse and edit your local Redis in the browser at `phpredisadmin.test`, the way phpMyAdmin works for databases. Install it from Services; it's set up for LampRabit's Redis with no login (it's only reachable from your computer).

### Default database (Available)
Run MariaDB and MySQL side by side and choose which one is the default: it's what `mysql` and `mysqldump` use, where new projects create their database, and the first server phpMyAdmin and Adminer offer (both list every installed database).

### Connection details (Available)
Every service shows how your app connects to it, as a URL (`mariadb://root@127.0.0.1:3306`) or as ready-to-paste `.env` lines (`DB_HOST`, `DB_PORT`, `REDIS_HOST`, `MAIL_PORT`…). One click copies them; "Copy everything as .env" copies them all.

### Keyboard first (Available)
Press `?` for every shortcut. Number keys switch pages; J/K (or the arrow keys) move between services, Space selects, Enter starts or stops, and single keys restart, open, copy connection details and pick a profile. Everything is also reachable with Tab, with visible focus. Quitting LampRabit stops its services. If it crashes, Windows stops them at once; on Linux and macOS, LampRabit stops any leftovers the next time it opens, so nothing keeps holding a port.

### One-click runtimes and versions (Available; platform-limited)
Install, switch and remove versions of Apache, Nginx, PHP, MariaDB, MySQL and Composer from the Services page. Downloads come straight from the official sources (Apache Lounge, nginx.org, windows.php.net, mariadb.org) and are checked against their published SHA-256 checksums before use (nginx.org publishes PGP signatures only; those downloads rely on HTTPS). Switching a version restarts the services that use it.

Platform note: Runtime downloads depend on platform and architecture. LampRabit builds Apache, Nginx and PHP for Linux and macOS; consult the Services catalog for current availability and checksums.

### Your stack at a glance (Available)
A live diagram of web server → PHP → MariaDB, animated while requests can flow, with each runtime's versions on a timeline: active, installed and available at a glance.

### Health checks (Available)
LampRabit checks running services every few seconds: Apache and Nginx must answer HTTP, MariaDB must accept connections, Mailpit's health endpoint must respond. A service that is running but not answering is flagged "Not responding" instead of looking fine. Web server configurations are tested before every start, so a broken config file is caught with the file and line number.

### Problems explained (Available)
When a service fails to start or stops unexpectedly, its card explains why in plain language and suggests a fix: port already in use, missing Microsoft Visual C++ runtime, invalid configuration line, MariaDB data from a different version, and more.

### Port conflict detection (Available)
If a port is taken, LampRabit names the program holding it (for example `node.exe, PID 6096`), explains common culprits (IIS / Windows HTTP Server, Skype, another XAMPP or Laragon, Docker), and offers to change the port or stop that program. Settings shows live status under every port field.

### Apache and Nginx (Available; Nginx on Windows)
Run Apache, Nginx, or both. With both, Apache keeps ports 80/443 and Nginx gets its own (8080/8443 by default), and you choose the server for each site. Overlapping ports are rejected before anything starts. Nginx sites get front-controller routing out of the box, so Laravel, Symfony and WordPress pretty URLs just work.

Platform note: Nginx downloads are available on Windows (nginx.org builds), Linux and macOS (LampRabit's builds); macOS remains in preview.

### System tray (Available)
LampRabit lives in the system tray: start or stop everything, open the www folder or a terminal, and quit from its menu. Closing the window keeps your services running (you can turn this off).

### Start with your computer (Available)
Start LampRabit when you sign in (it opens quietly in the tray), and optionally start all services as soon as it opens.

### Open a terminal (Available)
A terminal with PHP, Composer and the MariaDB client on the PATH, from the title bar, the tray or any site card (opens in the project folder).

### PHP, Composer and MariaDB on your PATH (Available)
One switch in Settings puts `php`, `composer`, `mysql`, `mariadb`, `mysqldump` and friends on your PATH, so they work in any terminal, IDE or script. They always run the versions you've made active, with LampRabit's php.ini and database port, and inside a site that has its own PHP version, `php` and `composer` use that version automatically. No admin rights needed; on macOS and Linux it's a marked block in your shell profile that's removed cleanly when you switch it off.

### Open in editor (Available)
Open any project in VS Code, Cursor, PhpStorm, Sublime Text, Zed or another editor command of your choice.

### Visual C++ runtime check (Available, Windows)
Apache and PHP on Windows need Microsoft's Visual C++ runtime. LampRabit checks for it and, if it's missing, installs it with one click (one administrator prompt).

### Safe MariaDB upgrades (Available)
LampRabit remembers which MariaDB version last used your databases. Moving to a newer release upgrades the system tables automatically; moving back to an older one is blocked with an explanation, because it can damage data.

---

## 2. PHP

### Multiple PHP versions side by side (Available; Windows downloads)
Install PHP 8.1, 8.2, 8.3, 8.4 and 8.5 together. Each site uses the default version or its own.

### Per-site PHP version (Available)
Pick the PHP version for a site from its card on the Sites page. LampRabit runs a pool of PHP FastCGI workers for each version in use and routes each site to the right one. In a terminal, `php` and `composer` inside that site's folder use the same version.

### Sensible defaults (Available)
Each PHP version gets its own `php.ini` with developer-friendly settings: errors shown, 512 MB memory limit, 128 MB uploads, OPcache on (with instant reloads), and common extensions enabled (curl, gd, intl, mbstring, mysqli, openssl, pdo_mysql, pdo_sqlite, sodium, zip and more). Per-project overrides work through a `.user.ini` file in the project.

### PHP extension manager (Available, Windows)
Browse every extension for each installed PHP version, switch them on or off, and add popular extras with one click: Xdebug, Redis (phpredis), APCu, igbinary, MongoDB, PCOV, Imagick and YAML. Each extra shows the version available for that exact PHP release, or "no build for PHP 8.x yet". Xdebug comes preconfigured for step debugging (trigger mode, port 9003), and PHP restarts automatically when you change something. On macOS and Linux this arrives with LampRabit's PHP downloads for those platforms.

### Worker pools (Available)
On Windows each PHP version runs as several worker processes (4 by default, adjustable) so pages load in parallel. Crashed workers restart automatically; a worker that keeps crashing is stopped and reported instead of looping.

---

## 3. Sites

### Automatic sites (Available)
Every folder in `www/` becomes a site at `<folder>.test` (the suffix is configurable, for example `.localhost`). Sub-domains work too: `api.shop.test` is served by `shop`. If a project has a `public/` folder (Laravel, Symfony), that's what gets served.

### Local HTTPS with a trusted certificate (Available)
LampRabit creates a local certificate authority once; trust it with one click (Windows asks you to confirm; on macOS it uses your login keychain). Every site then gets a certificate for its domain and sub-domains, renewed automatically. HTTPS is on by default for new sites and can be turned off per site or globally.

### Hosts file managed for you (Available)
When Apache starts, LampRabit adds your sites to the system hosts file, with one administrator prompt only when the list changed. Only LampRabit's own marked section is touched; everything else is kept byte for byte, read-only hosts files are handled, and a backup of the previous file is kept. With the `.localhost` suffix no hosts file is needed at all.

### Apply changes (Available)
Add a folder while the server is running and LampRabit offers to apply it: domains, certificates and the hosts file update together.

### Create a project (Available)
A guided "New project" dialog: name the project (with a live preview of its URL), pick a template, choose the PHP version, web server and HTTPS, and optionally create a database. Progress and tool output stream live. Requirements (PHP version, extensions, Node, Git) are checked before anything is created, and if a step fails, nothing is left behind and you can try again.

### Project templates (Available)
- **Blank PHP** and **Static site**: ready instantly.
- **Laravel**: the latest Laravel via Composer, with `.env` pointing at the new database and Mailpit, and migrations run. Optionally builds the frontend with npm.
- **Symfony**: the Symfony skeleton with `DATABASE_URL` and `MAILER_DSN` set.
- **WordPress**: the latest release (checksum-verified) with `wp-config.php` written, including fresh security keys; finish the install in the browser.
- **CakePHP**, **CodeIgniter 4**, **Drupal** (database settings filled in for its installer), **Statamic** and **Magento** (Mage-OS or Adobe's edition, installed with an admin account).
- **Vite** (React, Vue, Svelte, Solid, Preact, Lit, Qwik, vanilla), **Next.js**, **Nuxt**, **Astro**, **Express** and **NestJS**, on the Node.js version and package manager you choose. Each asks the questions its own CLI does (TypeScript, linter, router, Tailwind, modules, integrations…), and only the ones that apply to your earlier answers.
- Every template can also initialise a Git repository.

Templates are YAML files, and you can add your own in the `templates` folder (**New project → Your templates**). LampRabit asks before running a risky step from a template you added (deleting files, piping a download into a shell, running as administrator…). See [the template reference](../templates.md).

Composer is downloaded and verified the first time it's needed.

### Node.js projects (Available)
A project with a package.json can run its dev server from its Sites page: on the Node.js version you pick from NVM (or what `.nvmrc` asks for; NVM's own default never changes) and with npm, pnpm, Yarn or Bun. You never type a serve command: it comes from the project's template or its package.json, with an optional override. Node projects start only when you start them, never with your services.

### The domain follows your dev server (Available)
LampRabit watches which ports each project's processes listen on, whether it started them or you did in a terminal (`npm run dev`, `php artisan serve`…), and points the project's domain and its HTTPS certificate at that port within a few seconds. When the dev server stops, the domain goes back to the project's files (its build folder, for example). With several ports, the lowest wins and you can pick another. PHP projects stay on PHP unless you choose a port, so Laravel's Vite never takes over the site. Web servers reload without dropping connections where they can.

### Project commands (Available)
Each project lists the commands it offers: its template's own, its package.json and composer.json scripts, and artisan, Symfony console, Magento, Statamic, Drush, CakePHP and CodeIgniter commands. Run any of them (or any command you type) in the project folder with its PHP and Node versions, see the output right there, and find it again in Logs. Keep the ones you use often.

### Project overview (Available)
Open a project to see everything about it: framework and version, language, PHP or Node.js version and what the project requires, where its domain points (the dev server's port, or which folder is served), every port it listens on, whether its dependencies are installed (with an Install button if not), common `.env` settings (never keys or passwords), its database, and its git branch, uncommitted changes and last commit.

### Coming soon
- **Extra domains and several processes per project** (a queue worker, the scheduler and Vite next to the site).

### Share a site on the internet (Available)
Put any site on a public URL from its card, for testing webhooks, showing a client or trying a phone. Requests reach the site with its own domain, so virtual hosts keep working. Share any other local port too (for example a Node dev server). Choose Cloudflare quick tunnels (no account, random trycloudflare.com address) or Ngrok (free account; your authtoken is kept in the system keychain, never in a file). Apps that store their own URL (WordPress, Laravel's `APP_URL`) may still redirect to the `.test` domain.

---

## 4. Databases

### MySQL (Available; Windows and Linux downloads)
Oracle MySQL runs next to MariaDB on its own port (3307 by default), so projects that need MySQL specifically get the real thing. Choose 8.4 LTS, the newest LTS or the Innovation release; each keeps its own data, so switching is always safe. Root has no password, like MariaDB.

### MariaDB (Available; Windows and Linux downloads)
Several MariaDB versions available, including long-term-support releases. The data directory is created on first start with a passwordless local `root` user, like XAMPP and Laragon. MariaDB shuts down gracefully so no data is lost.

### phpMyAdmin and Adminer (Available)
Install either or both from Services. They're served at `phpmyadmin.test` and `adminer.test` (and at `localhost/phpmyadmin`, `localhost/adminer`, which work without the hosts file). Local `root` logs in without a password. The MariaDB card opens phpMyAdmin directly; Overview has quick links.

### Command-line client (Available; Windows)
Open the MariaDB client in a terminal, already connected, from the MariaDB card's menu.

### Redis (Available)
Real Redis on every platform: LampRabit's own builds (8.10 or 7.2) on Linux and macOS, and on Windows, where Redis has no official build, the community redis-windows build (Redis compiled for Windows), checked against its published SHA-256. `redis-cli` comes with it and works in LampRabit's terminal, and in any terminal once Settings → Terminal → Add LampRabit's commands to PATH is on. phpRedisAdmin shows its data. It runs on port 6379 with health checks and logs like every other service. PHP apps can use it through Predis or the phpredis extension (one click in the PHP extension manager).

### Memcached (Available)
Install it from Services like any other runtime: LampRabit's own builds on Linux and macOS, and on Windows nono303's build (Memcached has no official Windows build), checked and repackaged by LampRabit. It runs on port 11211 with 128 MB of memory, with health checks and logs. PHP talks to it through the `memcached` extension: built into LampRabit's PHP on Linux and macOS, one click in the PHP extension manager on Windows.

---

## 5. Email testing

### Mailpit (Available)
A local mail server that catches every email your apps send and shows it in a web inbox at `http://localhost:8025` (or `mail.test`). Nothing reaches real inboxes.

### PHP mail() routed automatically (Available)
PHP's `mail()` is pointed at Mailpit for you (Windows: php.ini SMTP settings; macOS/Linux: Mailpit's sendmail mode). Frameworks that send over SMTP (Laravel, Symfony, WordPress SMTP plugins) just need `MAIL_HOST=127.0.0.1` and `MAIL_PORT=1025`; the Settings page shows the exact values.

---

## 6. Developer tools and runtimes

### Node.js through NVM (Available)
LampRabit manages Node.js versions through NVM. If you already use NVM it's detected and reused; otherwise it's installed only when you ask (Windows: the official nvm-windows installer, with one administrator prompt; macOS and Linux: the official nvm script). Install, switch the default and remove Node versions from Services, with LTS releases labelled. Sites show the Node version their `.nvmrc` asks for, and warn if it isn't installed. On Windows, switching versions needs administrator approval unless Developer Mode is on; LampRabit tells you which applies.

---

## 7. Logs and troubleshooting

### Live logs (Available)
One place for every log: Apache and Nginx error and access logs, PHP errors, MariaDB, Mailpit and LampRabit's own activity. Each line shows its time, service and severity. History is read from disk, so you see earlier runs too.

### Filter, search and export (Available)
Logs from services and from each project (its dev server and the commands you ran) in one place. Pick sources, search with case-sensitive, regex or exclude options, filter by severity, stream (output, errors, access, commands) and time range.

Filter by service, severity (error, warning, notice, info, access, debug) and time range; search text; pause the live view; export what you see to a file; clear all log files.

### System activity log (Available)
Everything LampRabit does is recorded: services starting, stopping, restarting or crashing, versions installed or switched, settings saved, sites applied, hosts file updates, certificate trust. Saved to `logs/lampRabit.log`.

### Diagnostics report (Available)
One click produces a report with versions, Microsoft runtime presence, service health, who is using each port, web server config tests, PHP workers, hosts and certificate status, and recent errors. Copy it into a bug report or save it, with your user name and home folder hidden.

---

## 8. App

### Modern interface (Available)
Overview dashboard with live status, quick links and recent activity; light, dark and system themes; custom title bar; works at any window size. Subtle motion makes it feel responsive: pages and dialogs ease in, buttons give under the cursor, and the rabbit in the title bar twitches its ears, looks around and wiggles its nose every few minutes.

### Motion you control (Available)
Settings → Appearance → Motion: follow the system's reduce-motion preference (default), full motion, reduced (gentle fades only; nothing slides, bounces or plays by itself), or off.

### Platform-aware (Available)
Settings and tools only appear where they apply. For example the PHP worker count is a Windows setting, and hosts-file paths and certificate steps match your OS.

### App updates (Available)
LampRabit checks GitHub for a new version when it starts (you can turn this off), shows it in the title bar, and installs it in place after you confirm. Updates are cryptographically signed; your environment, sites and databases are untouched.

### Runtime update notices (Available)
When a newer patch of an installed Apache, Nginx, PHP, MariaDB, Mailpit, Adminer, phpMyAdmin or Redis is available (for example PHP 8.4.27 → 8.4.30), Services shows a badge and a one-click install. New versions are installed alongside the old one; you switch when you're ready.

### Installers (Available)
Windows (`.exe` / `.msi`), macOS for Apple Silicon and Intel (`.dmg`), and Linux (`.AppImage`, `.deb`) from GitHub Releases. Builds aren't code-signed yet, so Windows and macOS show a warning on first launch; the installation guide explains the one-time step.

### Coming soon
- **Code-signed installers** for Windows and macOS.
- **Complete macOS parity and stability work.** macOS builds and runtime coverage are still in preview; check the current release notes for gaps.

---

## Comparison (for a table on the Features page)

| | LampRabit | XAMPP | Laragon |
|---|---|---|---|
| Automatic `project.test` domains | Yes | No (manual vhosts) | Yes |
| Trusted local HTTPS | Yes, per site | Self-signed, manual | Yes |
| Several PHP versions at once, per site | Yes | No | Switch globally |
| Apache and Nginx side by side | Yes | Apache only | One at a time |
| One-click version downloads | Yes | Reinstall | Manual download |
| Port conflict explanation | Names the program and suggests fixes | Error message | Error message |
| Health checks and diagnostics report | Yes | No | No |
| Mail catcher | Mailpit | Mercury | Mailpit option |
| Runs without administrator rights | Yes (asks only for hosts and certificates) | Often needs admin | Usually run as admin |
| Linux | Yes | Yes | Windows only |
| macOS | Preview (runtime coverage varies) | Yes | Windows only |
| Open source | Yes | Yes | Source available |

Keep the comparison factual and update it if competitors change; avoid disparaging language.
