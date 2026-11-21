# Download sources (phases 11, 15, 16, 24, 25) (checked 2026-11-01)

Where each runtime's downloads come from. Read before picking a download source for a new runtime.

FlyEnv (BSD-3) supports nearly all of these, but it downloads from its own mirror and uses Homebrew/MacPorts on macOS. That conflicts with 9.2 (official sources, pinned checksums), so FlyEnv is a reference for configs and start commands only.

| Runtime | Source | Checksum | Gaps |
|---|---|---|---|
| Caddy | GitHub caddyserver/caddy | SHA-512 file | none |
| FrankenPHP | GitHub php/frankenphp (native Windows since 1.12, 2026-03) | GitHub digest | Intel Mac |
| Java (Temurin) | api.adoptium.net/v3 | SHA-256 | none |
| Tomcat 9/10/11 | dlcdn.apache.org (archive.apache.org for older) | `.sha512` | needs Java |
| PostgreSQL 14–18 | GitHub theseus-rs/postgresql-binaries (all OS/arch, PostgreSQL licence) | check per release | replaces the EDB/own-CI idea |
| MongoDB | fastdl.mongodb.org, `downloads.mongodb.org/full.json` | SHA-256 | Linux builds are per distro |
| SQLite | sqlite.org CLI zips | SHA3-256 | none |
| ClickHouse | GitHub ClickHouse/ClickHouse | SHA-512 | no native Windows: the UI says WSL |
| Neo4j Community | dist.neo4j.org | SHA-256 | needs Java 21 |
| Meilisearch | GitHub meilisearch | GitHub digest | none |
| Typesense | dl.typesense.org | none published | no Windows |
| RustFS | GitHub rustfs/rustfs | GitHub digest | none |
| MinIO | LampRabit's CI build of the last release (`build-minio.sh`), every OS; upstream stopped Community binaries 2025-10 and archived the repo 2026-02 | built from a pinned source checksum | unmaintained upstream; revisit if it stops building (decided 2026-11-04) |
| OpenSearch / Elasticsearch | official tarballs/zips with a bundled JDK | SHA-512 | 1 GB+ RAM |
| RabbitMQ | GitHub rabbitmq-server (generic-unix, windows zip) | SHA-256 | needs Erlang; no official Linux Erlang binaries |
| Ollama | GitHub ollama | SHA-256 file | detect an existing install first |
| n8n | npm, run on LampRabit's Node | npm integrity | needs Node |
| Python | GitHub astral-sh/python-build-standalone (what uv uses) | SHA-256 | none |
| Go | go.dev/dl/?mode=json | SHA-256 | none |
| Rust | rustup (an existing `~/.cargo` is used as is) | rustup's own | none |
| Bun, Deno, uv | GitHub releases | SHA-256 | none |
| Maven / Gradle | Apache dist / services.gradle.org | SHA-512 / SHA-256 | need Java |
| conda | Miniforge | SHA-256 | heavy |
