import { useEffect, useMemo, useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Download, Loader2, Puzzle, Search, X } from "lucide-react";
import clsx from "clsx";
import { api, type ExtInfo } from "../lib/api";
import { useAppStore, notifyError } from "../lib/store";
import { Button, Switch } from "./ui";

/** Browse, enable and install PHP extensions for one installed PHP version. */
export default function ExtensionsDialog({
  open,
  onClose,
}: {
  open: boolean;
  onClose: () => void;
}) {
  const queryClient = useQueryClient();
  const notify = useAppStore((s) => s.notify);
  const installed = useQuery({
    queryKey: ["installed"],
    queryFn: api.installedRuntimes,
    enabled: open,
  });
  const versions =
    installed.data?.find((i) => i.runtime === "php")?.versions ?? [];
  const active =
    installed.data?.find((i) => i.runtime === "php")?.active ?? null;
  const [version, setVersion] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const php = version ?? active;

  // Rows keep the order they had when the list loaded, so a toggled extension stays where
  // you clicked it instead of jumping between groups.
  const order = useRef<{ php: string | null; names: string[] }>({
    php: null,
    names: [],
  });
  const [flash, setFlash] = useState<string | null>(null);
  useEffect(() => {
    if (open) setQuery("");
    order.current = { php: null, names: [] };
  }, [open]);

  const list = useQuery({
    queryKey: ["php-extensions", php],
    queryFn: () => api.phpExtensions(php!),
    enabled: open && !!php,
  });
  const refresh = () =>
    queryClient.invalidateQueries({ queryKey: ["php-extensions", php] });
  const toggle = useMutation({
    mutationFn: ({ name, enabled }: { name: string; enabled: boolean }) =>
      api.setPhpExtension(php!, name, enabled),
    onSuccess: (_, { name }) => {
      setFlash(name);
      window.setTimeout(() => setFlash((f) => (f === name ? null : f)), 1200);
      return refresh();
    },
    onError: notifyError,
  });
  const install = useMutation({
    mutationFn: (name: string) => api.installPhpExtension(php!, name),
    onSuccess: (_, name) => {
      notify("info", `${name} installed and enabled for PHP ${php}`);
      refresh();
    },
    onError: notifyError,
  });

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    return (list.data ?? []).filter(
      (e) =>
        !q || e.name.includes(q) || e.description?.toLowerCase().includes(q),
    );
  }, [list.data, query]);
  const extras = filtered.filter((e) => e.kind === "extra");
  if (list.data && order.current.php !== php) {
    order.current = {
      php,
      names: list.data
        .filter((e) => e.kind === "bundled")
        .sort(
          (a, b) =>
            Number(b.enabled) - Number(a.enabled) ||
            a.name.localeCompare(b.name),
        )
        .map((e) => e.name),
    };
  }
  const position = (name: string) => {
    const i = order.current.names.indexOf(name);
    return i < 0 ? Number.MAX_SAFE_INTEGER : i;
  };
  const bundled = filtered
    .filter((e) => e.kind === "bundled")
    .sort(
      (a, b) =>
        position(a.name) - position(b.name) || a.name.localeCompare(b.name),
    );

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [open, onClose]);

  if (!open) return null;
  const busy = toggle.isPending || install.isPending;

  const row = (e: ExtInfo) => {
    const toggling = toggle.isPending && toggle.variables?.name === e.name;
    return (
      <li
        key={e.name}
        className={clsx(
          "flex items-center gap-3 px-4 py-2.5 transition-colors duration-700",
          flash === e.name && "bg-accent-soft",
        )}
      >
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2">
            <span className="font-mono text-sm font-semibold">{e.name}</span>
            {e.kind === "extra" && e.installed && (
              <span className="rounded bg-surface-2 px-1.5 py-0.5 text-[10px] text-muted">
                installed
              </span>
            )}
          </div>
          {e.description && (
            <div className="text-xs text-muted">{e.description}</div>
          )}
          {e.note && e.kind === "extra" && (
            <div className="mt-0.5 text-[11px] text-muted/80">{e.note}</div>
          )}
        </div>
        {e.kind === "extra" && !e.installed ? (
          e.available ? (
            <Button
              size="sm"
              variant="soft"
              onClick={() => install.mutate(e.name)}
              disabled={busy}
            >
              {install.isPending && install.variables === e.name ? (
                <Loader2 size={14} className="animate-spin" />
              ) : (
                <Download size={14} />
              )}
              Install {e.available}
            </Button>
          ) : (
            <span className="text-xs text-muted">
              No build for PHP {php?.split(".").slice(0, 2).join(".")} yet
            </span>
          )
        ) : (
          <Switch
            // Flips straight away; the spinner in the knob shows PHP is being restarted.
            checked={toggling ? toggle.variables!.enabled : e.enabled}
            busy={toggling}
            onChange={(v) =>
              !busy && toggle.mutate({ name: e.name, enabled: v })
            }
            label={`${e.name} enabled`}
          />
        )}
      </li>
    );
  };

  return (
    <div
      className="motion-backdrop fixed inset-0 z-30 flex items-center justify-center bg-black/40 p-6"
      onMouseDown={onClose}
    >
      <div
        role="dialog"
        aria-label="PHP extensions"
        className="motion-dialog flex max-h-full w-full max-w-2xl flex-col rounded-2xl border border-line bg-surface shadow-float"
        onMouseDown={(e) => e.stopPropagation()}
      >
        <div className="flex items-center gap-3 border-b border-line px-6 py-4">
          <Puzzle size={20} className="text-indigo-500" />
          <div className="flex-1">
            <h2 className="text-lg font-bold tracking-tight">PHP extensions</h2>
            <p className="text-sm text-muted">
              Changes restart PHP for your sites automatically.
            </p>
          </div>
          <button
            aria-label="Close"
            onClick={onClose}
            className="rounded-lg p-1.5 text-muted hover:bg-surface-2"
          >
            <X size={18} />
          </button>
        </div>
        <div className="flex flex-wrap items-center gap-2 border-b border-line px-6 py-3">
          <select
            value={php ?? ""}
            onChange={(e) => setVersion(e.target.value)}
            className="h-9 rounded-lg border border-line bg-surface px-2.5 text-sm"
            aria-label="PHP version"
          >
            {versions.map((v) => (
              <option key={v} value={v}>
                PHP {v}
                {v === active ? " (default)" : ""}
              </option>
            ))}
          </select>
          <label className="flex h-9 flex-1 items-center gap-2 rounded-lg border border-line bg-surface px-3">
            <Search size={15} className="text-muted" />
            <input
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder="Search extensions…"
              className="w-full bg-transparent text-sm outline-none placeholder:text-muted"
            />
          </label>
        </div>
        <div className="min-h-0 flex-1 overflow-y-auto">
          {list.isLoading ? (
            <p className="px-6 py-6 text-sm text-muted">
              Reading extensions and checking downloads…
            </p>
          ) : list.isError ? (
            <p className="px-6 py-6 text-sm text-danger-text">
              {String(list.error)}
            </p>
          ) : (
            <>
              <h3 className="px-6 pt-4 pb-1 text-xs font-medium tracking-wide text-muted uppercase">
                Popular extras
              </h3>
              <ul
                className={clsx(
                  "divide-y divide-line",
                  extras.length === 0 && "hidden",
                )}
              >
                {extras.map(row)}
              </ul>
              <h3 className="px-6 pt-5 pb-1 text-xs font-medium tracking-wide text-muted uppercase">
                Included with PHP {php} (
                {bundled.filter((e) => e.enabled).length} enabled)
              </h3>
              <ul className="divide-y divide-line pb-2">{bundled.map(row)}</ul>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
