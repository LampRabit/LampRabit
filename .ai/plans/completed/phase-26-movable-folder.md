# Phase 26: movable environment folder

Status: **done**. Listed in the [roadmap](../roadmap.md).

**Done (2026-11-01):** `relocate.rs` and Settings → LampRabit folder → Move. Checks (inside/around the folder, clashing folders, write access, system folders, free space, FAT/exFAT, network drives, Linux noexec) through sysinfo's disk list; programs using the folder (exe or working directory inside it, from sysinfo rather than the Windows Restart Manager) with Stop it; rename on the same drive, copy-check-delete across drives, rolled back on failure; PATH and shims switched, Python venvs (pgAdmin's, projects' .venv) repointed. First-run environment-folder selection is implemented in phase 18.

Decided 2026-11-01: the LampRabit folder (bin, etc, www, data, logs…) can live anywhere the OS allows, and changing it moves everything.
- Chosen on first run (onboarding) and changeable in Settings → Environment folder → Move.
- Move: stop services and project processes, check the target, move the folders LampRabit owns plus `www/`, `data/`, `templates/` (a rename on the same drive, copy-then-verify-then-delete across drives, with progress and resume after a failure), then regenerate configs, rewrite shims, update PATH, the terminal profile and anything else that stores the path (MariaDB/MySQL data paths, Caddy storage, the CA trust stays valid since the certificate is unchanged).
- A file in use: list which program holds it (Windows Restart Manager, `lsof`/`/proc` elsewhere) with "Stop it" (ends that process) and "Retry"; nothing is half-moved, so a cancelled move leaves the old folder working.
- Refuses targets that can't work, saying why: no write access, system folders, a drive mounted `noexec` (Linux/macOS external drives often are), FAT/exFAT (no permissions or symlinks; databases fail), network drives, not enough free space.
- Folders outside the root (MinIO/RustFS storage, see phase 16) aren't moved.
