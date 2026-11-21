# Rules for every AI agent working on LampRabit

These rules apply to every assistant: Claude Code, Codex, Copilot, Cursor, a sub-agent, or a second developer's AI. They exist because the user wants nothing they tell an agent to be lost between sessions (asked 2026-10-28 and again 2026-11-21).

## 1. Read before you work

1. Open [../README.md](../README.md), the index. It says which file holds what.
2. Read [../progress/status.md](../progress/status.md) to see where the work stands.
3. Before changing an area of the code, read that area's decision file in [../context/decisions/](../context/decisions/). Don't reverse a decision without asking the user.
4. Check `git log` and the code before trusting any note here. If a note is wrong, fix the note.

## 2. Keep every piece of context the user gives

Anything the user tells an agent that will matter later goes into `.ai/`, **in the same session, before you finish**. Don't leave it only in the chat, in an agent's private memory, or in a commit message.

| The user gives you… | Write it to |
|---|---|
| A decision or a choice between options ("use X, not Y") | The matching file in `context/decisions/`, with the date and "the user's choice" |
| A new feature idea, research notes, a pasted spec or list | `plans/backlog/<topic>.md` (pasted specs verbatim in `plans/source-specs/<date>-<name>.md`) |
| A feature to build now | `plans/active/phase-NN-<name>.md`, and a line in `plans/roadmap.md` |
| A preference about how to work, communicate or release | [../context/working-with-the-user.md](../context/working-with-the-user.md) |
| A rule for code style or testing | `instructions/conventions.md` or `instructions/checks.md` |
| A bug or trap that cost time | `instructions/pitfalls.md` (one line) |
| Bug reports from testers or the second developer | the active phase file, or `plans/backlog/` if not being fixed now |

Write the user's intent in plain words. Keep their exact wording when it carries meaning (names, numbers, priorities); fix the spelling otherwise.

## 3. Record each session

At the end of a session (or when it changes direction), add or update `sessions/<YYYY-MM-DD>-<topic>.md` with:

- what the user asked for (verbatim prompts can be exported, see [../sessions/README.md](../sessions/README.md));
- the decisions made, and which `.ai/` files you updated with them;
- what was finished, what is half-done, and the exact next step.

Then add a row to the table in [../sessions/README.md](../sessions/README.md) and update [../progress/status.md](../progress/status.md).

## 4. Keep it organised

- **One topic per file.** If a file grows past roughly 400 lines or starts covering two topics, split it and update the index.
- **Update the index.** Every new file gets a line in [../README.md](../README.md). An agent should find anything from the index alone, without searching.
- **Move, don't copy.** When a phase finishes, move its file from `plans/active/` to `plans/completed/` and tick it in the roadmap. When an idea becomes a phase, move it out of the backlog.
- **Dates are absolute** (2026-11-21, never "yesterday").
- **Nothing private.** The repository is public. No tokens, passwords, email addresses, machine paths with user names or customer data in `.ai/`.
- **No AI files at the repository root** except `CLAUDE.md` and `AGENTS.md`, which only point here. User-facing docs stay where they are: `README.md`, `CHANGELOG.md`, `CONTRIBUTING.md`, `RELEASING.md`, `SECURITY.md` and `docs/`.

## 5. Agent-private memory is not enough

Tools like Claude Code keep their own memory outside the repository. That memory may hold a short pointer, but the content belongs in `.ai/`, where other agents and developers can see it.
