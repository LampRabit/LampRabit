# Phase 22: Git

Status: **done**. Listed in the [roadmap](../roadmap.md).

**Done (2026-11-12):** `git.rs` and the project card's Git tab: changes with diffs, stage/unstage/discard, commit and amend, history with diffs, branches (create, switch incl. remote ones, merge, rename, delete), fetch/pull/push (first push sets the upstream), merge/rebase in progress with Continue/Abort, remotes, the repository's own identity, init, and Clone on Sites. Never prompts on a terminal; credentials are stripped from URLs and messages. Since (2026-11-12): editor-style diffs, commit all, next-step actions, commit details, HTTPS/SSH remotes with push URLs and sync settings, progress and Cancel for network operations, stash, hunk staging, conflict resolution, tags, revert and restore, and Git itself as a runtime (MinGit on Windows). Phase 22 is complete.

Decided 2026-11-01: Git installed by LampRabit plus a Git panel per project; no local Git server (Forgejo). Windows: MinGit/portable Git for Windows from GitHub (SHA-256) under `bin/git`. Linux/macOS: the system git. When it's missing, LampRabit installs it the easiest way the OS offers: `xcode-select --install` on macOS (Apple's own install dialog), and on Linux the distro's package (`apt`/`dnf`/`pacman`/`zypper`, detected) through pkexec, showing the command first. If neither works, it links the official download page. A `git` shim like the others.

Through the `git` CLI, so the user's credential helpers and SSH keys work: status, stage/unstage, commit, branches (create, switch, rename, delete), history with diffs, fetch/pull/push, merge conflicts listed, clone into `www/`, remotes. Per-repository identity editor (`git config --local user.name/email`), never changing global config unless asked.
