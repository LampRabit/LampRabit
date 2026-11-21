# Phase 9: Linux parity and developer dashboard

Status: **done**. Listed in the [roadmap](../roadmap.md).

## 9.1 Fixes
- Single instance: a second launch brings the running window forward (tauri-plugin-single-instance).
- Composer is a runtime on the Services page: install, switch and remove versions (current and 2.2 LTS); presets install it when needed.
- Linux: the AppImage aborted on systems without libGLESv2 (fixed with `WEBKIT_DISABLE_DMABUF_RENDERER=1`); Apache and Nginx daemonized and went untracked (now run in the foreground); services left behind by a crash are stopped on the next launch.

## 9.2 Linux runtimes (decided: LampRabit's own CI builds)
Apache, Nginx and PHP are built by `.github/workflows/binaries.yml` from `build/linux/` and published to the `binaries` release with a checksummed manifest (details in RELEASING.md). Distro packages were considered and rejected: one PHP version only, per-distro paths and service conflicts. Ports 80/443 need one administrator prompt (`/etc/sysctl.d/50-lamprabit.conf`). HTTPS trust covers the system store plus Chrome/Firefox NSS databases (`certutil`). Open terminal works with GNOME's terminals; the MariaDB client button opens a terminal.

## 9.3 MySQL
Oracle MySQL next to MariaDB (port 3307 by default): 8.4 LTS, the newest LTS and the Innovation release, from MySQL's CDN (Linux: the minimal build), each release line with its own data folder.

## 9.4 Dashboard redesign
- Service list with selection and bulk Start/Stop; each row shows status, port, PID, uptime, port conflicts and problems, CPU and RAM sparklines (PHP workers' usage on the web-server rows) and a click-to-copy connection string.
- **Stack profiles**: named sets of services (saved in settings). Starting a profile starts its services and stops the others; the title bar and Shift+S follow the active profile.
- "Connect your app" panel: every service as a URL or as ready-to-paste `.env` lines, plus "Copy everything as .env".
- Keyboard: `?` lists every shortcut; 1–5 switch pages; J/K or arrows move between services, Space selects, Enter starts/stops, S/X/R/O/C/E/P/L act on the selection or focused service. Dialogs trap focus and restore it; menus use arrow keys.

## 9.5 Databases, Redis and page redesigns
- Default database setting (MariaDB or MySQL); phpMyAdmin and Adminer list every installed database, the default first (fixes logins failing when MariaDB wasn't the running server).
- Redis per platform: Garnet on Windows, with `redis-cli` from the community redis-windows build; real Redis (LampRabit's builds, 8.10 and 7.2) with its own `redis-cli` on Linux and macOS. phpRedisAdmin for browsing Redis data, served like phpMyAdmin. (A custom redis-cli and in-app browser were tried first and removed.) Garnet was replaced by real Redis in 10.1.
- Sites: project cards show framework, git branch, database (opens in phpMyAdmin), Node version, last change, PHP/server/HTTPS controls and per-project sharing; search, framework filter and sort. Port sharing moved to Services → Sharing.
- Services: grouped (web and PHP, databases, developer tools, sharing); the version in use is a large tile, other installed versions are tiles, downloads a list. Node.js uses the same layout.
- Settings: one column of compact sections with a section index; duplicate Mailpit .env block removed (it's on the Overview).
