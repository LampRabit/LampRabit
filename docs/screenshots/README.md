# Product screenshots

`overview.png`, `overview-dark.png`, `sites.png`, `services.png`, `git-workflow.png` and `ai-assistants.png` are the images embedded in the root README. Capture the app's current layout using the fake-backend browser harness in `scripts/screenshots/`; it avoids exposing a real environment's project names, paths or credentials.

From the repository root, with Node.js dependencies installed and Edge or Chrome available:

```sh
npm run screenshots -- home home-dark sites services logs settings assistants git-changes git-history git-branches
```

The harness writes PNGs to `scripts/screenshots/out/`. The README views map to `home` â†’ `overview.png`, `home-dark` â†’ `overview-dark.png`, `sites` â†’ `sites.png`, `services` â†’ `services.png`, `git-changes` â†’ `git-workflow.png`, and `assistants` â†’ `ai-assistants.png`. Review them before copying. The fixtures are representative UI data, not screenshots of a live user's services. Do not commit unreviewed screenshots containing personal paths or tokens.

The harness also has additional states such as first-run onboarding, project Health, Git changes/history/branches and the AI assistant settings page. Query options are documented at the top of `scripts/screenshots/harness.tsx`.
