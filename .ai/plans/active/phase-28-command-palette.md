# Command palette and "where is it"

Home: global. Decisions: [../../context/decisions/ux-principles.md](../../context/decisions/ux-principles.md) rules 1–2. Source: [spec](../source-specs/2026-11-21-feature-context.md) "UX principles" §2–3. Status: **phase 28, steps 1–2 built 2026-11-24** on `dev-1.0`, for 0.8. Size: M. The spec recommends it **first**, since every later feature registers with it.

## Already in LampRabit

- `SHORTCUTS` in `src/lib/shortcuts.ts` (the `?` dialog) lists keyboard actions.
- `control_command` in `cmd/control.rs` already maps one list of requests onto commands for both the CLI and MCP. The palette's registry should build on these two, not add a third list.
- "Open config" exists for some services; no general Open/Reveal/Copy path list.

## Feasibility

Feasible, frontend-heavy. Searching docs needs a small docs index bundled with the app (from `docs/`). Config-file search needs each service/runtime to declare its files: add `config_files` to service manifests (phase 27).

## Plan

1. Action registry (id, title, keywords/synonyms, home, run, enabled-when) seeded from `SHORTCUTS` and page actions; Ctrl/Cmd+K opens it (add to `SHORTCUTS`, `useHotkeys`).
2. Sources: actions, projects, services, settings sections, config files, docs pages; a synonym table ("phone/adb", "ssl/https/certificate", "jdk/java_home/gradle").
3. Config files panel per runtime/service: Open in editor · Reveal in folder · Copy path, which file is in effect (e.g. php.ini of the active PHP), "restart to apply" after edits.
4. Later: expose the same registry through `control_command` so CLI/MCP gain any missing actions.

## Built (2026-11-24)

- `src/lib/palette.ts`: `PaletteItem`, `search` (every word must match; title start > word start > contains > group/hint/keywords; synonyms count at 0.6; recently used get a small lift and lead an empty query), `expand` (synonym table), recent ids in localStorage. Tests: `palette.test.ts`.
- `src/components/CommandPalette.tsx`: Ctrl/Cmd+K anywhere (also while typing), and a Search button in the title bar. Items: pages, global actions (keys from `SHORTCUTS` by `id`, so shortcuts stay one list), every project (open, browser, editor, terminal), installed services (start/stop/restart, logs), every runtime (versions and settings, or Install), Settings sections (`lib/settingsSections.ts`, with keywords), service config files (open in editor), docs pages (GitHub). Test: `CommandPalette.test.tsx`. Screenshots: `palette`, `palette-dark`.
- Store: `openService(id)` (Services selects it, switching to Everything when needed), `openSettings(section)` (Settings scrolls to it), `paletteOpen`. `Dialog` has `bare` for the palette.
- New features add their items in `useItems()` (definition of done, rule 2).

## Left

- Step 3: config files beyond services (php.ini of the active PHP, Composer's config), "which file is in effect", "restart to apply".
- Step 4: the same actions through `control_command` for the CLI and MCP.
- A Settings row search from the palette (individual settings, not only sections).

## Open questions

- Recently used and frequency ranking: store locally (localStorage is fine, it's a per-device convenience).
