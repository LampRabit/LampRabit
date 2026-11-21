# Phase 11: service foundation and web servers

Status: **done**. Listed in the [roadmap](../roadmap.md).

**Foundation first.** Each service used to be wired by hand in about 20 `match` arms (backend and frontend), which doesn't scale to the services planned here:
- `Versions` keeps its fields and gains a flattened map for new runtimes, so `settings.toml` loads unchanged. `Settings::version(id)`/`set_version` replace the per-field matches.
- `ServiceKind::def()` is one table per service (id, label, runtime, ports, log, data folder), so the getters stop being matches.
- `web_server` (Apache | Nginx | Both) becomes a set of enabled servers (`web_servers`), read from the old field when missing. The first enabled server keeps the main ports; every other server has its own pair in `Ports`.
- Runtimes declare what they need (Tomcat, Neo4j, Maven → Java; n8n → Node; RabbitMQ → Erlang), and the UI offers to install what's missing.
- A platform gap is a reason string per runtime, shown in the UI instead of hiding the runtime.

**Done (2026-11-01):** the `Versions` map, `ServiceKind::def()`, `web_servers` (with migration and a Settings UI: a switch per server and a primary picker), and Caddy end to end (catalog, Caddyfile, validate, reload, stop, health, ports, Sites/New project pickers). Then Java (Temurin, shims with JAVA_HOME), FrankenPHP (the Caddy renderer with `php_server`, conf.d ini through `PHP_INI_SCAN_DIR`, no LampRabit PHP pools) and Tomcat 9/10.1/11 (Java checked per line, `CATALINA_BASE` in etc/tomcat, one `<Host>` per Java project found by `sites::java_webapp`, proxied from the site's web server, restarted when the apps change). FrankenPHP worker mode per site done (2026-11-04: a switch in the project's settings, Octane's worker or index.php, "Restart workers" through the admin API). Foundation done too (checked 2026-11-12): `catalog.rs` declares each runtime's needs (versions per release) and why it can't run on a platform, the Services page reads groups, labels and needs from it, and installs ask through `RequirementsDialog`. Phase 11 is complete.

**Web servers.** Caddy, FrankenPHP and Tomcat run next to Apache and Nginx: each site picks its server when more than one is enabled, and each extra server gets its own ports.
- **Caddy**: single binary from GitHub releases (all three OSes). LampRabit writes a Caddyfile: one site block per site, `php_fastcgi` to the existing PHP pools (so per-site PHP and the extension manager keep working), `reverse_proxy` for proxied sites/tools, `tls <cert> <key>` with LampRabit's CA and `auto_https off` so Caddy doesn't create a second CA. `caddy validate` before start (like `httpd -t`), `caddy reload` when sites change.
- **FrankenPHP**: the assessment from 7.3 still applies. It embeds its own PHP, so its version is shown read-only and its built-in extensions are listed; php.ini through `PHP_INI_SCAN_DIR` (Mailpit settings). Worker mode is an opt-in per-site toggle with a "restart workers" action. Intel Macs: no build, so the UI says so.
- Spike first on Windows (newest FrankenPHP platform): start, our certificates, extensions, `.user.ini`.
- **Java** (Temurin LTS lines 8, 11, 17, 21, 25) is a runtime first, because Tomcat needs it; phase 24 adds per-project versions and serving.
- **Tomcat 9/10/11**: LampRabit's own `CATALINA_BASE` in `etc/tomcat` (conf copied from the install, `server.xml` generated with one `<Host>` per Java site, webapps from the project folder) and `JAVA_HOME` from the active Java. HTTP only on its own port, with Apache/Nginx/Caddy proxying the `.test` domain (TLS stays in one place).
- Health probe, port conflicts, logs and the diagnostics report cover all three.
