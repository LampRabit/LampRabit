# Features from the 2026-11-24 issue list

Source: [../source-specs/2026-11-24-usability-issues.md](../source-specs/2026-11-24-usability-issues.md). The bugs in that list were fixed the same day ([session](../../sessions/2026-11-24-usability-fixes.md)); these three need the user's decisions first.

## 1. A default client per service

**Done 2026-11-24** as the user clarified: a default among the web tools (KarotX, phpMyAdmin, Adminer, phpPgAdmin, pgAdmin) per database, in Settings → Databases. Kept below for history.

Exists: databases (MariaDB, MySQL, PostgreSQL) have a ⋯ menu of installed web tools (phpMyAdmin, Adminer, phpPgAdmin, pgAdmin, KarotX) and remember the last one picked (`writePref("dbTool.<kind>")`). Missing: desktop apps (DBeaver, TablePlus, DataGrip, HeidiSQL, Compass, RedisInsight…), a setting to choose the default, and the same for non-database services. Questions: which desktop apps; detect installed ones automatically or let the user add any program; where the setting lives (Settings → a "Clients" section, or the service's own menu).

## 10. Public sub-domains through Cloudflare

**Built 2026-11-24** (user's answer: yes, with the Cloudflare sign-in): see [app-and-platform](../../context/decisions/app-and-platform.md). Not yet tried against a real Cloudflare account.

Exists: Cloudflare quick tunnels (random `*.trycloudflare.com`, no account) in `tunnels.rs`. Asked: a project's own public sub-domain (`shop.example.com`). That needs a named tunnel: a Cloudflare account, a domain on Cloudflare, `cloudflared tunnel login` (browser), one tunnel with an ingress rule and a DNS route per sub-domain. Questions: the user's own domain only, or a LampRabit-provided one; one tunnel for all projects; token kept in the OS keychain like ngrok's.

## 11. Self-signed signature

The user (2026-11-24): minisign, to prevent SmartScreen. Minisign doesn't affect SmartScreen; options were given (see app-and-platform). Asked again later that day as "a self-signed signature" for macOS and SmartScreen: self-signing helps neither, and macOS is already ad-hoc signed. Waiting for a choice of a trusted certificate (SignPath, Azure Trusted Signing, OV; Apple Developer ID for macOS).

Unclear which signature: (a) Windows code signing of the installer with a self-signed certificate (free, but SmartScreen still warns unless the certificate is installed on each machine), (b) the services registry index signature already planned (minisign, phase 27 D), or (c) the updater's signature (Tauri already signs updates). Ask.
