import { useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { AlertTriangle, Stethoscope, X } from "lucide-react";
import { api, type Problem, type ServiceKind } from "../lib/api";
import { useAppStore, notifyError } from "../lib/store";
import { Button } from "./ui";

/** Why a service failed, what to do about it, and one-click fixes where possible. */
export default function ProblemPanel({ kind, problem }: { kind: ServiceKind; problem: Problem }) {
  const queryClient = useQueryClient();
  const notify = useAppStore((s) => s.notify);
  const navigate = useAppStore((s) => s.navigate);
  const openDiagnostics = useAppStore((s) => s.setDiagnosticsOpen);
  const [confirmStop, setConfirmStop] = useState(false);
  const refresh = () => queryClient.invalidateQueries({ queryKey: ["services"] });

  const dismiss = useMutation({ mutationFn: () => api.clearProblem(kind), onSuccess: refresh });
  const stopOwner = useMutation({
    // Another stack's Windows service is stopped as a service (admin prompt): Windows would
    // restart a killed process.
    mutationFn: () => (problem.owner!.service ? api.stopWindowsService(problem.owner!.service) : api.stopProcess(problem.owner!.pid)),
    onSuccess: () => {
      notify("info", `Stopped ${problem.owner!.name}. You can start the service now.`);
      setConfirmStop(false);
      dismiss.mutate();
    },
    onError: notifyError,
  });

  return (
    <div className="mt-5 rounded-xl border border-line bg-surface-2/40 p-4 text-sm">
      <div className="flex items-start gap-3">
        <AlertTriangle size={18} className="mt-0.5 shrink-0 text-danger-text" />
        <div className="min-w-0 flex-1">
          <div className="font-medium text-danger-text">{problem.title}</div>
          <p className="selectable mt-1 break-words text-fg/80">{problem.detail}</p>
          {problem.fixes.length > 0 && (
            <ul className="mt-2 list-disc space-y-1 pl-4 text-fg/80">
              {problem.fixes.map((f) => (
                <li key={f}>{f}</li>
              ))}
            </ul>
          )}
          <div className="mt-3 flex flex-wrap gap-2">
            {problem.port !== null && (
              <Button size="sm" onClick={() => navigate("settings")}>
                Change port
              </Button>
            )}
            {(problem.owner?.stoppable || problem.owner?.service) &&
              (confirmStop ? (
                <>
                  <Button size="sm" variant="danger" onClick={() => stopOwner.mutate()} disabled={stopOwner.isPending}>
                    {problem.owner.service ? `Stop the ${problem.owner.service} service (asks for admin)` : `Stop ${problem.owner.name} (PID ${problem.owner.pid})`}
                  </Button>
                  <Button size="sm" onClick={() => setConfirmStop(false)}>
                    Cancel
                  </Button>
                </>
              ) : (
                <Button size="sm" onClick={() => setConfirmStop(true)}>
                  {problem.owner.service ? `Stop the ${problem.owner.service} service…` : `Stop ${problem.owner.name}…`}
                </Button>
              ))}
            <Button size="sm" variant="ghost" onClick={() => openDiagnostics(true)}>
              <Stethoscope size={14} /> Diagnostics
            </Button>
          </div>
        </div>
        <button
          aria-label="Dismiss"
          onClick={() => dismiss.mutate()}
          className="rounded-md p-1 text-muted hover:bg-surface hover:text-fg"
        >
          <X size={15} />
        </button>
      </div>
    </div>
  );
}
