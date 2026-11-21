# Local BaaS (Firebase emulators, PocketBase) and payments sandbox

Home: Services (binaries) + Projects (per-project instances). Source: [spec](../source-specs/2026-11-21-feature-context.md) "Feature 10". Status: **not started**; **confirmed by the user** (2026-11-21): ship both, never a Firebase clone. Size: L.

## Key design point: project-scoped services

MySQL/Redis are one shared instance. Firebase emulators and PocketBase run **one instance per project** (own config/data). Add `scope: shared | project` to service manifests (phase 27, step E): the binary is installed once and versioned globally; each project runs its own instance as a process with endpoints.

## Feasibility (checked)

- **Firebase emulators:** needs Node 16+, JDK 11+, `firebase-tools` (npm). LampRabit already manages Node and Java, and manifests already support `npm` sources with integrity checks, so `firebase-tools` fits. Emulator JARs are downloaded by the CLI itself.
- **PocketBase:** single Go binary, GitHub releases with a checksums file → a manifest `source.github` entry.
- **Stripe CLI** and **stripe-mock:** GitHub releases with checksums → manifests.
- Nothing of this exists in the code yet.

## Firebase plan

1. `firebase.json` present → "Run Firebase emulators"; absent → `firebase init emulators` guided.
2. Ports: defaults (UI 4000, Hub 4400, Auth 9099, Firestore 8080, Database 9000, Storage 9199, Functions 5001, Hosting 5000, Pub/Sub 8085) clash often (8080; 5000 is AirPlay on macOS). Propose free ports and write the `emulators` block with a diff preview.
3. `firebase emulators:start --project <id> --import <dir> --export-on-exit`; offer `demo-<name>` project ids for offline use; respect `singleProjectMode`.
4. Emulator UI gets `firebase.<project>.test`; each port listed; Hosting shareable.
5. Java heap setting (`JAVA_TOOL_OPTIONS=-Xmx…`), suggested when logs show "Java heap space".
6. Binding writes `FIRESTORE_EMULATOR_HOST`, `FIREBASE_AUTH_EMULATOR_HOST`, `FIREBASE_DATABASE_EMULATOR_HOST`, `FIREBASE_STORAGE_EMULATOR_HOST`, `PUBSUB_EMULATOR_HOST`, `GCLOUD_PROJECT`; client SDK snippets shown, never inserted.
7. Phones: adb reverse of each emulator port ([mobile-dev-hub.md](mobile-dev-hub.md)).

## PocketBase plan

`pocketbase serve --http=127.0.0.1:<port> --dir=<project>/pb_data` (+ `--migrationsDir`, `--hooksDir`); detect `pb_data/`, `pb_migrations/`, `pb_hooks/`; `pb.<project>.test` with the admin UI at `/_/`; first run creates a superuser (password in the OS keychain, shown once); backups snapshot `pb_data` like SQLite backups; binding writes `POCKETBASE_URL`.

## Payments plan

- Stripe: login, `stripe listen --forward-to <endpoint>/<path>` (no tunnel), capture `whsec_…` into `STRIPE_WEBHOOK_SECRET`, one-click `stripe trigger <event>`, live event log; stripe-mock as an offline service.
- Razorpay, PayPal (no CLI): webhook catcher + Share tunnel (Tools #35) with saved sample payloads.
- Later, only with Docker: Supabase/Appwrite local.
