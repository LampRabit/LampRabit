# Pitfalls already hit

Add one line here whenever something costs time and could bite again.

- `sysinfo` lists every thread as a process on Linux; filter with `thread_kind().is_none()` or memory is counted many times over.
- `httpd` and `nginx` daemonize by default outside Windows; they're started with `-D FOREGROUND` / `daemon off;` so the supervisor tracks the real process.
- The AppImage can't bundle GL libraries; `main.rs` sets `WEBKIT_DISABLE_DMABUF_RENDERER=1` on Linux, or WebKitGTK aborts without `libGLESv2` (and draws blank windows on some GPUs).
- GNOME's terminals don't inherit the spawning process's environment; the terminal launcher passes `PATH` through `env` in the command.
- A full-width fixed wrapper (like the bottom nav's) swallows clicks; give it `pointer-events-none` and the visible part `pointer-events-auto`.
- The redis-windows build is MSYS2-based: pass paths with forward slashes (`--dir C:/…`); it accepts them, spaces included.
- Adminer 6 rejects a login post without its CSRF `token` field; scripted tests must send it (browsers do).
- Settings inputs: `inputClass` contains `w-full`; strip it for narrow fields (`numberInput`), or the width class loses.
- Pass callbacks that change every render (like `onClose`) into effects through a ref, or the effect re-runs, and the Overview re-renders every second.
- Vitest started from Git Bash collects nothing ("no tests", "failed to find the current suite"); run it from PowerShell or through `npm test`.
- Many source files have CRLF line endings: a Python edit script must normalise them (and write them back), or its patterns never match.
- Prettier isn't how this code is formatted: `.prettierrc` (width 200) only keeps format-on-save close. Never run it over whole files or the repository.
- `Site.framework` names are capitalised as shown ("Laravel", "Statamic"); compare them case-insensitively (the doctor's Laravel checks never ran because they compared with "laravel").
- Component tests that use `serviceLabels` must fill it (the app loads it from the backend before the first render), or every service counts as unknown.
- A `const` read inside a callback that runs during render (`.find(e => e === runtime)`) before the line that declares it throws at runtime, and `tsc` doesn't flag it. The Services page broke this way (2026-11-21); `npm run screenshots` renders the real pages and catches it.
- MySQL backups made by `mysqldump` carry `SET @@GLOBAL.GTID_PURGED`, which MySQL refuses when the dump is loaded back into a server (ERROR 3546); dump with `--set-gtid-purged=OFF` (MariaDB has no such option). Found by the smoke test's backup round trip (2026-11-22); restores had been failing before.
- On Windows the system PATH comes before the user PATH: LampRabit's shims on the user PATH lose to XAMPP's `php` in the system PATH (`overrides::shadows` finds it).
- A newly installed web tool (phpMyAdmin) answered "unavailable" until a project was created: installs didn't update the hosts file without a running web server, nor refresh the frontend's `sites` query. `install_runtime` now runs `sync_sites` and emits `routes-changed` (2026-11-23).
- An upstream release can be published broken: pgAdmin 9.19's PyPI wheel lacks its JavaScript bundles (the page loads, every `/static/js/generated/*` is a 404). When a web tool hangs on its loading screen, check the installed package before the proxy; `skip:` in the source leaves such a version out.
- LampRabit's own PATH is the one it started with: a tool installed afterwards (NVM, Node, Git) is missing from it while every new terminal has it ("Node.js isn't installed" inside the app). Commands LampRabit runs take `overrides::fresh_path()` (registry PATH on Windows, the login shell's elsewhere), never `std::env::var_os("PATH")`.
- In this Windows setup the Bash tool's heredocs (even `<<'EOF'`) can turn `\\` into `\` and `\n` into a newline: write edit scripts to a file and run them, don't inline them in a heredoc.
- A YAML flow map (`{ value: x, label: … }`) breaks on a comma inside the label: quote it.
- Python on Windows opens files as cp1252 by default: an edit script must pass `encoding="utf-8"`, or "…" and "→" are written as invalid UTF-8.
- A dropdown inside a card with `opacity-*` (Home dims services outside the active profile) inherits the fade, and nothing inside can undo it: `Menu` draws its list in a portal on `<body>` (2026-11-26).
- A manifest's `origin` limited to one platform (`on: [windows]`) is empty elsewhere, and the catalog test fails only on Linux and macOS CI: give the last case no `on` (now checked by `built_in_manifests_are_valid` on every platform, 2026-11-27).
- `live_manifest_catalogs` makes ~60 GitHub API calls; without a token it hits the 60-an-hour limit. Run it with `GITHUB_TOKEN=$(gh auth token)`.
