import { describe, expect, it } from "vitest";
import { atLeast, projectNeeds, type NeedsInput } from "./projectNeeds";

const laravel = { usesPhp: true, usesNode: false, nodeSteps: true, minPhp: "8.2", python: null, java: null, go: null, rust: null, engines: ["mariadb", "mysql", "postgres", "sqlite"], database: "required" } as NeedsInput["template"];
const base: NeedsInput = {
  template: laravel,
  versions: { php: "8.4.14", mariadb: "11.8.3" },
  phpVersions: ["8.4.14"],
  phpInUse: "8.4.14",
  nodeVersions: ["22.11.0"],
  hasDatabase: true,
  withDb: true,
  pythonManager: "pip",
  packageManager: "npm",
  defaultDatabase: "mariadb",
};

describe("what a new project installs first", () => {
  it("installs nothing when everything is there", () => {
    expect(projectNeeds(base).toInstall).toEqual([]);
  });

  it("installs PHP when there's none, or one that fits when the default is too old", () => {
    const none = projectNeeds({ ...base, phpVersions: [], phpInUse: null, versions: {} });
    expect(none.toInstall[0]).toEqual(["php", "8.2"]);
    expect(none.noPhp).toBe(true);
    const old = projectNeeds({ ...base, phpInUse: "8.1.30" });
    expect(old.toInstall).toEqual([["php", "8.2"]]);
    expect(old.noPhp).toBe(false);
  });

  it("installs the default database server when the template needs one and none is there", () => {
    const plan = projectNeeds({ ...base, hasDatabase: false, defaultDatabase: "mysql" });
    expect(plan.installEngine).toBe("mysql");
    expect(plan.toInstall).toContainEqual(["mysql", null]);
    // The default isn't one the template can use: its first server (never SQLite, which is a file).
    const pg = projectNeeds({ ...base, hasDatabase: false, template: { ...laravel!, engines: ["sqlite", "postgres"] } });
    expect(pg.installEngine).toBe("postgres");
    // An optional database is installed only when it's switched on.
    const optional = { ...laravel!, database: "optional" as const };
    expect(projectNeeds({ ...base, hasDatabase: false, withDb: false, template: optional }).installEngine).toBeNull();
    expect(projectNeeds({ ...base, hasDatabase: false, withDb: true, template: optional }).installEngine).toBe("mariadb");
  });

  it("installs the language, Node.js and the package manager picked", () => {
    const django = { ...laravel!, usesPhp: false, nodeSteps: false, python: "3.10", database: "none" as const };
    expect(projectNeeds({ ...base, template: django, versions: {}, pythonManager: "uv" }).toInstall).toEqual([
      ["python", "3.10"],
      ["uv", null],
    ]);
    const vite = { ...laravel!, usesPhp: false, usesNode: true, database: "none" as const };
    expect(projectNeeds({ ...base, template: vite, nodeVersions: [], packageManager: "bun" }).toInstall).toEqual([
      ["node", null],
      ["bun", null],
    ]);
    // While NVM's list is still loading, Node.js isn't counted as missing.
    expect(projectNeeds({ ...base, template: vite, nodeVersions: null }).toInstall).toEqual([]);
  });

  it("compares versions by their numbers", () => {
    expect(atLeast("8.4.27", "8.2")).toBe(true);
    expect(atLeast("8.1.30", "8.2")).toBe(false);
    expect(atLeast("8.2.0RC1", "8.2")).toBe(true);
  });
});
