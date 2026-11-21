# WebSockets: proxy, tester and WS doctor

Home: Projects (proxy) + Tools (tester). Source: [spec](../source-specs/2026-11-21-feature-context.md) "Feature 11" and "Feature 12". Status: **proxy mostly shipped**; tester not started. Size: S (proxy) + L (tester + doctor).

## Already in LampRabit (checked in `sites.rs`)

- Apache: `ProxyPass / … upgrade=websocket`.
- Nginx: `proxy_http_version 1.1`, `Upgrade $http_upgrade`, `Connection $connection_upgrade` (with the `map`), `X-Forwarded-Host/Proto`.
- Caddy and FrankenPHP: `reverse_proxy` handles upgrades on its own.
- Cloudflare tunnels and ngrok pass WebSockets.

## Remaining proxy work (S)

1. Nginx `proxy_read_timeout` is the default 60 s, which drops idle sockets (Reverb, Socket.IO): raise it for proxied locations.
2. A process with its own WS server (Reverb, Soketi, Socket.IO, Centrifugo) gets its own subdomain (`ws.app.test`): already possible with subdomains → port; document it.
3. Vite/webpack HMR behind an HTTPS domain: detect and suggest `server.hmr` (`protocol: 'wss'`, `clientPort: 443`, `host`). Doctor check.
4. Write client env (`VITE_REVERB_HOST/PORT/SCHEME`, `PUSHER_*`) via service binding ([service-binding.md](service-binding.md)).
5. Test: after generating vhosts, perform a WS upgrade against a test endpoint.

## Tester (Tools #33–34)

Runs in Rust (tokio-tungstenite): the webview's WebSocket API can't set custom headers. Connect ws/wss with headers and subprotocols, send text/JSON/binary, timestamped log, saved messages, reconnect, ping. Modes: raw, Socket.IO/Engine.IO, Pusher protocol (Reverb/Soketi channels with an auth endpoint). Endpoints prefilled from projects.

## WS doctor

In order, stop at the first failure with an explanation and a fix: DNS → TCP → TLS trusted → upgrade returns 101 (vs 400/404/426) → proxy headers → origin rejection → idle-timeout disconnect → mixed content (ws:// from an https page) → reachable through the tunnel/LAN.
