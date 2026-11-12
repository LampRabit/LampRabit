import { useEffect, useLayoutEffect, useRef, useState, type KeyboardEvent, type PointerEvent } from "react";
import { FileText, Globe, Grip, House, Layers, Settings } from "lucide-react";
import clsx from "clsx";
import { useQuery } from "@tanstack/react-query";
import { api } from "../lib/api";
import { motionAllowed } from "../lib/motion";
import { useAppStore, type NavEdge, type PageId } from "../lib/store";
import { useRuntimeUpdates } from "../lib/updates";

const items: { id: PageId; label: string; icon: typeof House }[] = [
  { id: "overview", label: "Home", icon: House },
  { id: "sites", label: "Sites", icon: Globe },
  { id: "services", label: "Services", icon: Layers },
  { id: "logs", label: "Logs", icon: FileText },
  { id: "settings", label: "Settings", icon: Settings },
];

/**
 * The page tab bar. It docks to one edge of the window's content (bottom by default, or top,
 * left, right) and takes its own space there, so it never covers the page. Drag it (from
 * the grip, or anywhere on it) to another edge: a preview follows the pointer, the target
 * edge lights up, and on release the bar glides into place while the page slides over to
 * make room. The grip also takes the arrow keys (dock to that edge) and Escape (back to
 * the bottom). App.tsx places it with `navLayout`; Settings → Appearance sets it too.
 */

/** How App lays the body out for an edge: the flex direction, and whether the bar comes first. */
export function navLayout(edge: NavEdge) {
  return { direction: edge === "left" || edge === "right" ? "row" : "column", navFirst: edge === "top" || edge === "left" } as const;
}

type Drag = { pointerId: number; startX: number; startY: number; grabX: number; grabY: number; moved: boolean; w: number; h: number };

/** The edge of the body nearest a point, with how far it is (for the drop preview). */
function nearestEdge(x: number, y: number, body: DOMRect): NavEdge {
  const distances: [NavEdge, number][] = [
    ["left", (x - body.left) / body.width],
    ["right", (body.right - x) / body.width],
    ["top", (y - body.top) / body.height],
    ["bottom", (body.bottom - y) / body.height],
  ];
  return distances.sort((a, b) => a[1] - b[1])[0][0];
}

/** Animates an element from where it was (`from`) to where layout put it, by transform only. */
function glide(el: Element | null, from: DOMRect | null, options: { scale?: boolean } = {}) {
  if (!el || !from || !motionAllowed()) return;
  const to = el.getBoundingClientRect();
  const dx = from.left - to.left;
  const dy = from.top - to.top;
  if (Math.abs(dx) < 1 && Math.abs(dy) < 1) return;
  const scale = options.scale && to.width && to.height ? ` scale(${from.width / to.width}, ${from.height / to.height})` : "";
  el.animate([{ transform: `translate(${dx}px, ${dy}px)${scale}`, transformOrigin: "top left" }, { transform: "none", transformOrigin: "top left" }], {
    duration: 420,
    easing: "cubic-bezier(0.22, 1.2, 0.36, 1)",
  });
}

export default function BottomNav() {
  const page = useAppStore((s) => s.page);
  const navigate = useAppStore((s) => s.navigate);
  const edge = useAppStore((s) => s.navPosition);
  const setEdge = useAppStore((s) => s.setNavPosition);
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

  const bar = useRef<HTMLDivElement>(null);
  const drag = useRef<Drag | null>(null);
  const suppressClick = useRef(false);
  // While dragging: where the bar floats (its top-left corner) and the edge it would dock to.
  const [ghost, setGhost] = useState<{ x: number; y: number; edge: NavEdge } | null>(null);
  const vertical = (ghost?.edge ?? edge) === "left" || (ghost?.edge ?? edge) === "right";

  // Where the bar and the page were after the last render: when the edge changes (a drop,
  // the arrow keys, Settings), both glide from there to their new places. A drop starts the
  // bar's glide from where it was let go instead.
  const last = useRef<{ edge: NavEdge; bar: DOMRect | null; page: DOMRect | null } | null>(null);
  const dropFrom = useRef<DOMRect | null>(null);
  const pageEl = () => document.querySelector("[data-page-scroller]");
  const dockTo = (next: NavEdge, from?: DOMRect) => {
    dropFrom.current = from ?? null;
    if (next === edge && from) {
      // Dropped on its own edge: glide back from the drop point.
      requestAnimationFrame(() => {
        glide(bar.current, dropFrom.current);
        dropFrom.current = null;
      });
      return;
    }
    setEdge(next);
  };
  useLayoutEffect(() => {
    const was = last.current;
    if (was && was.edge !== edge) {
      glide(bar.current, dropFrom.current ?? was.bar);
      glide(pageEl(), was.page);
      dropFrom.current = null;
    }
    last.current = { edge, bar: bar.current?.getBoundingClientRect() ?? null, page: pageEl()?.getBoundingClientRect() ?? null };
  });

  // The highlight measures the active tab, so it lines up in either orientation.
  const tabs = useRef<Partial<Record<PageId, HTMLButtonElement | null>>>({});
  const [highlight, setHighlight] = useState<{ x: number; y: number; w: number; h: number } | null>(null);
  useLayoutEffect(() => {
    const el = tabs.current[page];
    setHighlight(el ? { x: el.offsetLeft, y: el.offsetTop, w: el.offsetWidth, h: el.offsetHeight } : null);
  }, [page, vertical]);

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
    setGhost({ x, y, edge: nearestEdge(e.clientX, e.clientY, body) });
  }

  function onPointerUp(e: PointerEvent<HTMLDivElement>) {
    const d = drag.current;
    if (!d || d.pointerId !== e.pointerId) return;
    drag.current = null;
    if (d.moved && ghost) {
      // The drop glides from where the ghost was let go.
      const from = e.currentTarget.getBoundingClientRect();
      suppressClick.current = true;
      window.setTimeout(() => (suppressClick.current = false), 0);
      setGhost(null);
      dockTo(ghost.edge, from);
    } else setGhost(null);
  }

  function onGripKey(e: KeyboardEvent<HTMLButtonElement>) {
    const keys: Record<string, NavEdge> = { ArrowUp: "top", ArrowDown: "bottom", ArrowLeft: "left", ArrowRight: "right", Escape: "bottom", Home: "bottom" };
    const next = keys[e.key];
    if (!next) return;
    e.preventDefault();
    e.stopPropagation();
    dockTo(next);
  }

  const dropZone = (zone: NavEdge) => {
    const on = ghost?.edge === zone;
    const pos = { top: "inset-x-0 top-0 h-16", bottom: "inset-x-0 bottom-0 h-16", left: "inset-y-0 left-0 w-24", right: "inset-y-0 right-0 w-24" }[zone];
    return <div key={zone} className={clsx("absolute rounded-2xl border-2 border-dashed transition-all duration-200", pos, on ? "border-accent bg-accent-soft/50 opacity-100" : "border-line opacity-40")} aria-hidden />;
  };

  const barEl = (
    <div
      ref={bar}
      data-nav-bar
      className={clsx(
        "relative flex touch-none items-center gap-1 rounded-2xl border border-line bg-nav p-1.5 shadow-float backdrop-blur-md select-none",
        vertical ? "flex-col" : "flex-row",
        ghost ? "cursor-grabbing ring-2 ring-accent/40 transition-none" : "cursor-grab",
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
        aria-label="Move the page bar: arrow keys dock it to that edge, Escape puts it back at the bottom"
        title="Drag to another edge"
        onKeyDown={onGripKey}
        className={clsx("flex shrink-0 items-center justify-center rounded-lg text-muted/70 hover:bg-surface-2 hover:text-fg", vertical ? "h-5 w-full" : "h-8 w-5")}
      >
        <Grip size={14} className={vertical ? "rotate-90" : ""} />
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
            vertical ? "w-[4.5rem] gap-0.5 py-1.5" : "w-[5.25rem] gap-1 py-2",
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
  );

  return (
    <nav aria-label="Pages" className={clsx("flex shrink-0 justify-center", vertical && !ghost ? "items-center px-2 py-4" : "px-4 pt-1.5 pb-3", edge === "top" && !ghost && "pt-3 pb-1.5")}>
      {/* While dragging, the bar floats; a placeholder keeps the page where it is until the drop. */}
      {ghost && drag.current && <div style={{ width: drag.current.w, height: drag.current.h }} aria-hidden />}
      {barEl}
      {ghost && <div className="pointer-events-none fixed inset-x-3 top-14 bottom-3 z-[70]">{(["top", "bottom", "left", "right"] as NavEdge[]).map(dropZone)}</div>}
    </nav>
  );
}
