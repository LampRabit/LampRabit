import type { DbEngine, PackageManager, ProjectTemplate, PythonManager } from "./api";

/** What New Project knows about this machine and the user's choices. */
export interface NeedsInput {
  template: Pick<ProjectTemplate, "usesPhp" | "usesNode" | "nodeSteps" | "minPhp" | "python" | "java" | "go" | "rust" | "engines" | "database"> | undefined;
  /** Active versions by runtime id (settings.versions). */
  versions: Record<string, string | null | undefined>;
  /** Installed PHP versions. */
  phpVersions: string[];
  /** The PHP the project would use: the one picked, else the default. */
  phpInUse: string | null;
  /** Node.js versions NVM has (null while unknown). */
  nodeVersions: string[] | null;
  /** A database server of the template's engines is installed (SQLite always counts). */
  hasDatabase: boolean;
  withDb: boolean;
  pythonManager: PythonManager;
  packageManager: PackageManager;
  defaultDatabase: string | null | undefined;
}

export interface NeedsPlan {
  /** [runtime id or "node", oldest version that works], for `api.projectRequirements`. */
  toInstall: [string, string | null][];
  /** The database server installed with the project, when none is there yet. */
  installEngine: DbEngine | null;
  /** No PHP at all: the one installed becomes the default. */
  noPhp: boolean;
}

/** "8.4.27" >= "8.2" */
export function atLeast(version: string, minimum: string): boolean {
  const [a, b] = [version, minimum].map((v) => v.split(/\D+/).filter(Boolean).map(Number));
  for (let i = 0; i < Math.max(a.length, b.length); i++) {
    if ((a[i] ?? 0) !== (b[i] ?? 0)) return (a[i] ?? 0) > (b[i] ?? 0);
  }
  return true;
}

/**
 * What a new project needs that isn't there, installed at the start of Create instead of
 * sending the user to Services: PHP (a fitting one when the default is too old), Node.js,
 * the template's language, uv/Conda/Bun when picked, and a database server (the default
 * one when the template can use it).
 */
export function projectNeeds(i: NeedsInput): NeedsPlan {
  const t = i.template;
  const out: NeedsPlan = { toInstall: [], installEngine: null, noPhp: false };
  if (!t) return out;
  if (t.usesPhp) {
    out.noPhp = i.phpVersions.length === 0;
    const tooOld = !!t.minPhp && !!i.phpInUse && !atLeast(i.phpInUse, t.minPhp);
    if (out.noPhp || tooOld) out.toInstall.push(["php", t.minPhp ?? null]);
  }
  if (t.usesNode && i.nodeVersions !== null && i.nodeVersions.length === 0) out.toInstall.push(["node", null]);
  for (const l of ["python", "java", "go", "rust"] as const) {
    if (t[l] && !i.versions[l]) out.toInstall.push([l, t[l] ?? null]);
  }
  if (t.python && i.pythonManager !== "pip" && !i.versions[i.pythonManager]) out.toInstall.push([i.pythonManager, null]);
  if ((t.usesNode || t.nodeSteps) && i.packageManager === "bun" && !i.versions.bun) out.toInstall.push(["bun", null]);
  const dbUse = t.database ?? "none";
  if (!i.hasDatabase && (dbUse === "required" || (dbUse === "optional" && i.withDb))) {
    const choices = t.engines ?? ["mariadb", "mysql"];
    out.installEngine = choices.find((e) => e === i.defaultDatabase) ?? choices.find((e) => e !== "sqlite") ?? null;
    if (out.installEngine) out.toInstall.push([out.installEngine, null]);
  }
  return out;
}
