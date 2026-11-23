import { useState, type ReactNode } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import clsx from "clsx";
import { api, type PortStatus, type ServiceInfo, type ServiceKind, type SitesInfo, type Tunnel } from "../lib/api";
import { notifyError, useAppStore } from "../lib/store";
import { formatBytes, formatCpu, useUsage } from "../lib/usage";
import { useAppStatuses } from "./ProjectRun";
import { Button } from "./ui";

/** A port another program holds, found before the service is started. */
export function conflictFor(service: ServiceInfo, ports?: PortStatus[]) {
  if (service.state === "running" || service.state === "starting") return null;
  return ports?.find((p) => p.port === service.port && !p.free && !p.usedByService) ?? null;
}

export type Issue = { id: string; tone: "danger" | "warn"; title: string; detail?: string; kind?: ServiceKind; actions: ReactNode };

/**
 * What needs doing, from what Home already knows: services that failed or stopped answering,
 * ports another program holds, shared URLs that went down. Each comes with its fix.
 */
export function useIssues(
  services: ServiceInfo[],
  ports: PortStatus[] | undefined,
  tunnels: Tunnel[] | undefined,
  shadows: { command: string; path: string }[] | undefined,
  onShow: (kind: ServiceKind) => void,
): Issue[] {
  const queryClient = useQueryClient();
  const navigate = useAppStore((s) => s.navigate);
  const showLogs = (kind: ServiceKind) => useAppStore.getState().showLogs(kind);
  const refresh = () => queryClient.invalidateQueries({ queryKey: ["services"] });
  const restart = useMutation({ mutationFn: (kind: ServiceKind) => api.restartService(kind), onSuccess: refresh, onError: notifyError });
  const start = useMutation({ mutationFn: (kind: ServiceKind) => api.startService(kind), onSuccess: refresh, onError: notifyError });

  const issues: Issue[] = [];
  for (const s of services.filter((s) => s.installed)) {
    const running = s.state === "running";
    if (running && s.health?.status === "degraded") {
      issues.push({
        id: `degraded-${s.kind}`,
        tone: "danger",
        kind: s.kind,
        title: `${s.label} isn't answering`,
        detail: s.health.detail ?? "The process runs but doesn't answer its health check",
        actions: (
          <>
            <Button size="sm" onClick={() => restart.mutate(s.kind)} disabled={restart.isPending}>
              Restart
            </Button>
            <Button size="sm" variant="ghost" onClick={() => showLogs(s.kind)}>
              Logs
            </Button>
          </>
        ),
      });
    } else if (s.problem && !running) {
      issues.push({
        id: `problem-${s.kind}`,
        tone: "danger",
        kind: s.kind,
        title: s.problem.title,
        detail: s.problem.detail,
        actions: (
          <>
            <Button size="sm" onClick={() => onShow(s.kind)}>
              Show fixes
            </Button>
            <Button size="sm" variant="ghost" onClick={() => start.mutate(s.kind)} disabled={start.isPending}>
              Try again
            </Button>
          </>
        ),
      });
    } else {
      const taken = conflictFor(s, ports);
      if (taken)
        issues.push({
          id: `port-${s.kind}`,
          tone: "warn",
          kind: s.kind,
          title: `Port ${taken.port} is in use`,
          detail: `${taken.owner ? `${taken.owner.name} (PID ${taken.owner.pid})` : "Another program"} holds it, so ${s.label} can't start on it`,
          actions: (
            <>
              {taken.owner?.service ? (
                // Another stack's Windows service: killing it isn't enough, Windows restarts it.
                <Confirm label={`Stop the ${taken.owner.service} service`} run={() => api.stopWindowsService(taken.owner!.service!)} title="Stops it and sets it to start by hand; Windows asks for administrator approval" />
              ) : (
                taken.owner?.stoppable && <Confirm label={`Stop ${taken.owner.name}`} run={() => api.stopProcess(taken.owner!.pid)} />
              )}
              <Button size="sm" variant="ghost" onClick={() => navigate("settings")}>
                Change port
              </Button>
            </>
          ),
        });
    }
  }
  if (shadows?.length) {
    const names = [...new Set(shadows.map((s) => s.command))];
    issues.push({
      id: "path",
      tone: "warn",
      title: `${names.slice(0, 3).join(", ")}${names.length > 3 ? "…" : ""} from another install ${names.length === 1 ? "answers" : "answer"} first in terminals`,
      detail: `${shadows[0].path} is earlier in the system PATH, which Windows reads before yours`,
      actions: <Confirm label="Put LampRabit first" run={() => api.putCommandsFirst()} title="Puts LampRabit's commands first in the system PATH; Windows asks for administrator approval" refresh="path-shadows" />,
    });
  }
  for (const t of tunnels ?? []) {
    if (t.status !== "error") continue;
    issues.push({
      id: `tunnel-${t.id}`,
      tone: "warn",
      title: `Sharing ${t.site ?? t.target} stopped`,
      detail: t.error ?? undefined,
      actions: (
        <Button size="sm" variant="ghost" onClick={() => navigate("sites")}>
          Open project
        </Button>
      ),
    });
  }
  return issues;
}

/** Overriding another program or a system setting takes a second click. */
function Confirm({ label, run, title, refresh = "port-status" }: { label: string; run: () => Promise<void>; title?: string; refresh?: string }) {
  const queryClient = useQueryClient();
  const [sure, setSure] = useState(false);
  const act = useMutation({
    mutationFn: run,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: [refresh] }),
    onError: notifyError,
    onSettled: () => setSure(false),
  });
  return (
    <Button size="sm" variant={sure ? "danger" : "secondary"} title={title} onClick={() => (sure ? act.mutate() : setSure(true))} onBlur={() => setSure(false)} disabled={act.isPending}>
      {sure ? `${label}?` : label}
    </Button>
  );
}

/** The figures at the top of Home: is everything fine, what runs, and what it costs. */
export function StatusStrip({ services, sites, tunnels, issues }: { services: ServiceInfo[]; sites?: SitesInfo; tunnels?: Tunnel[]; issues: number }) {
  const installed = services.filter((s) => s.installed);
  const running = installed.filter((s) => s.state === "running");
  const apps = useAppStatuses();
  const projects = sites?.sites.length ?? 0;
  const projectsUp = Object.values(apps).filter((a) => a.state === "running").length;
  const shared = (tunnels ?? []).filter((t) => t.status === "online").length;
  const latest = useUsage((s) => s.latest);
  const live = running.length > 0 && Object.keys(latest).length > 0;
  const memory = Object.values(latest).reduce((sum, u) => sum + (u.memory ?? 0), 0);
  const label = (id: string) => (id === "php" ? "PHP" : (services.find((s) => s.kind === id)?.label ?? id));
  const shares = Object.values(latest)
    .filter((u) => u.memory > 0)
    .sort((a, b) => b.memory - a.memory)
    .map((u) => ({ id: u.id, name: label(u.id), memory: u.memory }));
  const cpu = Object.values(latest).reduce((sum, u) => sum + (u.cpu ?? 0), 0);

  return (
    <section aria-label="Status" className="grid grid-cols-2 overflow-hidden rounded-xl border border-line bg-surface shadow-card lg:grid-cols-[1.4fr_1fr_1fr_1fr]">
      <Cell label="Status" sub={issues > 0 ? "Fixes are listed below" : installed.length === 0 ? "Nothing installed yet" : running.length === 0 ? "Nothing is running" : "Every running service answers"}>
        <span className="flex items-center gap-2.5">
          <span className={clsx("size-2.5 rounded-full", issues > 0 ? "bg-danger" : running.length > 0 ? "bg-ok" : "bg-muted/60")} aria-hidden />
          {issues > 0 ? `${issues} ${issues === 1 ? "issue" : "issues"}` : running.length > 0 ? "All good" : "Idle"}
        </span>
      </Cell>
      <Cell label="Services" sub={installed.length === 0 ? "Install from Services" : `${installed.length - running.length} stopped`}>
        {running.length}
        <span className="text-muted"> / {installed.length}</span>
      </Cell>
      <Cell label="Projects" sub={[projectsUp > 0 && `${projectsUp} running`, shared > 0 && `${shared} shared`].filter(Boolean).join(" · ") || "In www"}>
        {projects}
      </Cell>
      <Cell
        label="Memory"
        sub={live ? `CPU ${formatCpu(cpu)}${shares[0] ? ` · most: ${shares[0].name}` : ""}` : "When services run"}
        foot={
          live &&
          shares.length > 0 && (
            // Which services use it: the largest in the accent, the rest in neutral steps.
            <div className="mt-3 flex h-1.5 gap-0.5 overflow-hidden rounded-full" role="img" aria-label={shares.map((s) => `${s.name} ${formatBytes(s.memory)}`).join(", ")}>
              {shares.map((s, i) => (
                <span key={s.id} title={`${s.name} ${formatBytes(s.memory)}`} className={clsx("h-full", i === 0 ? "bg-accent" : i === 1 ? "bg-muted/50" : "bg-muted/25")} style={{ flexGrow: s.memory }} />
              ))}
            </div>
          )
        }
      >
        {live ? formatBytes(memory) : "—"}
      </Cell>
    </section>
  );
}

function Cell({ label, sub, foot, children }: { label: string; sub: string; foot?: ReactNode; children: ReactNode }) {
  return (
    <div className="border-line px-5 py-4 not-first:border-l max-lg:nth-3:border-l-0 max-lg:nth-[n+3]:border-t">
      <div className="text-xs text-muted">{label}</div>
      <div className="mt-2 text-2xl font-medium tabular-nums">{children}</div>
      <div className="mt-1 truncate text-xs text-muted">{sub}</div>
      {foot}
    </div>
  );
}

/** Only when something is wrong: each issue with its fix, most serious first. */
export function NeedsAttention({ issues }: { issues: Issue[] }) {
  if (issues.length === 0) return null;
  const sorted = [...issues].sort((a, b) => (a.tone === b.tone ? 0 : a.tone === "danger" ? -1 : 1));
  return (
    <section aria-labelledby="attention-heading" className="rounded-xl border border-line bg-surface shadow-card">
      <h2 id="attention-heading" className="px-5 pt-4 pb-2 text-sm font-medium">
        Needs attention
      </h2>
      <ul className="divide-y divide-line">
        {sorted.map((i) => (
          <li key={i.id} className="flex flex-wrap items-center gap-x-4 gap-y-2 px-5 py-3">
            <span className={clsx("size-2 shrink-0 rounded-full", i.tone === "danger" ? "bg-danger" : "bg-warn")} aria-hidden />
            <div className="min-w-0 flex-1">
              <div className="text-sm font-medium text-fg">{i.title}</div>
              {i.detail && <div className="selectable mt-0.5 line-clamp-2 text-xs text-muted">{i.detail}</div>}
            </div>
            <div className="flex shrink-0 items-center gap-1.5">{i.actions}</div>
          </li>
        ))}
      </ul>
    </section>
  );
}
