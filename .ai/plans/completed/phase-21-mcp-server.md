# Phase 21: MCP server

Status: **done**. Listed in the [roadmap](../roadmap.md).

**Done (2026-11-12):** `lamprabit mcp` (`mcp.rs`, stdio JSON-RPC, protocol 2025-06-18): read-only tools (status, services, projects, logs with filters, doctor, git status, runtimes, templates) and annotated ones that change things (start/stop/restart a service, start/stop a project, apply a doctor fix, create a project without a user template's risky steps, install a runtime). No settings, no credentials. Then, from the app (2026-11-12): Settings → AI assistants connects Claude Desktop, Claude Code, Codex, Cursor, VS Code, GitHub Copilot CLI, Windsurf, Gemini CLI, Antigravity, LM Studio, Cline, Roo Code, Kiro and opencode where installed (`mcp_clients.rs`) and can run the server inside LampRabit over HTTP (`ports.mcp`, token, Origin check).

`lamprabit mcp` (stdio) on top of the phase 20 backend: list projects, services and their health, read filtered logs and dumps, run the doctor, start/stop, create a project from a template, git status. Read-only tools by default; tools that change things are marked so the client asks first; credentials and keys are never returned, only connection metadata.
