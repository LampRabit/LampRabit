// @vitest-environment jsdom
import { describe, expect, it } from "vitest";
import type { Settings, SitesInfo } from "./api";
import { databaseUrl, databasesOf, dbTools, preferredTool } from "./tools";

const sites = { hostsOk: true, tools: ["phpmyadmin", "adminer", "karotx"].map((tool) => ({ tool, url: `https://${tool}.test/` })) } as unknown as SitesInfo;
const settings = { defaultDatabase: "mariadb", versions: { mariadb: "11.8" }, ports: { services: { "mariadb.mysql": 3306, "karotx.http": 8090 } } } as unknown as Settings;
const choosing = (tool: string) => ({ ...settings, dbClients: { mariadb: tool } }) as Settings;

describe("database tools", () => {
  it("opens the default chosen for the database, KarotX until one is chosen", () => {
    const tools = dbTools("mariadb", sites, settings);
    expect(tools.map((t) => t.tool)).toEqual(["karotx", "phpmyadmin", "adminer"]);
    expect(preferredTool("mariadb", tools, settings)?.tool).toBe("karotx");
    expect(preferredTool("mariadb", tools, choosing("adminer"))?.tool).toBe("adminer");
    expect(databaseUrl("shop", sites, choosing("adminer"))).toBe("https://adminer.test/?server=127.0.0.1:3306&username=root&db=shop");
    expect(databaseUrl("shop", sites, choosing("phpmyadmin"))).toBe("https://phpmyadmin.test/?server=1&db=shop");
    // A chosen app that was removed falls back to KarotX.
    expect(preferredTool("mariadb", tools, choosing("pgadmin"))?.tool).toBe("karotx");
  });

  it("knows which databases each client opens", () => {
    expect(databasesOf("phpmyadmin")).toEqual(["mariadb", "mysql"]);
    expect(databasesOf("karotx")).toEqual(["mariadb", "mysql", "postgres", "mongodb", "redis"]);
    expect(databasesOf("apache")).toEqual([]);
  });
});
