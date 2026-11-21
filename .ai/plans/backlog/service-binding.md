# One-click service binding to projects

Home: Projects. Source: [spec](../source-specs/2026-11-21-feature-context.md) "Feature 9". Status: **partly built**. Size: L.

"Add Redis / Mail / Storage / Search / Queue to this project" from the project page.

## Already in LampRabit

- Creating a project can create its database and write `.env` lines (templates list their `engines:`).
- Each service manifest has `connection` details (`ServiceInfo.connection`), shown as URL or `.env` lines with copy buttons.
- The Dependencies tab can install packages through `dependency_plan` → confirm → run.
- The doctor applies previewed fixes.

## Feasibility per isolation step

| Service | Per-project resource | How |
|---|---|---|
| MariaDB/MySQL/PostgreSQL/MongoDB | database (+ user) | existing code paths for project creation |
| Redis | DB index or key prefix | config only |
| MinIO / RustFS | bucket + scoped key | S3/admin API over HTTP (needs request signing) — medium |
| RabbitMQ | vhost + user | management HTTP API (needs the management plugin on) |
| Meilisearch / Typesense | index prefix / scoped API key | HTTP API |
| Mailpit | shared inbox, tagged per project | SMTP username or a header; Mailpit supports tags |

## Plan

1. Flow: pick service → ensure installed and running → create the resource → write config → **diff preview** → apply (backup kept) → optional "test connection".
2. Framework adapters as data in service manifests (`connection` grows per-framework keys): Laravel (`REDIS_*`, `CACHE_STORE`, `QUEUE_CONNECTION`, `MAIL_*`, `FILESYSTEM_DISK=s3` + `AWS_ENDPOINT` + path-style, `SCOUT_DRIVER`), Node/Next (`REDIS_URL`, `SMTP_*`, `S3_*`), Django/FastAPI, Spring Boot (`application-local.properties`), .NET (`appsettings.Development.json`/user-secrets), Rails, Go/Rust `.env`.
3. Rules: edit only `.env`/dev-only config; never edit source code silently (show a snippet); mark lines `# managed by LampRabit`; record the binding in `lamprabit.toml`; "Unbind" reverses it.
4. Offer the matching package (`predis`, `ioredis`, `league/flysystem-aws-s3-v3`) through the Dependencies tab, previewed.

## Depends on

Phase 27 manifests (adapters live there); [project-model.md](project-model.md) for `services[]` on the project.
