# Remote home: SSH, SFTP/FTP, deployed app logs and health

Home: Remote (new). Source: [spec](../source-specs/2026-11-21-feature-context.md) "Feature 13" and "Feature 15". Status: **not started**. Size: XL. Monitoring is **local-only**, the user's decision ([decisions/product.md](../../context/decisions/product.md)).

## Feasibility (checked against the code and decisions)

- Nothing SSH-related exists yet.
- **Recommendation: use the system OpenSSH client (`ssh`, `scp`, `sftp`) for terminals and tunnels**, as `git.rs` uses the user's own `git` CLI: `~/.ssh/config`, ProxyJump, keys, agents (OpenSSH agent on Windows, ssh-agent elsewhere) and known_hosts then just work. Windows 10+ ships OpenSSH. Same safety rules as Git: no prompts that hang, input validated, credentials never logged.
- A pure-Rust library (`russh` + an SFTP crate) is the fallback for the file browser, where a programmatic API is needed; check its Windows agent support before choosing.
- The terminal needs a PTY (`portable-pty`) + xterm.js. FTP/FTPS: `suppaftp`.
- Host keys: strict verification, show the fingerprint on first connect, never auto-accept.

## SSH/SFTP plan (Feature 13)

1. Connections from `~/.ssh/config` + saved ones (secrets in the OS keychain, already used for ngrok).
2. Terminal tabs; SFTP browser (drag-drop, edit remotely in the local editor with save-back, permissions); FTP/FTPS fallback; transfer queue with resume.
3. Tunnels: local (`-L` remote MySQL → `localhost:13306`), remote, SOCKS; saved tunnels that start with a project; usable in service binding.
4. Integrations that make it worth it: **pull a remote database into a local one** (`mysqldump`/`pg_dump` over SSH → existing restore, anonymising later); **deploy a folder** (rsync over SSH when present, SFTP fallback, dry-run diff first); remote log tail.
5. Don't chase Termius/MobaXterm/WinSCP parity.

## Deployed apps plan (Feature 15)

1. Live remote logs over SSH (`tail -f`, `journalctl -fu`, `docker logs -f`, `pm2 logs`) in the same log viewer and explanations as local logs.
2. Server snapshot: CPU/RAM/disk, `docker ps`/`stats`, systemd status, open ports.
3. Health pings on deployed URLs **only while LampRabit runs**, labelled so.
4. On wake/start: backfill missed logs (`journalctl --since`, `docker logs --since`, saved file offsets) and mark the gap ("Not monitored 23:10–07:45, logs backfilled"); pings show as a gap.
5. Read status from Uptime Kuma / Sentry / GlitchTip if the user has them; no own always-on agent unless demand is clear.
