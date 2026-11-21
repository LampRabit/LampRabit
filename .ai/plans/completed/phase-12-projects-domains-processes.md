# Phase 12: projects (several domains, processes, Node.js)

Status: **done**. Listed in the [roadmap](../roadmap.md).

**Done (2026-10-31, branch node-projects):**
- Node.js projects run with NVM's versions (per project, `.nvmrc`), any package manager (npm, pnpm, Yarn, Bun), from the template's `serve:` command or package.json. Projects start only from Sites, never with services.
- Port discovery (`ports.rs`): every port a project's processes listen on, whoever started them (LampRabit or your terminal), routes its domain; the lowest wins, or the one you pick; back to its files when nothing listens. PHP projects route only to a picked port. Web servers reload gracefully on a change (Nginx everywhere, Apache on Linux/macOS).
- Commands per project, from YAML (`commands:`) and discovery (package.json, composer.json, artisan, bin/console, bin/magento, please, drush, cake, spark), with output in the project and in Logs.
- Sites redesign: project list with live status, Overview (`project_facts`: versions, requirements, dependencies, .env, git), Commands, Settings.

**Done (2026-11-04):** extra domains per project, each pointing like the main one can: at the project's files or a folder in it, a port, one of its processes, or wherever the main domain goes (`SiteSettings.domains`; `sites::served` turns them into sites of their own, before the projects so Apache doesn't send them to `*.<project>`). Several processes per project (`SiteSettings.processes`: command, optional port as `{port}`/$PORT, environment, restart after a crash / always / never with at most 3 restarts in 5 minutes, start with the app), run by apps.rs as `<project>/<name>` with their own output. Their ports don't take over the main domain. Sites → Domains and Processes tabs, with Laravel/Symfony/Vite presets. The smoke test checks a project with both through every web server.

The groundwork for most later phases. A project stays a folder in `www/`; `SiteSettings` gains optional fields, so existing settings need no migration:

```toml
[sites.shop]
php = "8.3"
ssl = true

[[sites.shop.domains]]       # extra domains: to a folder, or to one of the processes
host = "admin.shop.test"
root = "admin/public"

[[sites.shop.domains]]
host = "api.shop.test"
process = "api"

[[sites.shop.processes]]     # long-running commands
name = "api"
command = "npm run dev"
port = 3000                  # or "auto": a free port, passed as $PORT
env = { NODE_ENV = "development" }
restart = "on-failure"
```

- Domains render through the existing vhost code (Apache, Nginx, Caddy), which already proxies for tools (`Site.proxy`). Before writing configs, check for duplicate hosts, ports in use and missing roots.
- Processes run under the supervisor with `procgroup`, with PATH set like the shims so `php`, `composer` and `node` are LampRabit's. Start/stop/restart one or all, restart policy, health (does the port answer), crash causes via `diagnose::explain_exit`.
- Node.js projects: detect `package.json` and the package manager from the lockfile (npm, pnpm, yarn, bun), suggest the `dev` script as the web process, use the `.nvmrc` version. A Node site is a process plus a proxied domain; the router knows nothing about Node.
- Stack profiles can include project processes.
- Project detail view (open from the card): domains, processes with their logs, quick actions.
- `lamprabit.toml` in the project folder (same shape, committable) only if sharing setups through the repo turns out to be needed; settings.toml would override it.
