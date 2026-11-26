import { useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { MoreHorizontal, MoreVertical } from "lucide-react";
import clsx from "clsx";

export interface MenuItem {
  label: string;
  icon?: ReactNode;
  onSelect: () => void;
}

/** A "⋯" button with a small dropdown; `plain` renders a borderless "⋮" for list rows. */
export default function Menu({
  items,
  label = "More actions",
  plain = false,
}: {
  items: MenuItem[];
  label?: string;
  plain?: boolean;
}) {
  const [open, setOpen] = useState(false);
  const root = useRef<HTMLDivElement>(null);
  const list = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState<{ top: number; left: number } | null>(null);

  useEffect(() => {
    if (!open) return;
    const close = (e: MouseEvent | KeyboardEvent) => {
      if (e instanceof KeyboardEvent ? e.key === "Escape" : !root.current?.contains(e.target as Node) && !list.current?.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    // The menu is placed once when it opens; scrolling or resizing would leave it behind.
    const dismiss = () => setOpen(false);
    document.addEventListener("mousedown", close);
    document.addEventListener("keydown", close);
    window.addEventListener("resize", dismiss);
    window.addEventListener("scroll", dismiss, true);
    return () => {
      document.removeEventListener("mousedown", close);
      document.removeEventListener("keydown", close);
      window.removeEventListener("resize", dismiss);
      window.removeEventListener("scroll", dismiss, true);
    };
  }, [open]);

  // The list is drawn in a portal on <body>, so a dimmed card (opacity) or a clipping
  // parent can't fade or cut it. It opens below the button, or above when there's no room.
  useLayoutEffect(() => {
    if (!open) {
      setPos(null);
      return;
    }
    const button = root.current?.getBoundingClientRect();
    const menu = list.current?.getBoundingClientRect();
    if (!button || !menu) return;
    const gap = 6;
    const below = button.bottom + gap;
    const top = below + menu.height > window.innerHeight - 8 && button.top - gap - menu.height > 8 ? button.top - gap - menu.height : below;
    const left = Math.max(8, Math.min(button.right - menu.width, window.innerWidth - menu.width - 8));
    setPos({ top, left });
  }, [open]);

  return (
    <div ref={root} className="relative">
      <button
        aria-label={label}
        aria-expanded={open}
        onClick={() => setOpen((o) => !o)}
        className={clsx(
          "motion-press flex size-8 items-center justify-center rounded-lg text-muted hover:bg-surface-2 hover:text-fg",
          !plain && "border border-line",
        )}
      >
        {plain ? <MoreVertical size={17} /> : <MoreHorizontal size={16} />}
      </button>
      {open &&
        createPortal(
          <div
            ref={list}
            role="menu"
            style={pos ? { top: pos.top, left: pos.left } : { top: 0, left: 0, visibility: "hidden" }}
            className="motion-pop fixed z-50 min-w-48 rounded-xl border border-line bg-surface p-1 shadow-float"
          >
            {items.map((item) => (
              <button
                key={item.label}
                role="menuitem"
                onClick={() => {
                  setOpen(false);
                  item.onSelect();
                }}
                className="flex w-full items-center gap-2.5 whitespace-nowrap rounded-lg px-2.5 py-2 text-left text-sm text-fg hover:bg-surface-2"
              >
                <span className="text-muted">{item.icon}</span>
                {item.label}
              </button>
            ))}
          </div>,
          document.body,
        )}
    </div>
  );
}
