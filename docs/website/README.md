# LampRabit website content pack

Content notes and source copy for the LampRabit marketing and documentation website. Keep feature claims aligned with the current app and platform support.

## What LampRabit is

LampRabit is a free, open-source desktop environment for local web development. It manages web and app servers, languages, databases, project processes and developer tools, with platform availability shown in the app. Projects in `www/` can be served at `https://<folder>.test` with a local certificate. It is built with Rust, Tauri and React.

**One-line pitch:** Your local development stack, without the setup.

**Short description (about 160 characters, for meta tags):** LampRabit manages local web servers, languages, databases and project tools. Put a project in www and serve it at a local .test domain. Free and open source.

## Files in this pack

| File | Use it for |
|---|---|
| [features.md](features.md) | Features page, homepage highlights, comparison table |
| [installation.md](installation.md) | Download / Install page and "Getting started" docs |
| [migrate-from-xampp.md](migrate-from-xampp.md) | Docs page: moving from XAMPP |
| [migrate-from-laragon.md](migrate-from-laragon.md) | Docs page: moving from Laragon |
| [faq.md](faq.md) | FAQ and troubleshooting page |

## Feature status

LampRabit is under active development. Every feature in these files is marked:

- **Available**: in the app today.
- **Coming soon**: planned but not yet shipped; don't imply a release date unless one is confirmed.

Show status and platform limits where relevant. Verify claims against `PLAN.md`, `src-tauri/src/catalog.rs` and the current release before publishing.

Some features are limited by platform. Runtime availability varies by OS, architecture and service; use [installation.md](installation.md) and the in-app catalog rather than a blanket platform claim.

## Suggested site map

1. **Home**: hero (pitch, download button, screenshot of the Overview page), three to six feature highlights, "Switching from XAMPP or Laragon?" call-out, footer.
2. **Features**: grouped as in [features.md](features.md).
3. **Download**: OS-detected download button, system requirements, links to installation docs.
4. **Docs**
   - Getting started (installation.md)
   - Migrating from XAMPP
   - Migrating from Laragon
   - FAQ and troubleshooting
5. **Changelog**: placeholder until releases start (GitHub Releases can feed it).
6. **GitHub** link: https://github.com/Shivam-SV/LampRabit

## Brand

### Name

Always written **LampRabit**: lowercase "lamp", capital "R", single "b". In the logo lockup, "Rabit" is set in the accent colour.

### Logo

- Mark only (no wordmark): `public/logo.svg` in the repository.
- Original with wordmark: `public/LampRabit-logo.svg`.
- App icon (white rounded tile): `src-tauri/app-icon.svg`; PNG sizes in `src-tauri/icons/`.

The mark is a charcoal rabbit head with mint ears and `< >` code brackets for eyes. On dark backgrounds the app lightens the head to `#3a4452` so it stays visible.

### Colours

| Token | Light theme | Dark theme | Use |
|---|---|---|---|
| Background | `#f4f7f6` | `#13171d` | Page background |
| Surface | `#ffffff` | `#1b2028` | Cards |
| Surface 2 | `#eef2f1` | `#232a34` | Subtle fills |
| Border | `#dde3e1` | `#2d3540` | Card borders, dividers |
| Text | `#1b2028` | `#e7ecf1` | Body text |
| Muted text | `#5d6775` | `#8d98a7` | Secondary text |
| Accent | `#168a64` | `#59d1a2` | Buttons, links, highlights |
| Accent soft | `#e2f5ed` | mint at 12% opacity | Pills, selected states |
| Danger | `#d1435b` | `#ef6b7f` | Errors |

Brand core: charcoal `#1b2028`, mint `#59d1a2`, white. In light mode the accent is the deeper green `#168a64` because mint on white fails contrast for text.

Service colours used across the app (icons and badges): Apache rose `#f43f5e`, Nginx emerald `#059669`, PHP indigo `#6366f1`, MariaDB amber `#d97706`, Mailpit sky `#0ea5e9`, Adminer teal `#14b8a6`, phpMyAdmin orange `#f97316`.

### Type

Inter (variable), with system UI fonts as fallback. Headings bold to extra-bold with slightly tight tracking; code and ports in a monospace (JetBrains Mono / Cascadia Code / Consolas).

### Voice

Plain, friendly, specific. Say what happens ("Drop a folder in www and it's live at myapp.test") rather than abstract claims ("seamless developer experience"). Avoid jargon where a plain word works; when a technical term matters to developers (FastCGI, vhost), use it.

## Screenshots to capture

Take these from the app in both light and dark themes at 1280×860:

1. Overview with Apache, MariaDB and Mailpit running (hero image).
2. Services page: "Your stack" diagram with services running, plus a version rail.
3. Sites page with two or three sites, one on a custom PHP version.
4. Logs page with a mix of Apache, PHP and System lines.
5. Settings: Web server card set to "Both".
6. A service card showing a port-conflict problem panel (start a service while another program holds its port).
7. Diagnostics dialog.

Use project names like `shop`, `blog`, `api` rather than personal folders, and keep the Windows user name out of shots (Diagnostics has a "Hide user name" switch).
