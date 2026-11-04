import { useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { ChevronDown, Cpu, Pencil, Play, Plus, RotateCw, Sparkles, Square, Trash2 } from "lucide-react";
import clsx from "clsx";
import { api, type ProjectProcess, type RestartPolicy, type Settings, type Site } from "../lib/api";
import { useAppStore } from "../lib/store";
import { FolderSelect } from "./DomainTargetPicker";
import { AppState, isActive, OutputPanel, useAppStatuses } from "./ProjectRun";
import { Button, Card, inputClass, Switch } from "./ui";

/** What a process goes by in statuses and output: "shop/queue". */
export const processKey = (project: string, process: string) => `${project}/${process}`;

const blank: ProjectProcess = { name: "", command: "", cwd: "", port: null, env: {}, restart: "on-failure", withApp: true };

/** Common long-running commands for the project's framework, offered as one click. */
function presets(site: Site): ProjectProcess[] {
  const p = (name: string, command: string, port: number | null = null): ProjectProcess => ({ ...blank, name, command, port });
  const list: ProjectProcess[] = [];
  if (site.framework === "Laravel") {
    list.push(p("queue", "php artisan queue:work --tries=3"), p("scheduler", "php artisan schedule:work"));
  }
  if (site.framework === "Symfony") list.push(p("messenger", "php bin/console messenger:consume async -vv"));
  // PHP projects' package.json dev script is their asset server (Vite).
  if (site.nodeProject && site.language === "php") list.push(p("vite", "npm run dev"));
  return list;
}

const policies: { value: RestartPolicy; label: string }[] = [
  { value: "on-failure", label: "Restart after a crash" },
  { value: "always", label: "Restart whenever it exits" },
  { value: "never", label: "Don't restart" },
];

const envText = (env: Record<string, string>) =>
  Object.entries(env)
    .map(([k, v]) => `${k}=${v}`)
    .join("\n");
const envFrom = (text: string) =>
  Object.fromEntries(
    text
      .split("\n")
      .map((l) => l.trim())
      .filter((l) => l && !l.startsWith("#") && l.includes("="))
      .map((l) => [l.slice(0, l.indexOf("=")).trim(), l.slice(l.indexOf("=") + 1).trim()]),
  );

const narrow = (width: string) => clsx(inputClass.replace("w-full", ""), width);

/** Why a process can't be saved yet, or null. */
function problem(p: ProjectProcess, others: ProjectProcess[]): string | null {
  if (!p.name) return "Give it a name, like queue or vite.";
  if (!/^[a-z0-9][a-z0-9-]*$/.test(p.name)) return "Names use lowercase letters, digits and dashes.";
  if (others.some((o) => o.name === p.name)) return `There's already a process called ${p.name}.`;
  if (!p.command.trim()) return "Enter the command it runs.";
  if (p.port != null && !(p.port >= 1024 && p.port <= 65535)) return "Ports go from 1024 to 65535.";
  if (p.port != null && others.some((o) => o.port === p.port)) return `Port ${p.port} is another process's.`;
  return null;
}

/** One process's settings; `onSave` gets the edited process. */
function ProcessForm({ site, initial, others, busy, onSave, onCancel, onDelete }: {
  site: Site;
  initial: ProjectProcess;
  others: ProjectProcess[];
  busy: boolean;
  onSave: (p: ProjectProcess) => void;
  onCancel: () => void;
  onDelete?: () => void;
}) {
  const notify = useAppStore((s) => s.notify);
  const [p, setP] = useState(initial);
  const [env, setEnv] = useState(envText(initial.env));
  const set = (patch: Partial<ProjectProcess>) => setP({ ...p, ...patch });
  const issue = problem(p, others);
  const freePort = () => api.appFreePort().then((port) => set({ port }), (e) => notify("error", String(e)));

  return (
    <div className="space-y-3 rounded-xl border border-line bg-surface-2/40 p-4">
      <div className="grid gap-3 sm:grid-cols-[10rem_minmax(0,1fr)]">
        <label className="block">
          <span className="mb-1 block text-xs font-semibold text-muted">Name</span>
          <input
            className={clsx(inputClass, "font-mono")}
            value={p.name}
            placeholder="queue"
            onChange={(e) => set({ name: e.target.value.toLowerCase().replace(/[^a-z0-9-]/g, "-") })}
            data-autofocus={!initial.name ? true : undefined}
          />
        </label>
        <label className="block">
          <span className="mb-1 block text-xs font-semibold text-muted">Command</span>
          <input className={clsx(inputClass, "font-mono")} value={p.command} placeholder="php artisan queue:work" onChange={(e) => set({ command: e.target.value })} />
        </label>
      </div>
      <div className="flex flex-wrap items-end gap-x-5 gap-y-3">
        <label className="block">
          <span className="mb-1 block text-xs font-semibold text-muted">Runs in</span>
          <FolderSelect project={site.name} value={p.cwd} onChange={(cwd) => set({ cwd })} none="Project folder" label="Folder it runs in" />
        </label>
        <div>
          <span className="mb-1 block text-xs font-semibold text-muted">Port (if it serves anything)</span>
          <div className="flex items-center gap-2">
            <input
              type="number"
              min={1024}
              max={65535}
              className={narrow("w-28 font-mono")}
              value={p.port ?? ""}
              placeholder="None"
              onChange={(e) => set({ port: e.target.value ? Number(e.target.value) : null })}
              aria-label="Port"
            />
            <Button size="sm" variant="ghost" onClick={freePort}>
              Pick a free one
            </Button>
          </div>
        </div>
        <label className="block">
          <span className="mb-1 block text-xs font-semibold text-muted">When it exits</span>
          <select className={narrow("w-52")} value={p.restart} onChange={(e) => set({ restart: e.target.value as RestartPolicy })}>
            {policies.map((o) => (
              <option key={o.value} value={o.value}>
                {o.label}
              </option>
            ))}
          </select>
        </label>
        <span className="flex h-9 items-center gap-2 text-sm">
          <Switch checked={p.withApp} onChange={(v) => set({ withApp: v })} label="Start with the project" />
          Start with the project
        </span>
      </div>
      <label className="block">
        <span className="mb-1 block text-xs font-semibold text-muted">Environment (one KEY=value per line; the port is also $PORT and {"{port}"} in the command)</span>
        <textarea className={clsx(inputClass, "min-h-14 font-mono")} rows={2} value={env} onChange={(e) => setEnv(e.target.value)} placeholder="QUEUE_CONNECTION=database" />
      </label>
      <div className="flex flex-wrap items-center gap-2">
        {issue ? <span className="text-xs text-danger-text">{issue}</span> : <span className="text-xs text-muted">Saving doesn't restart it; use Restart to apply changes.</span>}
        <span className="flex-1" />
        {onDelete && (
          <Button size="sm" variant="ghost" onClick={onDelete} disabled={busy} className="text-danger-text">
            <Trash2 size={14} /> Delete
          </Button>
        )}
        <Button size="sm" variant="ghost" onClick={onCancel} disabled={busy}>
          Cancel
        </Button>
        <Button size="sm" variant="primary" onClick={() => onSave({ ...p, env: envFrom(env) })} disabled={busy || !!issue}>
          {busy ? "Saving…" : "Save"}
        </Button>
      </div>
    </div>
  );
}

/**
 * A project's processes: queue workers, schedulers, Vite next to PHP, an API. Each runs in
 * a folder of the project with its Node and LampRabit's commands, can start with the
 * project, restarts after a crash if asked, and has its own output. One with a port can
 * get a subdomain (Domains).
 */
export default function ProjectProcesses({ site, settings }: { site: Site; settings?: Settings }) {
  const queryClient = useQueryClient();
  const notify = useAppStore((s) => s.notify);
  const statuses = useAppStatuses();
  const own = settings?.sites[site.name];
  const saved = own?.processes ?? [];
  // The process being edited (its name), "+" for a new one; and its preset, if any.
  const [editing, setEditing] = useState<string | null>(null);
  const [preset, setPreset] = useState<ProjectProcess | null>(null);
  const [open, setOpen] = useState<string | null>(null);
  const onError = (e: unknown) => notify("error", String(e));

  const save = useMutation({
    mutationFn: (list: ProjectProcess[]) => api.setSiteProcesses(site.name, list),
    onSuccess: () => {
      setEditing(null);
      setPreset(null);
      queryClient.invalidateQueries({ queryKey: ["settings"] });
    },
    onError,
  });
  const act = useMutation({
    mutationFn: async ({ name, what }: { name: string; what: "start" | "stop" | "restart" }) => {
      if (what === "stop") await api.processStop(site.name, name);
      else await (what === "start" ? api.processStart(site.name, name) : api.processRestart(site.name, name));
    },
    onError,
  });
  const all = useMutation({ mutationFn: (start: boolean) => api.processesAll(site.name, start), onError });

  const offers = presets(site).filter((p) => !saved.some((s) => s.name === p.name));
  const anyRunning = saved.some((p) => isActive(statuses[processKey(site.name, p.name)]));
  // Subdomains that point at each process, shown on its row.
  const domainsOf = (name: string) =>
    (own?.domains ?? []).filter((d) => d.target.kind === "process" && d.target.name === name).map((d) => d.host);
  const startNew = (p: ProjectProcess) => {
    setPreset(p);
    setEditing("+");
  };

  return (
    <Card className="overflow-hidden">
      <div className="flex flex-wrap items-center gap-3 border-b border-line px-5 py-3">
        <Cpu size={16} className="text-muted" aria-hidden />
        <div className="min-w-0 flex-1">
          <h3 className="text-sm font-bold">Processes</h3>
          <p className="text-xs text-muted">Long-running commands next to the site: queue workers, schedulers, Vite, an API.</p>
        </div>
        {saved.length > 0 && (
          <Button size="sm" variant="ghost" onClick={() => all.mutate(!anyRunning)} disabled={all.isPending}>
            {anyRunning ? <Square size={11} className="fill-current" /> : <Play size={12} className="fill-current" />} {anyRunning ? "Stop all" : "Start all"}
          </Button>
        )}
        <Button size="sm" variant="soft" onClick={() => startNew({ ...blank })} disabled={editing !== null}>
          <Plus size={14} /> Add a process
        </Button>
      </div>

      {saved.length === 0 && editing === null && (
        <p className="px-5 py-4 text-sm text-muted">No processes yet. They run next to the site, start with it if you like, and show their output here.</p>
      )}

      <ul className="divide-y divide-line">
        {saved.map((p) => {
          const key = processKey(site.name, p.name);
          const status = statuses[key];
          const active = isActive(status);
          const domains = domainsOf(p.name);
          if (editing === p.name) {
            return (
              <li key={p.name} className="px-5 py-4">
                <ProcessForm
                  site={site}
                  initial={p}
                  others={saved.filter((o) => o.name !== p.name)}
                  busy={save.isPending}
                  onSave={(next) => save.mutate(saved.map((o) => (o.name === p.name ? next : o)))}
                  onCancel={() => setEditing(null)}
                  onDelete={() => save.mutate(saved.filter((o) => o.name !== p.name))}
                />
              </li>
            );
          }
          return (
            <li key={p.name}>
              <div className="flex flex-wrap items-center gap-3 px-5 py-3">
                <div className="min-w-0 flex-1">
                  <div className="flex flex-wrap items-center gap-2 text-sm font-semibold">
                    {p.name}
                    <AppState status={status} />
                  </div>
                  <div className="truncate font-mono text-xs text-muted" title={p.command}>
                    {p.command}
                  </div>
                  <div className="mt-1 flex flex-wrap gap-1.5 text-[11px] text-muted">
                    {p.cwd && <span className="rounded bg-surface-2 px-1.5 py-0.5 font-mono">{p.cwd}/</span>}
                    {p.port != null && <span className="rounded bg-surface-2 px-1.5 py-0.5 font-mono">:{p.port}</span>}
                    {domains.map((d) => (
                      <span key={d} className="rounded bg-accent-soft px-1.5 py-0.5 font-mono text-accent-text">
                        {d}
                      </span>
                    ))}
                    <span className="rounded bg-surface-2 px-1.5 py-0.5">{policies.find((o) => o.value === p.restart)?.label}</span>
                    {p.withApp && <span className="rounded bg-surface-2 px-1.5 py-0.5">Starts with the project</span>}
                  </div>
                </div>
                {active ? (
                  <>
                    <Button size="sm" variant="ghost" onClick={() => act.mutate({ name: p.name, what: "restart" })} aria-label={`Restart ${p.name}`} title="Restart">
                      <RotateCw size={14} />
                    </Button>
                    <Button size="sm" onClick={() => act.mutate({ name: p.name, what: "stop" })} aria-label={`Stop ${p.name}`}>
                      <Square size={11} className="fill-current" /> Stop
                    </Button>
                  </>
                ) : (
                  <Button size="sm" variant="soft" onClick={() => act.mutate({ name: p.name, what: "start" })} aria-label={`Start ${p.name}`}>
                    <Play size={12} className="fill-current" /> Start
                  </Button>
                )}
                <Button size="sm" variant="ghost" onClick={() => setEditing(p.name)} disabled={editing !== null} aria-label={`Edit ${p.name}`} title="Edit">
                  <Pencil size={14} />
                </Button>
                <Button size="sm" variant="ghost" onClick={() => setOpen(open === key ? null : key)} aria-expanded={open === key} aria-label={`Output of ${p.name}`} title="Output">
                  <ChevronDown size={14} className={clsx("transition-transform", open === key && "rotate-180")} />
                </Button>
              </div>
              {open === key && (
                <div className="px-5 pb-3">
                  <OutputPanel name={key} height="max-h-72 min-h-24" />
                </div>
              )}
            </li>
          );
        })}
        {editing === "+" && preset && (
          <li className="px-5 py-4">
            <ProcessForm
              site={site}
              initial={preset}
              others={saved}
              busy={save.isPending}
              onSave={(next) => save.mutate([...saved, next])}
              onCancel={() => (setEditing(null), setPreset(null))}
            />
          </li>
        )}
      </ul>

      {offers.length > 0 && editing === null && (
        <div className="flex flex-wrap items-center gap-2 border-t border-line px-5 py-3">
          <span className="flex items-center gap-1.5 text-xs font-semibold text-muted">
            <Sparkles size={13} aria-hidden /> Suggested for {site.framework ?? "this project"}
          </span>
          {offers.map((o) => (
            <Button key={o.name} size="sm" variant="ghost" onClick={() => startNew(o)} title={o.command}>
              <Plus size={13} /> {o.name}
            </Button>
          ))}
        </div>
      )}
    </Card>
  );
}
