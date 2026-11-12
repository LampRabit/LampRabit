# The `lamprabit` command line and MCP server

`lamprabit` controls LampRabit from a terminal, a script or an AI assistant. It's in LampRabit's own terminal (the title bar's terminal button), and in every terminal once **Settings → Terminal → Add LampRabit's commands to PATH** is on.

It talks to the running app, so everything goes through the same code as the buttons. When LampRabit isn't running, the command starts it in the background (in the tray, no window) and waits for it; `--no-launch` fails instead.

## Commands

| Command | What it does |
|---|---|
| `lamprabit status` | Version, folder, installed services with state and port, number of projects |
| `lamprabit services` | Every service and its state |
| `lamprabit service start\|stop\|restart <id>` | One service (`apache`, `nginx`, `mariadb`, `mysql`, `postgres`, `redis`, `mailpit`…) |
| `lamprabit start [<profile>]` | Everything installed, or a stack profile |
| `lamprabit stop` | Every service |
| `lamprabit profiles` | Stack profiles |
| `lamprabit projects` | Projects in `www/` with their `.test` URLs |
| `lamprabit project start\|stop <name>` | A project's dev server |
| `lamprabit project open <name>` | Its site, in the browser |
| `lamprabit project create <name> --template <id> [--php <v>] [--database <db>]` | A new project from a template (`lamprabit templates` lists them) |
| `lamprabit runtimes [--available <id>]` | Installed runtimes, or what `<id>` can install |
| `lamprabit runtime install <id> [version]` | Download (checksum checked) and use it; a line like `8.4` picks its newest |
| `lamprabit runtime use <id> <version>` | Switch the version in use |
| `lamprabit runtime remove <id> <version> --yes` | Delete a version |
| `lamprabit logs [--source <id>] [--lines N] [--follow]` | Recent log lines; the source is a service id, `php`, `system` or `project:<name>` |
| `lamprabit doctor <project> [--fix <id> \| --fix-all] [--dry-run]` | The project doctor; `--dry-run` shows what the fixes would do |
| `lamprabit git <project>` | Branch, upstream, changed files |
| `lamprabit config [key.path]` | The settings, or one value (`lamprabit config ports.http`) |
| `lamprabit mcp` | The MCP server on stdin/stdout |

Options: `--json` prints the answer as JSON; `--yes` confirms a destructive action; `--dry-run` shows a change without making it; `--no-launch` doesn't start LampRabit.

Exit codes: `0` done, `1` the request failed (the message says why), `2` usage, `3` LampRabit couldn't be reached, `4` `doctor` found errors (handy in scripts: `lamprabit doctor shop || exit 1`).

## MCP for AI assistants

No terminal needed: **Settings → AI assistants** does it from the app.

- **Connect** adds LampRabit to Claude Desktop, Claude Code, Cursor, VS Code or Windsurf: it writes a `lamprabit` entry into that assistant's MCP config (keeping everything else, in order, and backing the file up the first time); **Disconnect** removes it. The assistant then starts LampRabit's MCP server itself, with LampRabit open or not.
- **Run the MCP server in LampRabit** serves it over HTTP at `http://127.0.0.1:39813/mcp` (Settings → Ports → MCP server) for assistants that connect by URL. Only this computer can reach it; requests need the token shown there (`Authorization: Bearer …`), and requests from web pages are refused. The section shows the `claude mcp add` command and the JSON to copy.

By hand, `lamprabit mcp` is the same server over stdio, for example:

```json
{ "mcpServers": { "lamprabit": { "command": "C:\\lampRabit\\shims\\lamprabit.cmd", "args": ["mcp"] } } }
```

Tools that only read: `status`, `list_services`, `list_projects`, `read_logs`, `project_doctor`, `git_status`, `list_runtimes`, `list_templates`.

Tools that change something, which clients ask about first (`readOnlyHint: false`; stopping and fixing are also `destructiveHint`): `start_service`, `stop_service`, `restart_service`, `start_project`, `stop_project`, `fix_project_issue`, `create_project` (a user template's risky steps are refused), `install_runtime`.

The settings aren't exposed, and nothing returns a password or token: local services have none, and Git remotes come back with credentials stripped.

## How it works

The app listens on `127.0.0.1` only, on a free port, and writes the port and a random token to `control.json` in its config folder (readable by the user only); requests without the token are refused. Each connection carries one JSON line, `{"token","cmd","args"}`, answered by one line `{"ok":true,"data":…}` or `{"ok":false,"error":"…"}`; `logs.follow` streams `{"line":…}` lines. See `src-tauri/src/control.rs`, `cli.rs` and `mcp.rs`.
