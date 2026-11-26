import { createContext, useCallback, useContext, useEffect, useMemo, useReducer, useRef, type ReactNode } from "react";
import { MoreHorizontal } from "lucide-react";
import type { PageId } from "../lib/store";

export type PageToolbarData = {
  title: string;
  subtitle?: ReactNode;
  actions?: ReactNode;
  menuActions?: ReactNode;
};

type ToolbarApi = {
  register: (page: PageId, data: PageToolbarData) => void;
  get: (page: PageId) => PageToolbarData | undefined;
  subscribe: (listener: () => void) => () => void;
};

const PageToolbarContext = createContext<ToolbarApi | null>(null);

export function PageToolbarProvider({ children }: { children: ReactNode }) {
  const entries = useRef<Partial<Record<PageId, PageToolbarData>>>({});
  const listeners = useRef(new Set<() => void>());
  const register = useCallback((page: PageId, data: PageToolbarData) => {
    entries.current[page] = data;
    listeners.current.forEach((listener) => listener());
  }, []);
  const get = useCallback((page: PageId) => entries.current[page], []);
  const subscribe = useCallback((listener: () => void) => {
    listeners.current.add(listener);
    return () => listeners.current.delete(listener);
  }, []);
  const value = useMemo(() => ({ register, get, subscribe }), [register, get, subscribe]);

  return <PageToolbarContext.Provider value={value}>{children}</PageToolbarContext.Provider>;
}

/** Pages register their own dynamic heading and contextual controls in the shared title bar. */
export function usePageToolbar(page: PageId, data: PageToolbarData) {
  const toolbar = useContext(PageToolbarContext);
  useEffect(() => toolbar?.register(page, data), [toolbar, page, data]);
}

export function useCurrentPageToolbar(page: PageId) {
  const toolbar = useContext(PageToolbarContext);
  const [, refresh] = useReducer((n: number) => n + 1, 0);
  useEffect(() => toolbar?.subscribe(refresh), [toolbar]);
  return toolbar?.get(page);
}

/**
 * The page's own header, at the top of its content: title, one line on what it's for, and
 * its actions. These used to sit in the title bar next to LampRabit's own buttons, which
 * crowded it on every page; the title bar now only carries what's the same everywhere.
 */
export function PageHeader({ page }: { page: PageId }) {
  const data = useCurrentPageToolbar(page);
  const menu = useRef<HTMLDetailsElement>(null);
  useEffect(() => {
    const close = (e: MouseEvent) => {
      if (menu.current?.open && !menu.current.contains(e.target as Node)) menu.current.removeAttribute("open");
    };
    document.addEventListener("mousedown", close);
    return () => document.removeEventListener("mousedown", close);
  }, []);
  if (!data) return null;
  return (
    <div className="mb-5 flex min-h-9 flex-wrap items-center gap-x-4 gap-y-2">
      <div className="min-w-0 flex-1">
        <h1 className="truncate text-xl font-medium tracking-tight">{data.title}</h1>
        {data.subtitle && <p className="mt-0.5 truncate text-sm text-muted">{data.subtitle}</p>}
      </div>
      {data.actions && <div className={data.menuActions ? "hidden shrink-0 items-center gap-1.5 md:flex" : "flex shrink-0 items-center gap-1.5"}>{data.actions}</div>}
      {data.menuActions && (
        <details ref={menu} className="relative md:hidden" onClick={(e) => (e.target as HTMLElement).closest("button") && menu.current?.removeAttribute("open")}>
          <summary className="flex size-9 cursor-pointer list-none items-center justify-center rounded-lg text-muted hover:bg-surface-2 hover:text-fg [&::-webkit-details-marker]:hidden" aria-label={`${data.title} actions`} title={`${data.title} actions`}>
            <MoreHorizontal size={18} />
          </summary>
          <div className="absolute top-full right-0 z-40 mt-1.5 flex min-w-48 flex-col gap-1 rounded-xl border border-line bg-surface p-2 shadow-float">{data.menuActions}</div>
        </details>
      )}
    </div>
  );
}
