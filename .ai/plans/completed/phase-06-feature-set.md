# Phase 6: feature set

Status: **done**. Listed in the [roadmap](../roadmap.md).

Platform rule for everything below: a setting or tool is shown only on platforms where it works; where it doesn't, say why instead of hiding it silently.

Order: 6.1 → 6.2 → 6.3 → 6.4 → 6.5 (reliability first; per-project PHP and presets build on the web-server work).

## 6.1 Service reliability ✅ (plus custom title bar)
- **Health checks**: beyond "process alive": Apache answers an HTTP probe, MariaDB answers `mariadb-admin ping`, configs pass `httpd -t` before start. States: running / stopped / degraded / failed, each with a plain-language cause and fix (known patterns: port in use, missing VC++ runtime, bad config line, InnoDB recovery).
- **Port conflicts**: identify the owning process (Windows: TCP table → PID → name; macOS/Linux: `lsof`), explain common culprits (IIS/`http.sys` as PID 4, Skype, another XAMPP/Laragon), offer "use another port" or "stop that process" when it's safe.
- **Activity logs**: existing System stream plus filters by severity and time range, and export (save dialog → `.log`/`.txt`).
- **Diagnostics report**: OS and app versions, runtimes, ports and owners, config test output, hosts/CA status, recent errors; copy or save as a file, with user paths optionally redacted.

## 6.2 Web servers ✅
- **Apache, Nginx or both**: Nginx from nginx.org (Windows zips; macOS/Linux via own builds or package manager). PHP for Nginx runs as FastCGI (`php-cgi` pool on Windows, `php-fpm` elsewhere). With both enabled, each site picks its server; the second server gets its own ports (default 8080/8443) and the UI blocks overlapping ports before start.

## 6.3 Projects ✅
- **Create project** wizard: name → folder in `www/` → preset → options (database, HTTPS) → done, with live progress.
- **Presets**: Blank PHP, Laravel, WordPress, Symfony, static. A preset can create a database and user, pick the PHP version, and enable tools (Mailpit, Redis). Needs Composer (downloaded `composer.phar`) and WP-CLI or the WordPress zip.
- **Per-project PHP version** (decided: yes): PHP runs as FastCGI per installed version (`php-cgi` pool on Windows, `php-fpm` elsewhere) instead of a single `mod_php`; each site picks its version, default is the global active one. Shared with Nginx support in 6.2, so 6.2 lands first.

## 6.4 Tools and tunnels ✅
- **Node.js via NVM** (decided): not installed by default. When the user asks, LampRabit installs NVM and from then on manages Node versions through it (list, install, use, `.nvmrc` per project).
  - Windows: nvm-windows. Its `nvm use` creates a symlink, which needs admin rights or Developer Mode; the UI says so before installing.
  - macOS/Linux: nvm's official install script into `~/.nvm`; commands run through a login shell that sources `nvm.sh`.
  - An existing NVM install is detected and used instead of installing a second copy.
- **Tunnels: Ngrok + Cloudflare** (decided): app-level (expose `localhost`) and per-project (expose `myapp.test` with a host-header rewrite, or a manual port).
  - Ngrok (default): binary downloaded on demand, auth token stored in the OS keychain, free-plan limits explained in the UI.
  - Cloudflare quick tunnels (`cloudflared`): no account needed, random `trycloudflare.com` URL.
- **Redis / Memcached**: no official Windows builds for either. Windows: Redis-compatible alternative (Garnet) or Memurai, or "not available" with explanation; macOS/Linux: manage a package-manager install (brew/apt) or a downloaded build. PHP extensions (`redis`, `memcached`) offered only where builds exist for the active PHP. (Superseded in 10.1: real Redis and downloadable Memcached on every OS.)

## 6.5 Updates ✅
- **App updates**: Tauri updater with signed releases on GitHub; check on start (opt-out) and from Settings. Depends on the release pipeline in phase 8.
- **Runtime updates**: badge when a newer patch of an installed Apache/PHP/MariaDB is available.
