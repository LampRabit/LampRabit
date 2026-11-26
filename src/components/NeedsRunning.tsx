import { useMutation, useQuery } from "@tanstack/react-query";
import { Play } from "lucide-react";
import { api, type InstalledRuntime, type Runtime, type ServiceKind } from "../lib/api";
import { infoFor, labelFor, useCatalog } from "../lib/catalog";
import { names, stoppedNeeds } from "../lib/dependencies";
import { notifyError } from "../lib/store";
import { Button, Card } from "./ui";

/**
 * On a service's page: what it needs running that isn't (pgAdmin without PostgreSQL,
 * phpMyAdmin with no database up), with Start buttons. Hidden until the service is installed.
 */
export default function NeedsRunning({ runtime, installed }: { runtime: Runtime; installed?: InstalledRuntime[] }) {
  const catalog = useCatalog();
  const services = useQuery({ queryKey: ["services"], queryFn: api.listServices });
  const start = useMutation({ mutationFn: async (kinds: ServiceKind[]) => { for (const k of kinds) await api.startService(k); }, onError: notifyError });
  if (!installed?.find((i) => i.runtime === runtime)?.active) return null;
  const { all, oneOf } = stoppedNeeds(runtime, infoFor(catalog.data, runtime), services.data);
  if (all.length === 0 && oneOf.length === 0) return null;
  const name = labelFor(catalog.data, runtime);
  const text = all.length
    ? `${name} needs ${names(all, "and")} running${oneOf.length ? `, and ${names(oneOf, "or")}` : ""}.`
    : `${name} has no database to open: start ${names(oneOf, "or")}.`;
  return (
    <Card className="mt-4 flex flex-wrap items-center gap-3 px-5 py-3">
      <span className="size-1.5 shrink-0 rounded-full bg-warn" aria-hidden />
      <p className="min-w-0 flex-1 text-sm">{text}</p>
      {all.length > 0 && (
        <Button size="sm" variant="soft" disabled={start.isPending} onClick={() => start.mutate(all.map((s) => s.kind))}>
          <Play size={13} /> {start.isPending ? "Starting…" : `Start ${names(all, "and")}`}
        </Button>
      )}
      {all.length === 0 &&
        oneOf.map((s) => (
          <Button key={s.kind} size="sm" variant="soft" disabled={start.isPending} onClick={() => start.mutate([s.kind])}>
            <Play size={13} /> Start {s.label}
          </Button>
        ))}
    </Card>
  );
}
