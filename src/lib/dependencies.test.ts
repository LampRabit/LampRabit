import { describe, expect, it } from "vitest";
import type { RuntimeInfo, ServiceInfo } from "./api";
import { names, stoppedNeeds } from "./dependencies";

const svc = (kind: string, state: string, installed = true) => ({ kind, label: kind[0].toUpperCase() + kind.slice(1), state, installed }) as unknown as ServiceInfo;
const info = (needs: string[]) => ({ needs }) as unknown as RuntimeInfo;

describe("services another one needs running", () => {
  it("lists a needed service that's installed but stopped", () => {
    const services = [svc("postgres", "stopped"), svc("pgadmin", "running")];
    expect(stoppedNeeds("pgadmin", info(["python", "postgres"]), services)).toEqual({ all: [services[0]], oneOf: [] });
    expect(stoppedNeeds("pgadmin", info(["python", "postgres"]), [svc("postgres", "running")]).all).toEqual([]);
    // Not installed: Related services offers the install instead.
    expect(stoppedNeeds("pgadmin", info(["postgres"]), [svc("postgres", "stopped", false)]).all).toEqual([]);
  });

  it("asks a database client for one of its databases", () => {
    const services = [svc("mariadb", "stopped"), svc("mysql", "stopped"), svc("redis", "stopped")];
    expect(stoppedNeeds("phpmyadmin", info([]), services).oneOf.map((s) => s.kind)).toEqual(["mariadb", "mysql"]);
    expect(stoppedNeeds("phpmyadmin", info([]), [svc("mariadb", "running"), svc("mysql", "stopped")]).oneOf).toEqual([]);
    expect(names(stoppedNeeds("phpmyadmin", info([]), services).oneOf, "or")).toBe("Mariadb or Mysql");
  });
});
