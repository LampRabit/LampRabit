# LampRabit Tools tab — research and feature ideas

The Tools tab can collect the small utilities developers and designers repeatedly open in separate browser tabs. Keep the first version focused on fast, dependable tools that work locally, need no account, and can pass their output directly into another tool. Treat this as a candidate catalog, not a commitment to implement every item.

Research checked 2026-11-14. Links below point to established examples and primary tool pages; availability and capabilities can change.

## Product principles

- **Local first:** process pasted text, code, and files on the user's machine whenever possible. Clearly label any tool that needs a network request or third-party service. Don't send secrets, source code, or private files to a remote converter.
- **Useful in a desktop dev environment:** prioritize tools that fit local projects, runtimes, APIs, logs, and assets over generic office suites or cloud collaboration platforms.
- **Input to output:** support paste, drag and drop, copy, save, and opening a result in a related tool. Keep the original available and show errors at the relevant input location.
- **Composable:** let users chain compatible tools (for example JSON → YAML → URL-encode), with undo and a clear reset action.
- **Discoverable:** searchable catalog, categories, favorites, recent tools, keyboard shortcuts, and smart suggestions based on clipboard/input type.
- **Honest about results:** show source and target formats, lossy conversions, limits, and accessibility/security caveats.

## My regular workflow — requested priorities

These are the maintainer's recurring use cases and should drive the Tools tab design and prioritization.

### Writing, comparing, and saving

- **Rich Markdown editor:** comfortable editing with live preview, split view, headings/lists/tables, code fences, task lists, links/images, Mermaid preview, and copy/export to Markdown or HTML. Keep raw Markdown easy to edit and preserve it on disk.
- **Full diff and compare:** compare text/code files and pasted content; side-by-side and unified views, syntax highlighting, whitespace/line-ending options, ignore-case option, and clear add/remove/change markers. Include JSON-aware comparison and image comparison as related modes.
- **Snippet library with GitHub Gists:** save, tag, search, edit, and copy code snippets locally; optionally connect a GitHub account to list/create/update gists and sync selected snippets. Show whether each gist is public or secret, and clearly explain that secret gists are unlisted rather than private. Use GitHub's supported authorization flow with the narrowest permissions, store credentials in the OS credential store, and never sync without the user's choice.

### Design-to-code and visual design

- **SVG to React component:** paste/drop SVG, preview it, choose TypeScript or JavaScript, component name, props, title/accessibility handling, and options for SVGR-compatible JSX output. Keep conversion local.
- **CSS to Tailwind generator:** paste CSS or select CSS from a project and convert declarations into Tailwind classes. Support current Tailwind v4 CSS-first theme variables and v3 config conventions; show a side-by-side mapping, flag unsupported selectors/values, and retain unconverted CSS rather than silently dropping it.
- **Logo-to-theme builder:** load a logo/image or enter brand colors; extract/edit a palette, then preview a small UI with light/dark modes and export semantic tokens for plain CSS variables, Tailwind v3/v4, shadcn/ui, and Bootstrap. Let users map primary, secondary, accent, surface, foreground, border, muted, success, warning, and destructive roles; check contrast before export.
- **Palette discovery and exploration:** extract dominant colors from a logo/image, generate related palettes (complementary, analogous, triadic, monochromatic, and curated alternatives), lock colors, adjust hue/lightness/chroma, simulate color vision deficiencies, and preview palettes in a UI. Provide copyable HEX/RGB/HSL/OKLCH and CSS token output.
- **Contrast checker:** test any foreground/background pair, alpha/transparency, text sizes and UI component colors; show ratio and WCAG AA/AAA results plus nearby passing choices. Support checking all pairs in a generated theme.
- **Gradient builder:** visual linear/radial/conic editor, multiple stops, angle/position controls, repeating gradients, CSS preview, and export to CSS/Tailwind-compatible arbitrary values.
- **CSS blob builder:** create organic SVG/CSS blob shapes with editable points, smoothness, rotation, fill, and gradients; preview responsive sizing and export SVG or CSS `border-radius`/clip-path with an editable source.
- **Font pairing helper:** compare heading/body/mono combinations in a sample page, with weights, variable-font axes, line-height and letter-spacing controls, language/glyph coverage, and CSS/import export. Include theme preview so users can see the fonts with their colors and components.

### Encoding, generators, and placeholders

- **URL tools:** encode/decode full URLs and individual components; parse query parameters, path, fragment, and entities; show duplicate parameters and malformed escapes without losing the original input.
- **Case converter:** camelCase, PascalCase, kebab-case, snake_case, CONSTANT_CASE, Title Case, sentence case, and space/ dot/path variants; preserve acronyms and allow batch conversion.
- **ID generators:** bulk UUID v4/v7, ULID, and NanoID with count, separator, prefix, copy, and export controls. Use secure randomness where required.
- **Placeholder content:** configurable Lorem Ipsum by paragraphs/sentences/words; placeholder images by width × height, aspect ratio, color, label, and format, with local SVG/PNG output and optional generated URL templates for common placeholder services.
- **Memorable secure passwords:** generate passphrases from a local word list using a cryptographically secure random source; set word count, separator, capitalization, and optional digits/symbols. Show an estimate/entropy context without implying a password is safe for every policy.

### API design and requests

- **OpenAPI / Swagger workspace:** create, import (Via project or direct), detect from project or manual discovery, edit, validate, and preview OpenAPI JSON/YAML; browse paths, schemas, parameters, auth and examples; generate starter schemas from sample request/response JSON.
- **HTTP requester:** send requests based on a selected OpenAPI operation or manually; configure environment variables, headers, query/path params, body and auth; show response status/headers/body/timing and save request collections. Make it clear when a request leaves localhost and redact secrets from saved examples/logs.
- **Mock API from schema:** optionally serve local example endpoints from an OpenAPI document so a frontend can be built before its backend is ready.

## More related ideas

- **Theme accessibility report:** scan exported token pairs for text and control contrast, check light and dark modes, and export a concise report with failing tokens and suggested adjustments.
- **Design token interchange:** import/export W3C Design Tokens Community Group JSON, CSS variables, Tailwind theme values, shadcn semantic variables, and Bootstrap Sass/CSS variables. Preserve token names and explain unsupported mappings.
- **CSS selector/HTML playground:** preview a small HTML fragment with CSS and responsive breakpoints to check generated utilities before copying them into a project.
- **Logo asset packager:** generate common favicon sizes, Apple touch icon, Android/PWA icons, and a `site.webmanifest` from a logo; preview transparent padding and background choices.
- **Theme snapshot and compare:** save named versions of a theme, view token-level differences, and restore a previous palette or typography pairing.
- **API collection import/export:** import and export OpenAPI plus common HTTP collection formats; generate cURL and fetch snippets from a selected request.
- **Request-to-fixture helper:** capture a sanitized HTTP response and turn it into a JSON fixture, TypeScript type, or Markdown example.
- **Color from screenshot:** sample colors from a dropped screenshot or local project image, then add chosen colors to a palette.
- **Shareable local exports:** save a complete theme/snippet/tool output as a file or copyable text bundle. Make remote sharing optional and visible.

## Beyond Tools: product improvements, add-ons, and accessibility

LampRabit already has project doctor, Git, backups, templates, stack profiles, onboarding, keyboard shortcuts, reduced-motion settings, a CLI, and MCP. The following ideas build on those capabilities and strengthen the main local-development workflow.

### Faster everyday access

- **Global command palette:** search projects, services, settings, templates, commands, and actions from one keyboard shortcut. Support fuzzy search, recent actions, and context-sensitive commands for the selected project/service.
- **Quick project switcher:** favorites and recent projects, filters for running/stopped/unhealthy, and shortcuts to open the project URL, terminal, editor, Git, dependencies, logs, or health panel.
- **Pinned workspace:** let users choose which services, projects, and status cards appear on the home screen; provide a compact “start this project's stack” action that starts only its declared dependencies.
- **Actionable notifications:** summarize a failed start, stopped process, expiring certificate, or completed backup with a direct button to the affected log or fix. Make noisy notifications configurable.
- **Remembered context:** restore the last page, selected project, filters, and panel position after restart, with a setting to opt out on shared machines.

### Reproducible projects and useful add-ons

- **Environment lock / share file:** extend the existing project configuration with an optional, reviewable file that records required service/runtime versions, project processes, domains, and stack profile. A teammate can preview the plan and install missing components before applying it. Never put passwords, tokens, or private `.env` values in the file.
- **Docker Compose companion:** detect a project's Compose file, show its services and port conflicts beside native LampRabit services, and offer start/stop/log actions when Docker is installed. Keep container volumes and native database data clearly distinct; don't take over Compose files silently.
- **Trusted recipe/add-on catalog:** install versioned templates, project checks, service presets, and workflow recipes from a curated source or a local file. Show publisher, permissions, files/commands to be run, version and checksum; preview changes and ask before executing commands. Start with data-only recipes and keep executable extensions opt-in.
- **Editor and browser integrations:** detect installed editors and browsers, remember per-project choices, and offer direct actions to open the project, local URL, logs, or a selected file. Keep manual paths and custom commands available.
- **Project run configurations:** save named combinations of processes, environment variables, arguments, and required services (for example “frontend + API + worker”). Mask secret values, don't export them by default, and show exactly which processes will launch.
- **Team bootstrap bundle:** export/import a project template, non-secret settings, tool versions, and example environment variables as a reviewable archive. This complements templates and stack profiles without syncing private data.

### Reliability and project insight

- **Service dependency map:** show which project processes depend on PHP/Node/Python versions, web servers, databases, queues, and search services; explain why a service is starting or blocked, and let a user start/stop a selected project's dependency set.
- **Backup confidence:** extend the existing database backup/restore tools with optional schedules, retention rules, destination checks, and a restore-to-temporary-database verification flow. Show last successful backup and last verified restore, and keep backup folders on the safe-uninstall list.
- **Environment compatibility view:** provide a project-by-project matrix for OS, architecture, runtime versions, and unavailable services, linked to the existing service catalog's reasons. Useful when moving a project between machines.
- **Unified activity timeline:** combine project commands, Git operations, service restarts, health changes, backups, and fixes into a searchable local timeline with links back to the relevant panel/log. Do not collect or upload telemetry by default.
- **One-click support bundle:** assemble app/runtime versions, health results, relevant logs, and platform diagnostics into a previewable archive with paths and secret-like values redacted before the user saves or shares it.

### Accessibility and ease of use

- **Screen-reader pass:** audit every page, dialog, tab, menu, table, status badge, progress bar, and live log update for names, roles, keyboard order, and useful announcements. Announce service state changes without repeatedly reading the whole dashboard.
- **Keyboard-only workflows:** make project/service selection, command palette search, inline actions, dialogs, tab movement, and log filtering fully operable without a pointer. Keep visible focus and return focus to the control that opened a dialog.
- **Readable interface options:** add a text-size control, stronger focus outlines, a high-contrast theme, and compact/comfortable density. Respect OS contrast and reduced-motion preferences; ensure status is communicated with text/icons as well as color.
- **Larger interaction targets:** provide a comfortable control-size option and avoid requiring precise dragging for tab order, splitters, or small icon buttons.
- **Clear error recovery:** use plain-language errors with the affected project/service, likely cause, safe next action, and “copy details” control. Keep risky actions previewed and reversible where possible.
- **Accessible onboarding:** offer guided setup by goal (PHP site, Node app, Python API, database-only), skippable steps, keyboard support, and a persistent “what's next?” checklist that can be reopened.

### Suggested product priorities

1. Global command palette and a polished project switcher.
2. Screen-reader, focus, text-size, high-contrast, and status-announcement improvements across existing screens.
3. Project run configurations and an environment lock/share file based on the existing project settings and stack profiles.
4. Backup scheduling and verified restore, building on the existing backup panel.
5. Docker Compose companion as an optional integration for developers who already use containers.
6. Trusted recipe/add-on catalog after permission previews, signing/checksum checks, and upgrade/rollback behavior are defined.

The accessibility ideas use [WCAG 2.2](https://www.w3.org/TR/WCAG22/) as a reference for focus, keyboard access, and target sizing. Compose support should follow Docker's [Compose application model](https://docs.docker.com/compose/intro/compose-application-model/) and preserve user-owned files. Token interchange can track the [W3C Design Tokens Community Group format](https://www.w3.org/community/reports/design-tokens/CG-FINAL-format-20251028/).

## Developer feed and Playground

These are optional companion features for keeping up with the developer community and taking short breaks. Keep them secondary to project, service, and tool workflows; start with an Overview card or optional page before committing to permanent top-level tabs.

### Developer feed

- **External-source reader:** show developer stories from sources the user chooses, such as the official Hacker News API and user-added RSS/Atom feeds. Consider daily.dev as an optional connector if its API terms and stability fit the integration.
- **Useful feed controls:** source and topic filters, sort by newest/popular where the source supports it, refresh status, de-duplication, bookmarks/read-later, and “open original” links. Keep the initial version read-only; no voting, posting, or app-hosted publishing.
- **Respect the source:** label every item with its source, link to the original, and only display the metadata/excerpts allowed by that source. Avoid copying full articles or building a LampRabit-hosted content feed. Make refresh frequency configurable and show when the feed was last updated.
- **Offline behavior:** keep a small, user-controlled cache for recently viewed headlines and clearly show that new items require an internet connection. Let users remove sources and clear cached items.
- **Placement:** start as a collapsible Overview card or an optional “Discover” page. Promote it to a permanent tab only if it becomes a frequent part of the app's daily workflow.

### Just for fun: Playground

- **Short break activities:** optional logic/brain puzzles, pattern challenges, lightweight algorithm prompts, regex puzzles, and SQL challenges using a disposable local SQLite dataset.
- **Developer mini-games:** examples include trace-the-output, spot-the-bug, arrange-the-Git-commits, find-the-regex-match, and query-the-sample-database. Keep sessions short and let users reveal hints or skip.
- **Progress without pressure:** local streaks, personal bests, difficulty settings, and optional reminders; no account, leaderboard, or social pressure required.
- **Offline and safe by default:** puzzle content should run locally and not execute arbitrary user code. If code challenges are added later, run them in a constrained, disposable sandbox with clear time/resource limits.
- **Placement:** open it from a small Playground/mascot menu or optional Overview card. Keep it out of the primary navigation initially so it adds personality without competing with developer workflows.

### Suggested order

1. Add an optional, read-only feed card with Hacker News and custom RSS/Atom sources.
2. Add a small collection of offline puzzles reachable from the mascot or Overview.
3. Consider dedicated Discover/Playground pages only after usage shows that either feature merits a permanent navigation item.

Reference sources: [Hacker News API](https://github.com/HackerNews/API) provides public story data; [daily.dev API documentation](https://docs.daily.dev/welcome-to-the-daily-dev-api/) describes a public API that is currently beta and subject to its terms. Check each source's content and API terms before adding a built-in connector.

## Candidate tools

### 1. Everyday developer utilities

- **Format and validate:** JSON, XML, YAML, TOML, SQL, HTML, CSS, JavaScript/TypeScript, PHP, Markdown, and `.env` files; minify where useful.
- **Convert structured data:** JSON ↔ YAML ↔ TOML; CSV ↔ JSON; JSON ↔ TypeScript types / interfaces; JSON ↔ JSON Schema; XML ↔ JSON; HTML ↔ Markdown; Markdown ↔ HTML; `.env` ↔ JSON.
- **Compare and inspect:** text diff, side-by-side diff, JSON-aware structural diff, deep merge, JSONPath/JQ-style query preview, JSON flatten/unflatten, CSV table preview, and line/column count.
- **Encoding and escaping:** URL encode/decode, Base64, HTML entities, Unicode/code point inspector, JSON string escape, SQL string escape, and common language string literals.
- **Hashing and identifiers:** SHA-256/SHA-512 and file checksums, HMAC, UUID v4/v7, ULID, NanoID, secure random strings, and password/passphrase generation. Never call weak hashes suitable for password storage without an explicit warning.
- **Regex workbench:** test patterns against multiple sample strings, show matches/groups, explain syntax, and preview replacements.
- **Date and time:** Unix timestamp seconds/milliseconds ↔ readable date, ISO 8601, timezone conversion, duration calculation, and cron expression explanation with upcoming run times.
- **Number and unit conversion:** binary/octal/decimal/hex, bytes and storage units, common length/weight/temperature units, and CSS units (px/rem/em/vw).
- **Currency conversion:** optional live exchange rates with provider and “rate updated” timestamp. Requires internet; do not imply a quote is current without a timestamp.
- **Network/data helpers:** IP/CIDR/subnet calculator, URL parser, user-agent parser, QR code generation/read, and MIME type lookup.
- **Command helpers:** cURL-to-fetch/Axios/Python/Go conversion, shell quoting/escaping, `.gitignore` generator, `.editorconfig` generator, and Docker Compose / config snippets.
- **Text utilities:** case conversion, slugify, sort/deduplicate lines, trim whitespace, count words/characters/bytes, lorem ipsum, and redaction of common secret patterns before sharing logs.

### 2. API, backend, and local-stack helpers

- **HTTP request scratchpad:** send GET/POST/etc. requests to localhost or a chosen URL, edit headers/query/body, format JSON responses, inspect status/headers/timing, and copy a cURL command. Start with a simple REST client; consider GraphQL and WebSocket later.
- **Webhook receiver/tester:** show requests arriving at a temporary localhost endpoint; generate a replayable request and optionally forward to a configured local project. Keep captures local and make retention/clear controls obvious.
- **JWT inspector:** decode header and payload, show claims/expiry; make clear that decoding does not verify a signature. Signature verification should require an explicitly supplied key and remain local.
- **Certificate and key inspector:** inspect PEM/X.509 metadata and expiry; avoid displaying private key contents by default.
- **SQL workbench helpers:** format SQL, explain common connection-string fields, convert between common DSN formats, and generate safe sample insert statements from a table definition.
- **Project helpers:** `.env` comparison that masks secret values, port availability checker, local URL/hosts lookup, PHP/Node/Python version and path inspector, and a one-click copy of project diagnostics.
- **Test data generator:** create realistic but fake names, addresses, dates, UUIDs, and JSON/CSV fixtures from a schema. Never use real personal data as sample output.
- **Regex and log helpers:** parse/pretty-print common Apache/Nginx/PHP logs, highlight timestamps/levels/IPs, filter by pattern, and convert a selected error to a shareable redacted snippet.

### 3. Design, accessibility, and frontend tools

- **Color workbench:** HEX/RGB/HSL/HSV/OKLCH conversion, eyedropper where supported, complementary/analogous palette suggestions, shades/tints, gradient builder, CSS variable export, and color-blindness preview.
- **Contrast matcher:** foreground/background contrast ratio and WCAG AA/AAA checks for normal and large text plus UI components; live sample text and suggested nearest passing color. Explain the chosen threshold.
- **Typography preview:** compare font families, weights, sizes, line heights, and letter spacing using sample UI text; copy CSS font stacks and Google Fonts import snippets. Font downloads need network; support system fonts offline.
- **Spacing and layout calculator:** px ↔ rem/em, fluid type/spacing clamp() generator, grid/flexbox playground, aspect ratio calculator, and breakpoint preview.
- **CSS gradient/shadow/radius tools:** visual editors with copyable CSS and a preview panel.
- **SVG tools:** SVG preview, inspect dimensions/viewBox, optimize/minify with before/after size, and convert SVG to data URI. Preserve a view of the original because optimization settings can change appearance.
- **Image tools:** resize/crop, convert PNG/JPEG/WebP/AVIF where codecs are available, compress with quality/size preview, create favicon/app icon sizes, extract a palette, and compare before/after. Process locally and report format/metadata limitations.
- **Image format helper:** explain the trade-offs among raster formats and help choose a format for photos, transparency, icons, and animation; don't promise universal codec support.
- **Markdown and diagrams:** Markdown editor with preview; Mermaid diagram preview/export; generate a README/table of contents from headings; copy as Markdown/HTML.
- **Browser support lookup:** search browser feature compatibility (such as CSS or JS APIs), then link to the underlying compatibility record and date/source.
- **Page audit shortcut:** open Lighthouse/PageSpeed guidance for a selected project URL, or run a local audit if a suitable installed browser/CLI is available. Distinguish local audit output from remote PageSpeed analysis.
- **Code screenshot / snippet card:** render a syntax-highlighted snippet for docs or issue reports, with language/theme controls and a warning before exporting sensitive text.

### 4. General workflow helpers

- **QR and barcode tools:** create QR codes for local URLs, Wi-Fi, text, and contact cards; read codes from an image. Warn before encoding a URL that is only reachable on the user's machine.
- **Clipboard history for the Tools tab:** keep a short, user-controlled history of tool inputs/outputs; exclude secrets by default and provide clear/delete controls.
- **Scratchpad and notes:** temporary snippets with language labels, project association, and optional persistence in the LampRabit environment folder.
- **Templates and generators:** common config starters (`.env.example`, Docker Compose, Nginx/Apache snippets, GitHub Actions, editor config), with project/runtime-aware values where available.
- **Cheat sheets:** searchable references for cron, regex, HTTP status codes, MIME types, Git commands, SQL date formats, and common shell commands.

## Suggested rollout

### First release: high-frequency, low-dependency

1. Rich Markdown editor and full text/code diff
2. SVG-to-React converter and SVG preview/optimizer
3. Color palette extraction, palette explorer, contrast checker, and gradient builder
4. URL encoder/decoder/parser, string case converter, UUID/ULID/NanoID bulk generator
5. Lorem Ipsum and placeholder image generator
6. Memorable secure passphrase generator
7. CSS-to-Tailwind helper and logo-to-theme exports (CSS, Tailwind, shadcn/ui, Bootstrap)
8. Font pairing preview with theme support
9. OpenAPI editor/viewer connected to an HTTP requester
10. Local snippets library, followed by optional GitHub Gist sync
11. CSS blob builder
12. JSON formatter/validator, JSON ↔ YAML converter, time converter, regex tester, SQL formatter, and common unit converters

### Later: deeper integration or higher maintenance

- Webhook capture, project `.env` diff, port checker, test data generator, mock API from schema, API collection import/export, and request-to-fixture helper.
- Theme token interchange, accessibility report, logo asset packager, theme snapshots, Mermaid preview, browser support lookup, and CSS/HTML playground.
- Optional live currency rates, remote page audits, cloud font catalog, and GitHub sync. These depend on network access and should disclose their provider and privacy behavior.
- Add file conversion only where format quality, file size, licensing, and local codec support can be maintained. Avoid promising arbitrary conversion across office/document formats without dedicated engines.

## Existing tools to study

These sites establish useful patterns and coverage; they are references for research, not dependencies or endorsements. Recheck licensing, privacy behavior, supported formats, and maintenance before adapting an implementation.

- [DevToys](https://devtoys.app/) — desktop toolbox; its smart detection pattern suggests tools from clipboard/input type and moves output between compatible tools.
- [CyberChef](https://gchq.github.io/CyberChef/) — composable operations for encoding, parsing, hashing, compression, timestamps, and other data transformations; also offers a standalone copy.
- [regex101](https://regex101.com/) — interactive regex editor with match explanation, debugger, code generation, and tests.
- [Crontab.guru](https://crontab.guru/) — cron syntax explanation and schedule preview.
- [WebAIM Contrast Checker](https://webaim.org/resources/contrastchecker/) — contrast ratio and WCAG pass/fail for text and interface colors.
- [Coolors](https://coolors.co/) — palette generation, color extraction, contrast checking, previews, and exports.
- [SVGOMG](https://jakearchibald.github.io/svgomg/) / [SVGO](https://svgo.dev/docs/usage/) — interactive SVG optimization and the underlying optimizer interfaces.
- [Squoosh](https://squoosh.app/) — image encoding/compression preview and format comparison.
- [Chrome Lighthouse](https://developer.chrome.com/docs/devtools/lighthouse) — performance, accessibility, best practices, and SEO audits within Chrome DevTools.
- [Hoppscotch](https://hoppscotch.io/) — API request development across REST, GraphQL, and WebSocket workflows.
- [Can I use](https://caniuse.com/) — browser compatibility lookup for web platform features.
- [Mockaroo](https://mockaroo.com/) — schema-driven generation of realistic fake test data.
- [Carbon](https://carbon.now.sh/) — styled code snippet images for documentation and sharing.
- [SVGR](https://react-svgr.com/docs/what-is-svgr/) — transforms SVG files into React components.
- [Tailwind CSS theme variables](https://tailwindcss.com/docs/theme) — current theme variable and utility generation model; the CSS-to-Tailwind tool needs version-aware output.
- [shadcn/ui theming](https://ui.shadcn.com/docs/theming) — semantic CSS variable tokens and visual theme preset workflow.
- [Swagger Editor](https://www.swaggereditor.org/) — create, edit, validate, and visualize OpenAPI JSON/YAML descriptions.
- [GitHub Gists REST API](https://docs.github.com/en/rest/gists/gists) — list, create, update, and delete gists; use an explicit authorization flow and carefully explain public vs secret visibility.
- [Google Fonts](https://fonts.google.com/) — searchable font catalog that can inform an optional online font picker; local/system font preview should still work offline.

## Implementation notes

- Reuse existing Rust/JS libraries where licenses and maintenance are suitable; keep complex codecs and engines optional rather than bloating the base installer.
- Use file size limits, cancellation, and progress for file operations. Keep originals unchanged and write converted files to a user-selected destination.
- For cryptographic utilities, state whether an operation is encoding, hashing, signing, or encryption; don't imply Base64 is encryption or JWT decoding is verification.
- For accessibility utilities, identify the WCAG version/criteria applied and link to the standard/reference. A contrast check doesn't replace a full accessibility audit.
- Tools that depend on currency feeds, package registries, font catalogs, or remote audit services need clear offline states and source timestamps.
