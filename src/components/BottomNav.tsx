import { FileText, Globe, House, Layers, Settings } from "lucide-react";
import clsx from "clsx";
import { useAppStore, type PageId } from "../lib/store";
import { useRuntimeUpdates } from "../lib/updates";

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
              {id === "services" && runtimeUpdates > 0 && (
                <span className="absolute -top-0.5 -right-1 size-2 rounded-full bg-accent ring-2 ring-surface" aria-label="Updates available" />
              )}
            </span>
            {label}
          </button>
        ))}
      </div>
    </nav>
  );
}
