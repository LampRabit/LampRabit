import { useEffect, useMemo, useRef, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { ArrowUpCircle, Download, Loader2, PackagePlus, RefreshCw, Search, Trash2, TriangleAlert } from "lucide-react";
import clsx from "clsx";
import { api, type AppStatus, type DependencyEcosystem, type DependencyManager, type DependencyPlan, type DependencyRequest, type OutdatedPackage, type Site } from "../lib/api";
import { useAppStore, notifyError } from "../lib/store";
import Dialog from "./Dialog";
import { OutputPanel } from "./ProjectRun";
import { Button, Card, Pill, inputClass } from "./ui";

/** What to type in the Add field, per package manager. */
const addHint: Partial<Record<DependencyManager, string>> = {
  composer: "vendor/package or vendor/package:^2.0",
  npm: "package or package@version",
  pnpm: "package or package@version",
  yarn: "package or package@version",
  bun: "package or package@version",
  deno: "jsr:@std/path or npm:package",
  uv: "package or package==version",
  poetry: "package or package@^1.0",
  pip: "package or package==version",
  cargo: "crate or crate@version",
  go: "module path, e.g. github.com/gin-gonic/gin",
};

/** Maven and Gradle versions live in the build file: Gradle's can't be changed by a command, Maven's only to a given version. */
const canUpdate = (m: DependencyManager) => m !== "maven" && m !== "gradle";
const canUpgrade = (m: DependencyManager) => m !== "gradle";

/**
 * A project's dependencies per package manager: what the manifest declares, what is installed, what has
 * newer releases, and install / add / update / remove. Every change shows its command first and runs
 * only when confirmed; the output streams next to the list (and into the Commands tab).
 */
export default function ProjectDependencies({ site, status }: { site: Site; status?: AppStatus }) {
  const queryClient = useQueryClient();
  const list = useQuery({ queryKey: ["dependencies", site.name], queryFn: () => api.projectDependencies(site.name) });
  const [pending, setPending] = useState<{ request: DependencyRequest; plan: DependencyPlan } | null>(null);
  const [starting, setStarting] = useState(false);
  const running = status?.task ?? null;

  // A command just ended: the installed versions and lock files may have changed.
  const previous = useRef(running);
  useEffect(() => {
    if (previous.current && !running) {
      queryClient.invalidateQueries({ queryKey: ["dependencies", site.name] });
      queryClient.invalidateQueries({ queryKey: ["outdated", site.name] });
      queryClient.invalidateQueries({ queryKey: ["project-facts", site.name] });
    }
    previous.current = running;
  }, [running, queryClient, site.name]);

  const ask = (request: DependencyRequest) =>
    api.dependencyPlan(site.name, request).then(
      (plan) => setPending({ request, plan }),
      notifyError,
    );
  const confirm = () => {
    if (!pending) return;
    setStarting(true);
    api
      .runDependencyAction(site.name, pending.request)
      .then(() => setPending(null), notifyError)
      .finally(() => setStarting(false));
  };

  const ecosystems = list.data ?? [];
  return (
    <div className="grid grid-cols-1 items-start gap-4 lg:grid-cols-[minmax(0,1fr)_minmax(0,24rem)]">
      <div className="min-w-0 space-y-4">
        {list.isLoading ? (
          <p className="text-sm text-muted">Reading the project's dependencies…</p>
        ) : list.isError ? (
          <p className="text-sm text-danger-text">{String(list.error)}</p>
        ) : ecosystems.length === 0 ? (
          <Card className="p-5">
            <h3 className="font-semibold">No dependencies yet</h3>
            <p className="mt-1 text-sm text-muted">
              This tab lists the packages from composer.json, package.json, deno.json, pyproject.toml, requirements.txt, Cargo.toml, go.mod, pom.xml or build.gradle once the project has
              one.
            </p>
          </Card>
        ) : (
          ecosystems.map((eco) => <EcosystemCard key={eco.manager} site={site} eco={eco} running={running} onAsk={ask} />)
        )}
      </div>
      <div className="min-w-0 lg:sticky lg:top-4">
        <OutputPanel name={site.name} height="h-[26rem]" title={<span className="flex-1 text-xs font-semibold text-slate-300">{running ? `${running}…` : "Output"}</span>} />
      </div>

      {pending && (
        <Dialog
          open
          onClose={() => !starting && setPending(null)}
          title={pending.plan.label}
          icon={<PackagePlus size={18} />}
          footer={
            <>
              <Button variant="ghost" onClick={() => setPending(null)} disabled={starting}>
                Cancel
              </Button>
              <Button variant="primary" onClick={confirm} disabled={starting || !!running} data-autofocus title={running ? `Wait for "${running}" to finish` : undefined}>
                {starting ? <Loader2 size={14} className="animate-spin" /> : null} Run
              </Button>
            </>
          }
        >
          <div className="space-y-3 text-sm">
            {pending.plan.edit && <p>{pending.plan.edit}, then runs:</p>}
            {!pending.plan.edit && <p>Runs in {site.name}:</p>}
            <pre className="selectable overflow-x-auto rounded-lg bg-terminal px-3 py-2 font-mono text-xs text-slate-200">{pending.plan.command}</pre>
            <p className="text-muted">The output shows next to the list. Nothing changes until you run it.</p>
          </div>
        </Dialog>
      )}
    </div>
  );
}

function EcosystemCard({ site, eco, running, onAsk }: { site: Site; eco: DependencyEcosystem; running: string | null; onAsk: (r: DependencyRequest) => void }) {
  const notify = useAppStore((s) => s.notify);
  const [filter, setFilter] = useState("");
  const [name, setName] = useState("");
  const [dev, setDev] = useState(false);
  const outdated = useQuery({
    queryKey: ["outdated", site.name, eco.manager],
    queryFn: () => api.dependenciesOutdated(site.name, eco.manager),
    enabled: false,
    retry: false,
  });
  useEffect(() => {
    if (outdated.error) notify("error", String(outdated.error));
  }, [outdated.error, notify]);
  const newer = useMemo(() => new Map((outdated.data ?? []).map((o: OutdatedPackage) => [o.name.toLowerCase(), o])), [outdated.data]);
  const checked = outdated.isSuccess;
  const busy = !!running || !!eco.missing;
  const shown = eco.packages.filter((p) => !filter || p.name.toLowerCase().includes(filter.toLowerCase()));
  const run = (r: Omit<DependencyRequest, "manager">) => onAsk({ manager: eco.manager, ...r });

  return (
    <Card className="overflow-hidden">
      <div className="flex flex-wrap items-center gap-x-3 gap-y-2 border-b border-line px-4 py-3">
        {/* The title keeps its width; the buttons wrap under it when the card is narrow. */}
        <div className="min-w-0 flex-1 basis-56">
          <h3 className="flex flex-wrap items-center gap-x-2 gap-y-1 font-semibold">
            {eco.label}
            <span className="font-mono text-xs font-normal text-muted">
              {eco.manifest}
              {eco.lock && ` · ${eco.lock}`}
            </span>
            {eco.installed === false && <Pill tone="warn">Not installed</Pill>}
            {eco.installed === true && <Pill tone="ok">Installed</Pill>}
          </h3>
          <p className="text-xs text-muted">
            {eco.packages.length} {eco.packages.length === 1 ? "package" : "packages"}
            {checked && ` · ${outdated.data?.length ? `${outdated.data.length} with newer releases` : "all up to date"}`}
          </p>
        </div>
        <div className="flex flex-wrap gap-2">
        <Button size="sm" variant={eco.installed === false ? "primary" : "secondary"} onClick={() => run({ action: "install" })} disabled={busy} title={eco.install}>
          <Download size={14} /> Install
        </Button>
        <Button size="sm" onClick={() => outdated.refetch()} disabled={!!eco.missing || outdated.isFetching || eco.packages.length === 0} title="Look up the newest release of each package">
          <RefreshCw size={14} className={clsx(outdated.isFetching && "animate-spin")} /> Check for updates
        </Button>
        {eco.updateAll && (
          <Button size="sm" onClick={() => run({ action: "updateall" })} disabled={busy} title={`${eco.updateAll} (within the versions ${eco.manifest} allows)`}>
            <ArrowUpCircle size={14} /> Update all
          </Button>
        )}
        </div>
      </div>

      {eco.missing && (
        <p className="flex items-start gap-2 border-b border-line bg-warn/10 px-4 py-2 text-sm">
          <TriangleAlert size={15} className="mt-0.5 shrink-0 text-warn" aria-hidden /> {eco.missing}
        </p>
      )}

      {eco.packages.length > 10 && (
        <div className="relative border-b border-line px-4 py-2">
          <Search size={14} className="absolute top-1/2 left-6.5 -translate-y-1/2 text-muted" aria-hidden />
          <input className={clsx(inputClass, "py-1.5 pl-8")} placeholder={`Filter ${eco.packages.length} packages`} value={filter} onChange={(e) => setFilter(e.target.value)} aria-label="Filter packages" />
        </div>
      )}

      {eco.packages.length === 0 ? (
        <p className="px-4 py-3 text-sm text-muted">{eco.manifest} lists no dependencies.</p>
      ) : (
        <div className="max-h-[28rem] overflow-y-auto">
          <table className="w-full text-sm">
            <thead className="sticky top-0 bg-surface text-left text-xs text-muted">
              <tr>
                <th className="w-full px-4 py-2 font-medium">Package</th>
                <th className="px-2 py-2 font-medium">Wanted</th>
                <th className="px-2 py-2 font-medium">Installed</th>
                <th className="px-2 py-2 font-medium">Latest</th>
                <th className="px-4 py-2" aria-label="Actions" />
              </tr>
            </thead>
            <tbody>
              {shown.map((p) => {
                const n = newer.get(p.name.toLowerCase());
                return (
                  <tr key={`${p.name}-${p.dev}`} className="group border-t border-line/60">
                    <td className="w-full max-w-0 px-4 py-1.5">
                      <span className="flex min-w-0 items-center gap-2">
                        <span className="truncate font-mono text-[13px]" title={p.name}>
                          {p.name}
                        </span>
                        {p.dev && <span className="shrink-0 rounded bg-surface-2 px-1.5 text-[10px] font-semibold text-muted uppercase">dev</span>}
                      </span>
                    </td>
                    <td className="px-2 py-1.5 font-mono text-xs whitespace-nowrap text-muted">{p.wanted ?? "any"}</td>
                    <td className={clsx("px-2 py-1.5 font-mono text-xs whitespace-nowrap", !p.installed && "text-muted")}>{p.installed ?? (eco.installed === null ? "–" : "not installed")}</td>
                    <td className="px-2 py-1.5 font-mono text-xs whitespace-nowrap">
                      {n ? <span className="text-warn">{n.latest}</span> : checked ? <span className="text-muted">up to date</span> : <span className="text-muted">–</span>}
                    </td>
                    <td className="px-4 py-1.5">
                      <span className="flex items-center justify-end gap-1">
                        {n && canUpdate(eco.manager) && (
                          <Button size="sm" variant="ghost" onClick={() => run({ action: "update", package: p.name, dev: p.dev })} disabled={busy} title={`Update within ${p.wanted ?? "the allowed versions"}`}>
                            Update
                          </Button>
                        )}
                        {n && canUpgrade(eco.manager) && (
                          <Button size="sm" variant="ghost" onClick={() => run({ action: "upgrade", package: p.name, dev: p.dev, version: n.latest })} disabled={busy} title={`Change ${eco.manifest} to ${n.latest}`}>
                            To {n.latest}
                          </Button>
                        )}
                        {eco.canEdit && (
                          <button
                            type="button"
                            onClick={() => run({ action: "remove", package: p.name, dev: p.dev })}
                            disabled={busy}
                            className="rounded p-1 text-muted opacity-0 group-hover:opacity-100 hover:text-danger-text focus-visible:opacity-100 disabled:opacity-0"
                            aria-label={`Remove ${p.name}`}
                            title={`Remove ${p.name}`}
                          >
                            <Trash2 size={14} />
                          </button>
                        )}
                      </span>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}

      {eco.canEdit ? (
        <form
          className="flex flex-wrap items-center gap-2 border-t border-line px-4 py-3"
          onSubmit={(e) => {
            e.preventDefault();
            if (!name.trim()) return;
            run({ action: "add", package: name.trim(), dev });
            setName("");
          }}
        >
          <input className={clsx(inputClass, "min-w-48 flex-1 py-1.5 font-mono")} placeholder={addHint[eco.manager] ?? "package"} value={name} onChange={(e) => setName(e.target.value)} aria-label={`Add a ${eco.label} package`} />
          {eco.manager !== "go" && eco.manager !== "pip" && (
            <label className="flex items-center gap-1.5 text-xs text-muted">
              <input type="checkbox" checked={dev} onChange={(e) => setDev(e.target.checked)} /> Development only
            </label>
          )}
          <Button size="sm" type="submit" disabled={busy || !name.trim()}>
            <PackagePlus size={14} /> Add
          </Button>
        </form>
      ) : null}
      {eco.note && <p className="border-t border-line px-4 py-2 text-xs text-muted">{eco.note}</p>}
    </Card>
  );
}
