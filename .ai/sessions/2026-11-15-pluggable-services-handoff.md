# Session 2026-11-15: pluggable services (handoff)

The previous working session before the `.ai/` reorganisation. Session id `77567282`; the verbatim prompts are in [2026-11-15-77567282.md](2026-11-15-77567282.md).

## What the user asked

1. "Our stable release is working perfectly, now make it a beast": turn services into **pluggable services**. Built-in services ship with the app; users can build their own and contribute them to a community `LampRabit/Services` repository (YAML file + PR, merged by the user), listed on a marketplace page on the website. Review every current service and design a solid YAML structure.
2. Start branch `dev-1.0`, and:
   - manifests find newer software versions themselves, so software updates and service updates are independent;
   - move **all** built-in services into YAML;
   - the user is the only signer of services.
   After this: "tools and plugins", different from services, like Chrome extensions.
3. The user questioned whether the YAML was only informational (`origin` looked like a plain string). Answer: `origin` is display text; `source` drives downloads. They asked to finish phase B so the hard-coded parts are washed out.
4. The user asked that service options be declared in the manifest's `settings:` and appear in that service's Settings card automatically.

## Decisions

Recorded in [../context/decisions/services-manifests.md](../context/decisions/services-manifests.md).

## Done (committed on `dev-1.0` as `f18e816`)

- **A. Engine:** `src-tauri/src/manifest.rs` (strict parsing, community-manifest safety checks, placeholders, generic GitHub catalog with a mandatory digest) and 24 manifests in `src-tauri/services/`. 15 declarative services plus MariaDB/MySQL launch from YAML; their Rust launch code and `configs::write_*` writers were removed after golden tests matched command lines and config files on Windows and Linux. About 1,250 lines removed. `connections.ts`'s table replaced by `ServiceInfo.connection`.
- **B1. Downloads from YAML:** version strategies, tag suffixes, vendor URLs with SHA files, npm, LampRabit builds, per-platform sources. 17 of 24 services download from their manifest; 11 hand-written catalogs deleted. Still in Rust: MariaDB, MySQL, MongoDB, Tomcat, pgAdmin, Apache/Nginx on Windows (a test pins that list). Releases without a checksum are left out. Network test: `runtimes::tests::live_manifest_catalogs` (`--ignored`).
- **B2. Settings:** `settings:` (text, number, toggle, choice) in the ServiceSettings card, stored in `Settings.service_options` (only non-defaults), `{{setting.x}}` and `if: setting.x`, restart on Apply. Used by Memcached, Redis, Mailpit, Elasticsearch, OpenSearch.
- Behaviour changes: Mailpit/Meilisearch/RustFS/Ollama refuse releases without a GitHub checksum; n8n is now checked against npm's SHA-512; Typesense stays `checksum: none` (it publishes none).
- Checks at the time: 144 backend tests, 83 frontend tests, tsc, build, screenshots; also on Linux (WSL). macOS left to CI.

## Next step

**B3:** `ServiceKind::Plugin(id)` so a service needs no enum variant; `Ports.extra` (`<id>.<port>`) for non-built-in services' ports; `Release.runtime` by id; install/remove for manifest-only services; port fields and labels on the Services page from the manifest (`settingsFields::portsFor` is still a table). Then C (local service files), D (registry and signing), E (wider format). Details in [../plans/active/phase-27-pluggable-services.md](../plans/active/phase-27-pluggable-services.md).
