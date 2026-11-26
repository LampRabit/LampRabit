import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ArrowUpCircle, Search } from "lucide-react";
import clsx from "clsx";
import { api, type PgExtension } from "../lib/api";
import { useAppStore, notifyError } from "../lib/store";
import { Button, Card, Switch, inputClass } from "./ui";

/** The ones most projects reach for, shown first while nothing is searched. */
const POPULAR = ["pgcrypto", "uuid-ossp", "pg_trgm", "citext", "hstore", "unaccent", "fuzzystrmatch", "btree_gin", "btree_gist", "tablefunc", "postgis", "vector"];

/** Needs `shared_preload_libraries` before it does anything; creating it alone isn't enough. */
const PRELOAD = new Set(["pg_stat_statements", "auto_explain", "pg_prewarm"]);

/**
 * PostgreSQL extensions, per database: those the server ships (pgcrypto, uuid-ossp, pg_trgm,
 * hstore, citext…) switched on with CREATE EXTENSION, off with DROP EXTENSION, and updated
 * when the server has a newer version than the database uses.
 */
export default function PgExtensionsPanel() {
  const queryClient = useQueryClient();
  const notify = useAppStore((s) => s.notify);
  const services = useQuery({ queryKey: ["services"], queryFn: api.listServices });
  const running = services.data?.find((s) => s.kind === "postgres")?.state === "running";
  const databases = useQuery({ queryKey: ["databases", "postgres"], queryFn: () => api.listDatabases("postgres"), enabled: running });
  const [picked, setPicked] = useState<string | null>(null);
  // A project's database first; the postgres maintenance database only when there's no other.
  const database = picked ?? databases.data?.find((d) => d !== "postgres") ?? databases.data?.[0] ?? null;
  const list = useQuery({ queryKey: ["pg-extensions", database], queryFn: () => api.pgExtensions(database!), enabled: running && !!database });
  const [query, setQuery] = useState("");
  const [showAll, setShowAll] = useState(false);

  const change = useMutation({
    mutationFn: ({ name, action }: { name: string; action: "enable" | "disable" | "update" }) => api.setPgExtension(database!, name, action),
    onSuccess: (_, { name, action }) => {
      notify("info", action === "enable" ? `${name} is on in ${database}` : action === "disable" ? `${name} removed from ${database}` : `${name} updated in ${database}`);
      if (action === "enable" && PRELOAD.has(name)) notify("info", `${name} also needs shared_preload_libraries = '${name}' in postgresql.conf, then a restart.`);
      queryClient.invalidateQueries({ queryKey: ["pg-extensions", database] });
    },
    onError: (e) => {
      notifyError(e);
      queryClient.invalidateQueries({ queryKey: ["pg-extensions", database] });
    },
  });

  const all = list.data ?? [];
  const enabled = all.filter((e) => e.installedVersion);
  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    const rest = (list.data ?? []).filter((e) => !e.installedVersion);
    if (q) return rest.filter((e) => e.name.includes(q) || e.comment.toLowerCase().includes(q));
    if (showAll) return rest;
    return POPULAR.map((n) => rest.find((e) => e.name === n)).filter((e): e is PgExtension => !!e);
  }, [list.data, query, showAll]);
  const available = all.length - enabled.length;

  const row = (e: PgExtension) => {
    const busy = change.isPending && change.variables?.name === e.name;
    const outdated = e.installedVersion && e.defaultVersion && e.installedVersion !== e.defaultVersion;
    return (
      <li key={e.name} className="flex items-center gap-3 px-5 py-2">
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-baseline gap-x-2">
            <span className="font-mono text-sm font-medium text-fg">{e.name}</span>
            <span className="font-mono text-[11px] text-muted">
              {e.installedVersion ?? e.defaultVersion}
              {e.schema && e.schema !== "public" && ` · ${e.schema}`}
            </span>
            {PRELOAD.has(e.name) && <span className="text-[11px] text-muted">needs shared_preload_libraries</span>}
          </div>
          {e.comment && <div className="truncate text-xs text-muted" title={e.comment}>{e.comment}</div>}
        </div>
        {outdated && (
          <Button size="sm" variant="ghost" onClick={() => change.mutate({ name: e.name, action: "update" })} disabled={change.isPending} title={`ALTER EXTENSION … UPDATE to ${e.defaultVersion}`}>
            <ArrowUpCircle size={14} /> Update to {e.defaultVersion}
          </Button>
        )}
        {e.name === "plpgsql" ? (
          <span className="text-xs text-muted">Built in</span>
        ) : (
        <Switch
          checked={busy ? change.variables!.action !== "disable" : !!e.installedVersion}
          busy={busy}
          onChange={(on) => !change.isPending && change.mutate({ name: e.name, action: on ? "enable" : "disable" })}
          label={`${e.name} in ${database}`}
        />
        )}
      </li>
    );
  };

  return (
    <Card className="mt-4 overflow-hidden">
      <div className="flex flex-wrap items-center gap-3 border-b border-line px-5 py-3">
        <div className="min-w-0 flex-1">
          <h3 className="text-sm font-medium">Extensions</h3>
          <p className="text-xs text-muted">Each database has its own. Switching one on runs CREATE EXTENSION (with what it depends on); off runs DROP EXTENSION.</p>
        </div>
        {running && (databases.data ?? []).length > 0 && (
          <select value={database ?? ""} onChange={(e) => setPicked(e.target.value)} aria-label="Database" className={clsx(inputClass.replace("w-full", ""), "w-48 py-1 font-mono text-xs")}>
            {databases.data!.map((d) => (
              <option key={d} value={d}>
                {d}
              </option>
            ))}
          </select>
        )}
      </div>
      {!running ? (
        <p className="px-5 py-3 text-sm text-muted">Start PostgreSQL to see and change its databases' extensions.</p>
      ) : databases.isError ? (
        <p className="px-5 py-3 text-sm text-danger-text">{String(databases.error)}</p>
      ) : list.isLoading || databases.isLoading ? (
        <p className="px-5 py-3 text-sm text-muted">Reading extensions…</p>
      ) : list.isError ? (
        <p className="px-5 py-3 text-sm text-danger-text">{String(list.error)}</p>
      ) : (
        <>
          <h4 className="px-5 pt-3 pb-1 text-xs font-medium text-muted">
            In {database} ({enabled.length})
          </h4>
          {enabled.length === 0 ? <p className="px-5 pb-2 text-sm text-muted">None yet, apart from the built-in plpgsql.</p> : <ul className="divide-y divide-line">{enabled.map(row)}</ul>}
          <div className="mt-2 flex flex-wrap items-center gap-2 border-t border-line px-5 py-2.5">
            <h4 className="text-xs font-medium text-muted">{query.trim() ? "Matching" : showAll ? `Available (${available})` : "Popular"}</h4>
            <span className="flex-1" />
            <label className="flex h-8 w-56 items-center gap-2 rounded-lg border border-line bg-surface px-2.5">
              <Search size={14} className="text-muted" />
              <input value={query} onChange={(e) => setQuery(e.target.value)} placeholder={`Search ${available} extensions…`} className="w-full bg-transparent text-xs outline-none placeholder:text-muted" aria-label="Search extensions" />
            </label>
          </div>
          {filtered.length === 0 ? (
            <p className="px-5 pb-3 text-sm text-muted">{query.trim() ? "No extension matches." : "Every popular extension is already on."}</p>
          ) : (
            <ul className="divide-y divide-line">{filtered.map(row)}</ul>
          )}
          {!query.trim() && (
            <button type="button" onClick={() => setShowAll((v) => !v)} className="w-full border-t border-line py-2 text-xs font-medium text-accent-text hover:underline">
              {showAll ? "Show popular only" : `Show all ${available} available`}
            </button>
          )}
        </>
      )}
    </Card>
  );
}
