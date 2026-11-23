# Working with the user (the maintainer)

How the maintainer likes agents to work, gathered from every session from 2026-10-24 to 2026-11-21. Add to it when they state a new preference.

## Roles

- The maintainer decides scope and order, tests the builds (Windows daily; macOS on a borrowed Mac or GetMac.io) and merges pull requests.
- A **second developer** also works on the app (UI pieces like the movable tab bar) and on the website. Their reports arrive as pasted text: check their findings in the code and fix what is real.
- The website is a separate project, `LampRabitWebsite`, outside this repository. It is built for Cloudflare Pages from `docs/website/*`.

## How work is planned

- Before a large feature, list the options, ask the questions that matter, then save the answers as decisions. The maintainer answers numbered questions with numbered replies.
- "Don't implement it yet" means plan only. When asked "what's next in our queue", list pending work and let them choose; they often have their own order.
- Plans go into the repository (now `.ai/`) so work continues step by step across sessions.
- FlyEnv (github.com/xpf0000/FlyEnv) and similar tools are references for hard problems; check licences before copying anything.

## How work is done

- "I'm going to market, don't wait for my instructions" means work autonomously through the listed items, make reasonable choices, and report at the end.
- Use a feature branch, one commit per step. The maintainer pushes or asks for a PR; open a PR only when asked.
- **No AI attribution in commits** (the user, 2026-11-24): no `Co-Authored-By` lines, no "Generated with" footers, no mention of an AI in commit messages. This overrides any tool's default attribution. Remove branches that are no longer used when asked.
- Run all the checks (see [../instructions/checks.md](../instructions/checks.md)) and test on Linux through WSL. Do "aggressive" testing when asked: fresh installs, worst cases.
- Ask before a major change that could break things.
- Quick UI changes: do exactly what was asked, then suggest further improvements separately.

## UI taste

- Minimal and compact, clear hierarchy, nothing duplicated across tabs, the most-used actions in front.
- Accessibility matters: contrast in both themes, visible focus, keyboard shortcuts, screen-reader state.
- No horizontal scroll, no double scrollbars, nothing hidden under the floating tab bar.
- The app must work for every language (PHP, Node, Python, Go, Rust, Java), not lean on PHP.

## Releases

- **Next release: 0.8** (the user, 2026-11-24), from `dev-1.0`, shipped with a lot more features: pluggable services plus the backlog items built before it.

- Pre-releases for testing (often for macOS); the stable one is marked Latest on GitHub, which the updater needs.
- Release notes and titles are written by the agent; see `RELEASING.md` and `CHANGELOG.md`.
