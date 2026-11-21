import type { EnvPaths, Runtime, Server, Settings } from "./api";

type Os = EnvPaths["os"];
// `services` holds every service's ports, listed by the catalog from their manifests.
export type PortKey = Exclude<keyof Settings["ports"], "services">;

/**
 * LampRabit's own ports (web servers, PHP, dumps, MCP); services' ports come from their
 * manifests through the catalog. `only` limits a field to the platforms where it does anything;
 * `server` to while that web server runs next to the primary one (`admin`: whenever it's
 * enabled). `runtime` is the service whose details show the field on the Services page.
 */
export const portFields: { key: PortKey; label: string; hint: string; runtime?: Runtime; only?: Os[]; server?: Server; admin?: boolean }[] = [
  { key: "http", label: "HTTP", hint: "Primary web server" },
  { key: "https", label: "HTTPS", hint: "Primary web server, SSL" },
  { key: "apacheHttp", label: "Apache HTTP", hint: "Apache next to the primary server", server: "apache" },
  { key: "apacheHttps", label: "Apache HTTPS", hint: "Apache next to the primary server", server: "apache" },
  { key: "nginxHttp", label: "Nginx HTTP", hint: "Nginx next to the primary server", server: "nginx" },
  { key: "nginxHttps", label: "Nginx HTTPS", hint: "Nginx next to the primary server", server: "nginx" },
  { key: "caddyHttp", label: "Caddy HTTP", hint: "Caddy next to the primary server", server: "caddy" },
  { key: "caddyHttps", label: "Caddy HTTPS", hint: "Caddy next to the primary server", server: "caddy" },
  { key: "caddyAdmin", label: "Caddy admin", hint: "Local API for reloads, 127.0.0.1 only", server: "caddy", admin: true },
  { key: "frankenphpHttp", label: "FrankenPHP HTTP", hint: "FrankenPHP next to the primary server", server: "frankenphp" },
  { key: "frankenphpHttps", label: "FrankenPHP HTTPS", hint: "FrankenPHP next to the primary server", server: "frankenphp" },
  { key: "frankenphpAdmin", label: "FrankenPHP admin", hint: "Local API for reloads, 127.0.0.1 only", server: "frankenphp", admin: true },
  { key: "phpFpm", label: "PHP FastCGI", hint: "First of 10 ports per PHP version", runtime: "php" },
  { key: "dumps", label: "Dumps", hint: "Where dump() sends dumps for Logs, while catching is on", runtime: "php" },
  { key: "mcp", label: "MCP server", hint: "LampRabit's MCP server over HTTP, 127.0.0.1 only; applies when it's next switched on" },
];


export const serverNotes: Record<Server, string> = {
  apache: "Honours .htaccess files, like XAMPP and Laragon.",
  nginx: "Ignores .htaccess; unknown URLs go to index.php, which suits Laravel, Symfony and WordPress.",
  caddy: "Ignores .htaccess; unknown URLs go to index.php. Its config is short and readable (etc/caddy/Caddyfile).",
  frankenphp: "Caddy with its own PHP built in: fast, but LampRabit's PHP versions and extensions don't apply to its sites. Ignores .htaccess.",
};

/** Whether a port field applies with these settings (platform, enabled servers). */
export const portShown = (f: (typeof portFields)[number], settings: Settings, os?: Os) =>
  (!f.only || (!!os && f.only.includes(os))) &&
  (!f.server || (settings.webServers.includes(f.server) && (f.admin || settings.webServers[0] !== f.server)));

/** The ports a service's details show: a web server's own pair (the main one while it's primary), others theirs. */
export function portsFor(runtime: Runtime, settings: Settings, os?: Os) {
  const server = runtime as Server;
  if ((["apache", "nginx", "caddy", "frankenphp"] as Runtime[]).includes(runtime)) {
    const primary = settings.webServers[0] === server;
    return portFields.filter((f) => (f.server === server && (f.admin || !primary)) || (primary && (f.key === "http" || f.key === "https")));
  }
  return portFields.filter((f) => f.runtime === runtime && portShown(f, settings, os));
}
