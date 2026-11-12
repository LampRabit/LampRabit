import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Check, Copy, Plug, Unplug } from "lucide-react";
import clsx from "clsx";
import { api, type McpStatus } from "../lib/api";
import { useAppStore } from "../lib/store";
import { Button, Switch } from "./ui";

/**
 * Settings → AI assistants: LampRabit's MCP server, without a terminal. One click adds
 * LampRabit to an assistant's MCP config (it then starts LampRabit's server itself, app open
 * or not), and a switch runs the server inside the app over HTTP for assistants that
 * connect by URL. The pieces to copy cover everything else.
 */
export default function McpSettings() {
  const queryClient = useQueryClient();
  const notify = useAppStore((s) => s.notify);
  const onError = (e: unknown) => notify("error", String(e));
  const status = useQuery({ queryKey: ["mcp-status"], queryFn: api.mcpStatus });
  const refresh = () => queryClient.invalidateQueries({ queryKey: ["mcp-status"] });
  const toggle = useMutation({
    mutationFn: (on: boolean) => api.setMcpServer(on),
    onSuccess: (_, on) => {
      notify("info", on ? "LampRabit's MCP server is running" : "MCP server stopped");
      refresh();
      queryClient.invalidateQueries({ queryKey: ["settings"] });
    },
    onError,
  });
  const connect = useMutation({
    mutationFn: ({ id, on }: { id: string; on: boolean }) => api.mcpConnect(id, on),
    onSuccess: (note) => {
      notify("info", note);
      refresh();
    },
    onError,
  });
  const s = status.data;
  if (!s) return <div className="px-5 py-4 text-sm text-muted">{status.isError ? String(status.error) : "Loading…"}</div>;

  return (
    <div className="divide-y divide-line">
      <div className="space-y-3 px-5 py-4">
        <div>
          <p className="text-sm font-medium text-fg">Connect an assistant</p>
          <p className="text-xs text-muted">
            Adds LampRabit to the assistant's MCP settings. It can then see your projects, services, logs and project health, and start or fix things after asking you. Restart the assistant afterwards.
          </p>
        </div>
        <ul className="grid gap-2 sm:grid-cols-2">
          {s.clients.map((c) => (
            <li key={c.id} className={clsx("flex items-center gap-3 rounded-lg border px-3 py-2", c.connected ? "border-ok/30 bg-ok/5" : "border-line")}>
              <span className="min-w-0 flex-1">
                <span className="block text-sm font-medium text-fg">{c.name}</span>
                <span className="block truncate text-[11px] text-muted" title={c.path ?? undefined}>
                  {c.connected ? "Connected" : c.installed ? "Not connected" : "Not found on this computer"}
                </span>
              </span>
              {c.connected ? (
                <Button size="sm" variant="ghost" onClick={() => connect.mutate({ id: c.id, on: false })} disabled={connect.isPending}>
                  <Unplug size={13} /> Disconnect
                </Button>
              ) : (
                <Button size="sm" variant={c.installed ? "primary" : "secondary"} onClick={() => connect.mutate({ id: c.id, on: true })} disabled={connect.isPending || !c.installed}>
                  <Plug size={13} /> Connect
                </Button>
              )}
            </li>
          ))}
        </ul>
        <p className="text-[11px] text-muted">LampRabit only adds or removes its own &quot;lamprabit&quot; entry; the rest of each file is kept, and the original is backed up next to it the first time.</p>
      </div>

      <div className="space-y-3 px-5 py-4">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div className="min-w-0">
            <p className="text-sm font-medium text-fg">Run the MCP server in LampRabit</p>
            <p className="text-xs text-muted">For assistants that connect by URL. Only this computer can reach it, with the token below.</p>
          </div>
          <Switch checked={s.running} onChange={(v) => toggle.mutate(v)} disabled={toggle.isPending} label="Run the MCP server in LampRabit" />
        </div>
        {s.running && <HttpConfig status={s} />}
      </div>

      <div className="space-y-2 px-5 py-4">
        <p className="text-sm font-medium text-fg">Other assistants</p>
        <p className="text-xs text-muted">Most take this in their MCP settings; it starts LampRabit&apos;s server (and LampRabit, in the tray, when it isn&apos;t open).</p>
        <CopyBlock text={JSON.stringify({ mcpServers: { lamprabit: { command: s.command, args: s.args } } }, null, 2)} what="the MCP config" />
      </div>
    </div>
  );
}

function HttpConfig({ status }: { status: McpStatus }) {
  const header = `Bearer ${status.token}`;
  const json = JSON.stringify({ mcpServers: { lamprabit: { type: "http", url: status.url, headers: { Authorization: header } } } }, null, 2);
  const claudeCode = `claude mcp add --transport http lamprabit ${status.url} --header "Authorization: ${header}"`;
  return (
    <div className="space-y-2">
      <p className="flex items-center gap-2 text-xs text-ok">
        <span className="size-2 animate-pulse rounded-full bg-ok" aria-hidden /> Running at <code className="font-mono text-fg">{status.url}</code>
      </p>
      <p className="text-xs text-muted">Claude Code:</p>
      <CopyBlock text={claudeCode} what="the command" />
      <p className="text-xs text-muted">Cursor, VS Code and others that take a URL:</p>
      <CopyBlock text={json} what="the config" />
    </div>
  );
}

function CopyBlock({ text, what }: { text: string; what: string }) {
  const notify = useAppStore((s) => s.notify);
  const [copied, setCopied] = useState(false);
  return (
    <div className="relative">
      <pre className="selectable overflow-x-auto rounded-md bg-surface-2 py-2 pr-20 pl-3 font-mono text-xs whitespace-pre-wrap break-all">{text}</pre>
      <Button
        size="sm"
        variant="ghost"
        className="absolute top-1.5 right-1.5"
        onClick={() =>
          navigator.clipboard.writeText(text).then(
            () => {
              setCopied(true);
              setTimeout(() => setCopied(false), 1500);
            },
            () => notify("error", `Couldn't copy ${what}`),
          )
        }
        aria-label={`Copy ${what}`}
      >
        {copied ? <Check size={13} /> : <Copy size={13} />} {copied ? "Copied" : "Copy"}
      </Button>
    </div>
  );
}
