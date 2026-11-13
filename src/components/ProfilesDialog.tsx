import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Download, Layers, Upload } from "lucide-react";
import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
import clsx from "clsx";
import { api, serviceLabels, type ServiceKind, type StackProfile } from "../lib/api";
import { useAppStore } from "../lib/store";
import { formatBytes } from "../lib/usage";
import Dialog from "./Dialog";
import { InstallStep } from "./FirstRunDialog";
import { Button, inputClass } from "./ui";

/** Create or edit one stack profile. */
export default function ProfilesDialog({
  onClose,
  initial,
  profile,
}: {
  onClose: () => void;
  initial?: ServiceKind[];
  profile?: StackProfile;
}) {
  const queryClient = useQueryClient();
  const notify = useAppStore((s) => s.notify);
  const activeProfile = useAppStore((s) => s.activeProfile);
  const setActiveProfile = useAppStore((s) => s.setActiveProfile);
  const settings = useQuery({ queryKey: ["settings"], queryFn: api.getSettings });
  const services = useQuery({ queryKey: ["services"], queryFn: api.listServices });
  const [name, setName] = useState(profile?.name ?? "");
  const [chosen, setChosen] = useState<ServiceKind[]>(profile?.services ?? initial ?? []);
  const [processes, setProcesses] = useState<string[]>(profile?.processes ?? []);
  // Every project's processes, as "project/process".
  const processChoices = Object.entries(settings.data?.sites ?? {}).flatMap(([project, own]) => (own.processes ?? []).map((p) => `${project}/${p.name}`));
  const choices = (services.data ?? []).filter((service) => service.installed);
  const duplicate = !!name.trim() && (settings.data?.profiles ?? []).some((p) => p.name !== profile?.name && p.name.toLowerCase() === name.trim().toLowerCase());
  const invalid = !name.trim() || duplicate || chosen.length === 0;

  const save = useMutation({
    mutationFn: async () => {
      if (!settings.data) throw new Error("Settings are still loading");
      // A process that was removed from its project drops out of the profile on save.
      const savedProfile = { name: name.trim(), services: chosen, processes: processes.filter((k) => processChoices.includes(k)) };
      const profiles = profile
        ? settings.data.profiles.map((p) => (p.name === profile.name ? savedProfile : p))
        : [...settings.data.profiles, savedProfile];
      return api.saveSettings({ ...settings.data, profiles });
    },
    onSuccess: (saved) => {
      queryClient.setQueryData(["settings"], saved);
      if (profile && activeProfile === profile.name) setActiveProfile(name.trim());
      notify("info", profile ? "Profile updated" : "Profile created");
      onClose();
    },
    onError: (e) => notify("error", String(e)),
  });

  return (
    <Dialog
      open
      onClose={onClose}
      title={profile ? "Edit profile" : "New profile"}
      icon={<Layers size={20} />}
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" onClick={() => save.mutate()} disabled={invalid || save.isPending || !settings.data}>
            {profile ? "Save profile" : "Create profile"}
          </Button>
        </>
      }
    >
      <div className="space-y-4">
        <div>
          <label htmlFor="profile-name" className="mb-1.5 block text-sm font-medium">Profile name</label>
          <input
            id="profile-name"
            autoFocus
            value={name}
            onChange={(event) => setName(event.target.value)}
            placeholder="e.g. Laravel stack"
            maxLength={40}
            className={clsx(inputClass, duplicate && "border-danger")}
            aria-invalid={duplicate || !name.trim() || undefined}
          />
          {duplicate && <p className="mt-1 text-xs text-danger-text">Another profile already has this name.</p>}
        </div>
        <fieldset>
          <legend className="mb-2 text-sm font-medium">Services in this profile</legend>
          <div className="flex flex-wrap gap-2">
            {choices.map((service) => {
              const checked = chosen.includes(service.kind);
              return (
                <label
                  key={service.kind}
                  className={clsx(
                    "flex cursor-pointer items-center gap-2 rounded-lg border px-3 py-1.5 text-sm transition-colors has-focus-visible:ring-2 has-focus-visible:ring-accent/40",
                    checked ? "border-accent/40 bg-accent-soft text-accent-text" : "border-line hover:bg-surface-2",
                  )}
                >
                  <input
                    type="checkbox"
                    className="size-3.5 accent-(--accent)"
                    checked={checked}
                    onChange={() => setChosen((current) => checked ? current.filter((kind) => kind !== service.kind) : [...current, service.kind])}
                  />
                  {serviceLabels[service.kind]}
                </label>
              );
            })}
          </div>
          {choices.length === 0 && <p className="text-sm text-muted">Install services on the Services page first.</p>}
          {choices.length > 0 && chosen.length === 0 && <p className="mt-2 text-xs text-muted">Choose at least one service.</p>}
        </fieldset>
        {processChoices.length > 0 && (
          <fieldset>
            <legend className="mb-1 text-sm font-medium">Project processes</legend>
            <p className="mb-2 text-xs text-muted">Started after the services when this profile starts, stopped when another profile starts.</p>
            <div className="flex flex-wrap gap-2">
              {processChoices.map((key) => {
                const checked = processes.includes(key);
                return (
                  <label
                    key={key}
                    className={clsx(
                      "flex cursor-pointer items-center gap-2 rounded-lg border px-3 py-1.5 font-mono text-xs transition-colors has-focus-visible:ring-2 has-focus-visible:ring-accent/40",
                      checked ? "border-accent/40 bg-accent-soft text-accent-text" : "border-line hover:bg-surface-2",
                    )}
                  >
                    <input type="checkbox" className="size-3.5 accent-(--accent)" checked={checked} onChange={() => setProcesses((cur) => (checked ? cur.filter((k) => k !== key) : [...cur, key]))} />
                    {key}
                  </label>
                );
              })}
            </div>
          </fieldset>
        )}
      </div>
    </Dialog>
  );
}

/** An import waiting on what to do about services that aren't installed here. */
type Review = { incoming: StackProfile[]; missing: ServiceKind[] };

/**
 * Export the stack profiles to a JSON file and import them on another machine. An imported
 * profile with the same name replaces the one here; services this LampRabit doesn't know
 * are dropped (an older version), and so are processes of projects that aren't here.
 * Services it knows but that aren't installed are reviewed first: each is installed, or
 * taken out of the profiles (see ImportReview).
 */
export function ProfilesTransfer() {
  const queryClient = useQueryClient();
  const notify = useAppStore((s) => s.notify);
  const settings = useQuery({ queryKey: ["settings"], queryFn: api.getSettings });
  const services = useQuery({ queryKey: ["services"], queryFn: api.listServices });
  const profiles = settings.data?.profiles ?? [];
  const [review, setReview] = useState<Review | null>(null);
  const [installing, setInstalling] = useState<ServiceKind[] | null>(null);

  /** Adds the profiles, replacing ones with the same name. */
  const commit = async (incoming: StackProfile[]) => {
    const current = await api.getSettings();
    const names = new Set(incoming.map((p) => p.name.toLowerCase()));
    const saved = await api.saveSettings({ ...current, profiles: [...current.profiles.filter((p) => !names.has(p.name.toLowerCase())), ...incoming] });
    queryClient.setQueryData(["settings"], saved);
    notify("info", `Imported ${incoming.length} ${incoming.length === 1 ? "profile" : "profiles"}`);
  };
  const exportAll = useMutation({
    mutationFn: async () => {
      const path = await saveDialog({ defaultPath: "lamprabit-profiles.json", filters: [{ name: "LampRabit profiles", extensions: ["json"] }] });
      if (!path) return false;
      await api.saveText(path, JSON.stringify({ lamprabitProfiles: 1, profiles }, null, 2) + "\n");
      return true;
    },
    onSuccess: (done) => done && notify("info", `Exported ${profiles.length} ${profiles.length === 1 ? "profile" : "profiles"}`),
    onError: (e) => notify("error", String(e)),
  });
  const importFile = useMutation({
    mutationFn: async () => {
      const path = await openDialog({ multiple: false, directory: false, filters: [{ name: "LampRabit profiles", extensions: ["json"] }] });
      if (!path || !settings.data) return null;
      let data: unknown;
      try {
        data = JSON.parse(await api.readText(path));
      } catch {
        throw new Error("That file isn't a LampRabit profile export (it isn't JSON).");
      }
      const list = (data as { profiles?: unknown })?.profiles;
      if (!Array.isArray(list)) throw new Error("That file isn't a LampRabit profile export (no profiles in it).");
      const known = new Set<string>(Object.keys(serviceLabels));
      const processes = new Set(Object.entries(settings.data.sites ?? {}).flatMap(([project, own]) => (own.processes ?? []).map((p) => `${project}/${p.name}`)));
      const incoming: StackProfile[] = [];
      for (const raw of list as { name?: unknown; services?: unknown; processes?: unknown }[]) {
        const name = typeof raw?.name === "string" ? raw.name.trim() : "";
        const services = (Array.isArray(raw?.services) ? raw.services : []).filter((s): s is ServiceKind => typeof s === "string" && known.has(s));
        if (!name || services.length === 0) continue;
        const procs = (Array.isArray(raw?.processes) ? raw.processes : []).filter((p): p is string => typeof p === "string" && processes.has(p));
        incoming.push({ name, services, processes: procs });
      }
      if (incoming.length === 0) throw new Error("No usable profiles in that file: each needs a name and services this LampRabit knows.");
      // Present: installed, or run by another app (Ollama's own).
      const here = services.data ?? (await api.listServices());
      const present = new Set(here.filter((s) => s.installed || s.external).map((s) => s.kind));
      const missing = [...new Set(incoming.flatMap((p) => p.services))].filter((k) => !present.has(k));
      if (missing.length) {
        setReview({ incoming, missing });
        return;
      }
      await commit(incoming);
    },
    onError: (e) => notify("error", String(e)),
  });
  const confirm = useMutation({
    mutationFn: async ({ install, remove }: { install: ServiceKind[]; remove: ServiceKind[] }) => {
      if (!review) return;
      const kept = review.incoming.map((p) => ({ ...p, services: p.services.filter((s) => !remove.includes(s)) }));
      // A profile left with no services has nothing to start.
      const emptied = kept.filter((p) => p.services.length === 0).map((p) => p.name);
      const usable = kept.filter((p) => p.services.length > 0);
      if (usable.length) await commit(usable);
      if (emptied.length) notify("info", `Not imported, no services left: ${emptied.join(", ")}`);
      setReview(null);
      // Saved first, so a failed download doesn't lose the import; its service stays in the
      // profile and can be installed on the Services page later.
      if (install.length) setInstalling(install);
    },
    onError: (e) => notify("error", String(e)),
  });
  return (
    <div className="flex items-center gap-1">
      {review && <ImportReview review={review} busy={confirm.isPending} onCancel={() => setReview(null)} onConfirm={(install, remove) => confirm.mutate({ install, remove })} />}
      {installing && (
        <InstallStep
          runtimes={installing}
          title="Installing the profiles' services"
          nextLabel="Done"
          labels={Object.fromEntries(installing.map((k) => [k, serviceLabels[k]]))}
          onClose={() => setInstalling(null)}
          onNext={() => setInstalling(null)}
        />
      )}
      <Button size="sm" variant="ghost" onClick={() => importFile.mutate()} disabled={importFile.isPending || !settings.data} title="Add profiles from a file exported on another machine">
        <Upload size={13} /> Import
      </Button>
      <Button size="sm" variant="ghost" onClick={() => exportAll.mutate()} disabled={exportAll.isPending || profiles.length === 0} title="Save the profiles to a file">
        <Download size={13} /> Export
      </Button>
    </div>
  );
}

/**
 * The services an import needs that aren't installed here: each one is installed (with its
 * download size) or removed from the profiles that use it. One that can't be installed on
 * this computer is set to Remove, with the reason.
 */
function ImportReview({ review, busy, onCancel, onConfirm }: { review: Review; busy: boolean; onCancel: () => void; onConfirm: (install: ServiceKind[], remove: ServiceKind[]) => void }) {
  const plan = useQuery({ queryKey: ["starter-plan", review.missing.join(",")], queryFn: () => api.starterPlan(review.missing), staleTime: 5 * 60_000 });
  const items = new Map((plan.data ?? []).map((item) => [item.runtime as string, item]));
  const blocked = (k: ServiceKind) => items.get(k)?.action.kind === "unavailable";
  const [choice, setChoice] = useState<Record<string, "install" | "remove">>(() => Object.fromEntries(review.missing.map((k) => [k, "install"])));
  const pick = (k: ServiceKind) => (blocked(k) ? "remove" : choice[k]);
  const install = review.missing.filter((k) => pick(k) === "install");
  const remove = review.missing.filter((k) => pick(k) === "remove");
  const size = install.reduce((sum, k) => sum + (items.get(k)?.size ?? 0), 0);
  const usedBy = (k: ServiceKind) => review.incoming.filter((p) => p.services.includes(k)).map((p) => p.name);
  // Profiles that would be left with nothing to start.
  const emptied = review.incoming.filter((p) => p.services.every((s) => remove.includes(s))).map((p) => p.name);
  const one = review.missing.length === 1;

  return (
    <Dialog
      open
      onClose={onCancel}
      title={one ? "A service isn't installed" : "Some services aren't installed"}
      icon={<Layers size={20} />}
      width="max-w-xl"
      footer={
        <>
          <Button onClick={onCancel} className="mr-auto">
            Cancel
          </Button>
          <Button variant="primary" onClick={() => onConfirm(install, remove)} disabled={busy || plan.isLoading || emptied.length === review.incoming.length} data-autofocus>
            {install.length ? `Install ${install.length} and import` : "Import"}
          </Button>
        </>
      }
    >
      <p className="mb-3 text-sm text-muted">
        The imported {review.incoming.length === 1 ? "profile uses" : "profiles use"} {one ? "a service that isn't" : "services that aren't"} installed on this computer. Install{" "}
        {one ? "it" : "them"}, or take {one ? "it" : "them"} out of the profiles.
      </p>
      <ul className="divide-y divide-line rounded-xl border border-line">
        {review.missing.map((k) => {
          const item = items.get(k);
          const a = item?.action;
          const note = plan.isLoading
            ? "Checking…"
            : a?.kind === "install"
              ? `Downloads ${a.release.version}${item?.size ? ` · ${formatBytes(item.size)}` : ""}`
              : a?.kind === "activate"
                ? `Installed: ${a.version} (becomes the version in use)`
                : a?.kind === "unavailable"
                  ? a.reason
                  : "";
          return (
            <li key={k} className="flex flex-wrap items-center gap-x-3 gap-y-1.5 px-3 py-2">
              <span className="min-w-0 flex-1">
                <span className="block text-sm font-medium text-fg">{serviceLabels[k]}</span>
                <span className="block text-xs text-muted">In {usedBy(k).join(", ")}</span>
                {note && <span className={clsx("block text-xs", a?.kind === "unavailable" ? "text-warn" : "text-muted")}>{note}</span>}
              </span>
              <span role="radiogroup" aria-label={`${serviceLabels[k]}: install or remove`} className="flex rounded-lg border border-line p-0.5 text-xs">
                {(["install", "remove"] as const).map((option) => (
                  <button
                    key={option}
                    type="button"
                    role="radio"
                    aria-checked={pick(k) === option}
                    disabled={option === "install" && blocked(k)}
                    onClick={() => setChoice((c) => ({ ...c, [k]: option }))}
                    className={clsx("rounded-md px-2.5 py-1 font-medium transition-colors disabled:opacity-40", pick(k) === option ? "bg-accent-soft text-accent-text" : "text-muted hover:text-fg")}
                  >
                    {option === "install" ? "Install" : "Remove"}
                  </button>
                ))}
              </span>
            </li>
          );
        })}
      </ul>
      {install.length > 0 && size > 0 && <p className="mt-2 text-xs text-muted">About {formatBytes(size)} to download. The profiles are saved first, so a failed download can be retried.</p>}
      {emptied.length > 0 && (
        <p className="mt-2 text-xs text-warn">
          {emptied.length === review.incoming.length ? "Every profile would be left without services: install at least one." : `Left without services, so not imported: ${emptied.join(", ")}.`}
        </p>
      )}
    </Dialog>
  );
}
