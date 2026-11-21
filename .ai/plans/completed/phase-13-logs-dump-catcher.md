# Phase 13: logs redesign and dump catcher

Status: **done**. Listed in the [roadmap](../roadmap.md).

**Done (2026-11-05, finished 2026-11-12):** sources sidebar, virtualised list, line details, per-source clear, regex/case/exclude search, severity, stream and time-range filters, pause, copy and export, rotation (Settings → Logs) and the dump catcher. Then, from user reports: every service's timestamp format is parsed (MySQL 8, PostgreSQL, MongoDB, Redis, Caddy, Elasticsearch, Tomcat, RabbitMQ, Go and Rust loggers; before, their lines showed the load time), lines without their own time borrow the nearest one and are shown in italics, repeats fold into one line with a count, a Problems view and error/warning counts, and error rows stand out.

- **Sources**: every service, every project process (phase 12; `LogLine.service` becomes an owned name like `shop/queue`), PHP errors, web-server access/error logs, and dumps.
- **Dump catcher**: PHP `dump()` and `dd()` appear in Logs as their own source, with the project, file:line, time and a collapsible tree of the value. Route: an `auto_prepend_file` in the generated php.ini sets Symfony VarDumper's server mode (`VAR_DUMPER_FORMAT=server`, `VAR_DUMPER_SERVER=127.0.0.1:<port>`), and LampRabit listens on that port. The dump-server payload is PHP-serialized, so spike the decoding: a small PHP receiver that re-emits JSON, or a LampRabit dumper class set as the handler. A setting decides whether dumps still show in the page too. Plain `var_dump` can't be intercepted; the UI says that.
- **Redesign**: source list (services, projects → processes, dumps) next to the stream; filters for level, source, time range and text (regex optional); highlighted matches; pause/follow; virtualised list so large logs don't block the UI; click a line for context (surrounding lines, full text, copy); export the filtered view; per-source clear.
- **Retention**: size cap per log file and total, configurable in Settings; old files rotated.
- Errors that need action (fatal, failed start) are highlighted; warnings aren't treated as failures.
