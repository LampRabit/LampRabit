import { useEffect, useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { openUrl } from "@tauri-apps/plugin-opener";
import { CornerDownRight, ExternalLink, Globe, Plus, Trash2 } from "lucide-react";
import clsx from "clsx";
import { api, type DomainTarget, type Settings, type Site } from "../lib/api";
import { useAppStore } from "../lib/store";
import DomainTargetPicker, { targetHint } from "./DomainTargetPicker";
import { Button, Card } from "./ui";

/** A subdomain being edited: what's typed before the project's domain, and where it goes. */
interface Row {
  label: string;
  target: DomainTarget;
}

const automatic: DomainTarget = { kind: "main" };
const LABEL = /^[a-z0-9]([a-z0-9-]*[a-z0-9])?(\.[a-z0-9]([a-z0-9-]*[a-z0-9])?)*$/;

/** The site's URL with another host: the same scheme and port as the main domain. */
const urlFor = (site: Site, host: string) => site.url.replace(site.domain, host);

/** "public/", or "the project folder". */
function folderOf(site: Site) {
  const rest = site.docroot.slice(site.path.length).replace(/^[\\/]+/, "").replace(/\\/g, "/");
  return rest ? `${rest}/` : "the project folder";
}

/** Why a row can't be saved yet, or null. */
function problem(row: Row, rows: Row[]): string | null {
  if (!row.label) return "Type the subdomain, like admin or api.";
  if (!LABEL.test(row.label)) return "Use lowercase letters, digits, dashes and dots.";
  if (rows.filter((r) => r.label === row.label).length > 1) return "This subdomain is listed twice.";
  if (row.target.kind === "port" && !(row.target.port >= 1 && row.target.port <= 65535)) return "Enter a port between 1 and 65535.";
  if (row.target.kind === "process" && !row.target.name) return "Pick a process.";
  return null;
}

/**
 * Where the project's main domain goes (automatically, or set by hand) and its subdomains
 * (admin.shop.test), each pointing at its files or a folder, a port or one of its processes.
 * Saved together; the web server picks them up with new certificates.
 */
export default function ProjectDomains({ site, settings }: { site: Site; settings?: Settings }) {
  const queryClient = useQueryClient();
  const notify = useAppStore((s) => s.notify);
  const own = settings?.sites[site.name];
  const suffix = `.${site.domain}`;
  const processes = (own?.processes ?? []).filter((p) => p.port != null).map((p) => p.name);

  const savedMain = own?.main ?? automatic;
  const savedRows: Row[] = (own?.domains ?? []).map((d) => ({ label: d.host.endsWith(suffix) ? d.host.slice(0, -suffix.length) : d.host, target: d.target }));
  const savedKey = JSON.stringify([savedMain, savedRows]);
  const [main, setMain] = useState<DomainTarget>(savedMain);
  const [rows, setRows] = useState<Row[]>(savedRows);
  // A save (or a change elsewhere) resets the form to what's stored.
  useEffect(() => {
    const [m, r] = JSON.parse(savedKey) as [DomainTarget, Row[]];
    setMain(m);
    setRows(r);
  }, [savedKey]);

  const mainChanged = JSON.stringify(main) !== JSON.stringify(savedMain);
  const rowsChanged = JSON.stringify(rows) !== JSON.stringify(savedRows);
  const problems = rows.map((r) => problem(r, rows));
  const mainProblem = problem({ label: "main", target: main }, []);
  const valid = problems.every((p) => !p) && !mainProblem;

  const save = useMutation({
    mutationFn: async () => {
      if (mainChanged) await api.setSiteMain(site.name, main.kind === "main" ? null : main);
      if (rowsChanged) await api.setSiteDomains(site.name, rows.map((r) => ({ host: `${r.label}${suffix}`, target: r.target })));
    },
    onSuccess: () => {
      notify("info", `Saved ${site.name}'s domains`);
      for (const key of ["settings", "sites", "services"]) queryClient.invalidateQueries({ queryKey: [key] });
    },
    onError: (e) => notify("error", String(e)),
  });

  const update = (i: number, patch: Partial<Row>) => setRows(rows.map((r, j) => (j === i ? { ...r, ...patch } : r)));
  const open = (host: string) => openUrl(urlFor(site, host)).catch((e) => notify("error", String(e)));
  const dirty = mainChanged || rowsChanged;
  const now = site.route ? `port ${site.route.port}${savedMain.kind === "main" ? " (the dev server)" : ""}` : `its files in ${folderOf(site)}`;
  const savedLabels = new Set(savedRows.map((r) => r.label));

  return (
    <Card className="overflow-hidden">
      <div className="flex flex-wrap items-center gap-3 border-b border-line px-5 py-3">
        <Globe size={16} className="text-muted" aria-hidden />
        <div className="min-w-0 flex-1">
          <h3 className="text-sm font-bold">Domains</h3>
          <p className="text-xs text-muted">Where {site.domain} goes, and subdomains for parts of the project.</p>
        </div>
        {dirty && (
          <Button size="sm" variant="ghost" onClick={() => (setMain(savedMain), setRows(savedRows))} disabled={save.isPending}>
            Discard
          </Button>
        )}
        <Button size="sm" variant="primary" onClick={() => save.mutate()} disabled={!dirty || !valid || save.isPending}>
          {save.isPending ? "Saving…" : "Save"}
        </Button>
      </div>

      {/* The main domain */}
      <div className="space-y-2 border-b border-line bg-surface-2/40 px-5 py-4">
        <div className="flex flex-wrap items-center gap-2">
          <span className="rounded-md bg-accent-soft px-1.5 py-0.5 text-[11px] font-bold tracking-wide text-accent-text uppercase">Main</span>
          <span className="min-w-0 flex-1 truncate font-mono text-sm font-semibold">{site.domain}</span>
          <Button size="sm" variant="ghost" onClick={() => open(site.domain)} aria-label={`Open ${site.domain}`}>
            <ExternalLink size={14} />
          </Button>
        </div>
        <DomainTargetPicker project={site.name} value={main} onChange={setMain} processes={processes} automatic label={`Where ${site.domain} goes`} />
        <p className="text-xs text-muted">
          {targetHint(main, true)} {!mainChanged && <span className="text-fg">Now goes to {now}.</span>}
        </p>
        {mainProblem && <p className="text-xs text-danger-text">{mainProblem}</p>}
      </div>

      {/* Subdomains */}
      {rows.length === 0 ? (
        <p className="px-5 py-4 text-sm text-muted">
          No subdomains yet. Add one for an admin area, an API or a second app in this project, like <span className="font-mono">admin{suffix}</span>.
        </p>
      ) : (
        <ul className="divide-y divide-line">
          {rows.map((r, i) => {
            const host = `${r.label}${suffix}`;
            return (
              <li key={i} className="space-y-2 px-5 py-4">
                <div className="flex flex-wrap items-center gap-2">
                  <CornerDownRight size={15} className="shrink-0 text-muted" aria-hidden />
                  <div className={clsx("flex min-w-56 flex-1 items-stretch overflow-hidden rounded-lg border bg-bg focus-within:ring-2 focus-within:ring-accent/25", problems[i] && r.label ? "border-danger" : "border-line focus-within:border-accent")}>
                    <input
                      className="min-w-0 flex-1 bg-transparent px-3 py-2 text-right font-mono text-sm outline-none"
                      value={r.label}
                      placeholder="admin"
                      onChange={(e) => update(i, { label: e.target.value.toLowerCase().replace(/[^a-z0-9.-]/g, "") })}
                      aria-label={`Subdomain of ${site.domain}`}
                      data-autofocus={i === rows.length - 1 && !r.label ? true : undefined}
                    />
                    <span className="flex items-center border-l border-line bg-surface-2 px-3 font-mono text-sm text-muted select-none">{suffix}</span>
                  </div>
                  {savedLabels.has(r.label) && !rowsChanged && (
                    <Button size="sm" variant="ghost" onClick={() => open(host)} aria-label={`Open ${host}`}>
                      <ExternalLink size={14} />
                    </Button>
                  )}
                  <Button size="sm" variant="ghost" onClick={() => setRows(rows.filter((_, j) => j !== i))} aria-label={`Remove ${r.label ? host : "this subdomain"}`}>
                    <Trash2 size={14} />
                  </Button>
                </div>
                <div className="pl-6">
                  <DomainTargetPicker project={site.name} value={r.target} onChange={(target) => update(i, { target })} processes={processes} automatic={false} label={`Where ${host} goes`} />
                  <p className={clsx("mt-1.5 text-xs", problems[i] ? "text-danger-text" : "text-muted")}>{problems[i] ?? targetHint(r.target, false)}</p>
                </div>
              </li>
            );
          })}
        </ul>
      )}

      <div className="flex flex-wrap items-center gap-3 border-t border-line px-5 py-3">
        <Button size="sm" variant="soft" onClick={() => setRows([...rows, { label: "", target: { kind: "main" } }])}>
          <Plus size={14} /> Add a subdomain
        </Button>
        {processes.length === 0 && <span className="text-xs text-muted">Give a process a port (Processes tab) to point a domain at it.</span>}
      </div>
    </Card>
  );
}
