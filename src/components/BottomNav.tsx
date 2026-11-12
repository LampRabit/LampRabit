import { FileText, Globe, House, Layers, Settings } from "lucide-react";
import clsx from "clsx";
import { useAppStore, type PageId } from "../lib/store";
import { useRuntimeUpdates } from "../lib/updates";
import { useQuery } from "@tanstack/react-query";
import { api } from "../lib/api";

const items: { id: PageId; label: string; icon: typeof House }[] = [
  { id: "overview", label: "Home", icon: House },
  { id: "sites", label: "Sites", icon: Globe },
  { id: "services", label: "Services", icon: Layers },
  { id: "logs", label: "Logs", icon: FileText },
  { id: "settings", label: "Settings", icon: Settings },
];

export default function BottomNav() {
  const page = useAppStore((s) => s.page);
  const navigate = useAppStore((s) => s.navigate);
  const runtimeUpdates = useRuntimeUpdates().data?.length ?? 0;
  const services = useQuery({ queryKey: ["services"], queryFn: api.listServices });
  const failed = (services.data ?? []).some((s) => s.state === "error");
  // Newest lines are last; stop at the first one older than the last visit.
  const newErrors = useAppStore((s) => {
    if (s.page === "logs") return false;
    for (let i = s.logs.length - 1; i >= 0; i--) {
      const l = s.logs[i];
      if (l.time.getTime() <= s.logsSeenAt) return false;
      if (l.level === "error") return true;
    }
    return false;
  });
  const dot: Partial<Record<PageId, { label: string; tone: string }>> = {
    ...(failed && { overview: { label: "A service failed", tone: "bg-danger" } }),
    ...(runtimeUpdates > 0 && { services: { label: "Updates available", tone: "bg-accent" } }),
    ...(newErrors && { logs: { label: "New errors", tone: "bg-danger" } }),
  };

  return (
    // Docked under the page's scroll area rather than floating over it: a floating bar hid
    // whatever scrolled under it (Settings' sections, long lists).
    <nav aria-label="Pages" className="flex shrink-0 justify-center px-4 pt-1.5 pb-3">
      <div className="pointer-events-auto relative flex items-center gap-1 rounded-2xl border border-line bg-nav p-1.5 shadow-float backdrop-blur-md">
        {/* One highlight that glides to the active tab (button width + gap = 5.5rem). */}
        {/* Pages outside the bar leave nothing highlighted. */}
        {items.some((i) => i.id === page) && (
          <span
            aria-hidden
            className="absolute inset-y-1.5 left-1.5 w-[5.25rem] rounded-xl bg-accent-soft transition-transform duration-300 ease-(--ease-spring)"
            style={{ transform: `translateX(${items.findIndex((i) => i.id === page) * 5.5}rem)` }}
          />
        )}
        {items.map(({ id, label, icon: Icon }) => (
          <button
            key={id}
            onClick={() => navigate(id)}
            aria-current={page === id ? "page" : undefined}
            aria-keyshortcuts={String(items.findIndex((i) => i.id === id) + 1)}
            title={`${label} (${items.findIndex((i) => i.id === id) + 1})`}
            className={clsx(
              "group relative flex w-[5.25rem] flex-col items-center gap-1 rounded-xl py-2 text-xs transition-colors",
              page === id ? "font-semibold text-accent-text" : "text-muted hover:bg-surface-2/60 hover:text-fg",
            )}
          >
            <span className="relative transition-transform duration-200 ease-(--ease-spring) group-hover:-translate-y-0.5 group-active:scale-90 still:translate-none still:scale-100">
              <Icon size={21} strokeWidth={page === id ? 2 : 1.6} />
              {dot[id] && <span className={clsx("absolute -top-0.5 -right-1 size-2 rounded-full ring-2 ring-surface", dot[id]!.tone)} aria-label={dot[id]!.label} />}
            </span>
            {label}
          </button>
        ))}
      </div>
    </nav>
  );
}
