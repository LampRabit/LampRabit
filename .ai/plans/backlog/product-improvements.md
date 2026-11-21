# Product improvements, add-ons and accessibility

From the maintainer's research notes (2026-11-14), split from the old `Features.md`. See also [tools-tab.md](tools-tab.md).

LampRabit already has project doctor, Git, backups, templates, stack profiles, onboarding, keyboard shortcuts, reduced-motion settings, a CLI, and MCP. The following ideas build on those capabilities and strengthen the main local-development workflow.

## Faster everyday access

- **Global command palette:** search projects, services, settings, templates, commands, and actions from one keyboard shortcut. Support fuzzy search, recent actions, and context-sensitive commands for the selected project/service.
- **Quick project switcher:** favorites and recent projects, filters for running/stopped/unhealthy, and shortcuts to open the project URL, terminal, editor, Git, dependencies, logs, or health panel.
- **Pinned workspace:** let users choose which services, projects, and status cards appear on the home screen; provide a compact “start this project's stack” action that starts only its declared dependencies.
- **Actionable notifications:** summarize a failed start, stopped process, expiring certificate, or completed backup with a direct button to the affected log or fix. Make noisy notifications configurable.
- **Remembered context:** restore the last page, selected project, filters, and panel position after restart, with a setting to opt out on shared machines.

## Reproducible projects and useful add-ons

- **Environment lock / share file:** extend the existing project configuration with an optional, reviewable file that records required service/runtime versions, project processes, domains, and stack profile. A teammate can preview the plan and install missing components before applying it. Never put passwords, tokens, or private `.env` values in the file.
- **Docker Compose companion:** detect a project's Compose file, show its services and port conflicts beside native LampRabit services, and offer start/stop/log actions when Docker is installed. Keep container volumes and native database data clearly distinct; don't take over Compose files silently.
- **Trusted recipe/add-on catalog:** install versioned templates, project checks, service presets, and workflow recipes from a curated source or a local file. Show publisher, permissions, files/commands to be run, version and checksum; preview changes and ask before executing commands. Start with data-only recipes and keep executable extensions opt-in.
- **Editor and browser integrations:** detect installed editors and browsers, remember per-project choices, and offer direct actions to open the project, local URL, logs, or a selected file. Keep manual paths and custom commands available.
- **Project run configurations:** save named combinations of processes, environment variables, arguments, and required services (for example “frontend + API + worker”). Mask secret values, don't export them by default, and show exactly which processes will launch.
- **Team bootstrap bundle:** export/import a project template, non-secret settings, tool versions, and example environment variables as a reviewable archive. This complements templates and stack profiles without syncing private data.

## Reliability and project insight

- **Service dependency map:** show which project processes depend on PHP/Node/Python versions, web servers, databases, queues, and search services; explain why a service is starting or blocked, and let a user start/stop a selected project's dependency set.
- **Backup confidence:** extend the existing database backup/restore tools with optional schedules, retention rules, destination checks, and a restore-to-temporary-database verification flow. Show last successful backup and last verified restore, and keep backup folders on the safe-uninstall list.
- **Environment compatibility view:** provide a project-by-project matrix for OS, architecture, runtime versions, and unavailable services, linked to the existing service catalog's reasons. Useful when moving a project between machines.
- **Unified activity timeline:** combine project commands, Git operations, service restarts, health changes, backups, and fixes into a searchable local timeline with links back to the relevant panel/log. Do not collect or upload telemetry by default.
- **One-click support bundle:** assemble app/runtime versions, health results, relevant logs, and platform diagnostics into a previewable archive with paths and secret-like values redacted before the user saves or shares it.

## Accessibility and ease of use

- **Screen-reader pass:** audit every page, dialog, tab, menu, table, status badge, progress bar, and live log update for names, roles, keyboard order, and useful announcements. Announce service state changes without repeatedly reading the whole dashboard.
- **Keyboard-only workflows:** make project/service selection, command palette search, inline actions, dialogs, tab movement, and log filtering fully operable without a pointer. Keep visible focus and return focus to the control that opened a dialog.
- **Readable interface options:** add a text-size control, stronger focus outlines, a high-contrast theme, and compact/comfortable density. Respect OS contrast and reduced-motion preferences; ensure status is communicated with text/icons as well as color.
- **Larger interaction targets:** provide a comfortable control-size option and avoid requiring precise dragging for tab order, splitters, or small icon buttons.
- **Clear error recovery:** use plain-language errors with the affected project/service, likely cause, safe next action, and “copy details” control. Keep risky actions previewed and reversible where possible.
- **Accessible onboarding:** offer guided setup by goal (PHP site, Node app, Python API, database-only), skippable steps, keyboard support, and a persistent “what's next?” checklist that can be reopened.

## Suggested product priorities

1. Global command palette and a polished project switcher.
2. Screen-reader, focus, text-size, high-contrast, and status-announcement improvements across existing screens.
3. Project run configurations and an environment lock/share file based on the existing project settings and stack profiles.
4. Backup scheduling and verified restore, building on the existing backup panel.
5. Docker Compose companion as an optional integration for developers who already use containers.
6. Trusted recipe/add-on catalog after permission previews, signing/checksum checks, and upgrade/rollback behavior are defined.

The accessibility ideas use [WCAG 2.2](https://www.w3.org/TR/WCAG22/) as a reference for focus, keyboard access, and target sizing. Compose support should follow Docker's [Compose application model](https://docs.docker.com/compose/intro/compose-application-model/) and preserve user-owned files. Token interchange can track the [W3C Design Tokens Community Group format](https://www.w3.org/community/reports/design-tokens/CG-FINAL-format-20251028/).
