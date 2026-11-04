import { useEffect, useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { openUrl } from "@tauri-apps/plugin-opener";
import { ExternalLink, Globe, Plus, Trash2 } from "lucide-react";
import clsx from "clsx";
import { api, type DomainTarget, type ExtraDomain, type Settings, type Site } from "../lib/api";
import { useAppStore } from "../lib/store";
import { Button, Card, inputClass } from "./ui";

type Kind = DomainTarget["kind"];

const kinds: { value: Kind; label: string; hint: string }[] = [
  { value: "main", label: "Same as main", hint: "Wherever the main domain goes: its files, or its dev server while that runs." },
  { value: "files", label: "Files", hint: "A folder in the project, served by the web server (PHP included)." },
  { value: "port", label: "Port", hint: "A local port, whoever listens on it." },
  { value: "process", label: "Process", hint: "One of the project's processes, at its port." },
];

/** A target of the given kind, keeping what fits from the one before. */
function retarget(kind: Kind, processes: string[]): DomainTarget {
  switch (kind) {
    case "main":
      return { kind };
    case "files":
      return { kind, root: "" };
    case "port":
      return { kind, port: 3001 };
    case "process":
      return { kind, name: processes[0] ?? "" };
  }
}

/** The site's URL with another host: same scheme and port as the main domain. */
const urlFor = (site: Site, host: string) => site.url.replace(site.domain, host);

/**
 * A project's extra domains (admin.shop.test, api.shop.test), each pointing like the main
 * one can: at its files (or a folder in it), a port, one of its processes, or wherever the
 * main domain goes. Saved together; the web server picks them up with new certificates.
 */
export default function ProjectDomains({ site, settings }: { site: Site; settings?: Settings }) {
  const queryClient = useQueryClient();
  const notify = useAppStore((s) => s.notify);
  const own = settings?.sites[site.name];
  const saved = own?.domains ?? [];
  const processes = (own?.processes ?? []).filter((p) => p.port != null).map((p) => p.name);
  const [draft, setDraft] = useState<ExtraDomain[]>(saved);
  const savedKey = JSON.stringify(saved);
  // A save (or a change elsewhere) resets the form to what's stored.
  useEffect(() => setDraft(JSON.parse(savedKey)), [savedKey]);
  const dirty = JSON.stringify(draft) !== savedKey;
  const tld = settings?.tld ?? "test";

  const save = useMutation({
    mutationFn: () => api.setSiteDomains(site.name, draft),
    onSuccess: () => {
      notify("info", draft.length ? `Saved ${draft.length} domain${draft.length === 1 ? "" : "s"} for ${site.name}` : `Removed ${site.name}'s extra domains`);
      for (const key of ["settings", "sites", "services"]) queryClient.invalidateQueries({ queryKey: [key] });
    },
    onError: (e) => notify("error", String(e)),
  });

  const update = (i: number, patch: Partial<ExtraDomain>) => setDraft(draft.map((d, j) => (j === i ? { ...d, ...patch } : d)));
  const savedHosts = new Set(saved.map((d) => d.host));
  const main = site.route ? `port ${site.route.port}` : "its files";

  return (
    <Card className="overflow-hidden">
      <div className="flex flex-wrap items-center gap-3 border-b border-line px-5 py-3">
        <Globe size={16} className="text-muted" aria-hidden />
        <div className="min-w-0 flex-1">
          <h3 className="text-sm font-bold">Domains</h3>
          <p className="text-xs text-muted">More addresses for this project, each pointing at its files, a port or one of its processes.</p>
        </div>
        {dirty && (
          <Button size="sm" variant="ghost" onClick={() => setDraft(saved)} disabled={save.isPending}>
            Discard
          </Button>
        )}
        <Button size="sm" variant="primary" onClick={() => save.mutate()} disabled={!dirty || save.isPending}>
          {save.isPending ? "Saving…" : "Save"}
        </Button>
      </div>

      <ul className="divide-y divide-line">
        <li className="flex flex-wrap items-center gap-3 px-5 py-3">
          <span className="min-w-0 flex-1 truncate font-mono text-sm">{site.domain}</span>
          <span className="text-xs text-muted">Main domain · goes to {main}</span>
        </li>
        {draft.map((d, i) => {
          const kind = kinds.find((k) => k.value === d.target.kind)!;
          return (
            <li key={i} className="space-y-2 px-5 py-3">
              <div className="flex flex-wrap items-center gap-2">
                <input
                  className={clsx(inputClass, "min-w-48 flex-1 font-mono")}
                  value={d.host}
                  placeholder={`admin.${site.domain}`}
                  onChange={(e) => update(i, { host: e.target.value.toLowerCase().trim() })}
                  aria-label="Domain"
                  data-autofocus={i === draft.length - 1 && !d.host ? true : undefined}
                />
                <select
                  className={clsx(inputClass.replace("w-full", ""), "w-40")}
                  value={d.target.kind}
                  onChange={(e) => update(i, { target: retarget(e.target.value as Kind, processes) })}
                  aria-label={`Where ${d.host || "this domain"} goes`}
                >
                  {kinds.map((k) => (
                    <option key={k.value} value={k.value} disabled={k.value === "process" && processes.length === 0}>
                      {k.label}
                    </option>
                  ))}
                </select>
                {d.target.kind === "files" && (
                  <input
                    className={clsx(inputClass.replace("w-full", ""), "w-48 font-mono")}
                    value={d.target.root}
                    placeholder="(the main domain's folder)"
                    onChange={(e) => update(i, { target: { kind: "files", root: e.target.value.replace(/\\/g, "/").replace(/^\/+/, "") } })}
                    aria-label="Folder in the project"
                  />
                )}
                {d.target.kind === "port" && (
                  <input
                    type="number"
                    min={1}
                    max={65535}
                    className={clsx(inputClass.replace("w-full", ""), "w-28 font-mono")}
                    value={d.target.port}
                    onChange={(e) => update(i, { target: { kind: "port", port: Number(e.target.value) } })}
                    aria-label="Port"
                  />
                )}
                {d.target.kind === "process" && (
                  <select
                    className={clsx(inputClass.replace("w-full", ""), "w-40")}
                    value={d.target.name}
                    onChange={(e) => update(i, { target: { kind: "process", name: e.target.value } })}
                    aria-label="Process"
                  >
                    {processes.map((p) => (
                      <option key={p} value={p}>
                        {p}
                      </option>
                    ))}
                  </select>
                )}
                {savedHosts.has(d.host) && !dirty && (
                  <Button size="sm" variant="ghost" onClick={() => openUrl(urlFor(site, d.host)).catch((e) => notify("error", String(e)))} aria-label={`Open ${d.host}`}>
                    <ExternalLink size={14} />
                  </Button>
                )}
                <Button size="sm" variant="ghost" onClick={() => setDraft(draft.filter((_, j) => j !== i))} aria-label={`Remove ${d.host || "this domain"}`}>
                  <Trash2 size={14} />
                </Button>
              </div>
              <p className="text-xs text-muted">{kind.hint}</p>
            </li>
          );
        })}
      </ul>

      <div className="border-t border-line px-5 py-3">
        <Button size="sm" variant="soft" onClick={() => setDraft([...draft, { host: "", target: { kind: "main" } }])}>
          <Plus size={14} /> Add a domain
        </Button>
        <p className="mt-2 text-xs text-muted">
          Domains end in .{tld}. Processes appear here once they have a port (Processes tab).
        </p>
      </div>
    </Card>
  );
}
