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
