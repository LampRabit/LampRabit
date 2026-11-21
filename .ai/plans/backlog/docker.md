# Docker: config builder and interop

Home: Tools (builder) + Services (interop). Source: [spec](../source-specs/2026-11-21-feature-context.md) "Feature 14" and gap 7. Status: **not started** (the code only mentions Docker in diagnostics). Size: L each.

## Config builder (Tools #40): feasible, strong

`docker init` is generic; LampRabit knows the exact runtime versions, PHP extensions, framework (`facts.rs`), bound services and their versions, processes and cron schedules. Pure file generation; Docker is needed only to validate.

1. Generate, with preview and diff, never overwriting without consent: multi-stage `Dockerfile`, `docker-compose.yml` (app + services at matching image versions, volumes, healthchecks), `.dockerignore`, `.env.docker.example`.
2. Templates per framework, stored as data: Laravel (php-fpm + nginx or FrankenPHP; queue worker and scheduler as services), Next.js (standalone), Node API, Django/FastAPI (gunicorn/uvicorn), Spring Boot (layered jar), Go/Rust (static → distroless/scratch), .NET (sdk → aspnet).
3. Processes → compose services; cron → a scheduler container (supercronic).
4. Defaults: pinned tags, non-root user, no secrets baked in, healthchecks, small final images. Say clearly the output is a starting point.
5. If Docker is installed: "Build & run locally", errors through the plain-language explainer.

## Interop: feasible in part

- **Import services from `docker-compose.yml`**: map images to LampRabit services by name and version (`mysql:8.4` → MySQL 8.4, `redis:7` → Redis 7, `postgres:16` …) and offer to install/start them; unmapped images are listed, not run. Feasible without Docker.
- **Show running containers as services**: needs the Docker CLI/API; read-only status first.
- Also unlocks LocalStack, SQL Server on non-Windows, Supabase/Appwrite — only when Docker is present. LampRabit stays not-a-Docker-replacement ([decisions/project-model.md](../../context/decisions/project-model.md) guardrails).
