import { useEffect, useRef, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Check, Copy, ExternalLink, Loader2, Radio, Share2, Square } from "lucide-react";
import clsx from "clsx";
import { api, type Site, type Tunnel, type TunnelProvider } from "../lib/api";
import { useAppStore, notifyError } from "../lib/store";
import { Button, Card, inputClass } from "./ui";

const providerLabel: Record<TunnelProvider, string> = { ngrok: "Ngrok", cloudflare: "Cloudflare" };

export function useTunnels() {
  return useQuery({ queryKey: ["tunnels"], queryFn: api.tunnelList, refetchInterval: 10_000 });
}

/** Which tunnel tools are installed. */
function useProviders() {
  const installed = useQuery({ queryKey: ["installed"], queryFn: api.installedRuntimes });
  const has = (r: string) => !!installed.data?.find((i) => i.runtime === r)?.active;
  return { ngrok: has("ngrok"), cloudflare: has("cloudflared") };
}

function useTunnelActions() {
  const queryClient = useQueryClient();
  const refresh = () => queryClient.invalidateQueries({ queryKey: ["tunnels"] });
  const start = useMutation({
    mutationFn: ({ provider, site, port }: { provider: TunnelProvider; site: string | null; port: number | null }) =>
      api.tunnelStart(provider, site, port),
    onSuccess: refresh,
    onError: notifyError,
  });
  const stop = useMutation({ mutationFn: (id: string) => api.tunnelStop(id), onSuccess: refresh });
  return { start, stop };
}

/** One tunnel: status, public URL with copy/open, and stop. */
export function TunnelRow({ tunnel, compact = false }: { tunnel: Tunnel; compact?: boolean }) {
  const { stop } = useTunnelActions();
  const [copied, setCopied] = useState(false);
  const copy = async () => {
    if (!tunnel.url) return;
    await navigator.clipboard.writeText(tunnel.url);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  return (
    <div
      className={clsx(
        "flex flex-wrap items-center gap-2 rounded-xl border px-3 py-2 text-sm",
        tunnel.status === "error" ? "border-danger/30 bg-danger-soft" : "border-violet-500/25 bg-violet-500/5",
      )}
    >
      {tunnel.status === "starting" ? (
        <Loader2 size={15} className="animate-spin text-violet-500" />
      ) : (
        <Radio size={15} className={tunnel.status === "error" ? "text-danger-text" : "text-violet-500"} />
      )}
      <div className="min-w-0 flex-1">
        {!compact && (
          <div className="text-xs text-muted">
            {tunnel.target} via {providerLabel[tunnel.provider]}
          </div>
        )}
        <div className={clsx("selectable truncate font-mono text-xs", tunnel.status === "error" && "text-danger-text")}>
          {tunnel.url ?? (tunnel.status === "error" ? tunnel.error : "Connecting…")}
        </div>
      </div>
      {tunnel.url && (
        <>
          <button aria-label="Copy public URL" onClick={copy} className="rounded-md p-1 text-muted hover:bg-surface-2 hover:text-fg">
            {copied ? <Check size={14} /> : <Copy size={14} />}
          </button>
          <button aria-label="Open public URL" onClick={() => openUrl(tunnel.url!)} className="rounded-md p-1 text-muted hover:bg-surface-2 hover:text-fg">
            <ExternalLink size={14} />
          </button>
        </>
      )}
      <button
        aria-label="Stop sharing"
        onClick={() => stop.mutate(tunnel.id)}
        className="rounded-md p-1 text-muted hover:bg-surface-2 hover:text-danger-text"
      >
        <Square size={12} className="fill-current" />
      </button>
    </div>
  );
}

/** The live public URL of a project, if it's shared. */
export function useProjectTunnel(site: Site) {
  return useTunnels().data?.find((t) => t.site === site.name);
}

/** "Share" button on a project card: pick a provider to put the project on a public URL. */
export function ShareButton({ site }: { site: Site }) {
  const providers = useProviders();
  const navigate = useAppStore((s) => s.navigate);
  const { start } = useTunnelActions();
  const [open, setOpen] = useState(false);
  const root = useRef<HTMLDivElement>(null);
  const available = (["cloudflare", "ngrok"] as const).filter((p) => providers[p]);
  useEffect(() => {
    if (!open) return;
    const close = (e: MouseEvent) => !root.current?.contains(e.target as Node) && setOpen(false);
    document.addEventListener("mousedown", close);
    return () => document.removeEventListener("mousedown", close);
  }, [open]);
  return (
    <div ref={root} className="relative">
      <Button onClick={() => setOpen((o) => !o)} aria-haspopup="menu" aria-expanded={open} disabled={start.isPending}>
        {start.isPending ? <Loader2 size={15} className="animate-spin" /> : <Share2 size={15} />} Share
      </Button>
      {open && (
        <div
          role="menu"
          className="motion-pop absolute right-0 top-full z-30 mt-1.5 w-64 rounded-xl border border-line bg-surface p-1 shadow-float"
          onKeyDown={(e) => e.key === "Escape" && setOpen(false)}
        >
          <div className="px-2.5 pt-1.5 pb-1 text-xs text-muted">Put {site.domain} on a public URL</div>
          {available.length === 0 ? (
            <button role="menuitem" onClick={() => navigate("services")} className="w-full rounded-lg px-2.5 py-2 text-left text-sm hover:bg-surface-2">
              Install Cloudflare Tunnel or Ngrok first
            </button>
          ) : (
            available.map((p) => (
              <button
                key={p}
                role="menuitem"
                autoFocus={p === available[0]}
                onClick={() => {
                  setOpen(false);
                  start.mutate({ provider: p, site: site.name, port: null });
                }}
                className="flex w-full items-center justify-between rounded-lg px-2.5 py-2 text-left text-sm hover:bg-surface-2 focus-visible:bg-surface-2 focus-visible:outline-none"
              >
                {providerLabel[p]}
                <span className="text-xs text-muted">{p === "cloudflare" ? "No account needed" : "Your ngrok account"}</span>
              </button>
            ))
          )}
        </div>
      )}
    </div>
  );
}

/** Services page panel: every running tunnel plus sharing any local port. */
export function TunnelsPanel() {
  const tunnels = useTunnels();
  const providers = useProviders();
  const { start } = useTunnelActions();
  const [port, setPort] = useState("3000");
  const [provider, setProvider] = useState<TunnelProvider>("cloudflare");
  const list = tunnels.data ?? [];
  const anyProvider = providers.ngrok || providers.cloudflare;

  if (!anyProvider && list.length === 0) return null;

  return (
    <Card className="mb-5 p-5">
      <div className="mb-3 flex flex-wrap items-center gap-3">
        <Share2 size={18} className="text-violet-500" />
        <h2 className="flex-1 font-bold tracking-tight">Public URLs</h2>
        {anyProvider && (
          <div className="flex items-center gap-2">
            <select
              value={provider}
              onChange={(e) => setProvider(e.target.value as TunnelProvider)}
              className="h-8 rounded-lg border border-line bg-surface px-2 text-sm"
              aria-label="Tunnel provider"
            >
              {providers.cloudflare && <option value="cloudflare">Cloudflare</option>}
              {providers.ngrok && <option value="ngrok">Ngrok</option>}
            </select>
            <input
              className={clsx(inputClass, "h-8 w-24 py-1")}
              value={port}
              onChange={(e) => setPort(e.target.value.replace(/\D/g, ""))}
              aria-label="Local port"
              placeholder="Port"
            />
            <Button
              size="sm"
              onClick={() => start.mutate({ provider: providers[provider] ? provider : providers.cloudflare ? "cloudflare" : "ngrok", site: null, port: Number(port) })}
              disabled={!port || start.isPending}
            >
              Share port
            </Button>
          </div>
        )}
      </div>
      {list.length === 0 ? (
        <p className="text-sm text-muted">
          Nothing is shared. Share a project from its card on the Sites page, or any local port here (for example a Node dev
          server). Apps that store their own URL (WordPress, Laravel's APP_URL) may redirect to the .test domain.
        </p>
      ) : (
        <div className="grid gap-2 md:grid-cols-2">
          {list.map((t) => (
            <TunnelRow key={t.id} tunnel={t} />
          ))}
        </div>
      )}
    </Card>
  );
}
