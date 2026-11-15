import { useEffect, useState, type ReactNode } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import clsx from "clsx";
import { FolderOpen, Star } from "lucide-react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { allServers, api, runtimeLabels, type Runtime, type Server, type ServiceKind, type ServiceOption, type Settings } from "../lib/api";
import { useAppStore, notifyError } from "../lib/store";
import { portsFor, serverNotes } from "../lib/settingsFields";
import ApplyBar from "./ApplyBar";
import { Button, Card, Segmented, Switch, inputClass } from "./ui";

const numberInput = clsx(inputClass.replace("w-full", ""), "w-24 font-mono tabular-nums");

/** The settings a service's details can change; the rest of Settings stays as it is. */
const pick = (s: Settings) => ({
  ports: s.ports,
  storage: s.storage,
  webServers: s.webServers,
  phpWorkers: s.phpWorkers,
  routeMail: s.routeMail,
  defaultDatabase: s.defaultDatabase,
});

function Row({ label, hint, children }: { label: ReactNode; hint?: ReactNode; children: ReactNode }) {
  return (
    <div className="flex flex-wrap items-center gap-x-6 gap-y-2 border-b border-line px-5 py-3 last:border-b-0">
      <div className="min-w-0 flex-1 basis-60">
        <div className="text-sm font-semibold">{label}</div>
        {hint && <div className="text-xs text-muted">{hint}</div>}
      </div>
      {children}
    </div>
  );
}

/**
 * A service's own settings (ports, whether a web server runs and which one is primary,
 * PHP workers, mail routing, the default database), changed right from its details on
 * the Services page. The same values as in Settings, applied the same way. Below them,
 * the options its manifest declares (`settings:` in src-tauri/services/<id>.yaml).
 */
export default function ServiceSettings({ runtime }: { runtime: Runtime }) {
  const queryClient = useQueryClient();
  const notify = useAppStore((s) => s.notify);
  const settings = useQuery({ queryKey: ["settings"], queryFn: api.getSettings });
  const os = useQuery({ queryKey: ["paths"], queryFn: api.envPaths }).data?.os;
  const [draft, setDraft] = useState<Settings | null>(null);
  useEffect(() => {
    if (settings.data) setDraft(settings.data);
  }, [settings.data]);
  // Only services have manifest options; other runtimes (PHP, Java…) have none.
  const service = useQuery({ queryKey: ["services"], queryFn: api.listServices }).data?.find((s) => s.kind === runtime);
  const options = useQuery({
    queryKey: ["service-options", runtime],
    queryFn: () => api.serviceOptions(runtime as ServiceKind),
    enabled: !!service,
  });
  const [values, setValues] = useState<Record<string, string>>({});
  useEffect(() => {
    if (options.data) setValues(Object.fromEntries(options.data.map((o) => [o.name, o.value])));
  }, [options.data]);
  const changedOptions = (options.data ?? []).filter((o) => values[o.name] !== undefined && values[o.name] !== o.value);

  const apply = useMutation({
    mutationFn: async () => {
      if (!draft) return;
      // The options first: the backend checks each one, and a bad value stops here.
      for (const o of changedOptions) await api.setServiceOption(runtime as ServiceKind, o.name, values[o.name]);
      // Everything else comes from the latest copy, so other screens' changes are kept.
      const latest = await api.getSettings();
      await api.saveSettings({ ...latest, ...pick(draft), serviceOptions: latest.serviceOptions });
      // Options apply when the service starts.
      if (changedOptions.length > 0 && service?.state === "running") await api.restartService(service.kind);
    },
    onSuccess: () => {
      for (const key of ["settings", "services", "sites", "port-status", "service-options"]) queryClient.invalidateQueries({ queryKey: [key] });
      notify("info", `${runtimeLabels[runtime]} settings applied`);
    },
    onError: notifyError,
  });

  if (!draft || !settings.data) return null;
  const server = (allServers as Runtime[]).includes(runtime) ? (runtime as Server) : null;
  const fields = portsFor(runtime, draft, os);
  const workers = runtime === "php" && os === "windows";
  const database = runtime === "mariadb" || runtime === "mysql";
  const storage = runtime === "rustfs" || runtime === "minio";
  const declared = options.data ?? [];
  if (!server && fields.length === 0 && !workers && runtime !== "mailpit" && !database && !storage && declared.length === 0) return null;

  const set = (patch: Partial<Settings>) => setDraft({ ...draft, ...patch });
  const dirty = JSON.stringify(pick(draft)) !== JSON.stringify(pick(settings.data)) || changedOptions.length > 0;
  const enabled = !!server && draft.webServers.includes(server);
  const primary = !!server && draft.webServers[0] === server;

  return (
    <Card className="mt-4 overflow-hidden">
      <div className="border-b border-line px-5 py-3">
        <h3 className="text-sm font-bold">Settings</h3>
        <p className="text-xs text-muted">Also in Settings. Running services restart when you apply a change.</p>
      </div>
      {server && (
        <>
          <Row label={`Run ${runtimeLabels[server]}`} hint={serverNotes[server]}>
            <Switch
              checked={enabled}
              disabled={enabled && draft.webServers.length === 1}
              onChange={(v) => set({ webServers: v ? [...draft.webServers, server] : draft.webServers.filter((s) => s !== server) })}
              label={`Run ${runtimeLabels[server]}`}
            />
          </Row>
          {enabled && draft.webServers.length > 1 && (
            <Row
              label="Primary server"
              hint={primary ? "It has the main ports, localhost and the developer tools (phpMyAdmin, Adminer…)." : "The primary server has the main ports, localhost and the developer tools."}
            >
              {primary ? (
                <span className="flex items-center gap-1.5 text-sm font-semibold text-accent-text">
                  <Star size={14} /> Primary
                </span>
              ) : (
                <Button size="sm" variant="soft" onClick={() => set({ webServers: [server, ...draft.webServers.filter((s) => s !== server)] })}>
                  <Star size={14} /> Make primary
                </Button>
              )}
            </Row>
          )}
        </>
      )}
      {(!server || enabled) &&
        fields.map(({ key, label, hint }) => (
          <Row key={key} label={label} hint={hint}>
            <input
              type="number"
              min={1}
              max={65535}
              className={numberInput}
              value={draft.ports[key]}
              onChange={(e) => set({ ports: { ...draft.ports, [key]: Number(e.target.value) } })}
              aria-label={`${label} port`}
            />
          </Row>
        ))}
      {workers && (
        <Row label="PHP workers per version" hint="Each handles one request at a time. 4 suits most projects; raise it for parallel requests.">
          <input
            type="number"
            min={1}
            max={10}
            className={numberInput}
            value={draft.phpWorkers}
            onChange={(e) => set({ phpWorkers: Math.max(1, Math.min(10, Number(e.target.value))) })}
            aria-label="PHP workers per version"
          />
        </Row>
      )}
      {storage && (
        <Row
          label="Storage folder"
          hint="Where buckets and objects are kept. It can be on another drive; it doesn't move with the LampRabit folder and uninstalling never deletes it. Objects already stored stay in the old folder."
        >
          <span className="flex min-w-0 items-center gap-2">
            <span className="max-w-64 truncate font-mono text-xs text-muted" title={draft.storage[runtime] ?? "data/" + runtime}>
              {draft.storage[runtime] ?? `data/${runtime} (default)`}
            </span>
            <Button
              size="sm"
              onClick={async () => {
                const dir = await openDialog({ directory: true, title: `Choose where ${runtimeLabels[runtime]} keeps its objects` });
                if (typeof dir === "string") set({ storage: { ...draft.storage, [runtime]: dir } });
              }}
            >
              <FolderOpen size={14} /> Choose…
            </Button>
            {draft.storage[runtime] && (
              <Button size="sm" variant="ghost" onClick={() => set({ storage: Object.fromEntries(Object.entries(draft.storage).filter(([k]) => k !== runtime)) })}>
                Default
              </Button>
            )}
          </span>
        </Row>
      )}
      {runtime === "mailpit" && (
        <Row label="Send PHP's mail() to Mailpit" hint="Frameworks that send over SMTP use the SMTP port above.">
          <Switch checked={draft.routeMail} onChange={(routeMail) => set({ routeMail })} label="Send PHP's mail() to Mailpit" />
        </Row>
      )}
      {database && (
        <Row label="Default database" hint="New projects, the mysql command and the database tools use it first.">
          <Segmented
            value={draft.defaultDatabase}
            onChange={(defaultDatabase) => set({ defaultDatabase })}
            options={[
              { value: "mariadb", label: "MariaDB" },
              { value: "mysql", label: "MySQL" },
            ]}
          />
        </Row>
      )}
      {declared.map((o) => (
        <Row key={o.name} label={o.label} hint={o.description}>
          <OptionField option={o} value={values[o.name] ?? o.value} onChange={(v) => setValues({ ...values, [o.name]: v })} />
        </Row>
      ))}
      <ApplyBar
        dirty={dirty}
        busy={apply.isPending}
        onApply={() => apply.mutate()}
        onDiscard={() => {
          setDraft(settings.data ?? null);
          setValues(Object.fromEntries(declared.map((o) => [o.name, o.value])));
        }}
      />
    </Card>
  );
}

/** The input for one manifest option, by its type. */
function OptionField({ option, value, onChange }: { option: ServiceOption; value: string; onChange: (v: string) => void }) {
  switch (option.kind) {
    case "toggle":
      return <Switch checked={value === "true"} onChange={(v) => onChange(String(v))} label={option.label} />;
    case "number":
      return (
        <span className="flex items-center gap-2">
          <input
            type="number"
            min={option.min ?? undefined}
            max={option.max ?? undefined}
            className={numberInput}
            value={value}
            onChange={(e) => onChange(e.target.value)}
            aria-label={option.label}
          />
          {option.unit && <span className="text-sm text-muted">{option.unit}</span>}
        </span>
      );
    case "choice":
      return option.options.length <= 4 ? (
        <Segmented value={value} onChange={onChange} options={option.options.map((o) => ({ value: o, label: o }))} />
      ) : (
        <select className={clsx(inputClass.replace("w-full", ""), "w-48")} value={value} onChange={(e) => onChange(e.target.value)} aria-label={option.label}>
          {option.options.map((o) => (
            <option key={o} value={o}>
              {o}
            </option>
          ))}
        </select>
      );
    default:
      return <input className={clsx(inputClass.replace("w-full", ""), "w-64")} value={value} onChange={(e) => onChange(e.target.value)} aria-label={option.label} />;
  }
}
