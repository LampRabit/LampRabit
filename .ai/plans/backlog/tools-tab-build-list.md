# Tools tab: build list

Home: Tools. Source: [spec](../source-specs/2026-11-21-feature-context.md) "Feature 6" and "Tools tab: final build list". The maintainer's earlier research and per-tool details: [tools-tab.md](tools-tab.md). Status: **not started**. This list sets the order; tools-tab.md has the detail.

**Positioning:** basic converters are commodity (DevToys, IT-Tools, CyberChef). The tab is worth it for tools that know LampRabit's projects, endpoints, services and files. Everything offline, lazy-loaded, registered in the palette (and CLI/MCP where sensible).

Size: S = hours to a day · M = a few days · L = 1–3 weeks · XL = multi-week.

## S: small utilities (pure frontend)

1. Base64 (text, image ↔ data URI) · 2. URL encode/decode, HTML entities, escape · 3. JSON/YAML/XML/TOML/SQL format + validate · 4. JSON ↔ YAML ↔ CSV ↔ TOML · 5. Timestamp converter · 6. UUID/ULID/nanoid · 7. Hashes (MD5, SHA) · 8. bcrypt/argon2 hash + verify · 9. Secret/API key/password generator (passphrases too) · 10. Case & slug, lorem ipsum · 11. Text diff · 12. Markdown preview · 13. JWT decoder · 14. Regex tester (JS, PCRE, .NET, Dart flavours) · 15. Cron builder → "add to project scheduler" · 16. QR code · 17. Color converter + WCAG contrast · 18. Gradient / shadow / radius generators.

Note on 14: only the JS flavour runs natively in the webview; PCRE/.NET/Dart need their engines (PHP's `preg_match` via the project's PHP is feasible; .NET/Dart only once those runtimes exist). Ship JS + PCRE first.

## M: context-aware

19. JWT verify with a secret from the project's `.env` · 20. JSON → TS/C#/Dart/PHP/Go types · 21. Screen eyedropper + palette generator · 22. Logo → theme (CSS vars, Tailwind, Flutter ThemeData, MUI, shadcn) · 23. Font preview & pairing · 24. Image optimizer (WebP/AVIF) + SVG optimizer · 25. App icon & favicon generator written into the project · 26. Clipboard history + smart detect (opt-in, local, off by default) · 27. Hosts file viewer/editor (LampRabit's managed block locked) · 28. `.env` manager (diff vs `.env.example`, mask secrets, warn if tracked by git) · 29. Certificate inspector + export LampRabit's CA for phones/other browsers · 30. Network diagnostics (resolve `.test`, LAN/tunnel reachability, firewall, LAN IP + QR) · 31. Responsive preview · 32. Quick load test · 33. WebSocket tester (raw).

Feasibility notes: 21 needs a native screen capture (Tauri plugin or OS API), not just the webview. 27 writes the hosts file with elevation, like `sites.rs` does today. 33 must run in Rust (tokio-tungstenite): the browser WebSocket API can't set custom headers.

## L: differentiators

34. WebSocket advanced + WS doctor ([websockets.md](websockets.md)) · 35. Webhook catcher & replay (local URL + Share tunnel) · 36. Mock API server (OpenAPI/JSON → a process with an endpoint) · 37. HTTP client lite with OpenAPI workspace (project endpoints + env prefilled, saved per project; not a Postman clone) · 38. Fake data seeder (incl. Indian locale) into a database through the existing shims · 39. Disk reclaimer (node_modules, vendor, bin/obj, build caches, package-manager caches, unused runtime versions, old backups; always a preview, delete to the Recycle Bin like project delete) · 40. Docker config builder ([docker.md](docker.md)).

## XL

41. SSH + SFTP/FTP toolkit ([remote.md](remote.md)); likely moves to the Remote home.

## From the maintainer's earlier priorities (tools-tab.md), not in the 41

Keep these in the list; the maintainer named them as daily needs on 2026-11-14:
- Rich Markdown **editor** (split view, Mermaid, export), not only a preview (extends #12).
- Full diff incl. JSON-aware and image compare (extends #11).
- Snippet library with optional GitHub Gists sync (token in the OS keychain, which LampRabit already uses for ngrok).
- SVG → React component; CSS → Tailwind; CSS blob builder; placeholder images.

## Not in Tools

Port conflicts (shipped), database client (KarotX), mail inbox (Mailpit), Stripe/Firebase (service panels, [local-baas-payments.md](local-baas-payments.md)), remote logs ([remote.md](remote.md)). Skip: Postman clone, image editor, AI chat, anything needing an online service.

## Framework

Each tool is a module (id, name, keywords, input/output), lazy-loaded (pages are already split into chunks), favourites and recent, context entry points from other panels (e.g. Mock API from a project's endpoints). Chaining output into another tool comes later.
