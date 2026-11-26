import { useQuery } from "@tanstack/react-query";
import { BookOpen, Copy, FileCode2, FolderOpen } from "lucide-react";
import { openPath } from "@tauri-apps/plugin-opener";
import { openUrl } from "../lib/open";
import { api, type ServiceKind } from "../lib/api";
import { useAppStore, notifyError } from "../lib/store";
import { Button, Card } from "./ui";

const folderOf = (path: string) => path.replace(/[\\/][^\\/]*$/, "");

/**
 * Where a service's config files are ("where is php.ini?" is always one click): each with
 * Open in editor, Reveal in folder and Copy path, and its documentation.
 */
export default function ConfigFiles({ kind, docs }: { kind: ServiceKind; docs: string | null }) {
  const notify = useAppStore((s) => s.notify);
  const files = useQuery({ queryKey: ["config-files", kind], queryFn: () => api.serviceConfigFiles(kind) }).data ?? [];
  if (files.length === 0 && !docs) return null;
  return (
    <Card className="mt-4 overflow-hidden">
      <div className="flex items-center justify-between gap-3 border-b border-line px-5 py-3">
        <h3 className="text-sm font-medium">Config files</h3>
        {docs && (
          <Button size="sm" variant="ghost" onClick={() => openUrl(docs).catch(notifyError)}>
            <BookOpen size={14} /> Documentation
          </Button>
        )}
      </div>
      <ul className="divide-y divide-line">
        {files.map((f) => (
          <li key={f.path} className="flex flex-wrap items-center gap-3 px-5 py-2.5">
            <FileCode2 size={15} className="shrink-0 text-muted" aria-hidden />
            <span className="min-w-0 flex-1">
              <span className="block text-sm font-medium">{f.label}</span>
              <span className="block truncate font-mono text-xs text-muted" title={f.path}>
                {f.path}
                {!f.exists && " (written when it starts)"}
              </span>
            </span>
            <span className="flex gap-1">
              <Button size="sm" variant="ghost" onClick={() => api.openInEditor(f.path).catch(notifyError)} disabled={!f.exists} aria-label={`Open ${f.label} in the editor`}>
                Open
              </Button>
              <Button size="sm" variant="ghost" onClick={() => openPath(folderOf(f.path)).catch(notifyError)} aria-label={`Show ${f.label} in its folder`} title="Reveal in folder">
                <FolderOpen size={14} />
              </Button>
              <Button
                size="sm"
                variant="ghost"
                onClick={() => navigator.clipboard.writeText(f.path).then(() => notify("info", "Path copied"), notifyError)}
                aria-label={`Copy the path of ${f.label}`}
                title="Copy path"
              >
                <Copy size={14} />
              </Button>
            </span>
          </li>
        ))}
      </ul>
    </Card>
  );
}
