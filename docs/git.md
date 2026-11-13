# Project Git workflow

Open a project in **Sites** and select its **Git** tab. LampRabit uses Git in that project directory and keeps credentials in Git's normal credential helpers or SSH agent. It strips credentials from remote URLs and error messages shown in the app.

The Git panel supports:

- Review changed files and diffs; stage or unstage files and individual hunks; discard changes.
- Commit staged changes or all changes, amend the previous commit, and inspect commit details and history.
- Create and switch branches (including remote branches), merge branches, and continue or abort an in-progress merge or rebase.
- Fetch, pull and push, with progress and a cancel action for network operations. The first push can set the upstream.
- Manage remotes, tags and stashes; revert a commit; restore a file; resolve conflicts.
- Set the repository's local author name and email, initialize a repository, or clone a repository from Sites.

Git identity is configured per repository, so LampRabit does not silently change your global Git settings. On Windows, LampRabit can install its Git runtime (MinGit); on macOS and Linux it uses the system Git installation.

The terminal command `lamprabit git <project>` prints a project's branch, upstream and changed files. The MCP `git_status` tool provides read-only status to connected assistants.
