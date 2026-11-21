# Phase 7: comfort and PHP tooling

Status: **done**. Listed in the [roadmap](../roadmap.md).

## 7.1 Comfort ✅
- Tray menu (start/stop all, open sites, quit), start with the OS, optionally start services on launch.
- "Open terminal" with PHP, Composer, MariaDB and Node on PATH; "Open in editor" for projects.
- Microsoft Visual C++ runtime check on Windows, with a link to (or download of) the redistributable.
- MariaDB data directory per major version, or an automatic `mariadb-upgrade` when moving up.

## 7.2 PHP extension manager ✅
Browse, enable, disable and add PHP extensions per installed PHP version, with compatibility shown for each.

- **Bundled extensions**: list what each PHP build ships (Windows: `ext/php_*.dll`; elsewhere: `php -m` plus shared objects in the extension dir). Toggle them on or off per version; the choice is stored in settings and written into that version's generated php.ini, then the PHP pool restarts.
- **Extra extensions** (PECL and vendor builds): Xdebug, Redis, APCu, Imagick, MongoDB, PCOV, Memcached and similar.
  - Windows: download prebuilt DLLs matching the PHP minor version, thread safety (LampRabit's PHP builds are thread-safe) and compiler (vs16 / vs17) from downloads.php.net/~windows/pecl and xdebug.org. Show "available for PHP 8.3, not yet for 8.5" style compatibility from what actually exists for download.
  - macOS / Linux: depends on how PHP itself is provided there (static builds can't load shared extensions); show "built in" / "not available on this platform" accordingly.
- Extensions with extra requirements are flagged (Imagick needs ImageMagick DLLs; Xdebug gets a ready-made `xdebug.mode=debug` and IDE port setting).
- Status per extension: built in, enabled, disabled, downloadable, unavailable (with the reason).

## 7.3 FrankenPHP: brought back in phase 11

Decision (2026-10-24): not planned. Reversed on 2026-10-29: FrankenPHP and Caddy are phase 11. The assessment below still applies.

### Assessment
FrankenPHP is a PHP application server built on Caddy: the web server and PHP run in one process, with automatic HTTPS, HTTP/2 and HTTP/3, and a "worker mode" that keeps the application booted between requests (Laravel Octane, Symfony Runtime).

**Availability (checked against v1.13.3 releases):** Linux x86_64 and aarch64, macOS Apple Silicon (no Intel Mac build), and a Windows x86_64 zip. All three platforms can be offered; Intel Macs get "not available" with the reason.

**How it fits LampRabit**
- It is a third web server, not a PHP version: each FrankenPHP release embeds its own PHP, so it doesn't use the PHP pools, php.ini per version, or the extension manager. Its PHP version is shown read-only on the site card, and its bundled extensions are listed (static builds include the common ones).
- Per-site choice: Settings → Web server gains "FrankenPHP" as an option next to Apache and Nginx; sites can pick it like they pick Nginx today. It gets its own ports when another server holds 80/443.
- Config: LampRabit generates a Caddyfile with one `php_server` block per site, using LampRabit's own certificates (`tls <cert> <key>`) so the existing trusted CA keeps working, and Caddy's automatic HTTPS turned off to avoid a second CA.
- Worker mode as a per-site toggle (off by default) for projects that support it; a restart button on the site card reloads workers after code changes.
- Mail: `sendmail_path`/SMTP settings through FrankenPHP's php.ini (`PHP_INI_SCAN_DIR`) so Mailpit keeps catching mail.
- Logs, health checks (HTTP probe), port-conflict detection and the diagnostics report extend to it like the other servers.

**Risks / open questions**
- Windows build is the newest platform; needs a spike (start, PHP extensions loaded, our certificates, `.user.ini`) before committing.
- Extensions are fixed at build time; projects needing an extension that isn't in the build must use Apache/Nginx.
- Worker mode changes app behaviour (state persists between requests); keep it opt-in with a clear explanation.

**Recommendation at the time:** an optional third server after the extension manager, starting with a spike on Windows and Linux.
