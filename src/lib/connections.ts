import type { ServiceKind } from "./api";

/** How an app connects to a service: a URL, and the same as .env lines. */
export interface Connection {
  url: string;
  env: [string, string][];
}

// 127.0.0.1 rather than localhost: MySQL clients (PHP's included) read "localhost" as
// "use the Unix socket", which isn't where LampRabit's servers listen.
const HOST = "127.0.0.1";

export function connection(kind: ServiceKind, port: number): Connection {
  switch (kind) {
    case "apache":
    case "nginx":
    case "caddy":
    case "frankenphp": {
      const url = `http://localhost${port === 80 ? "" : `:${port}`}`;
      return { url, env: [["APP_URL", url]] };
    }
    case "mongodb":
      return {
        url: `mongodb://${HOST}:${port}`,
        env: [
          ["DB_CONNECTION", "mongodb"],
          ["MONGODB_URI", `mongodb://${HOST}:${port}`],
        ],
      };
    case "clickhouse":
      return {
        url: `http://${HOST}:${port}`,
        env: [
          ["CLICKHOUSE_HOST", HOST],
          ["CLICKHOUSE_PORT", String(port)],
          ["CLICKHOUSE_USER", "default"],
          ["CLICKHOUSE_PASSWORD", ""],
        ],
      };
    case "neo4j":
      return { url: `http://${HOST}:${port}`, env: [["NEO4J_HTTP_URL", `http://${HOST}:${port}`]] };
    case "pgadmin":
      return { url: `http://${HOST}:${port}`, env: [["PGADMIN_URL", `http://${HOST}:${port}`]] };
    case "tomcat":
      return { url: `http://${HOST}:${port}`, env: [["TOMCAT_URL", `http://${HOST}:${port}`]] };
    case "mariadb":
    case "mysql":
      return {
        url: `${kind}://root@${HOST}:${port}`,
        env: [
          ["DB_CONNECTION", kind],
          ["DB_HOST", HOST],
          ["DB_PORT", String(port)],
          ["DB_USERNAME", "root"],
          ["DB_PASSWORD", ""],
        ],
      };
    case "postgres":
      return {
        url: `postgresql://root@${HOST}:${port}/postgres`,
        env: [
          ["DB_CONNECTION", "pgsql"],
          ["DB_HOST", HOST],
          ["DB_PORT", String(port)],
          ["DB_USERNAME", "root"],
          ["DB_PASSWORD", ""],
        ],
      };
    case "redis":
      return {
        url: `redis://${HOST}:${port}`,
        env: [
          ["REDIS_HOST", HOST],
          ["REDIS_PASSWORD", "null"],
          ["REDIS_PORT", String(port)],
        ],
      };
    case "memcached":
      return {
        url: `${HOST}:${port}`,
        env: [
          ["MEMCACHED_HOST", HOST],
          ["MEMCACHED_PORT", String(port)],
        ],
      };
    case "mailpit":
      return {
        url: `smtp://${HOST}:${port}`,
        env: [
          ["MAIL_MAILER", "smtp"],
          ["MAIL_HOST", HOST],
          ["MAIL_PORT", String(port)],
          ["MAIL_USERNAME", "null"],
          ["MAIL_PASSWORD", "null"],
          ["MAIL_ENCRYPTION", "null"],
        ],
      };
  }
}

export const envText = (env: [string, string][]) => env.map(([k, v]) => `${k}=${v}`).join("\n");

/** The text copied for a connection in the given mode. */
export const copyText = (c: Connection, mode: "url" | "env") => (mode === "url" || c.env.length === 0 ? c.url : envText(c.env));
