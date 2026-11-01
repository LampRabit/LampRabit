import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Download, Link2 } from "lucide-react";
import { api, runtimeLabels, type InstalledRuntime, type Runtime } from "../lib/api";
import { pickRelease, related } from "../lib/related";
import { installKey, useAppStore } from "../lib/store";
import { runtimeTiles } from "./runtimes";
import { Button, Card } from "./ui";

/**
 * What a service needs and what goes well with it, each with an install button and one
 * "Install all". Things already installed aren't listed; the panel hides when nothing is left.
 */
export default function RelatedPanel({ runtime, installed, onSelect }: { runtime: Runtime; installed?: InstalledRuntime[]; onSelect: (r: Runtime) => void }) {
  const queryClient = useQueryClient();
  const notify = useAppStore((s) => s.notify);
  const installs = useAppStore((s) => s.installs);
  const has = (r: Runtime) => !!installed?.find((i) => i.runtime === r)?.active;
  const entry = related[runtime] ?? {};
  const needs = (entry.needs ?? []).filter((r) => !has(r));
  // Companions only make sense once the service itself is there.
  const suggests = has(runtime) ? (entry.suggests ?? []).filter((r) => !has(r)) : [];

  const install = useMutation({
    // One at a time, needs first, so pgAdmin finds its Python already installed.
    mutationFn: async (list: Runtime[]) => {
      for (const r of list) {
        const release = pickRelease(r, await api.runtimeCatalog(r));
        if (!release) throw new Error(`No ${runtimeLabels[r]} download for this platform`);
        await api.installRuntime(release);
      }
    },
    onSuccess: (_, list) => notify("info", `Installed ${list.map((r) => runtimeLabels[r]).join(", ")}`),
    onError: (e) => notify("error", String(e)),
    onSettled: () => {
      for (const key of ["installed", "services", "settings", "sites"]) queryClient.invalidateQueries({ queryKey: [key] });
    },
  });

  if (needs.length === 0 && suggests.length === 0) return null;
  const busy = (r: Runtime) => install.isPending && install.variables?.includes(r);
  const progress = (r: Runtime) => Object.entries(installs).some(([k]) => k.startsWith(installKey(r, "")));

  const row = (title: string, hint: string, list: Runtime[]) =>
    list.length > 0 && (
      <div className="px-5 py-3">
        <div className="mb-2 text-xs font-bold tracking-wider text-muted uppercase">{title}</div>
        <p className="mb-3 text-xs text-muted">{hint}</p>
        <ul className="flex flex-wrap gap-2">
          {list.map((r) => (
            <li key={r} className="flex items-center gap-1 rounded-lg border border-line bg-surface-2/50 py-1 pr-1 pl-2">
              <button onClick={() => onSelect(r)} className="flex items-center gap-2 text-sm font-medium hover:text-accent-text" title={`Show ${runtimeLabels[r]}`}>
                <span className={`flex size-6 items-center justify-center rounded-md ${runtimeTiles[r].className}`}>{runtimeTiles[r].icon(14)}</span>
                {runtimeLabels[r]}
              </button>
              <Button size="sm" variant="ghost" disabled={install.isPending} onClick={() => install.mutate([r])} aria-label={`Install ${runtimeLabels[r]}`}>
                {busy(r) || progress(r) ? "Installing…" : <Download size={14} />}
              </Button>
            </li>
          ))}
        </ul>
      </div>
    );

  return (
    <Card className="mt-4 overflow-hidden">
      <div className="flex flex-wrap items-center gap-3 border-b border-line px-5 py-3">
        <Link2 size={16} className="text-muted" aria-hidden />
        <h3 className="min-w-0 flex-1 text-sm font-bold">Related services</h3>
        <Button size="sm" variant="soft" disabled={install.isPending} onClick={() => install.mutate([...needs, ...suggests])}>
          <Download size={14} /> {install.isPending ? "Installing…" : `Install all (${needs.length + suggests.length})`}
        </Button>
      </div>
      {row("Needs", `${runtimeLabels[runtime]} can't run without ${needs.length === 1 ? "this" : "these"}.`, needs)}
      {row("Goes well with", "Tools to look at its data and work with it.", suggests)}
    </Card>
  );
}
