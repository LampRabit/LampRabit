# Phase 16: more services

Status: **done**. Listed in the [roadmap](../roadmap.md).

**Done (2026-11-01), batch 1:** Meilisearch (Community builds, development mode), Typesense (dl.typesense.org, Linux/macOS, fixed local API key), RustFS (S3 on 9100, console 9101; `Settings.storage` puts its buckets anywhere, outside the LampRabit folder too) and Ollama (its own builds incl. Linux .tar.zst through `ruzstd`; models in data/ollama; an Ollama installed elsewhere is pointed out).

**Batch 2 (same day):** Elasticsearch 8/9 and OpenSearch 2/3 (bundled Java, ES_PATH_CONF / OPENSEARCH_PATH_CONF in etc/, single node, security off, 512 MB heap; OpenSearch has no macOS build), n8n (npm install into bin/n8n on NVM's Node 24+, editor at n8n.test), RabbitMQ 4 with its management console (Erlang: official zip on Windows, the system's elsewhere), MinIO's last release built from source in `binaries.yml` (`build-minio.sh`, Go 1.24, AGPL source linked, labelled unmaintained). Phase 16 is complete; MinIO 2025.10.15 built and published for all five targets on 2026-11-05 (run 42081376875).

Same standards as existing runtimes: catalog per OS/arch with checksums, data under `data/`, health probe, logs, ports in Settings, connection details on the Overview, `.test` domain for web UIs, data kept on upgrade/remove. Resource needs (RAM, Java, Erlang) shown before installing. Rough order, cheapest first:
1. Meilisearch, Typesense (single binaries; Typesense has no Windows build)
2. RustFS and MinIO, both S3-compatible (decided 2026-11-01: offer both). RustFS from its GitHub releases, console at `rustfs.test`. MinIO stopped publishing binaries in 2025 and was archived in 2026, so it comes from LampRabit's CI build of its last release (a Go build in `binaries.yml`, AGPL source linked), labelled "no longer maintained upstream, no security fixes" in the UI. Each keeps its objects in `data/<name>` by default; a separate storage-folder setting per service can put them anywhere (another drive, a NAS), independent of the LampRabit folder. Moving it offers to move the existing objects, and an outside folder is never deleted on uninstall.
3. n8n (an npm package run on LampRabit's Node; data in `data/n8n`; needs Node installed first)
4. OpenSearch / Elasticsearch (bundled JDK, 1 GB+ RAM)
5. RabbitMQ (needs Erlang; Windows support to be checked)
6. Ollama (detect an existing install first; often installed by the user)
