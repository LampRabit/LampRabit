import type { Release, Runtime } from "./api";

/**
 * What each runtime needs to work (installed first) and what goes well with it (a UI to
 * look at its data, its shell, its package manager). The Services page offers both with
 * one-click installs.
 */
export const related: Partial<Record<Runtime, { needs?: Runtime[]; suggests?: Runtime[] }>> = {
  php: { suggests: ["composer"] },
  composer: { needs: ["php"] },
  mariadb: { suggests: ["phpmyadmin", "adminer"] },
  mysql: { suggests: ["phpmyadmin", "adminer"] },
  postgres: { suggests: ["adminer", "phppgadmin", "pgadmin"] },
  mongodb: { suggests: ["mongosh", "compass", "mongotools"] },
  clickhouse: {},
  neo4j: { needs: ["java"] },
  sqlite: {},
  redis: { suggests: ["phpredisadmin"] },
  tomcat: { needs: ["java"] },
  pgadmin: { needs: ["python", "postgres"] },
  phppgadmin: { needs: ["postgres"] },
  phpredisadmin: { needs: ["redis"] },
  mongosh: { needs: ["mongodb"] },
  mongotools: { needs: ["mongodb"] },
  compass: { needs: ["mongodb"] },
};

/**
 * The version a one-click install picks: an LTS when there is one, Python 3.13 (what
 * pgAdmin's dependencies ship wheels for), otherwise the newest. Catalogs are newest first.
 */
export function pickRelease(runtime: Runtime, catalog: Release[]): Release | undefined {
  if (runtime === "python") return catalog.find((r) => r.version.startsWith("3.13.")) ?? catalog[0];
  return catalog.find((r) => r.label === "LTS") ?? catalog[0];
}
