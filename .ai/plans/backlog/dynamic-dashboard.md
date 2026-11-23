# Dynamic dashboard (Home)

Home: Home page (`src/pages/Overview.tsx`). Source: [spec](../source-specs/2026-11-21-feature-context.md) "Feature 7". Status: **redesign of an existing page**. Size: L.

Goal: in 3 seconds the user knows what's running, what's broken and what to do next.

## Already in LampRabit

Home shows stack profiles, services with CPU/RAM sparklines, shortcuts, connection strings, recent projects and activity. Testers found it does too much at once (2026-11-12 session). Backend events already exist (`routes-changed`, `process-crashed`, service status).

## Feasibility

Feasible. "Adaptive" = compute what to show from state already in the frontend store; "live" = existing Tauri events, no per-widget polling. Don't build a free-form grid.

## Plan

1. **Built 2026-11-23** (the user picked this over a live stack map): status strip (`components/HomeStatus.tsx`): status light + "All good"/"N issues", services running/installed, projects (running, shared), memory with CPU and a per-service share bar.
2. **Built 2026-11-23, first part:** "Needs attention" (only when non-empty) with fixes for a service that stopped answering (Restart, Logs), a failed start (Show fixes opens the row, Try again), a port another program holds (Stop it with a second click, Change port) and a shared URL that went down. Still to add: doctor failure, missing runtime for a project, failed cron run, certificate near expiry. Screenshot: `home-issues`.
3. Projects: running and recent first; card content by endpoints/kinds (URL/Open/Share; Run/Test/Build for CLI/packages; up/down + restarts for workers).
4. Contextual cards only when relevant (tunnel active → URL + QR + Stop; new Mailpit mail; next scheduled job).
5. Resources (existing sparklines), then quick actions and pinned tools ranked by use.
6. Widget registry (`relevance(state) → number | null`), user pins/hides override; layout saved locally.
7. First run: an onboarding checklist until the first project exists.

## Depends on

[project-model.md](project-model.md) for endpoint-aware cards; [command-palette.md](command-palette.md) for the action ranking.
