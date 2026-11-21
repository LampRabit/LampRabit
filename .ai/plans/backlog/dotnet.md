# ASP.NET Core and .NET support

Home: Services (runtime) + Projects. Source: [spec](../source-specs/2026-11-21-feature-context.md) "Feature 1". Status: **not started** (the only `dotnet` in the code is the removed Garnet's leftover cleanup). Size: L.

**Competition:** ServBay already supports .NET; Laragon, XAMPP, MAMP, Herd don't. Differentiators: free/open source, Windows polish, workflow depth (launchSettings, `dotnet watch`, user-secrets, EF Core).

## Scope

In: modern .NET (6, 8, 9, 10+) on Kestrel, every OS. Out for v1: classic ASP.NET Framework 4.x (needs IIS Express or Mono).

## Feasibility (checked against the code)

Feasible and fits existing pieces:
- **Runtime:** Microsoft's `releases-index.json` / per-channel `releases.json` list every SDK with a SHA-512 per file, so it can be a normal runtime with a checked archive download (prefer this over running `dotnet-install.ps1`, which runs a downloaded script). Shims for `dotnet`; `DOTNET_ROOT` per process.
- **Serving:** a .NET web app is a project process with a port; existing port routing already proxies `<name>.test` with LampRabit's CA, WebSocket upgrade included (Apache `upgrade=websocket`, Nginx `Upgrade`/`Connection`, Caddy). Kestrel stays plain HTTP on 127.0.0.1.
- **Proxy-awareness:** set `ASPNETCORE_FORWARDEDHEADERS_ENABLED=true` by default (no code change in the app); Nginx already sends `X-Forwarded-Proto`/`X-Forwarded-Host`. Check Apache and Caddy send `X-Forwarded-Proto` too.
- **Stopping:** `dotnet watch` spawns children; `procgroup` (Job Objects / process groups) already kills trees. A Ctrl+C first is nicer but optional.

## Plan

1. Runtime `dotnet` (SDK versions, checked downloads, shims), honouring `global.json`.
2. Detection: `*.csproj` with `Microsoft.NET.Sdk.Web` → web (kind Web/API); `.sln` → pick the startup project; `Microsoft.NET.Sdk.Worker` → worker; `<TargetFramework>` → offer the missing SDK; `Properties/launchSettings.json` → profiles and env.
3. Commands: `dotnet watch run` (default), `dotnet run`, `dotnet <App>.dll`; env `ASPNETCORE_URLS=http://127.0.0.1:<port>`, `ASPNETCORE_ENVIRONMENT=Development`, `DOTNET_WATCH_SUPPRESS_LAUNCH_BROWSER=1`.
4. Doctor checks: SDK present for the target framework, redirect loop hint (`UseHttpsRedirection` behind the proxy).
5. Templates via `dotnet new` (webapi, mvc, blazor, worker).
6. Later: EF Core panel (`dotnet ef migrations list`, `database update`), user-secrets viewer, connection strings in `appsettings.json` format.
7. Later: SQL Server LocalDB on Windows (`sqllocaldb`); other OSes only through Docker ([docker.md](docker.md)).

## Open questions

- Answered by the code: the reverse proxy is whichever web server is primary (Apache, Nginx or Caddy); TLD is `.test` with `.localhost` as an option.
- Download on demand (recommended; SDKs are ~200 MB).
