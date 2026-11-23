# Decisions: UX principles ("fit the elephant in a box")

The user's decisions of 2026-11-21; they apply to **every** new feature. Source: [the 2026-11-21 spec](../../plans/source-specs/2026-11-21-feature-context.md), "UX principles". Problem they solve: each feature makes the app harder for new users ("where's php.ini?"), so power stays invisible until needed.

## Five homes

Every feature lives in exactly one:

| Home | Holds | Today in the app |
|---|---|---|
| **Projects** | your code: run, endpoints, env, logs, cron, project tools | the Sites page |
| **Services** | runtimes (PHP, Node, .NET, Dart/Flutter, Python, Go, JDK… and their config files) **and** background services | the Services page |
| **Devices** | phones, emulators, simulators | doesn't exist; added with the first device feature |
| **Tools** | standalone utilities | doesn't exist; the Tools tab |
| **Remote** | servers, SSH/SFTP/FTP, tunnels to servers, deployed apps' logs and health | doesn't exist; added with the first remote feature |

- **No separate Runtimes tab** (the user's choice, 2026-11-21): runtimes stay inside Services, as today.
- No new top-level tab without a strong reason. Home, Logs and Settings stay as they are.

## Rules

1. **Command palette (Ctrl/Cmd+K)** searches actions, settings, config files, projects, services and docs, with synonyms ("phone/adb" → Devices, "ssl/https" → endpoint SSL). Every feature registers its actions and keywords in one registry, which also feeds the CLI and MCP (one action list, three surfaces).
2. **"Where is it?" is one click:** every managed config file or binary has Open in editor · Reveal in folder · Copy path; each runtime/service lists its config files and which one is in effect; edits offer "restart to apply".
3. **Progressive disclosure:** show only panels relevant to a project's kinds/endpoints; heavy capabilities (mobile, .NET, Flutter) are opt-in modules downloaded on first use; Advanced sections are collapsed, never removed.
4. **Contextual nudges:** suggestions on events (phone plugged in, `.csproj` without an SDK, port conflict), rate-limited, with "don't show again". Every error says what happened, why, and gives a one-click fix or a docs link; raw output goes under Details.
5. **Guided first runs** per feature (empty states with a short checklist); global onboarding stays short.
6. **Docs are part of the product:** every panel has a "?" to a task-titled docs page ("Where is php.ini in LampRabit"); the palette finds docs too.
7. **Telemetry only opt-in**, anonymous and documented (most useful signal: palette searches with no result).

## Visual style (the user's choices, 2026-11-23)

The UI was polished to stop looking AI-built (an "unslop" pass), keeping the brand (charcoal and mint from the logo), Inter with JetBrains Mono, the layouts and every feature.

- **Home's title is "Home"** with a status line ("2 of 5 services running · 3 projects", or what needs attention). The time-of-day greeting, the random quote and the "Your name" setting (and the setup guide's name step) are gone. `developerName` stays in settings.toml, unused.
- **Service, runtime and project icons are neutral** (`TILE` in `components/runtimes.tsx`: `bg-surface-2 text-muted`). Colour means status: the **status light** (a dot: mint running, gray stopped, amber degraded, red failed) is the one signature detail; `Pill` in `ui.tsx` is that dot plus a quiet word, no tinted capsule.
- The accent marks the one primary action, the selection and the update notice. Row actions (Start next to Stop) are quiet (`soft` now looks like `secondary`).
- Sentence case everywhere (no uppercase tracked labels), weights 400–500, text no smaller than 11px, ports and figures as plain mono text without boxes, no sparkle icons, no `transition-all`, no overshoot easing, pulses only on things in transition (starting, stopping, live streams).
- Logs: error and warning keep their colour; sources and info lines are neutral. Doctor findings: the icon carries the colour, the row stays neutral.
- Template `color:` still tints a template's tile (it's the user's choice in the template editor).

The palette's items are built in `useItems()` (`components/CommandPalette.tsx`); the CLI and MCP go through `control_command`. Settings rows are indexed from Settings.tsx's source, so a new `<Row label hint>` is searchable without extra work (2026-11-24).

## Definition of done for a new feature

1. It has one home. 2. Its actions are in the palette with keywords (and CLI/MCP where sensible). 3. Its config files are reachable with Open/Reveal/Copy path. 4. Its errors have a fix or a docs link. 5. It has an empty state or first-run hint. 6. It has a task-titled docs page.
