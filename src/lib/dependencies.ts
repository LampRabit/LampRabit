import type { RuntimeInfo, ServiceInfo } from "./api";
import { databasesOf } from "./tools";

const up = (s: ServiceInfo) => s.state === "running" || s.state === "starting";

/**
 * The installed services another one needs running but that aren't: `all` must each run
 * (pgAdmin needs PostgreSQL), `oneOf` when a database client has none of its databases
 * running (phpMyAdmin: MariaDB or MySQL). What isn't installed yet is Related services' job.
 */
export function stoppedNeeds(runtime: string, info: RuntimeInfo | undefined, services: ServiceInfo[] | undefined): { all: ServiceInfo[]; oneOf: ServiceInfo[] } {
  const installed = (services ?? []).filter((s) => s.installed);
  const all = installed.filter((s) => info?.needs.includes(s.kind) && !up(s));
  // A database it needs outright (pgAdmin's PostgreSQL) is already in `all`.
  const dbs = installed.filter((s) => databasesOf(runtime).includes(s.kind as never) && !info?.needs.includes(s.kind));
  return { all, oneOf: dbs.some(up) ? [] : dbs };
}

/** "PostgreSQL", "MariaDB or MySQL". */
export const names = (list: ServiceInfo[], joiner: "and" | "or") =>
  list.length <= 1 ? (list[0]?.label ?? "") : `${list.slice(0, -1).map((s) => s.label).join(", ")} ${joiner} ${list[list.length - 1].label}`;
