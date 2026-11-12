import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { openPath } from "@tauri-apps/plugin-opener";
import { useQuery } from "@tanstack/react-query";
import { Copy, Download, FolderOpen, Keyboard, Minus, Moon, MoreHorizontal, Play, Square, SquareTerminal, Sun, X } from "lucide-react";
import clsx from "clsx";
import { api } from "../lib/api";
import { useAppStore } from "../lib/store";
import { useAppUpdate } from "../lib/updates";
import { useStack } from "../lib/stack";
import { useEffectiveTheme, useTheme } from "../lib/theme";
import { useCurrentPageToolbar } from "./PageToolbar";
import Logo from "./Logo";
import { Button } from "./ui";

/**
 * The window's title bar. On Windows and Linux the native frame is off, so this draws
 * the minimize / maximize / close buttons; on macOS the native traffic lights sit on
 * top of it (overlay title bar), so it only leaves room for them.
 */
export default function Titlebar() {
  const notify = useAppStore((s) => s.notify);
  const navigate = useAppStore((s) => s.navigate);
  const page = useAppStore((s) => s.page);
  const pageToolbar = useCurrentPageToolbar(page);
  const update = useAppUpdate((s) => s.available);
  const os = useQuery({ queryKey: ["paths"], queryFn: api.envPaths }).data?.os;
  const services = useQuery({ queryKey: ["services"], queryFn: api.listServices });
  const paths = useQuery({ queryKey: ["paths"], queryFn: api.envPaths });
  const onError = (e: unknown) => notify("error", String(e));
  const stack = useStack();
  const anyRunning = stack.running.length > 0;
  // On a fresh install there is nothing to start yet; lead to Services instead.
  const nothingInstalled = services.data !== undefined && !services.data.some((s) => s.installed);

  return (
    <header
      data-tauri-drag-region
      className={clsx(
        "relative z-50 flex h-12 shrink-0 items-center gap-2 border-b border-line bg-surface/90 px-2 backdrop-blur-md select-none sm:gap-2.5 sm:px-3",
        os === "macos" ? "pl-20" : "",
      )}
    >
      <Logo animated className="pointer-events-none size-8 shrink-0" />
      {page === "overview" ? (
        <div className="pointer-events-none hidden min-w-0 leading-tight lg:block">
          <div className="truncate font-bold tracking-tight">Lamp<span className="text-accent-text">Rabit</span></div>
          <div className="hidden text-xs text-muted xl:block">Local dev environment</div>
        </div>
      ) : (
        <div className="pointer-events-none min-w-0 leading-tight">
          <h1 className="truncate text-sm font-bold tracking-tight">{pageToolbar?.title ?? pageLabel(page)}</h1>
          {pageToolbar?.subtitle && <div className="hidden max-w-72 truncate text-[11px] text-muted 2xl:block">{pageToolbar.subtitle}</div>}
        </div>
      )}
      <ThemeToggle />

      <div className="ml-auto flex min-w-0 items-center gap-1 sm:gap-1.5">
        {pageToolbar?.actions && pageToolbar.menuActions && <div className="hidden items-center gap-1 xl:flex">{pageToolbar.actions}</div>}
        {pageToolbar?.actions && !pageToolbar.menuActions && <div className="flex min-w-0 items-center gap-1">{pageToolbar.actions}</div>}
        {pageToolbar?.menuActions && (
          <details key={page} className="group relative xl:hidden">
            <summary className="flex size-9 cursor-pointer list-none items-center justify-center rounded-lg text-muted hover:bg-surface-2 hover:text-fg [&::-webkit-details-marker]:hidden" aria-label={`${pageToolbar.title} actions`} title={`${pageToolbar.title} actions`}>
              <MoreHorizontal size={18} />
            </summary>
            <div className="absolute top-full right-0 z-50 mt-1.5 flex min-w-48 flex-col gap-1 rounded-xl border border-line bg-surface p-2 shadow-float">
              {pageToolbar.menuActions}
            </div>
          </details>
        )}
        <div className="flex shrink-0 items-center gap-1 sm:gap-1.5">
        <Button size="sm" variant="ghost" className="hidden sm:inline-flex" onClick={() => useAppStore.getState().setShortcutsOpen(true)} aria-label="Keyboard shortcuts" title="Keyboard shortcuts (?)">
          <Keyboard size={15} />
        </Button>
        {update && (
          <button
            onClick={() => navigate("settings")}
            className="rounded-full bg-accent-soft px-2 py-1 text-xs font-semibold text-accent-text hover:brightness-95 sm:px-3"
            aria-label={`Update ${update.version} available`}
          >
            <span className="hidden 2xl:inline">Update {update.version} available</span><Download size={14} className="2xl:hidden" />
          </button>
        )}
        <Button size="sm" className="px-2 sm:px-3" onClick={() => paths.data && openPath(paths.data.www).catch(onError)} disabled={!paths.data} title="Open www (W)" aria-label="Open www">
          <FolderOpen size={15} /> <span className="hidden xl:inline">Open www</span>
        </Button>
        <Button size="sm" onClick={() => api.openTerminal(null).catch(onError)} aria-label="Open terminal" title="Terminal with php, composer and mariadb (T)">
          <SquareTerminal size={15} />
        </Button>
        {/* Home has its own (profile-aware) Start, Restart and Stop in its header. */}
        {page === "overview" ? null : anyRunning ? (
          <Button size="sm" onClick={stack.stopAll} disabled={stack.busy || stack.stoppable.length === 0} title="Stop all (Shift+X)" aria-label="Stop all">
            <Square size={12} className="fill-current" /> <span className="hidden 2xl:inline">Stop all</span>
          </Button>
        ) : nothingInstalled ? (
          <Button size="sm" variant="primary" onClick={() => navigate("services")} aria-label="Set up your stack" title="Set up your stack">
            <Download size={14} /> <span className="hidden 2xl:inline">Set up</span>
          </Button>
        ) : (
          <Button size="sm" variant="primary" onClick={stack.startStack} disabled={stack.busy} title="Shift+S" aria-label={stack.profile ? `Start ${stack.profile.name}` : "Start all"}>
            <Play size={14} /> <span className="hidden 2xl:inline">{stack.profile ? `Start ${stack.profile.name}` : "Start all"}</span>
          </Button>
        )}
        </div>
      </div>

      {os && os !== "macos" && <WindowControls />}
    </header>
  );
}

/** Flips between light and dark from what's on screen; Settings → Appearance still offers "System". */
function ThemeToggle() {
  const effective = useEffectiveTheme();
  const setTheme = useTheme((s) => s.setTheme);
  const next = effective === "dark" ? "light" : "dark";
  return (
    <button
      type="button"
      role="switch"
      aria-checked={effective === "dark"}
      aria-label="Dark theme"
      title={`Dark theme ${effective === "dark" ? "on" : "off"}`}
      onClick={() => setTheme(next)}
      className={clsx(
        "relative flex h-6 w-11 shrink-0 items-center rounded-full border p-0.5 transition-colors focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent",
        effective === "dark" ? "border-accent bg-accent" : "border-line bg-surface-2",
      )}
    >
      <span className={clsx("flex size-5 items-center justify-center rounded-full bg-surface text-muted shadow-sm transition-transform", effective === "dark" && "translate-x-5 text-accent-text")}>
        {effective === "dark" ? <Moon size={12} /> : <Sun size={12} />}
      </span>
    </button>
  );
}

function WindowControls() {
  const win = getCurrentWindow();
  const [maximized, setMaximized] = useState(false);

  useEffect(() => {
    win.isMaximized().then(setMaximized);
    const off = win.onResized(() => win.isMaximized().then(setMaximized));
    return () => {
      off.then((f) => f());
    };
  }, [win]);

  const base = "flex h-12 w-10 items-center justify-center text-muted transition-colors";
  return (
    <div className="ml-2 flex self-stretch">
      <button aria-label="Minimize" className={clsx(base, "hover:bg-surface-2 hover:text-fg")} onClick={() => win.minimize()}>
        <Minus size={16} />
      </button>
      <button
        aria-label={maximized ? "Restore" : "Maximize"}
        className={clsx(base, "hover:bg-surface-2 hover:text-fg")}
        onClick={() => win.toggleMaximize()}
      >
        {maximized ? <Copy size={13} className="-scale-x-100" /> : <Square size={13} />}
      </button>
      <button aria-label="Close" className={clsx(base, "hover:bg-[#e81123] hover:text-white")} onClick={() => win.close()}>
        <X size={17} />
      </button>
    </div>
  );
}

function pageLabel(page: string) {
  return ({ sites: "Sites", services: "Services", logs: "Logs", settings: "Settings" } as Record<string, string>)[page] ?? "LampRabit";
}
