<div align="center">

<img src="public/logo.svg" alt="LampRabit logo" width="112" />

# LampRabit

**A local development environment for PHP, JavaScript, Python, Go, Rust and Java.** Available on Windows and Linux, with macOS in preview.

Apache, Nginx, Caddy, FrankenPHP and Tomcat; databases from MariaDB and MySQL to PostgreSQL, MongoDB and ClickHouse; plus the tools around them. Install runtimes on demand. Every folder in `www` can become `https://<folder>.test`.

[![Latest release](https://img.shields.io/github/v/release/Shivam-SV/LampRabit?label=download&color=59d1a2)](https://github.com/Shivam-SV/LampRabit/releases/latest)
[![CI](https://github.com/Shivam-SV/LampRabit/actions/workflows/ci.yml/badge.svg)](https://github.com/Shivam-SV/LampRabit/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-1b2028)](LICENSE)
![Platforms](https://img.shields.io/badge/platforms-Windows%20%7C%20Linux%20%7C%20macOS%20(preview)-1b2028)

[**Website**](https://lamprabit.shivam-vastav33.workers.dev/) · [**Download**](https://github.com/Shivam-SV/LampRabit/releases/latest) · [Features](docs/website/features.md) · [Installation](docs/website/installation.md) · [FAQ](docs/website/faq.md) · [Documentation](#documentation)

<img src="docs/screenshots/overview.png" alt="LampRabit overview dashboard with local projects and services" width="820" />

</div>

## Why LampRabit

If you've used XAMPP or Laragon, you know the idea: a local development stack you don't have to assemble by hand. LampRabit extends it to multiple web and app servers, databases, languages and project processes. Sites get local HTTPS and domains; project Health and Git panels help you find and fix issues; the CLI and MCP server let scripts and supported AI clients work with the same backend as the desktop app.

## Features

**Your stack**
- **A broad local stack.** Apache, Nginx, Caddy, FrankenPHP, Tomcat; PHP, Node.js, Java, Python, Go and Rust; MariaDB, MySQL, PostgreSQL, MongoDB, SQLite, ClickHouse and Neo4j; plus search, storage, queues, AI and mail services. Availability depends on platform; Services explains gaps.
- **Several versions side by side.** Install runtimes on demand, switch versions, and select project-specific language versions where supported.
- **Stack profiles.** Save the services a kind of project needs, start that stack, and export or import profiles as JSON.
- **Tools included.** phpMyAdmin, Adminer, phpRedisAdmin and a Mailpit inbox that catches every email your apps send.
- **PHP extensions.** Switch bundled extensions on or off and install extras like Xdebug and Redis per PHP version.
- **Project doctor.** See runtime, dependency, database, HTTPS, hosts and configuration issues, with a preview before applying supported fixes. See [Project doctor](docs/project-doctor.md).
- **Project Git.** Review and stage changes, commit, manage branches and remotes, sync, stash and resolve conflicts in a project. See [Git workflow](docs/git.md).

**Your projects**
- **Sites without configuration.** `www/my-app` is served at `my-app.test`, with `public/` detection, sub-domains and the hosts file kept in sync.
- **HTTPS by default.** A local certificate authority issues trusted certificates for every site; turn it off per site if you like.
- **Node.js projects.** Vite, Next.js, Nuxt, Astro, Express or NestJS on the Node version you choose (from NVM, `.nvmrc` respected) with npm, pnpm, Yarn or Bun. Start the dev server from LampRabit, or run it yourself in a terminal: the domain follows whatever the project listens on, and falls back to its files when it stops.
- **New projects in seconds.** Templates for Laravel, Symfony, WordPress, Drupal, CakePHP, CodeIgniter, Statamic, Magento and the Node frameworks above, asking the same questions their own installers do (TypeScript, linter, router, modules…), optionally with a fresh database. Write your own in YAML: see [Project templates](docs/templates.md).
- **More languages and frameworks.** Python, Go, Rust and Java projects, with templates including Django, Flask, FastAPI, Gin, Axum and Spring Boot.
- **Project commands.** Run artisan, Composer and package.json scripts (and Symfony, Drupal, Magento, Statamic, CakePHP and CodeIgniter commands) from the project, with the output right there and in Logs.
- **Everything about a project at a glance.** Framework and version, PHP or Node version and what the project requires, whether dependencies are installed, `.env` settings, database and git state.
- **Per-site choices.** PHP version, web server and HTTPS for each site. In the terminal, `php` and `composer` follow the site's PHP version too.
- **Share a site.** Public URLs through Cloudflare Tunnel (no account needed) or ngrok.
- **Several domains and processes.** Route extra domains to project folders or processes; run workers and development servers with restart policies.

**Day to day**
- **Command line ready.** Put `php`, `composer`, `mysql`, `redis-cli` and friends on your PATH with one switch.
- **Know what's going on.** Live logs from services and projects with search (regex too), severity and source filters, and export; health checks; port-conflict detection that names the program in the way; and a one-click diagnostics report.
- **Lives in your tray,** starts with your computer if you want, follows your light or dark theme, has keyboard shortcuts for everything (`?`), and updates itself.
- **CLI and AI access.** Use `lamprabit` from a terminal or connect supported MCP clients, including Claude, Cursor, VS Code, Windsurf and Codex. See [CLI and MCP](docs/cli.md).
- **Move your environment.** Choose its folder on first run or move it later from Settings.

<table>
  <tr>
    <td><img src="docs/screenshots/sites.png" alt="Sites page with project health, domains and Git workflow" /></td>
    <td><img src="docs/screenshots/services.png" alt="Services page showing runtime groups and installed versions" /></td>
  </tr>
  <tr>
    <td align="center">Sites: project status and controls</td>
    <td align="center">Services: runtimes and versions</td>
  </tr>
</table>

<table>
  <tr>
    <td><img src="docs/screenshots/git-workflow.png" alt="Project Git panel with file changes and diff" /></td>
    <td><img src="docs/screenshots/ai-assistants.png" alt="AI assistant settings with the LampRabit MCP server enabled" /></td>
  </tr>
  <tr>
    <td align="center">Project Git workflow</td>
    <td align="center">MCP connections for AI assistants</td>
  </tr>
</table>

## Download

Get the latest version from the [**Releases page**](https://github.com/Shivam-SV/LampRabit/releases/latest):

| Platform | File | Status |
|---|---|---|
| Windows 10/11 (64-bit) | `…_x64-setup.exe` (recommended) or `…_x64_en-US.msi` | Ready |
| macOS, Apple Silicon | `…_aarch64.dmg` | Preview |
| macOS, Intel | `…_x64.dmg` | Preview |
| Linux, Debian/Ubuntu | `…_amd64.deb` | Ready |
| Linux, other distributions | `…_amd64.AppImage` | Ready |

### Platform support

The application and its core workflows run on Windows, macOS and Linux. Runtime downloads depend on operating system and architecture, and some services need other runtimes (for example Tomcat needs Java). Check **Services** for current availability, requirements and platform notes; the catalog changes as new builds land.

| Platform | App package | Runtime coverage |
|---|---|---|
| Windows 10/11, x64 | `.exe` / `.msi` | Broad runtime catalog; some community builds are used where upstream has no Windows release. |
| macOS, Apple Silicon and Intel | `.dmg` | Preview. Runtime availability varies by architecture and service; MariaDB and MySQL builds are included in the current work. |
| Linux, Debian/Ubuntu and other distributions | `.deb` / `.AppImage` | Broad runtime catalog, including LampRabit-built Apache, Nginx and PHP. |

For precise OS and architecture coverage, refer to the in-app Services catalog and release notes. The same workflow is available from the [CLI](docs/cli.md).
> [!NOTE]
> Builds aren't code-signed yet. On Windows, click **More info → Run anyway**. On macOS, open **System Settings → Privacy & Security** and click **Open Anyway**. Full steps are in the [installation guide](docs/website/installation.md).

## Quick start

1. Install and open LampRabit.
2. Go to **Services** and install Apache (or Nginx), PHP and MariaDB. Pick any versions you like.
3. Click **Start all**.
4. Put a project in the `www` folder (`C:\lampRabit\www` on Windows, `~/lampRabit/www` elsewhere), or click **New project** on the Sites page.
5. Open `https://<folder>.test`. For a Node.js project, press **Start** on its Sites page (or run `npm run dev` yourself) first.

The database user is `root` with no password, like XAMPP and Laragon.

## Coming from XAMPP or Laragon?

Step-by-step guides for moving your sites and databases:

- [Migrate from XAMPP](docs/website/migrate-from-xampp.md)
- [Migrate from Laragon](docs/website/migrate-from-laragon.md)

## Documentation

**Using LampRabit**
- [Features](docs/website/features.md): everything LampRabit does, page by page
- [Installation](docs/website/installation.md): installers, first run, and getting past unsigned-build warnings
- [FAQ](docs/website/faq.md): common questions and fixes
- [Project doctor](docs/project-doctor.md): project health checks and fixes
- [Git workflow](docs/git.md): project Git operations
- [CLI and MCP](docs/cli.md): terminal commands and AI assistant connections
- [Project templates](docs/templates.md): the YAML format for your own New project templates (variables, steps, serving, commands)
- [Migrate from XAMPP](docs/website/migrate-from-xampp.md) and [from Laragon](docs/website/migrate-from-laragon.md)

**Working on LampRabit**
- [Contributing](CONTRIBUTING.md): setup, checks and conventions
- [Releasing](RELEASING.md): versions, the release workflow and LampRabit's own runtime builds
- [Roadmap](PLAN.md): what's done and what's next
- [Security policy](SECURITY.md): reporting a vulnerability

## Building from source

You need Node.js 22+, Rust (stable) and the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for your OS.

```bash
git clone https://github.com/Shivam-SV/LampRabit.git
cd LampRabit
npm install
npm run tauri dev     # run in development
npm run tauri build   # build installers
```

LampRabit is built with [Tauri 2](https://v2.tauri.app/) (Rust) and React + TypeScript.

## Contributing

Bug reports, ideas and pull requests are welcome. Read [CONTRIBUTING.md](CONTRIBUTING.md) to get started, and report security issues privately as described in [SECURITY.md](SECURITY.md).

## License

[MIT](LICENSE)
