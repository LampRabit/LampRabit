# Contributing to LampRabit

Thanks for helping! Bug reports, ideas and pull requests are all welcome.

## Reporting bugs and ideas

- Search [existing issues](https://github.com/LampRabit/LampRabit/issues) first.
- For bugs, use the bug report template and attach the **diagnostics report**: in LampRabit, open Settings → Troubleshooting → Run diagnostics → Copy. It hides your user name and home folder.
- Security problems: please don't open a public issue; see [SECURITY.md](SECURITY.md).

## Development setup

You need [Node.js](https://nodejs.org/) 22+, [Rust](https://rustup.rs/) (stable) and the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for your OS (WebView2 on Windows, Xcode Command Line Tools on macOS, WebKitGTK on Linux).

```bash
git clone https://github.com/LampRabit/LampRabit.git
cd LampRabit
npm install
npm run tauri dev
```

The app uses `C:\lampRabit` (Windows) or `~/lampRabit` as its environment folder by default; you can change it in Settings.

### Project layout

| Path | What's there |
|---|---|
| `src/` | React + TypeScript interface (Tailwind CSS, TanStack Query, Zustand) |
| `src-tauri/src/` | Rust backend: service supervisor, runtime downloads, config generation, sites, SSL, logs |
| `docs/website/` | Feature list, installation and migration guides used by the website |
| `PLAN.md` | Roadmap and design notes |

### Checks

Run these before opening a pull request (CI runs them on Windows, macOS and Linux):

```bash
npm run build                                   # type-check and build the interface
cd src-tauri && cargo check && cargo test --lib # backend
```

## Pull requests

`main` is protected: every change goes through a pull request, and all CI checks must pass before it can be merged.

1. Fork the repository (or create a branch if you're a collaborator) and make your change on a branch.
2. Keep changes focused; one feature or fix per pull request.
3. Match the surrounding code style. Write user-facing text in plain language.
4. Describe what changed and how you tested it, and include a screenshot for interface changes.

By contributing, you agree that your contributions are licensed under the [MIT License](LICENSE).
