import { useMutation, useQuery } from "@tanstack/react-query";
import { disable as disableAutostart } from "@tauri-apps/plugin-autostart";
import { exit } from "@tauri-apps/plugin-process";
import { CheckCircle2, FolderCheck, Trash2 } from "lucide-react";
import { api } from "../lib/api";
import Dialog from "./Dialog";
import { Button } from "./ui";

/** What the app itself leaves behind, per platform, once its files are gone. */
const appItself: Record<string, string> = {
  windows: "Then uninstall the app from Windows Settings → Apps. Its uninstaller does this same cleanup, so running it after this is fine.",
  macos: "Then quit and move LampRabit from Applications to the Trash.",
  linux: "Then delete the LampRabit AppImage, or run sudo apt remove lamprabit if you installed the .deb.",
};

/**
 * Removes LampRabit from this computer: runtimes, configs, logs, certificates, hosts
 * entries, PATH entry and settings. Projects (www) and databases (data) are kept.
 * Mount only while open.
 */
export default function UninstallDialog({ onClose }: { onClose: () => void }) {
  const plan = useQuery({ queryKey: ["uninstallPlan"], queryFn: api.uninstallPlan, staleTime: 0 });
  const os = useQuery({ queryKey: ["paths"], queryFn: api.envPaths }).data?.os;
  const run = useMutation({
    mutationFn: async () => {
      // Autostart lives in the OS (a login item); the plugin removes it.
      await disableAutostart().catch(() => {});
      return api.uninstall();
    },
  });
  const done = run.data;
  // Once the files are gone the app can't work any more: closing quits.
  const close = done ? () => exit(0) : run.isPending ? () => {} : onClose;

  return (
    <Dialog
      open
      onClose={close}
      title={done ? "LampRabit was removed" : "Remove LampRabit from this computer"}
      icon={done ? <CheckCircle2 size={20} /> : <Trash2 size={20} />}
      width="max-w-xl"
      footer={
        done ? (
          <Button variant="primary" onClick={() => exit(0)} data-autofocus>
            Quit LampRabit
          </Button>
        ) : (
          <>
            <Button onClick={onClose} disabled={run.isPending} data-autofocus>
              Cancel
            </Button>
            <Button variant="danger" onClick={() => run.mutate()} disabled={run.isPending || !plan.data}>
              <Trash2 size={15} /> {run.isPending ? "Removing…" : "Remove"}
            </Button>
          </>
        )
      }
    >
      {done ? (
        <div className="space-y-4 text-sm">
          {done.kept.length > 0 && <PathList title="Kept" paths={done.kept} />}
          {done.problems.length > 0 ? (
            <div className="rounded-xl border border-danger/30 bg-danger-soft p-3">
              <p className="font-medium text-danger-text">Some things need doing by hand</p>
              <ul className="mt-1.5 list-disc space-y-1 pl-5 text-danger-text">
                {done.problems.map((p) => (
                  <li key={p}>{p}</li>
                ))}
              </ul>
            </div>
          ) : (
            <p className="text-muted">Everything else LampRabit set up has been removed.</p>
          )}
          {os && <p className="text-muted">{appItself[os]}</p>}
        </div>
      ) : (
        <div className="space-y-4 text-sm">
          <p className="text-muted">
            Running services are stopped first. Your projects and databases stay where they are, so a later install picks them up again.
          </p>
          {plan.isLoading && <p className="text-muted">Checking what's installed…</p>}
          {plan.error && <p className="text-danger-text">{String(plan.error)}</p>}
          {plan.data && (
            <>
              <PathList title="Deleted" paths={plan.data.remove} />
              <div>
                <h3 className="text-xs font-medium text-muted">Also undone</h3>
                <ul className="mt-1.5 list-disc space-y-0.5 pl-5 text-muted">
                  <li>The .test entries in the hosts file</li>
                  <li>The trusted "lampRabit Local CA" certificate</li>
                  <li>LampRabit's commands on your PATH, the saved ngrok token and starting with your computer</li>
                  {os === "linux" && <li>The setting that lets the web server use ports 80 and 443</li>}
                </ul>
                <p className="mt-1.5 text-xs text-muted">
                  {os === "linux"
                    ? "Your password is asked once, for the system files."
                    : "Your system asks for confirmation for the hosts file and the certificate."}
                </p>
              </div>
              <PathList title="Kept" paths={plan.data.keep} icon />
            </>
          )}
          {run.error && <p className="text-danger-text">{String(run.error)}</p>}
        </div>
      )}
    </Dialog>
  );
}

function PathList({ title, paths, icon }: { title: string; paths: string[]; icon?: boolean }) {
  return (
    <div>
      <h3 className="text-xs font-medium text-muted">{title}</h3>
      {paths.length === 0 ? (
        <p className="mt-1.5 text-muted">Nothing.</p>
      ) : (
        <ul className="mt-1.5 space-y-1">
          {paths.map((p) => (
            <li key={p} className="flex items-center gap-2 rounded-lg bg-surface-2 px-3 py-1.5 font-mono text-xs break-all">
              {icon && <FolderCheck size={14} className="shrink-0 text-accent-text" aria-hidden />}
              {p}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
