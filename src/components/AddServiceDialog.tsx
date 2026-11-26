import { useState, type ReactNode } from "react";
import { FilePlus2, TriangleAlert } from "lucide-react";
import Dialog from "./Dialog";
import { Button } from "./ui";
import { api, type ServiceFile } from "../lib/api";
import { notifyError } from "../lib/store";

function Fact({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="grid grid-cols-[7rem_minmax(0,1fr)] gap-3 py-1.5 text-sm">
      <dt className="text-muted">{label}</dt>
      <dd className="min-w-0 break-words">{children}</dd>
    </div>
  );
}

/**
 * What a service file downloads, runs and writes, for the user to approve before it's added.
 * Nobody has checked a local file, so nothing is added without this (the backend refuses
 * too). Mounted only while open.
 */
export default function AddServiceDialog({ path, file, onClose, onAdded }: { path: string; file: ServiceFile; onClose: () => void; onAdded: (id: string) => void }) {
  const [busy, setBusy] = useState(false);
  const r = file.review;
  const add = async () => {
    setBusy(true);
    try {
      onAdded(await api.addServiceFile(path, true));
    } catch (e) {
      notifyError(e);
      setBusy(false);
    }
  };
  return (
    <Dialog
      open
      onClose={() => !busy && onClose()}
      title={file.replaces ? `Update ${r.name}?` : `Add ${r.name}?`}
      icon={<FilePlus2 size={18} />}
      width="max-w-xl"
      footer={
        <>
          <Button onClick={onClose} disabled={busy}>
            Cancel
          </Button>
          <Button variant="primary" onClick={add} disabled={busy} data-autofocus>
            {busy ? "Adding…" : file.replaces ? "Update service" : "Add service"}
          </Button>
        </>
      }
    >
      <p className="flex gap-2 rounded-lg border border-warn/40 bg-warn/10 px-3 py-2 text-sm">
        <TriangleAlert size={16} className="mt-0.5 shrink-0 text-warn" aria-hidden />
        <span>This file hasn't been reviewed by LampRabit. Add it only if you trust where it came from: it downloads and runs the program below.</span>
      </p>
      <dl className="mt-3 divide-y divide-line">
        <Fact label="Service">
          {r.name} <span className="font-mono text-xs text-muted">({r.id}, revision {r.revision})</span>
          <span className="block text-xs text-muted">{r.blurb}</span>
        </Fact>
        {r.unavailable && <Fact label="On this computer">{r.unavailable}</Fact>}
        {r.downloads && <Fact label="Downloads from">{r.downloads}</Fact>}
        {r.checksum && <Fact label="Checked with">{r.checksum}</Fact>}
        {r.program && (
          <Fact label="Runs">
            <code className="font-mono text-xs">{[r.program, ...r.args].join(" ")}</code>
          </Fact>
        )}
        {r.commands.length > 0 && (
          <Fact label="Adds commands">
            <span className="block space-y-0.5">
              {r.commands.map((c) => (
                <code key={c} className="block font-mono text-xs">
                  {c}
                </code>
              ))}
            </span>
          </Fact>
        )}
        {r.ports.length > 0 && <Fact label="Ports">{r.ports.map(([label, port]) => `${label} ${port}`).join(", ")}</Fact>}
        {r.files.length > 0 && <Fact label="Writes">{r.files.join(", ")}</Fact>}
        {r.needs.length > 0 && <Fact label="Needs">{r.needs.join(", ")}</Fact>}
        {r.homepage && <Fact label="Homepage">{r.homepage}</Fact>}
      </dl>
      <p className="mt-3 truncate font-mono text-xs text-muted" title={path}>
        {path}
      </p>
    </Dialog>
  );
}
