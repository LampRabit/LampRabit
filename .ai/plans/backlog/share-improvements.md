# Share a site: improvements

Home: Projects (per endpoint). Source: [spec](../source-specs/2026-11-21-feature-context.md) "Feature 2" and gap 8. Status: **shipped**; these are follow-ups only. Size: S–M each.

## Already in LampRabit

`tunnels.rs` + `components/Tunnels.tsx`: Cloudflare quick tunnels (no account) and ngrok (authtoken in the OS keychain), requests forwarded with the site's own Host header so vhosts work, per site or any local port, stopped on exit. CLI/MCP control exists through `control_command`.

## Possible follow-ups (feasibility checked)

1. **QR code** for the public URL: easy (a small QR library). Also used by LAN testing ([mobile-dev-hub.md](mobile-dev-hub.md)).
2. **Base-URL warning:** when a shared Laravel/WordPress/ASP.NET project hardcodes its URL (`APP_URL`, `siteurl`, OAuth callbacks), show a framework-specific hint. Easy; `facts.rs` already reads `.env` keys.
3. **Auto-expiry:** stop a share after N minutes. Easy (a timer).
4. **"Exposed" indicator** on Home and the project card while a share is up. Easy.
5. **Password:** ngrok has a native basic-auth option (check the exact flag on the installed version). Cloudflare quick tunnels have none; LampRabit would have to require a password on requests arriving through the tunnel (cloudflared adds `Cf-*` headers). Medium, needs design; do last.
6. **Shares attach to endpoints**, not sites, once [project-model.md](project-model.md) lands, so any process with an http endpoint (.NET, Dart, Node) can be shared.
