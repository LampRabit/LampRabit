# Project overview

A local PHP development environment (Laragon/XAMPP-style) built with **Tauri v2 (Rust) + React 19 + TypeScript + Tailwind v4**. It downloads and runs Apache or Nginx, several PHP versions side by side, MariaDB, MySQL, Mailpit, Redis, Memcached, and serves every folder in `www/` as `https://<folder>.test`. Windows is the reference platform; Linux and macOS have full parity (macOS checked by hand on 2026-11-14 and supported since 0.6.0).

The roadmap is [../plans/roadmap.md](../plans/roadmap.md); releases and the Linux runtime builds are in [RELEASING.md](../../RELEASING.md) for releases and the Linux runtime builds.

## Code layout

- `src-tauri/src/`: the backend. `lib.rs` holds the app state (`AppState`), start-up (`run`), the tray and the background loops (port watcher, schedules, restarts). The Tauri commands are in `cmd/`, one file per area (`services`, `sites`, `projects`, `apps`, `git`, `runtimes`, `databases`, `settings`, `environment`, `control`, `sharing`, `logs`); each starts with `use crate::*;` and its items are `pub(crate)`, re-exported by `cmd/mod.rs`, so `generate_handler!` and other modules name them as before. Shared helpers: `version.rs` (version numbers), `error::last_line`, `procgroup::CREATE_NO_WINDOW`, `diagnose::now_ms`. Key modules:
  - `services.rs`: the supervisor (start/stop, health probes, port checks, launch specs per `ServiceKind`) and `reap_orphans`.
  - `runtimes.rs`: download catalogs per `Runtime`, install (download, checksum, extract), remove.
  - `configs.rs`: generates `httpd.conf`, `my.ini` (MariaDB and MySQL), `php.ini`, `php-fpm.conf`, and initialises database data folders.
  - `php.rs`: PHP FastCGI pools (`php-cgi` workers on Windows, `php-fpm` elsewhere). `sites.rs`: vhosts and the hosts file. `ssl.rs`: local CA and trust. `shims.rs`: `php`/`composer`/`mariadb` command shims. `procgroup.rs`: process trees (Windows Job Objects, Unix process groups).
  - `config.rs`: `Settings` (saved as `settings.toml` in the app config dir), including ports, active versions and stack profiles.
- `src/`: the frontend. `lib/api.ts` mirrors every command and type, so update it alongside `cmd/`. Pages are in `pages/`, shared pieces in `components/`.
- `build/linux/` and `.github/workflows/binaries.yml`: LampRabit's own Linux and macOS builds of Apache, Nginx, PHP, Redis and Memcached, plus the repackaged Windows Memcached (see below). The folder name predates macOS.
