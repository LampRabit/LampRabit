import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Download, Hexagon, Loader2, RefreshCw } from "lucide-react";
import clsx from "clsx";
import { api } from "../lib/api";
import { useAppStore, notifyError } from "../lib/store";
import { Button, Card, IconTile } from "./ui";
import { ActiveTile, AvailableList, InstalledTile, InstalledTiles, NoActiveTile, VersionsLayout } from "./versions";

/** Node.js through NVM, laid out like every other runtime: install NVM on request, then install, switch and remove versions. */
export default function NodeCard() {
  const queryClient = useQueryClient();
  const notify = useAppStore((s) => s.notify);
  const os = useQuery({ queryKey: ["paths"], queryFn: api.envPaths }).data?.os;
  const status = useQuery({ queryKey: ["node"], queryFn: api.nodeStatus });
  const available = useQuery({
    queryKey: ["node-available"],
    queryFn: api.nodeAvailable,
    enabled: !!status.data?.manager,
    staleTime: 60 * 60 * 1000,
  });
  const refresh = () => queryClient.invalidateQueries({ queryKey: ["node"] });

  const installNvm = useMutation({ mutationFn: api.nvmInstall, onSuccess: refresh, onError: notifyError });
  const install = useMutation({
    mutationFn: (v: string) => api.nodeInstall(v),
    onSuccess: (_, v) => {
      notify("info", `Node ${v} installed`);
      refresh();
    },
    onError: notifyError,
  });
  const use = useMutation({ mutationFn: (v: string) => api.nodeUse(v), onSuccess: refresh, onError: notifyError });
  const remove = useMutation({ mutationFn: (v: string) => api.nodeUninstall(v), onSuccess: refresh, onError: notifyError });
  const enablePms = useMutation({
    mutationFn: api.nodeEnablePackageManagers,
    onSuccess: () => {
      notify("info", "pnpm and Yarn are on");
      refresh();
    },
    onError: notifyError,
  });

  const data = status.data;
  const busy = install.isPending || use.isPending || remove.isPending;
  const others = (data?.installed ?? []).filter((v) => v !== data?.current);
  const choices = (available.data ?? []).filter((r) => !data?.installed.includes(r.version));
  const pmsOn = !!data?.packageManagers?.length && data.packageManagers.every((p) => p.enabled);

  return (
    <Card className="overflow-hidden">
      <div className="flex flex-wrap items-start gap-4 border-b border-line px-5 py-4">
        <IconTile className="bg-lime-600/10 text-lime-600">
          <Hexagon size={22} strokeWidth={1.75} />
        </IconTile>
        <div className="min-w-0 flex-1 basis-60">
          <h3 className="text-lg font-bold tracking-tight">Node.js</h3>
          <p className="text-sm text-muted">
            JavaScript runtime ·{" "}
            {data?.manager
              ? `Managed with ${data.manager}${data.managerVersion ? ` ${data.managerVersion}` : ""}`
              : "Versions are managed through NVM, installed only when you ask"}
          </p>
          {data?.needsElevation && (
            <p className="mt-1 text-xs font-medium text-accent-text">Switching versions asks for administrator approval (turn on Developer Mode to avoid it).</p>
          )}
        </div>
        {data?.manager && (
          <Button size="sm" onClick={() => available.refetch()} disabled={available.isFetching} aria-label="Check for new Node.js versions" title="Check for new versions">
            <RefreshCw size={14} className={clsx(available.isFetching && "animate-spin")} />
          </Button>
        )}
      </div>

      {status.isLoading ? (
        <p className="p-5 text-sm text-muted">Checking for NVM…</p>
      ) : !data?.manager ? (
        <VersionsLayout active={<NoActiveTile text="Node versions are installed through NVM, which LampRabit sets up when you ask." />}>
          <section className="space-y-3">
            <h4 className="text-xs font-bold tracking-wider text-muted uppercase">Set up NVM</h4>
            <p className="text-sm text-muted">
              {os === "windows"
                ? "Installs nvm-windows with its official installer. Windows asks for administrator approval once. Switching Node versions later needs admin rights unless Developer Mode is on."
                : "Runs the official nvm install script, which adds nvm to your shell profile."}
            </p>
            <Button variant="primary" onClick={() => installNvm.mutate()} disabled={installNvm.isPending}>
              {installNvm.isPending ? <Loader2 size={15} className="animate-spin" /> : <Download size={15} />} Install NVM
            </Button>
          </section>
        </VersionsLayout>
      ) : (
        <VersionsLayout
          active={
            data.current ? (
              <ActiveTile
                version={data.current}
                details={
                  <>
                    <div>Default for new terminals</div>
                    {data.installed.length > 1 && <div>{data.installed.length} versions installed</div>}
                  </>
                }
              />
            ) : (
              <NoActiveTile text="Install a version from the list; projects with an .nvmrc show the one they expect on the Sites page." />
            )
          }
        >
          {others.length > 0 && (
            <InstalledTiles>
              {others.map((v) => (
                <InstalledTile
                  key={v}
                  version={v}
                  label={available.data?.find((r) => r.version === v)?.lts ? "LTS" : null}
                  name={`Node ${v}`}
                  busy={busy}
                  onUse={() => use.mutate(v)}
                  onRemove={() => remove.mutate(v)}
                />
              ))}
            </InstalledTiles>
          )}
          {data.current && (
            <section className="space-y-2">
              <h4 className="text-xs font-bold tracking-wider text-muted uppercase">pnpm and Yarn</h4>
              {pmsOn ? (
                <p className="text-sm text-muted">On for Node {data.current} through corepack. Each project gets the version its package.json asks for.</p>
              ) : (
                <>
                  <p className="text-sm text-muted">
                    Turns on corepack's pnpm and yarn commands for Node {data.current}
                    {data.corepack ? "" : ", installing corepack from npm first (Node 25 and later don't include it)"}. Until then, projects run them through npx, which is slower.
                  </p>
                  <Button size="sm" onClick={() => enablePms.mutate()} disabled={enablePms.isPending}>
                    {enablePms.isPending ? <Loader2 size={14} className="animate-spin" /> : <Download size={14} />} Turn on pnpm and Yarn
                  </Button>
                </>
              )}
            </section>
          )}
          <AvailableList
            title={data.current ? "Available to download" : "Choose a version"}
            name="Node"
            loading={available.isLoading}
            error={available.isError ? available.error : undefined}
            limit={8}
            items={choices.map((r) => ({
              version: r.version,
              label: r.lts ? `LTS · ${r.lts}` : "Current",
              progress: install.isPending && install.variables === r.version ? null : undefined,
              stage: "Installing",
            }))}
            onInstall={(v) => !busy && install.mutate(v)}
          />
        </VersionsLayout>
      )}
    </Card>
  );
}
