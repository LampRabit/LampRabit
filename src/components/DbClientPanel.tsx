import { useMutation, useQuery } from "@tanstack/react-query";
import { Table2 } from "lucide-react";
import clsx from "clsx";
import { api, type InstalledRuntime, type Runtime } from "../lib/api";
import { labelFor, useCatalog } from "../lib/catalog";
import { notifyError } from "../lib/store";
import { DB_CLIENTS, databasesOf, dbTools, isDbKind, preferredTool, type DbKind } from "../lib/tools";
import { tileFor } from "./runtimes";
import { Card, Switch } from "./ui";

/**
 * Which app a database opens in, on the Services page: on a database's page every app that
 * can open it (KarotX, the default, first; ones not installed link to their page), on a
 * client's page the databases it can be the default for. Applies at once.
 */
export default function DbClientPanel({ runtime, installed, onSelect }: { runtime: Runtime; installed?: InstalledRuntime[]; onSelect: (r: Runtime) => void }) {
  const catalog = useCatalog();
  const sites = useQuery({ queryKey: ["sites"], queryFn: api.listSites });
  const settings = useQuery({ queryKey: ["settings"], queryFn: api.getSettings });
  const choose = useMutation({ mutationFn: ({ db, tool }: { db: string; tool: string | null }) => api.setDbClient(db, tool), onError: notifyError });
  const has = (r: string) => !!installed?.find((i) => i.runtime === r)?.active;
  // The app a database opens in now: the chosen one while it's installed, else KarotX or the first installed.
  const current = (db: DbKind) => preferredTool(db, dbTools(db, sites.data, settings.data), settings.data)?.tool;

  if (isDbKind(runtime)) {
    if (!has(runtime)) return null;
    const now = current(runtime);
    return (
      <Card className="mt-4 overflow-hidden">
        <Header title="Opens in" hint={`The app ${labelFor(catalog.data, runtime)}'s Open button uses on Home, and for projects' databases`} />
        <ul className="flex flex-wrap gap-2 px-5 py-3" role="radiogroup" aria-label={`Open ${labelFor(catalog.data, runtime)} in`}>
          {DB_CLIENTS[runtime].map((tool) => {
            const ready = has(tool);
            const picked = ready && now === tool;
            return (
              <li key={tool}>
                <button
                  type="button"
                  role="radio"
                  aria-checked={picked}
                  disabled={choose.isPending}
                  onClick={() => (ready ? choose.mutate({ db: runtime, tool: tool === "karotx" ? null : tool }) : onSelect(tool))}
                  title={ready ? `Open ${labelFor(catalog.data, runtime)} in ${labelFor(catalog.data, tool)}` : `Not installed: show ${labelFor(catalog.data, tool)} to install it`}
                  className={clsx(
                    "flex items-center gap-2 rounded-lg border py-1.5 pr-3 pl-1.5 text-sm transition-colors",
                    picked ? "border-accent bg-surface-2 font-medium text-fg" : "border-line text-fg hover:bg-surface-2",
                    !ready && "text-muted",
                  )}
                >
                  <span className={clsx("flex size-6 items-center justify-center rounded-md", tileFor(tool).className, !ready && "opacity-50")}>{tileFor(tool).icon(14)}</span>
                  {labelFor(catalog.data, tool)}
                  <span className="text-xs text-muted">{picked ? "Default" : ready ? "" : "Not installed"}</span>
                </button>
              </li>
            );
          })}
        </ul>
      </Card>
    );
  }

  const dbs = databasesOf(runtime).filter(has);
  if (!has(runtime) || dbs.length === 0) return null;
  return (
    <Card className="mt-4 overflow-hidden">
      <Header title="Default app for" hint={`Databases whose Open button uses ${labelFor(catalog.data, runtime)}. Without a choice they open in ${labelFor(catalog.data, "karotx")}.`} />
      <ul className="divide-y divide-line">
        {dbs.map((db) => (
          <li key={db} className="flex items-center gap-3 px-5 py-2.5">
            <span className={clsx("flex size-6 items-center justify-center rounded-md", tileFor(db).className)}>{tileFor(db).icon(14)}</span>
            <span className="min-w-0 flex-1 text-sm">
              {labelFor(catalog.data, db)}
              {current(db) !== runtime && current(db) && <span className="ml-2 text-xs text-muted">now opens in {labelFor(catalog.data, current(db)!)}</span>}
            </span>
            <Switch
              checked={current(db) === runtime}
              disabled={choose.isPending || (runtime === "karotx" && current(db) === "karotx")}
              onChange={(on) => choose.mutate({ db, tool: on ? (runtime === "karotx" ? null : runtime) : null })}
              label={`Open ${labelFor(catalog.data, db)} in ${labelFor(catalog.data, runtime)}`}
            />
          </li>
        ))}
      </ul>
    </Card>
  );
}

function Header({ title, hint }: { title: string; hint: string }) {
  return (
    <div className="flex items-center gap-3 border-b border-line px-5 py-3">
      <Table2 size={16} className="text-muted" aria-hidden />
      <div className="min-w-0 flex-1">
        <h3 className="text-sm font-medium">{title}</h3>
        <p className="text-xs text-muted">{hint}</p>
      </div>
    </div>
  );
}
