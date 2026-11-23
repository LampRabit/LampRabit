import { useEffect, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { save } from "@tauri-apps/plugin-dialog";
import { Check, Copy, Download, RefreshCw, Stethoscope, X } from "lucide-react";
import clsx from "clsx";
import { api } from "../lib/api";
import { useAppStore } from "../lib/store";
import { Button, Switch } from "./ui";

/** Modal with a troubleshooting report that can be copied or saved. */
export default function Diagnostics() {
  const open = useAppStore((s) => s.diagnosticsOpen);
  const setOpen = useAppStore((s) => s.setDiagnosticsOpen);
  const notify = useAppStore((s) => s.notify);
  const [redact, setRedact] = useState(true);
  const [copied, setCopied] = useState(false);
  const report = useQuery({
    queryKey: ["diagnostics", redact],
    queryFn: () => api.diagnostics(redact),
    enabled: open,
    staleTime: 0,
  });

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && setOpen(false);
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [open, setOpen]);

  if (!open) return null;

  const copy = async () => {
    if (!report.data) return;
    await navigator.clipboard.writeText(report.data);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };
  const saveFile = async () => {
    if (!report.data) return;
    const path = await save({ defaultPath: "lamprabit-diagnostics.md", filters: [{ name: "Markdown", extensions: ["md", "txt"] }] });
    if (!path) return;
    try {
      await api.saveText(path, report.data);
      notify("info", "Diagnostics saved");
    } catch (e) {
      notify("error", String(e));
    }
  };

  return (
    <div className="motion-backdrop fixed inset-0 z-30 flex items-center justify-center bg-black/40 p-6" onMouseDown={() => setOpen(false)}>
      <div
        role="dialog"
        aria-label="Diagnostics"
        className="motion-dialog flex max-h-full w-full max-w-3xl flex-col rounded-2xl border border-line bg-surface shadow-float"
        onMouseDown={(e) => e.stopPropagation()}
      >
        <div className="flex items-center gap-3 border-b border-line px-6 py-4">
          <Stethoscope size={20} className="text-accent-text" />
          <div className="flex-1">
            <h2 className="text-lg font-medium tracking-tight">Diagnostics</h2>
            <p className="text-sm text-muted">Versions, services, ports, config checks and recent errors.</p>
          </div>
          <button aria-label="Close" onClick={() => setOpen(false)} className="rounded-lg p-1.5 text-muted hover:bg-surface-2">
            <X size={18} />
          </button>
        </div>
        <div className="min-h-0 flex-1 overflow-auto bg-bg px-6 py-4">
          {report.isFetching && !report.data ? (
            <p className="text-sm text-muted">Collecting…</p>
          ) : report.isError ? (
            <p className="text-sm text-danger-text">{String(report.error)}</p>
          ) : (
            <pre className={clsx("selectable font-mono text-xs leading-relaxed whitespace-pre-wrap", report.isFetching && "opacity-60")}>
              {report.data}
            </pre>
          )}
        </div>
        <div className="flex flex-wrap items-center gap-3 border-t border-line px-6 py-4">
          <label className="flex items-center gap-2 text-sm">
            <Switch checked={redact} onChange={setRedact} label="Hide user name and home folder" />
            Hide user name and home folder
          </label>
          <span className="flex-1" />
          <Button size="sm" onClick={() => report.refetch()} disabled={report.isFetching} aria-label="Run again">
            <RefreshCw size={14} className={clsx(report.isFetching && "animate-spin")} />
          </Button>
          <Button size="sm" onClick={copy} disabled={!report.data}>
            {copied ? <Check size={14} /> : <Copy size={14} />} {copied ? "Copied" : "Copy"}
          </Button>
          <Button size="sm" variant="primary" onClick={saveFile} disabled={!report.data}>
            <Download size={14} /> Save…
          </Button>
        </div>
      </div>
    </div>
  );
}
