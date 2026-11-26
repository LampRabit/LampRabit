import type { Runtime, ServiceKind, Settings, SitesInfo } from "./api";

/**
 * Where a developer tool opens: its own domain once the hosts file knows it, otherwise
 * the localhost alias (which works without the hosts file).
 */
export function toolUrl(runtime: Runtime, sites: SitesInfo | undefined, mailUi: number | undefined, direct?: number): string | undefined {
  // Mailpit is its own web server too, at mail.test.
  if (runtime === "mailpit") direct = mailUi ?? 0;
  const tool = sites?.tools.find((t) => t.tool === runtime);
  if (!tool) return undefined;
  // A service that's its own web server (pgAdmin, KarotX…) has no localhost/<tool>/ alias:
  // its port is given.
  if (direct !== undefined) return sites?.hostsOk ? tool.url : direct ? `http://localhost:${direct}/` : undefined;
  return sites?.hostsOk ? tool.url : `http://localhost/${runtime}/`;
}

/**
 * Every app that can open each database, installed or not, KarotX (LampRabit's own client,
 * the default for all) first. The Services page offers these as the database's default.
 */
export const DB_CLIENTS = {
  mariadb: ["karotx", "phpmyadmin", "adminer"],
  mysql: ["karotx", "phpmyadmin", "adminer"],
  postgres: ["karotx", "pgadmin", "phppgadmin", "adminer"],
  mongodb: ["karotx"],
  redis: ["karotx", "phpredisadmin"],
} as const satisfies Partial<Record<ServiceKind, readonly Runtime[]>>;

export type DbKind = keyof typeof DB_CLIENTS;
export const DB_KINDS = Object.keys(DB_CLIENTS) as DbKind[];
export const isDbKind = (k: string): k is DbKind => k in DB_CLIENTS;

/** The databases a client can open (phpMyAdmin: MariaDB and MySQL). */
export const databasesOf = (client: string): DbKind[] => DB_KINDS.filter((d) => (DB_CLIENTS[d] as readonly string[]).includes(client));

const dbPort = (kind: ServiceKind, settings?: Settings) => settings?.ports.services[`${kind}.${kind === "postgres" ? "postgres" : "mysql"}`];

/**
 * The installed web tools that open a database server, each with the URL that lands on
 * that server: phpMyAdmin numbers its servers (default database first, as configs.rs
 * writes them), Adminer takes the address, phpPgAdmin and pgAdmin have LampRabit's
 * PostgreSQL in their own lists.
 */
export function dbTools(kind: ServiceKind, sites: SitesInfo | undefined, settings: Settings | undefined): { tool: Runtime; url: string }[] {
  const mailUi = settings?.ports.services["mailpit.ui"];
  const port = dbPort(kind, settings);
  const at = (tool: Runtime, url: string | undefined, query = "") => (url ? [{ tool, url: `${url}${query}` }] : []);
  // KarotX, LampRabit's own client, comes first: it lists every local database already.
  const karotx = at("karotx", toolUrl("karotx", sites, mailUi, settings?.ports.services["karotx.http"]));
  if (kind === "mongodb") return karotx;
  if (kind === "redis") return [...karotx, ...at("phpredisadmin", toolUrl("phpredisadmin", sites, mailUi))];
  if (kind === "mariadb" || kind === "mysql") {
    const order = (["mariadb", "mysql"] as const)
      .filter((d) => settings?.versions[d])
      .sort((a, b) => Number(b === settings?.defaultDatabase) - Number(a === settings?.defaultDatabase));
    return [
      ...karotx,
      ...at("phpmyadmin", toolUrl("phpmyadmin", sites, mailUi), `?server=${Math.max(1, order.findIndex((d) => d === kind) + 1)}`),
      ...at("adminer", toolUrl("adminer", sites, mailUi), `?server=127.0.0.1:${port}&username=root`),
    ];
  }
  if (kind === "postgres") {
    return [
      ...karotx,
      ...at("pgadmin", toolUrl("pgadmin", sites, mailUi, settings?.ports.services["pgadmin.http"])),
      ...at("phppgadmin", toolUrl("phppgadmin", sites, mailUi)),
      ...at("adminer", toolUrl("adminer", sites, mailUi), `?pgsql=127.0.0.1:${port}&username=root&db=postgres`),
    ];
  }
  return [];
}

/**
 * The tool a database opens in: the one chosen on the Services page (or in Settings) while
 * it's installed, else KarotX, else the first installed.
 */
export const preferredTool = (kind: ServiceKind, tools: { tool: Runtime; url: string }[], settings: Settings | undefined) =>
  tools.find((t) => t.tool === settings?.dbClients?.[kind]) ?? tools[0];

/** A project's database in the default tool of the default database server, on that database where the tool can. */
export function databaseUrl(db: string | null | undefined, sites: SitesInfo | undefined, settings: Settings | undefined): string | null {
  const kind = settings?.defaultDatabase ?? "mariadb";
  const tool = preferredTool(kind, dbTools(kind, sites, settings), settings);
  if (!tool) return null;
  // phpMyAdmin and Adminer open a named database; the others land on the server.
  return db && (tool.tool === "phpmyadmin" || tool.tool === "adminer") ? `${tool.url}&db=${encodeURIComponent(db)}` : tool.url;
}
