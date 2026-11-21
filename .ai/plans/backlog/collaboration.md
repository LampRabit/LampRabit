# Remote development and collaboration

Home: Remote. Source: [spec](../source-specs/2026-11-21-feature-context.md) "Feature 16". Status: **idea, drafted 2026-11-21**. Size: L (layer 1) to XL+ (layers 2–3).

The user's pain points: autosave, broken connections, several people at once, network limits.

**Honest framing:** real-time co-editing is crowded (Live Share, Code With Me, Zed, Replit) and LampRabit is not an editor. CRDTs make copies converge, not code correct; "conflict-free" mostly comes from awareness and resilient sync.

## Layer 1: resilient remote dev, single user — feasible

Edit locally in any editor; LampRabit syncs to the server in the background.
- Integrate **Mutagen** (bidirectional sync over SSH, used by DDEV and Docker Desktop) rather than writing a sync engine. Check its licence first: the core is MIT, some parts are SSPL.
- Offline queue, reconnect with backoff, status chip (Synced / Syncing 12 / Offline, 5 queued / Conflict).
- Conflicts: 3-way merge with the last synced version as base; unresolved → side-by-side resolver; never overwrite silently.
- Respect `.gitignore` + defaults (node_modules, vendor, build); install dependencies on the remote.
- Remote processes, logs and port forwards through the Remote home ([remote.md](remote.md)).

## Layer 2: shared session and presence — large, risky

Share the environment, not text: app URL (tunnel), live logs, read-only or interactive terminal, forwarded ports, doctor output, `lamprabit.toml`. Presence and soft locks via file watching ("Alex is editing UserController.php"); teammates' branches. Needs E2E encryption, expiring tokens, revoke, audit log, and NAT traversal (tunnel or WebRTC; TURN servers cost money). Treat as its own project after layer 1.

## Layer 3: co-editing — not recommended

Only as an experiment (a VS Code extension with Yjs, LampRabit as relay) and only if users ask after layers 1–2. First check whether pointing users to Live Share/Zed is enough.
