# Session 2026-11-21: AI context folder and new feature plans (handoff)

## What the user asked

1. Create `.ai/` for all AI context (instructions, context, plans, progress), export the previous sessions into it, move the planning files out of the root, add a rule that every piece of context given to any agent is kept in the project, and keep it organised with an index instead of one bloated file.
2. Drop the developer feed and "just for fun" (JFF) Playground ideas.
3. Add a new feature-planning document (exported from another LLM, `LAMPRABIT_FEATURE_CONTEXT.md`) to the plans, checking that each feature is feasible and not already in LampRabit.

## Decisions recorded

- [context/decisions/project-model.md](../context/decisions/project-model.md): not web-only; Project → Processes → Endpoints; kinds as UI presets.
- [context/decisions/ux-principles.md](../context/decisions/ux-principles.md): five homes (no Runtimes tab), palette, definition of done.
- [context/decisions/product.md](../context/decisions/product.md): dropped feed/Playground; homes; remote monitoring local-only with log backfill; Firebase emulators + PocketBase confirmed; manifests stay YAML.

## What the spec got wrong or already had (checked in the code)

- Share via Cloudflare/ngrok: shipped (`tunnels.rs`); only follow-ups remain.
- WebSocket proxying: shipped on Apache, Nginx and Caddy; only the Nginx idle timeout, HMR hints and a test remain.
- Service manifests: already phase 27 in YAML; the TOML suggestion was not taken.
- "JFF tab" listed as queued: dropped this session.
- Projects are only `www/` folders and imports copy on purpose, so "projects anywhere" needs a decision.
- No command palette, .NET, Flutter, adb, SSH, Docker, Firebase, PocketBase or Stripe code exists.

## Done

- `.ai/` created with index, rules, decisions per area, roadmap split into phase files, backlog, progress, sessions, export script. Root `CLAUDE.md`/`AGENTS.md` point to it.
- The spec saved verbatim in `plans/source-specs/2026-11-21-feature-context.md`; 15 backlog plans written; phase 27 gained the spec's manifest fields; the roadmap has a "Proposed next" order.
- Nothing committed yet (all staged on `dev-1.0`).

## Later in the session

- The user answered: linked projects from any folder/drive are wanted and get every feature ([decisions/projects.md](../context/decisions/projects.md)); service files stay YAML; the proposed order is confirmed.
- Phase 27 **B3 done** (uncommitted): services and runtimes that exist only as a manifest (`ServiceKind::Plugin`, `Runtime::Plugin`, `Ports.extra`). Details in the phase file.

- Committed: `cdc9952` (.ai/), `981d67d` (B3).
- Phase 27 **C done** (uncommitted): local service files, Add/Remove a service on the Services page, `lamprabit service check|try`. Also fixed a crash B3 introduced: `Services.tsx` read `runtime` before its `const` declaration (inside a callback, so tsc missed it), which would have broken the Services page.

## Next step

Phase 27 step D: the LampRabit/Services registry repository, the signed `index.json` and the Community section on the Services page. Needs the user: creating the repository and the signing key.
