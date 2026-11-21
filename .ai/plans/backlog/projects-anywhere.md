# Projects from any folder or drive, with safe recovery

Home: Projects. Source: [spec](../source-specs/2026-11-21-feature-context.md) "Feature 8". Status: **not started**, **approved by the user** (2026-11-21). Size: L.

Not the same as phase 26 ("Your folder, anywhere"), which moves LampRabit's own folder.

## How it works today

Projects are exactly the folders in `www/` (`sites.rs` scans it). Imports from other tools **copy** projects into `www/` and never link them, so deleting a project in LampRabit can't touch the original ([decisions/projects.md](../../context/decisions/projects.md), "Importing from other tools").

## Feasibility

Feasible, but it changes a core assumption:
- A project registry (absolute path + stable id) next to the `www/` scan; vhosts, PHP `[PATH=…]` sections, port routing and the doctor all take paths already, so most code follows.
- **Delete must never delete an external folder**: for linked projects "Remove" only unregisters. This keeps the spirit of the import decision.
- File watching needs a new dependency (`notify` crate); network drives need polling. Drive removal can be detected by the root disappearing while its volume is gone (sysinfo's disk list is already used by `relocate.rs`).
- Windows: running processes hold the folder; stop them first. macOS: permission prompts for external volumes and Documents. WSL paths (`\\wsl$\…`) won't work with Windows web servers reliably; refuse them with a reason.

## Plan

1. "Add existing folder" (link, don't copy) with a stable id written to `lamprabit.toml` (fallback: git remote + root commit).
2. A project whose folder disappears becomes **Offline**: stop its processes, pause its cron, close its shares; its domain serves a "Project offline" page; shared services and databases untouched; a "Needs attention" card.
3. When it reappears: quick doctor check, restart what was running.
4. Moved/renamed: if a folder with the same id appears, offer **Relink**.

## Decision (2026-11-21)

Yes. A developer may keep a project on a removable disk or another drive; "Link a folder" registers it and it gets every feature a `www/` project has. Remove only unregisters, never deletes files. Recorded in [decisions/projects.md](../../context/decisions/projects.md).
