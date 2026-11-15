import type { Connection, ServiceInfo } from "./api";

export type { Connection };

/**
 * How an app connects to a service. The backend renders it from the service's manifest
 * (src-tauri/services/<id>.yaml, `connection:`), so a new service needs nothing here.
 */
export function connection(service: Pick<ServiceInfo, "connection" | "port">): Connection {
  // Only a service without connection details (none today) shows its bare address.
  return service.connection ?? { url: `127.0.0.1:${service.port}`, env: [] };
}

export const envText = (env: [string, string][]) => env.map(([k, v]) => `${k}=${v}`).join("\n");

/** The text copied for a connection in the given mode. */
export const copyText = (c: Connection, mode: "url" | "env") => (mode === "url" || c.env.length === 0 ? c.url : envText(c.env));
