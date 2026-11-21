# Phase 10: macOS parity, stability on every OS, safe uninstall

Status: **done**. Listed in the [roadmap](../roadmap.md).

macOS is supported from 0.6.0 (checked by hand on 2026-11-14). LampRabit builds Apache, Nginx, PHP, Redis, Memcached and MariaDB for macOS; MySQL uses Oracle CDN archives. Some runtimes remain unavailable on specific architectures or operating systems, as reported by the catalog.

## 10.1 macOS runtimes, Redis and Memcached

- Compared with FlyEnv and EnvKit first. Decision: keep building our own portable runtimes rather than relying on Homebrew/system packages.
- [x] `.github/workflows/binaries.yml` builds Apache, Nginx, PHP, Redis and Memcached for Linux and macOS (x86_64 and aarch64); macOS Apache uses `@rpath` for APR, ad hoc signing and no file AIO. LampRabit also builds MariaDB LTS releases for both macOS architectures. MySQL downloads official macOS archives from Oracle CDN and tries compatible macOS archive variants.
- [x] The runtime manager reads the manifest for macOS builds and removes `com.apple.quarantine` after checksum verification so downloaded executables can run.
- [x] Real Redis on Windows replaced Garnet; Memcached is a downloadable runtime on each supported platform; PHP `memcached` and `igbinary` are included in the Linux/macOS PHP builds.
- [x] Latest CI on 2026-11-13 passed the smoke workflow on Apple Silicon (`macos-latest`) and Intel (`macos-15-intel`), and built and opened the app bundle on both runners. See [workflow run](https://github.com/LampRabit/LampRabit/actions/runs/43377759321). Pull-request smoke runs cover the standard runtime list; heavy services run on the scheduled/full workflow.
- [x] Hands-on Mac validation (2026-11-14, Apple silicon): Apache and Nginx (switching between them), PHP-FPM, MariaDB, Mailpit; a Laravel project at https://<name>.test with its migrations, phpMyAdmin, mail caught by Mailpit.
- [ ] Still to try on a Mac: Redis, Memcached and MySQL; a project folder with spaces and quotes in its name.

## 10.2 macOS platform pieces

- [x] Platform code is present for managed hosts entries, CA trust/removal via `security`, port-conflict discovery via `lsof`, Terminal.app launch, shell PATH shims, process groups and orphan cleanup.
- [x] Downloaded runtime quarantine is removed only after the archive checksum has been verified.
- [x] CI builds and opens the macOS app bundle on Apple Silicon and Intel, and runs the service smoke workflow on both.
- [x] Hands-on Mac validation (2026-11-14): hosts-file updates, CA trust (the padlock in Safari), ports 80/443, LampRabit's Terminal and PATH, runtimes launching without Gatekeeper prompts, closing the window with services kept running. Since then: the traffic lights are centred in the 48 pt title bar (`trafficLightPosition` y 26), and Add to PATH is on by default for new installs.
- [ ] Still to try on a Mac: CA removal, and that quitting leaves no service processes behind.
- [x] The macOS Terminal launcher now passes paths as `osascript` arguments and shell-quotes them in AppleScript instead of interpolating them into source. `cargo check` passed on Windows; verify opening a terminal from a path containing spaces and quotes on macOS.

## 10.3 Stability on all OSes
- ✅ A cross-platform smoke test (2026-11-04): `lamprabit --smoke-test <folder> <runtime>…` (smoke.rs) installs each runtime with the versions its needs call for into a throwaway folder (own settings, every port +20000), then start → healthy → second start ignored → stop → port free for every service, and one killed service has to come back through "Restart after a crash" and stay stopped after Stop. `.github/workflows/smoke.yml` runs it on Windows, Linux (Xvfb) and macOS for pull requests that touch the backend, weekly with the heavy services, and by hand; platform gaps are reported as skipped with their reason.
- [x] Runtime compatibility matrix (runtime × OS × processor, with the reasons) published in docs/website/installation.md (2026-11-14), from catalog.rs and the download catalogs.

## 10.4 Uninstall ✅ (2026-10-30)
- `uninstall.rs` removes the folders LampRabit creates under the root (`bin`, `etc`, `logs`, `tmp`, `ssl`, `shims`, nothing else), the root itself only if empty, the settings and (Windows uninstaller) WebView data, the hosts block, the trusted CA (system store and, on Linux, the browsers' NSS databases), the PATH entry, the ngrok token and the Linux sysctl file. Keeps `www/` and `data/`.
- Windows: NSIS `NSIS_HOOK_PREUNINSTALL` (src-tauri/windows/hooks.nsh) asks to close the app and runs `LampRabit.exe --uninstall-cleanup`; skipped for updates (`/UPDATE`). The uninstaller itself removes the autostart entry.
- Every OS: Settings → Troubleshooting → "Remove LampRabit from this computer" stops services cleanly, shows what is deleted and kept, reports what couldn't be undone, then quits. Linux does the system files in one pkexec prompt.
- ✅ MSI (2026-10-31): `windows/cleanup.wxs` runs `--uninstall-cleanup` as a deferred, impersonated custom action before RemoveFiles, on a real uninstall only (not for upgrades); linked through `bundle.windows.wix.componentRefs`.
- The .deb's removal can't reach per-user files, so Linux users use the Settings action.
- [ ] Verify the Settings uninstall flow on macOS removes hosts and CA trust while preserving `www/` and `data/`.
