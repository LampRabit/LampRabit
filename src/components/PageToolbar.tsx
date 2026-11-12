import { createContext, useCallback, useContext, useEffect, useMemo, useReducer, useRef, type ReactNode } from "react";
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
