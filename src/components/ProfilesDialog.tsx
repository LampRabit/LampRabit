import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Download, Layers, Upload } from "lucide-react";
import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
import clsx from "clsx";
import { api, serviceLabels, type ServiceKind, type StackProfile } from "../lib/api";
import { useAppStore } from "../lib/store";
import Dialog from "./Dialog";
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

/**
 * Export the stack profiles to a JSON file and import them on another machine. An imported
 * profile with the same name replaces the one here; services this LampRabit doesn't know
 * are dropped (an older version), and so are processes of projects that aren't here.
 */
export function ProfilesTransfer() {
  const queryClient = useQueryClient();
  const notify = useAppStore((s) => s.notify);
  const settings = useQuery({ queryKey: ["settings"], queryFn: api.getSettings });
  const profiles = settings.data?.profiles ?? [];
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
      const names = new Set(incoming.map((p) => p.name.toLowerCase()));
      const merged = [...settings.data.profiles.filter((p) => !names.has(p.name.toLowerCase())), ...incoming];
      return { saved: await api.saveSettings({ ...settings.data, profiles: merged }), count: incoming.length };
    },
    onSuccess: (result) => {
      if (!result) return;
      queryClient.setQueryData(["settings"], result.saved);
      notify("info", `Imported ${result.count} ${result.count === 1 ? "profile" : "profiles"}`);
    },
    onError: (e) => notify("error", String(e)),
  });
  return (
    <div className="flex items-center gap-1">
      <Button size="sm" variant="ghost" onClick={() => importFile.mutate()} disabled={importFile.isPending || !settings.data} title="Add profiles from a file exported on another machine">
        <Upload size={13} /> Import
      </Button>
      <Button size="sm" variant="ghost" onClick={() => exportAll.mutate()} disabled={exportAll.isPending || profiles.length === 0} title="Save the profiles to a file">
        <Download size={13} /> Export
      </Button>
    </div>
  );
}
