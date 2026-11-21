# Sessions

Every working session with an AI agent, newest first. Two kinds of files:

- `<date>-<short id>.md`: the user's prompts, **verbatim**, exported from the transcripts with [../tools/export-prompts.py](../tools/export-prompts.py). Raw material: the decisions taken from them already live in `context/decisions/`, so read these only to check what exactly was asked.
- `<date>-<topic>.md`: a handoff note written by the agent (asks, decisions, done, next step). Write one at the end of each session ([../instructions/agent-rules.md](../instructions/agent-rules.md) §3).

| Date | Prompts | Handoff | Topic |
|---|---|---|---|
| 2026-11-21 | (this session) | [handoff](2026-11-21-ai-context-and-feature-plans.md) | `.ai/` folder; feed/Playground dropped; 2026-11-21 feature context filed into plans |
| 2026-11-15 | [77567282](2026-11-15-77567282.md) | [handoff](2026-11-15-pluggable-services-handoff.md) | Pluggable services: YAML manifests, phase 27 A/B1/B2 on `dev-1.0` |
| 2026-11-14 | [5f567e1a](2026-11-14-5f567e1a.md) | — | Phases 23/25, pre-release review and refactors, Mac testing, 0.6.0 released as Latest |
| 2026-11-14 | [1da92e5f](2026-11-14-1da92e5f.md) | — | Pre-release for macOS testing |
| 2026-11-13 | [68284ea5](2026-11-13-68284ea5.md) | — | Floating tab bar, MCP with Claude Desktop and other assistants, doctor for every language, setup guide with imports, template editor |
| 2026-11-12 | [68016aa3](2026-11-12-68016aa3.md) | — | Phases 11/13/17/18, testers' findings, Git panel redesign, macOS smoke tests, movable tab bar, MCP from the app |
| 2026-11-05 | [bd640a1d](2026-11-05-bd640a1d.md) | — | Project card tabs, domains, Commands and processes, run history, logs, MinIO workflow |
| 2026-11-04 | [db5108d9](2026-11-04-db5108d9.md) | — | Bug fixes, service dependencies, auto-restart, smoke tests, extra domains, project header |
| 2026-11-01 | [1dac5d03](2026-11-01-1dac5d03.md) | — | New services and languages decided; Caddy, Tomcat, PostgreSQL, MongoDB, n8n; movable folder |
| 2026-10-29 | [a33526d6](2026-10-29-a33526d6.md) | — | LLM product spec → phases 10+, Redis/Memcached builds, uninstall, project templates, Node projects, port routing, 0.2 |
| 2026-10-28 | [0f1cfc97](2026-10-28-0f1cfc97.md) | — | Single instance, Composer, Linux fixes, stack profiles, MySQL, dashboard, phpRedisAdmin |
| 2026-10-24 | [52709a7b](2026-10-24-52709a7b.md) | — | Project start: plan, UI, phases 0–8, first release, website, repository rename |

## Exporting a session's prompts

```sh
python .ai/tools/export-prompts.py ~/.claude/projects/<project-slug> .ai/sessions <current-session-id>
```

Then add a row above. Other assistants: paste the user's prompts into a `<date>-<short id>.md` file in the same format.
