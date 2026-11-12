import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { AlertCircle, AlertTriangle, CheckCircle2, Info, RefreshCw, Stethoscope, Wrench } from "lucide-react";
import clsx from "clsx";
import { api, type DoctorIssue, type Site } from "../lib/api";
import { usePageActive } from "../lib/pageActive";
import { useAppStore } from "../lib/store";
import { Button } from "./ui";

/**
 * The project doctor (phase 19): what stands between the project and working, most serious
 * first, each with why it matters and, where LampRabit can, a fix. A fix shows what it will
 * do and runs only when confirmed; the backend checks the problem is still there first.
 */

const tone = {
  error: { icon: AlertCircle, text: "text-danger-text", ring: "border-danger/25 bg-danger-soft/40" },
  warning: { icon: AlertTriangle, text: "text-warn", ring: "border-warn/30 bg-warn/10" },
  info: { icon: Info, text: "text-accent-text", ring: "border-line bg-surface-2/40" },
};

export default function ProjectDoctor({ site, onCommands }: { site: Site; onCommands: () => void }) {
  const queryClient = useQueryClient();
  const notify = useAppStore((s) => s.notify);
  const active = usePageActive();
  const doctor = useQuery({ queryKey: ["doctor", site.name], queryFn: () => api.projectDoctor(site.name), refetchInterval: active ? 15_000 : false });
  const [confirming, setConfirming] = useState<string | null>(null);
  const fix = useMutation({
    mutationFn: (issue: DoctorIssue) => api.doctorFix(site.name, issue.id),
    onSuccess: (done, issue) => {
      setConfirming(null);
      notify("info", issue.fix?.action.kind === "runCommand" ? `${done}: running, its output is under Commands` : `Fixed: ${done}`);
      if (issue.fix?.action.kind === "runCommand") onCommands();
      // A fix can change nearly anything the project card shows.
      for (const key of ["doctor", "project-facts", "database-status", "sites", "services", "settings", "git-status"]) queryClient.invalidateQueries({ queryKey: [key] });
    },
    onError: (e) => {
      setConfirming(null);
      notify("error", String(e));
      doctor.refetch();
    },
  });

  const issues = doctor.data ?? [];
  const errors = issues.filter((i) => i.severity === "error").length;
  return (
    <section aria-label="Project health" className="rounded-xl border border-line bg-surface">
      <div className="flex items-center gap-2 border-b border-line/70 px-4 py-2.5">
        <Stethoscope size={15} className="text-muted" aria-hidden />
        <h3 className="text-sm font-semibold text-fg">Health</h3>
        <span className={clsx("text-xs", issues.length === 0 ? "text-ok" : errors ? "text-danger-text" : "text-warn")} aria-live="polite">
          {doctor.isLoading ? "Checking…" : issues.length === 0 ? "Everything checks out" : `${issues.length} ${issues.length === 1 ? "thing" : "things"} to look at`}
        </span>
        <span className="flex-1" />
        <button type="button" onClick={() => doctor.refetch()} disabled={doctor.isFetching} className="rounded p-1 text-muted hover:text-fg" aria-label="Check again" title="Check again">
          <RefreshCw size={13} className={clsx(doctor.isFetching && "animate-spin")} />
        </button>
      </div>
      {doctor.isError && <p className="px-4 py-3 text-sm text-danger-text">{String(doctor.error)}</p>}
      {issues.length === 0 && !doctor.isLoading && !doctor.isError && (
        <p className="flex items-center gap-2 px-4 py-3 text-sm text-muted">
          <CheckCircle2 size={15} className="text-ok" aria-hidden /> PHP, dependencies, .env, database, domain and certificate all look right.
        </p>
      )}
      {issues.length > 0 && (
        <ul className="space-y-2 p-3">
          {issues.map((issue) => {
            const t = tone[issue.severity];
            const Icon = t.icon;
            const open = confirming === issue.id;
            return (
              <li key={issue.id} className={clsx("rounded-lg border px-3 py-2", t.ring)}>
                <div className="flex items-start gap-2.5">
                  <Icon size={15} className={clsx("mt-0.5 shrink-0", t.text)} aria-label={issue.severity} />
                  <div className="min-w-0 flex-1">
                    <p className="text-sm font-medium text-fg">{issue.title}</p>
                    <p className="text-xs text-muted">{issue.detail}</p>
                  </div>
                  {issue.fix && !open && (
                    <Button size="sm" onClick={() => setConfirming(issue.id)} title={issue.fix.preview}>
                      <Wrench size={12} /> {issue.fix.label}
                    </Button>
                  )}
                </div>
                {issue.fix && open && (
                  <div className="mt-2 flex flex-wrap items-center gap-2 rounded-md bg-surface px-2.5 py-2 text-xs">
                    <span className="min-w-0 flex-1 text-fg">{issue.fix.preview}</span>
                    <Button size="sm" variant="primary" onClick={() => fix.mutate(issue)} disabled={fix.isPending} data-autofocus>
                      {fix.isPending ? "Working…" : issue.fix.label}
                    </Button>
                    <Button size="sm" variant="ghost" onClick={() => setConfirming(null)} disabled={fix.isPending}>
                      Cancel
                    </Button>
                  </div>
                )}
              </li>
            );
          })}
        </ul>
      )}
    </section>
  );
}
