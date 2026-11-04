import { useEffect, useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { ChevronDown, Cpu, Play, Plus, RotateCw, Square, Trash2 } from "lucide-react";
import clsx from "clsx";
import { api, type ProjectProcess, type RestartPolicy, type Settings, type Site } from "../lib/api";
import { useAppStore } from "../lib/store";
import { AppState, isActive, OutputPanel, useAppStatuses } from "./ProjectRun";
import { Button, Card, inputClass, Switch } from "./ui";

/** What a process goes by in statuses and output: "shop/queue". */
export const processKey = (project: string, process: string) => `${project}/${process}`;

const blank: ProjectProcess = { name: "", command: "", port: null, env: {}, restart: "on-failure", withApp: true };

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
  { value: "on-failure", label: "After a crash" },
  { value: "always", label: "Whenever it exits" },
  { value: "never", label: "Never" },
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

/**
 * A project's processes: queue workers, schedulers, Vite next to PHP, an API. Each runs in
 * the project folder with its Node and LampRabit's commands, can start with the project's
 * app, restarts after a crash if asked, and has its own output. A process with a port can
 * get a domain (Domains).
 */
export default function ProjectProcesses({ site, settings }: { site: Site; settings?: Settings }) {
  const queryClient = useQueryClient();
  const notify = useAppStore((s) => s.notify);
  const statuses = useAppStatuses();
  const saved = settings?.sites[site.name]?.processes ?? [];
  const savedKey = JSON.stringify(saved);
  const [draft, setDraft] = useState<ProjectProcess[] | null>(null);
  const [open, setOpen] = useState<string | null>(null);
  // Leave the editor when what's stored changes underneath (a save, another window).
  useEffect(() => setDraft(null), [savedKey]);
  const onError = (e: unknown) => notify("error", String(e));
  const refresh = () => queryClient.invalidateQueries({ queryKey: ["settings"] });

  const save = useMutation({
    mutationFn: (list: ProjectProcess[]) => api.setSiteProcesses(site.name, list),
    onSuccess: () => {
      notify("info", `Saved ${site.name}'s processes`);
      refresh();
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
  const freePort = async (i: number) => {
    try {
      const port = await api.appFreePort();
      setDraft((d) => d && d.map((p, j) => (j === i ? { ...p, port } : p)));
    } catch (e) {
      onError(e);
    }
  };

  const editing = draft !== null;
  const offers = presets(site).filter((p) => !(draft ?? saved).some((s) => s.name === p.name));
  const anyRunning = saved.some((p) => isActive(statuses[processKey(site.name, p.name)]));
  const update = (i: number, patch: Partial<ProjectProcess>) => setDraft((d) => d && d.map((p, j) => (j === i ? { ...p, ...patch } : p)));

  return (
    <Card className="overflow-hidden">
      <div className="flex flex-wrap items-center gap-3 border-b border-line px-5 py-3">
        <Cpu size={16} className="text-muted" aria-hidden />
        <div className="min-w-0 flex-1">
          <h3 className="text-sm font-bold">Processes</h3>
          <p className="text-xs text-muted">Long-running commands next to the app: queue workers, schedulers, Vite, an API.</p>
        </div>
        {editing ? (
          <>
            <Button size="sm" variant="ghost" onClick={() => setDraft(null)} disabled={save.isPending}>
              Cancel
            </Button>
            <Button size="sm" variant="primary" onClick={() => save.mutate(draft)} disabled={save.isPending || JSON.stringify(draft) === savedKey}>
              {save.isPending ? "Saving…" : "Save"}
            </Button>
          </>
        ) : (
          <>
            {saved.length > 0 && (
              <Button size="sm" variant="ghost" onClick={() => all.mutate(!anyRunning)} disabled={all.isPending}>
                {anyRunning ? <Square size={11} className="fill-current" /> : <Play size={12} />} {anyRunning ? "Stop all" : "Start all"}
              </Button>
            )}
            <Button size="sm" variant="soft" onClick={() => setDraft(saved.length ? saved : [{ ...blank }])}>
              {saved.length ? "Edit" : <><Plus size={14} /> Add a process</>}
            </Button>
          </>
        )}
      </div>

      {!editing && saved.length === 0 && (
        <p className="px-5 py-3 text-sm text-muted">No processes yet. They start with the project's Run button, or on their own here.</p>
      )}

      {!editing && saved.length > 0 && (
        <ul className="divide-y divide-line">
          {saved.map((p) => {
            const key = processKey(site.name, p.name);
            const status = statuses[key];
            const active = isActive(status);
            return (
              <li key={p.name}>
                <div className="flex flex-wrap items-center gap-3 px-5 py-3">
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2 text-sm font-semibold">
                      {p.name}
                      <AppState status={status} />
                    </div>
                    <div className="truncate font-mono text-xs text-muted" title={p.command}>
                      {p.command}
                      {p.port != null && ` · port ${p.port}`}
                      {p.withApp ? " · with the app" : ""}
                    </div>
                  </div>
                  {active ? (
                    <>
                      <Button size="sm" onClick={() => act.mutate({ name: p.name, what: "stop" })} aria-label={`Stop ${p.name}`}>
                        <Square size={11} className="fill-current" /> Stop
                      </Button>
                      <Button size="sm" variant="ghost" onClick={() => act.mutate({ name: p.name, what: "restart" })} aria-label={`Restart ${p.name}`}>
                        <RotateCw size={14} />
                      </Button>
                    </>
                  ) : (
                    <Button size="sm" variant="soft" onClick={() => act.mutate({ name: p.name, what: "start" })} aria-label={`Start ${p.name}`}>
                      <Play size={12} /> Start
                    </Button>
                  )}
                  <Button size="sm" variant="ghost" onClick={() => setOpen(open === key ? null : key)} aria-expanded={open === key} aria-label={`Output of ${p.name}`}>
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
        </ul>
      )}

      {editing && (
        <ul className="divide-y divide-line">
          {draft.map((p, i) => (
            <li key={i} className="space-y-2.5 px-5 py-3">
              <div className="flex flex-wrap items-center gap-2">
                <input
                  className={clsx(inputClass.replace("w-full", ""), "w-36 font-mono")}
                  value={p.name}
                  placeholder="queue"
                  onChange={(e) => update(i, { name: e.target.value.toLowerCase().replace(/[^a-z0-9-]/g, "-") })}
                  aria-label="Name"
                  data-autofocus={i === draft.length - 1 && !p.name ? true : undefined}
                />
                <input
                  className={clsx(inputClass, "min-w-56 flex-1 font-mono")}
                  value={p.command}
                  placeholder="php artisan queue:work"
                  onChange={(e) => update(i, { command: e.target.value })}
                  aria-label={`Command for ${p.name || "this process"}`}
                />
                <Button size="sm" variant="ghost" onClick={() => setDraft(draft.filter((_, j) => j !== i))} aria-label={`Remove ${p.name || "this process"}`}>
                  <Trash2 size={14} />
                </Button>
              </div>
              <div className="flex flex-wrap items-center gap-x-5 gap-y-2 text-sm">
                <label className="flex items-center gap-2">
                  <span className="text-muted">Port</span>
                  <input
                    type="number"
                    min={1024}
                    max={65535}
                    className={clsx(inputClass.replace("w-full", ""), "w-24 font-mono")}
                    value={p.port ?? ""}
                    placeholder="none"
                    onChange={(e) => update(i, { port: e.target.value ? Number(e.target.value) : null })}
                  />
                  <button type="button" className="text-xs font-semibold text-accent-text hover:underline" onClick={() => freePort(i)}>
                    Pick a free one
                  </button>
                </label>
                <label className="flex items-center gap-2">
                  <span className="text-muted">Restart</span>
                  <select className={clsx(inputClass.replace("w-full", ""), "w-44")} value={p.restart} onChange={(e) => update(i, { restart: e.target.value as RestartPolicy })}>
                    {policies.map((o) => (
                      <option key={o.value} value={o.value}>
                        {o.label}
                      </option>
                    ))}
                  </select>
                </label>
                <span className="flex items-center gap-2">
                  <Switch checked={p.withApp} onChange={(v) => update(i, { withApp: v })} label={`Start ${p.name || "this process"} with the app`} />
                  <span className="text-muted">With the app</span>
                </span>
              </div>
              <label className="block">
                <span className="mb-1 block text-xs text-muted">Environment, one KEY=value per line ({"{port}"} in the command and $PORT are the port)</span>
                <textarea
                  className={clsx(inputClass, "min-h-14 font-mono")}
                  rows={2}
                  value={envText(p.env)}
                  onChange={(e) => update(i, { env: envFrom(e.target.value) })}
                  aria-label={`Environment for ${p.name || "this process"}`}
                />
              </label>
            </li>
          ))}
          <li className="flex flex-wrap items-center gap-2 px-5 py-3">
            <Button size="sm" variant="soft" onClick={() => setDraft([...draft, { ...blank }])}>
              <Plus size={14} /> Add a process
            </Button>
            {offers.map((o) => (
              <Button key={o.name} size="sm" variant="ghost" onClick={() => setDraft([...draft, o])} title={o.command}>
                <Plus size={14} /> {o.name}
              </Button>
            ))}
          </li>
        </ul>
      )}
    </Card>
  );
}
