# Phase 17: templates and more frameworks

Status: **done**. Listed in the [roadmap](../roadmap.md).

Done (2026-10-30): YAML templates (`src-tauri/templates/`, user ones in `<root>/templates/`, reference in docs/templates.md) with variables, optional steps, `if:` conditions, requirement checks, risky-step approval for user templates, and the steps `composer`, `php`, `run`, `download`, `write`, `append`, `copy`, `env`, `replace`, `create_database`, `git_init`. Built in: the five presets plus CakePHP, CodeIgniter, Drupal and Statamic, each verified by creating a project in WSL. The site records `template = "id@version"`. Since then: Magento, and Node templates (Vite, Next.js, Nuxt, Astro, Express, NestJS) whose variables mirror each CLI's own questions (labelled options, conditions, flags, multi-select). Since then (2026-11-05): `processes:` and `domains:` in templates, the template editor (`TemplateEditor.tsx`) and Save as template from a project (`templategen.rs`). Phase 17 is complete.

- The hard-coded `Preset` enum becomes declarative, versioned templates (built in plus a user folder): variables with defaults and validation, prerequisites (PHP ≥, extensions, Node, a database), steps (`composer create-project`, commands, `.env` keys, create database, domains/processes from phase 12, git init), live output, cleanup on failure. The five current presets are ported first, behaving the same.
- New: CakePHP, CodeIgniter 4, Drupal, Statamic; Vite (React/Vue/Svelte), Next.js, Express API; Laravel + Vite. Magento last, with a requirements check (Adobe auth keys, OpenSearch from phase 16, memory).
- The project records which template and version created it.
