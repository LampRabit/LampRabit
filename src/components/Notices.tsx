import { X } from "lucide-react";
import clsx from "clsx";
import { useAppStore } from "../lib/store";

export default function Notices() {
  const notices = useAppStore((s) => s.notices);
  const dismiss = useAppStore((s) => s.dismiss);
  // The newest activity (a service started, stopped or failed; an install finished), read out
  // by screen readers wherever focus is; the toasts below are announced as they appear.
  const latest = useAppStore((s) => s.activity[0]);

  return (
    <>
      <div className="sr-only" role="status" aria-live="polite">
        {latest?.text}
      </div>
      <div className="pointer-events-none fixed flex w-80 flex-col gap-2" style={{ right: "calc(var(--nav-right, 0px) + 1rem)", bottom: "calc(var(--nav-bottom, 0px) + 1rem)" }} aria-live="polite">
      {notices.map((n) => (
        <div
          key={n.id}
          role={n.tone === "error" ? "alert" : "status"}
          className={clsx(
            "motion-toast pointer-events-auto flex items-start gap-2 rounded-lg border px-3 py-2 text-sm shadow-lg",
            n.tone === "error"
              ? "border-danger/30 bg-danger-soft text-danger-text backdrop-blur"
              : "border-line bg-surface text-fg",
          )}
        >
          <span className="selectable min-w-0 flex-1 break-words">
            {n.text}
            {n.hint && <span className="mt-1 block text-xs text-fg/80">{n.hint}</span>}
            {n.detail && n.detail.length > 0 && (
              <span className="mt-1.5 block rounded bg-terminal px-2 py-1.5 font-mono text-[11px] leading-snug break-all whitespace-pre-wrap text-slate-300">{n.detail.join("\n")}</span>
            )}
          </span>
          <button onClick={() => dismiss(n.id)} className="opacity-60 hover:opacity-100" aria-label="Dismiss">
            <X size={14} />
          </button>
        </div>
      ))}
      </div>
    </>
  );
}
