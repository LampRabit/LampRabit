import { useEffect, useLayoutEffect, useRef, useState, type KeyboardEvent, type PointerEvent } from "react";
import { FileText, Globe, Grip, House, Layers, Settings } from "lucide-react";
import clsx from "clsx";
import { useQuery } from "@tanstack/react-query";
import { api } from "../lib/api";
import { motionAllowed } from "../lib/motion";
import { useAppStore, type NavAlign, type NavEdge, type NavPlacement, type PageId } from "../lib/store";
import { useRuntimeUpdates } from "../lib/updates";

const items: { id: PageId; label: string; icon: typeof House }[] = [
  { id: "overview", label: "Home", icon: House },
  { id: "sites", label: "Sites", icon: Globe },
  { id: "services", label: "Services", icon: Layers },
  { id: "logs", label: "Logs", icon: FileText },
  { id: "settings", label: "Settings", icon: Settings },
];

/**
 * The page tab bar. It docks to one of twelve places, an edge of the window's content (top,
 * right, bottom, left) and a position along it (start, center, end), and floats over the
 * page there; the page pads that edge by the bar's size so nothing ends up hidden under it.
 * Drag it (by the grip, or anywhere on it): it floats under the pointer in the shape it will
 * have, the slot under the pointer lights up, and on release it glides into place. On the grip,
 * the arrow keys dock to that edge, Shift+arrows move it along the edge, and Escape puts it
 * back at the bottom center. App.tsx lays it out with `navLayout`; Settings → Appearance sets
 * it too. Its size is published as --nav-top/right/bottom/left, so floating bars (Apply
 * changes, notices) stay clear of it.
 */

const vertical = (edge: NavEdge) => edge === "left" || edge === "right";

/**
 * Where App puts the bar: a click-through strip along its edge, over the page, with the bar
 * placed along it. The page doesn't give up that strip; it pads its content by --nav-<edge>
 * instead, so content scrolls under the floating bar and the last of it still comes into view.
 */
export function navLayout({ edge, align }: NavPlacement) {
  const along = { start: "justify-start", center: "justify-center", end: "justify-end" }[align];
  const strip = { top: "inset-x-0 top-0", bottom: "inset-x-0 bottom-0", left: "inset-y-0 left-0", right: "inset-y-0 right-0" }[edge];
  return {
    // z-[15]: above the page and its sticky headers, below menus (z-20), floating bars and dialogs.
    wrapper: clsx("pointer-events-none absolute z-[15] flex", strip, vertical(edge) ? "flex-col" : "flex-row", along),
  } as const;
}

type Drag = { pointerId: number; startX: number; startY: number; grabX: number; grabY: number; moved: boolean; w: number; h: number };

/** The slot under a point: the nearest edge of the body, and which third along it. */
function slotAt(x: number, y: number, body: DOMRect): NavPlacement {
  const distances: [NavEdge, number][] = [
    ["left", (x - body.left) / body.width],
    ["right", (body.right - x) / body.width],
    ["top", (y - body.top) / body.height],
    ["bottom", (body.bottom - y) / body.height],
  ];
  const edge = distances.sort((a, b) => a[1] - b[1])[0][0];
  const t = vertical(edge) ? (y - body.top) / body.height : (x - body.left) / body.width;
  return { edge, align: t < 1 / 3 ? "start" : t > 2 / 3 ? "end" : "center" };
}

const same = (a: NavPlacement, b: NavPlacement) => a.edge === b.edge && a.align === b.align;

/** Animates an element from where it was (`from`) to where layout put it, by transform only. */
function glide(el: Element | null, from: DOMRect | null) {
  if (!el || !from || !motionAllowed()) return;
  const to = el.getBoundingClientRect();
  const dx = from.left - to.left;
  const dy = from.top - to.top;
  if (Math.abs(dx) < 1 && Math.abs(dy) < 1) return;
  el.animate([{ transform: `translate(${dx}px, ${dy}px)` }, { transform: "none" }], { duration: 420, easing: "cubic-bezier(0.22, 1.2, 0.36, 1)" });
}

const ALIGNS: NavAlign[] = ["start", "center", "end"];

export default function BottomNav() {
  const page = useAppStore((s) => s.page);
  const navigate = useAppStore((s) => s.navigate);
  const placement = useAppStore((s) => s.navPosition);
  const setPlacement = useAppStore((s) => s.setNavPosition);
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

  const nav = useRef<HTMLElement>(null);
  const bar = useRef<HTMLDivElement>(null);
  const drag = useRef<Drag | null>(null);
  const suppressClick = useRef(false);
  // While dragging: where the bar floats (its top-left corner), the slot under the pointer
  // and the body's rect (for drawing the slots).
  const [ghost, setGhost] = useState<{ x: number; y: number; slot: NavPlacement; body: DOMRect } | null>(null);
  const shown = ghost?.slot ?? placement;
  const isVertical = vertical(shown.edge);

  // Where the bar was after the last render: when the placement changes (a drop, the keys,
  // Settings), it glides from there to its new place. A drop starts the glide from where it
  // was let go.
  const last = useRef<{ placement: NavPlacement; bar: DOMRect | null } | null>(null);
  const dropFrom = useRef<DOMRect | null>(null);
  const dockTo = (next: NavPlacement, from?: DOMRect) => {
    dropFrom.current = from ?? null;
    if (same(next, placement) && from) {
      // Dropped on its own slot: glide back from the drop point.
      requestAnimationFrame(() => {
        glide(bar.current, dropFrom.current);
        dropFrom.current = null;
      });
      return;
    }
    setPlacement(next);
  };
  useLayoutEffect(() => {
    const was = last.current;
    if (was && !same(was.placement, placement)) {
      glide(bar.current, dropFrom.current ?? was.bar);
      dropFrom.current = null;
    }
    last.current = { placement, bar: bar.current?.getBoundingClientRect() ?? null };
  });

  // The room the bar takes at its edge: the page pads by it, and so does anything fixed to
  // the window's corners.
  useLayoutEffect(() => {
    const el = nav.current;
    if (!el) return;
    const publish = () => {
      const size = vertical(placement.edge) ? el.offsetWidth : el.offsetHeight;
      const root = document.documentElement.style;
      for (const side of ["top", "right", "bottom", "left"] as const) root.setProperty(`--nav-${side}`, `${side === placement.edge ? size : 0}px`);
    };
    publish();
    const observer = new ResizeObserver(publish);
    observer.observe(el);
    return () => observer.disconnect();
  }, [placement.edge]);

  // The highlight measures the active tab, so it lines up in either orientation.
  const tabs = useRef<Partial<Record<PageId, HTMLButtonElement | null>>>({});
  const [highlight, setHighlight] = useState<{ x: number; y: number; w: number; h: number } | null>(null);
  useLayoutEffect(() => {
    const el = tabs.current[page];
    setHighlight(el ? { x: el.offsetLeft, y: el.offsetTop, w: el.offsetWidth, h: el.offsetHeight } : null);
  }, [page, isVertical]);

  // Escape cancels a drag in progress.
  useEffect(() => {
    if (!ghost) return;
    const onKey = (e: globalThis.KeyboardEvent) => {
      if (e.key !== "Escape") return;
      drag.current = null;
      setGhost(null);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [ghost]);

  function onPointerDown(e: PointerEvent<HTMLDivElement>) {
    if (e.button !== 0) return;
    const rect = e.currentTarget.getBoundingClientRect();
    drag.current = { pointerId: e.pointerId, startX: e.clientX, startY: e.clientY, grabX: e.clientX - rect.left, grabY: e.clientY - rect.top, moved: false, w: rect.width, h: rect.height };
    // Captured right away, or a quick drag leaves the bar before it counts as one. On the
    // tab or grip pressed (not the bar), so a plain click still lands on that tab.
    ((e.target as HTMLElement).closest("button") ?? e.currentTarget).setPointerCapture(e.pointerId);
  }

  function onPointerMove(e: PointerEvent<HTMLDivElement>) {
    const d = drag.current;
    if (!d || d.pointerId !== e.pointerId) return;
    // A click on a tab isn't a drag until the pointer really moves.
    if (!d.moved) {
      if (Math.hypot(e.clientX - d.startX, e.clientY - d.startY) < 6) return;
      d.moved = true;
    }
    e.preventDefault();
    const body = document.querySelector("[data-app-body]")?.getBoundingClientRect();
    if (!body) return;
    // The bar stays under the pointer where it was grabbed, inside the window.
    const rect = e.currentTarget.getBoundingClientRect();
    const x = Math.min(Math.max(e.clientX - d.grabX, 4), window.innerWidth - rect.width - 4);
    const y = Math.min(Math.max(e.clientY - d.grabY, 4), window.innerHeight - rect.height - 4);
    setGhost({ x, y, slot: slotAt(e.clientX, e.clientY, body), body });
  }

  function onPointerUp(e: PointerEvent<HTMLDivElement>) {
    const d = drag.current;
    if (!d || d.pointerId !== e.pointerId) return;
    drag.current = null;
    if (d.moved && ghost) {
      const from = e.currentTarget.getBoundingClientRect();
      suppressClick.current = true;
      window.setTimeout(() => (suppressClick.current = false), 0);
      setGhost(null);
      dockTo(ghost.slot, from);
    } else setGhost(null);
  }

  function onGripKey(e: KeyboardEvent<HTMLButtonElement>) {
    const edges: Record<string, NavEdge> = { ArrowUp: "top", ArrowDown: "bottom", ArrowLeft: "left", ArrowRight: "right" };
    let next: NavPlacement | null = null;
    if (e.key === "Escape" || e.key === "Home") next = { edge: "bottom", align: "center" };
    else if (edges[e.key] && e.shiftKey) {
      // Along the edge: Left/Up toward the start, Right/Down toward the end.
      const step = e.key === "ArrowLeft" || e.key === "ArrowUp" ? -1 : 1;
      const i = Math.min(2, Math.max(0, ALIGNS.indexOf(placement.align) + step));
      next = { edge: placement.edge, align: ALIGNS[i] };
    } else if (edges[e.key]) next = { edge: edges[e.key], align: placement.align };
    if (!next) return;
    e.preventDefault();
    e.stopPropagation();
    dockTo(next);
  }

  // Twelve slots while dragging: three along each edge, the one under the pointer lit up.
  const slots = ghost && (
    <div className="pointer-events-none fixed z-[70]" style={{ left: ghost.body.left, top: ghost.body.top, width: ghost.body.width, height: ghost.body.height }} aria-hidden>
      {(["top", "right", "bottom", "left"] as NavEdge[]).flatMap((edge) =>
        ALIGNS.map((align, i) => {
          const on = same(ghost.slot, { edge, align });
          const along = `${(i * 100) / 3}%`;
          const style = vertical(edge)
            ? { top: `calc(${along} + 8px)`, height: "calc(33.333% - 16px)", width: 72, [edge]: 8 }
            : { left: `calc(${along} + 8px)`, width: "calc(33.333% - 16px)", height: 56, [edge]: 8 };
          return (
            <div
              key={`${edge}-${align}`}
              className={clsx("absolute rounded-2xl border-2 border-dashed transition-all duration-200", on ? "border-accent bg-accent-soft/60 opacity-100" : "border-line opacity-30")}
              style={style}
            />
          );
        }),
      )}
    </div>
  );

  return (
    <nav
      ref={nav}
      aria-label="Pages"
      className={clsx("pointer-events-none flex shrink-0", vertical(placement.edge) ? "flex-col px-2 py-4" : "flex-row px-4", placement.edge === "top" ? "pt-3 pb-1.5" : placement.edge === "bottom" && "pt-1.5 pb-3")}
    >
      {/* While dragging, the bar floats; a placeholder keeps the page where it is until the drop. */}
      {ghost && drag.current && <div style={{ width: drag.current.w, height: drag.current.h }} aria-hidden />}
      <div
        ref={bar}
        data-nav-bar
        className={clsx(
          "pointer-events-auto relative flex touch-none items-center gap-1 rounded-2xl border border-line bg-nav p-1.5 shadow-float backdrop-blur-md select-none",
          isVertical ? "flex-col" : "flex-row",
          ghost ? "cursor-grabbing ring-2 ring-accent/40" : "cursor-grab",
        )}
        style={ghost ? { position: "fixed", left: ghost.x, top: ghost.y, zIndex: 80, transform: "scale(1.04)", boxShadow: "0 18px 40px rgb(0 0 0 / 0.25)" } : undefined}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        onPointerCancel={onPointerUp}
        onClickCapture={(e) => {
          if (suppressClick.current) {
            e.preventDefault();
            e.stopPropagation();
          }
        }}
      >
        <button
          type="button"
          aria-label="Move the page bar: arrow keys dock it to that edge, Shift with an arrow moves it along the edge, Escape puts it back at the bottom"
          title="Drag to another place"
          onKeyDown={onGripKey}
          className={clsx("flex shrink-0 items-center justify-center rounded-lg text-muted/70 hover:bg-surface-2 hover:text-fg", isVertical ? "h-5 w-full" : "h-8 w-5")}
        >
          <Grip size={14} className={isVertical ? "rotate-90" : ""} />
        </button>
        {highlight && (
          <span
            aria-hidden
            className="pointer-events-none absolute top-0 left-0 rounded-xl bg-accent-soft transition-[transform,width,height] duration-300 ease-(--ease-spring) still:transition-none"
            style={{ transform: `translate(${highlight.x}px, ${highlight.y}px)`, width: highlight.w, height: highlight.h }}
          />
        )}
        {items.map(({ id, label, icon: Icon }, index) => (
          <button
            key={id}
            ref={(el) => {
              tabs.current[id] = el;
            }}
            onClick={() => navigate(id)}
            aria-current={page === id ? "page" : undefined}
            aria-keyshortcuts={String(index + 1)}
            title={`${label} (${index + 1})`}
            className={clsx(
              "group relative flex shrink-0 flex-col items-center rounded-xl text-xs transition-colors",
              isVertical ? "w-[4.5rem] gap-0.5 py-1.5" : "w-[5.25rem] gap-1 py-2",
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
      {slots}
    </nav>
  );
}
