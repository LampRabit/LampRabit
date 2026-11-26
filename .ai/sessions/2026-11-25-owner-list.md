# Session 2026-11-25: the owner's twelve-item list

## What the user asked

Twelve items, saved verbatim in [../plans/source-specs/2026-11-25-owner-list.md](../plans/source-specs/2026-11-25-owner-list.md): a crowded title bar, Laravel's serve commands, Install MySQL landing on the wrong page, links that land on "service unavailable" (with a list of every such link), Git pull/push to any branch, Python package manager missing in New project and fuller templates, templates from the CLI/MCP, a file of everything since 0.6, Git tag management, Node not found inside LampRabit, the profile button on Home, and three layouts for Home's services.

## Done (uncommitted on `dev-1.0`)

1. **Title bar**: only what's the same on every page (logo, Search, theme, Open www, terminal, Start/Stop all, window buttons). Each page's title, subtitle and actions moved into `PageHeader` (`components/PageToolbar.tsx`) at the top of the page; `usePageToolbar` is unchanged, so pages register as before. The keyboard-shortcuts button left the title bar (still `?` and the palette). Logs' main is a flex column so it fits under its header.
2. **Laravel's dev server**: `apps::suggest` gives Laravel `composer run dev` when composer.json has a `dev` script, else `php artisan serve`; both listen on LampRabit's port through `SERVER_PORT`/`SERVER_HOST` (set on every app start). `apps::serve_commands` (in `AppInfo.serveCommands`) lists the ready-made commands plus package.json's dev scripts. Start dev server is a split button: the arrow lists them (picking one saves it and starts) and "Configure in project settings". Laravel projects without a package.json get the button and the Dev server settings too (`isLaravel`). Saved configs aren't changed.
3. **Install buttons** go to that service's page (`openService`): Install MySQL/MariaDB in the project's database row, KarotX (Home, project toolbar), Git, Cloudflare/ngrok, a Home row's Install, Quick shortcuts, Settings' version rows and Mailpit.
4. **Links that need a service** (audit below): `prepare_url` now starts what a tool's domain needs before the browser opens: the tool's own service and its manifest `needs:` (pgAdmin → PostgreSQL), or a database for the PHP clients (phpMyAdmin/Adminer: the default database, phpRedisAdmin: Redis, phpPgAdmin: PostgreSQL); then waits (up to 45 s, outside the ops lock) for the tool's port. localhost fallbacks (`localhost:<port>/`, `localhost/<tool>/`) are matched too. `lib/open.ts` shows "Getting X ready…" after 0.7 s and "Started X to open Y". Quick shortcuts on Home are no longer disabled until ready ("Starts Mailpit, then opens").
5. **Git pull from / push to any branch**: ⋯ next to Pull/Push → dialog (remote, branch with the remote's known branches). `git_sync_branch`: `git pull <remote> <branch>`, `git push <remote> HEAD:refs/heads/<branch>`, streamed with progress and Cancel.
6. **Python package manager**: the "Stack and runtime" section only rendered for PHP/Node templates, so the pip/uv/conda choice never showed; it now renders for `template.python` and `nodeSteps` too. **Templates expanded**: Laravel (starter kit: none/React/Vue/Livewire; PHPUnit or Pest; route:list and storage:link commands), Symfony (web app via `composer require webapp`, or skeleton; debug:router, migrations), Django (Django REST framework), Flask (Flask-SQLAlchemy with `db.py`; requirements.txt from `pip freeze`), Spring Boot (Initializr dependencies, `multi`).
7. **Templates from the CLI and MCP**: control requests `template.show|example|from_project|save`; CLI `lamprabit template show <id> | example | from <project> | save <file|-> [--replace]`; MCP tools `get_template`, `template_format`, `draft_template_from_project` (read-only) and `save_template`. `templates::source` returns a template's YAML (yours first). Docs: docs/cli.md, docs/templates.md.
8. **[docs/since-0.6.md](../../docs/since-0.6.md)**: every feature and fix since 0.6.0, for the website and release notes; keep adding to it (working-with-the-user.md).
9. **Git Tags view** (its own tab next to Branches): tag HEAD or any commit/branch, filter, "only here"/"on origin" from one `ls-remote` (`git_remote_tags`), push one or all, delete here or on the remote, fetch tags, branch from a tag, copy the name.
10. **Node not found inside LampRabit**: project commands used the app's PATH from launch; NVM (or Node) installed afterwards was missing, while new terminals had it. `overrides::fresh_path()` builds PATH as a new terminal would (Windows: system + user PATH from the registry; elsewhere the login shell's PATH, read once with a 4 s limit, plus Volta/Homebrew/~/.local/bin) and `node_path`/`command_path` use it. `node::for_project` falls back to `node::system_node()` (the `node` on that PATH, with its version) when NVM's folder has no match, and the doctor counts it.
11. **Profile button**: hidden until something is ticked; "Profile" (save icon) saves a new one when no profile of your own is active; "Update profile" saves the ticks into the active profile when they differ from it.
12. **Service layouts**: a button next to it cycles rows → tiles (3 a row) → compact tiles (4 a row); kept per device (`home-services-layout`).

Checks: `cargo test --lib` (165), `cargo clippy --lib` (4 warnings, all in files not touched here), `npx tsc`, `npm test` (102, from PowerShell), `npm run build`, `npm run screenshots` (new scenes `home-tiles`, `home-compact`, `git-tags`).

## Item 4: every link that could land on a stopped service

| Where | Link | Before | Now |
|---|---|---|---|
| Project → Database & Environment | Open in manager (default client: KarotX…) | "service unavailable" when the client was stopped | starts the client (and its needs) first |
| Project → toolbar | database button | same; Install KarotX went to the top of Services | starts it; Install opens KarotX's page |
| Home → service row | Open (database tool, console) | tool stopped → unavailable | starts the tool |
| Home → row ⋯ | Open in phpMyAdmin/Adminer/KarotX… | same | starts it and a database |
| Home → Quick shortcuts | Mailpit, phpMyAdmin, Adminer, phpRedisAdmin | disabled until ready | open, starting what they need |
| Command palette | Open <tool>, Open <project> | same as the buttons | same fix (all go through `openUrl`) |
| Services → a tool's page | Open | same | starts it |
| Project → Open site, Domains tab | `<project>.test`, subdomains | web server started already (2026-11-24) | unchanged |
| No hosts file | `localhost:<port>/`, `localhost/<tool>/` | nothing started | matched to the tool and started |
| Install buttons (MySQL, KarotX, Git, tunnels, Mailpit, a Home row, Settings versions) | Services | top of Services | that service's page |

Not changed: a Node project's domain while its dev server is stopped serves the files (that's the Follows-what's-running rule); the tunnel's public URL only shows while it runs.

## Follow-up (same day)

- Stack profiles: the tall cards became one row of pills (Profile: All services, yours with a count, a running dot and ⋮ edit/delete, New profile, Import/Export). The cards' own Start buttons went: the header's Start already starts the selected profile. The fold (open/closed pref) is gone.
- Tile layouts: Start/Stop borderless (ghost), actions left-aligned, ⋮ (Menu `plain`) without a border.
- Harness: `profiles=1` adds two sample profiles (used by `home-tiles`).

## Second follow-up (same day)

- Profiles row: "Services" is its heading (the separate Services heading is gone); Import/Export moved into a ⋮ (`ProfilesTransfer menu`) next to the layout switch.
- Service cards are built from pieces (`startStop`, `openButton`, `copyButton`, `menu`, `chevron`, `selectName`, `compactName`) that each layout places. Running → icon and name in accent colour everywhere. Tiles: no chevron (details only on issues); open at the top right; start/restart left, copy/⋮ right. Compact: icon-only start/stop, no port, status only when unusual, running name is an underlined link that opens the service.
- Harness scenes: `home-list`, `home-tiles`, `home-compact` (all with `profiles=1`).

## Next step

The user tests on Windows: a Laravel project's Start dev server menu (composer run dev on Laravel 11+, artisan serve otherwise), karotx.test with KarotX stopped, Install MySQL, pull/push to another branch, the Tags view, a Django/Flask/Laravel project with the new choices, `lamprabit template from <project>`, and Node from a fresh NVM without restarting LampRabit. Then commit everything on `dev-1.0`.
