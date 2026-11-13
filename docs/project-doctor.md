# Project doctor

The Health panel on a project's Overview checks whether the project is ready to run. Open **Sites**, select a project, and review its Health panel. Each finding explains the issue and why it matters. When LampRabit can make a change, inspect the preview and choose **Fix**.

Checks cover:

- Web server installation, service state, domain/hosts entries, and stale or invalid site configuration.
- PHP version constraints from `composer.json` and required PHP extensions.
- Composer and npm dependencies that are missing or out of date compared with their lockfiles.
- `.env` presence when `.env.example` exists, and Laravel's `APP_URL`.
- Database runtime, server state and project database availability.
- Local certificate trust and Git author identity.

Available fixes use LampRabit's project and service actions: select a compatible PHP version, install or enable an extension, install dependencies, copy the example environment file, start a service, create or switch a database, correct `APP_URL`, apply site configuration, or trust the local certificate authority. LampRabit reruns the checks after a fix.

You can also run the doctor from a terminal:

```sh
lamprabit doctor my-project
lamprabit doctor my-project --json
lamprabit doctor my-project --fix-all --dry-run
lamprabit doctor my-project --fix <issue-id>
```

`--dry-run` previews changes. `--fix-all` applies available fixes after confirmation; `--yes` is useful for automation. The command returns exit code `4` when the project still has errors, which makes it useful in scripts and CI-style checks.

For AI clients, the MCP `project_doctor` tool reads findings; `fix_project_issue` applies a selected fix after the client requests confirmation.
