# Serverless local development

Home: Projects (+ Services for stand-ins). Source: [spec](../source-specs/2026-11-21-feature-context.md) "Feature 17" (platform table there). Status: **exploratory, drafted 2026-11-21**. Priority: medium. Size: L.

Don't replace platform CLIs; orchestrate them. Out of scope: deploying, billing, IaC editing.

## Feasibility (checked)

- The platform CLIs are npm packages (wrangler, netlify-cli, vercel, azure-functions-core-tools, firebase-tools) or GitHub releases (sam, supabase); manifests already support `npm` and GitHub sources with checksums, and Node comes from NVM.
- A function app's dev server is just a project process with an endpoint, so routing, SSL and Share already work.
- **Azurite** (Azure storage emulator) is an npm package → a normal service manifest.
- SAM, LocalStack and Supabase need Docker → only after [docker.md](docker.md) interop.

## Plan

1. Detect the platform (`wrangler.toml`/`wrangler.jsonc`, `vercel.json`, `netlify.toml`, `host.json`, `firebase.json` functions, `template.yaml` with `AWS::Serverless`, `serverless.yml`, `supabase/functions/`) → kind Serverless.
2. Run the right dev command with pinned runtimes (`wrangler dev`, `vercel dev`, `netlify dev`, `func start`); doctor checks (CLI version, Docker needed?, credentials only when required).
3. Endpoint `api.<project>.test` + Share for webhooks.
4. Local stand-ins through service binding: S3 → RustFS/MinIO, queues → RabbitMQ, KV → Redis, DB → Postgres/MySQL, Azure storage → Azurite.
5. Event trigger panel: fire saved sample events (HTTP, object created, queue message, scheduled) at local functions.
6. One env editor mapping `.dev.vars`, `local.settings.json`, `env.json`, `.env`; secrets kept out of git.
7. Unified logs tagged by function name.

Order: no-Docker platforms first (Workers, Vercel/Netlify, Firebase via [local-baas-payments.md](local-baas-payments.md), Azure Functions + Azurite), then SAM/LocalStack.
