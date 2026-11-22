import type { Runtime, SitesInfo } from "./api";

/**
 * Where a developer tool opens: its own domain once the hosts file knows it, otherwise
 * the localhost alias (which works without the hosts file). Mailpit is reached directly.
 */
export function toolUrl(runtime: Runtime, sites: SitesInfo | undefined, mailUi: number | undefined, direct?: number): string | undefined {
  if (runtime === "mailpit") return mailUi ? `http://localhost:${mailUi}/` : undefined;
  const tool = sites?.tools.find((t) => t.tool === runtime);
  if (!tool) return undefined;
  // pgAdmin and Neo4j Browser are their own web servers, without a localhost/<tool>/
  // alias: `direct` is their port.
  // A service that's its own web server (pgAdmin, KarotX…) has no localhost/<tool>/ alias:
  // its port is given.
  if (direct !== undefined) return sites?.hostsOk ? tool.url : direct ? `http://localhost:${direct}/` : undefined;
  return sites?.hostsOk ? tool.url : `http://localhost/${runtime}/`;
}
