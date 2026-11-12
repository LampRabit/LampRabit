import { create } from "zustand";
import type { InstallProgress, LogHistory, LogService, RawLogLine } from "./api";
import { parseLog, type LogEntry } from "./logparse";
import { explain } from "./explain";

// The Logs page draws only the rows in view, so it can keep this many.
const MAX_LOG_LINES = 10000;
const MAX_ACTIVITY = 100;

export type PageId = "overview" | "sites" | "services" | "logs" | "settings";
/** Where the page tab bar docks; it takes its own space there (BottomNav, App). */
export type NavEdge = "top" | "bottom" | "left" | "right";

/** The saved edge; an older saved position ({ edge, x, y }, or a free-floating one) maps to its edge or the bottom. */
function loadNavEdge(): NavEdge {
  const raw = readPref("navPosition");
  if (!raw) return "bottom";
  let edge: unknown = raw;
  try {
    const parsed = JSON.parse(raw);
    edge = typeof parsed === "string" ? parsed : parsed?.edge;
  } catch {
    // A plain edge name, saved as text.
  }
  return edge === "top" || edge === "left" || edge === "right" ? edge : "bottom";
}

export interface Notice {
  id: number;
  tone: "error" | "info";
  text: string;
  /** Output lines shown under the text (a crash's last lines). */
  detail?: string[];
  /** What to do about it, in plain words (lib/explain). */
  hint?: string | null;
}

export interface Activity {
  id: number;
  at: Date;
  text: string;
  source: string;
  tone: "ok" | "info" | "muted" | "error";
}

export const installKey = (runtime: string, version: string) => `${runtime}@${version}`;

interface AppStore {
  page: PageId;
  navigate: (page: PageId) => void;
  /** Opens a project on the Sites page (from Home); Sites picks it up and clears it. */
  siteFocus: string | null;
  openSite: (path: string | null) => void;
  /** When Logs was last on screen, for the "new errors" dot on its tab. */
  logsSeenAt: number;
  /** Service the Logs page opens filtered to, set by "View logs" on a service. */
  /** A service, or "project:<name>". */
  logFilter: string | null;
  /** Opens Logs on one source: a service, or "project:<name>". */
  showLogs: (source: LogService | `project:${string}`) => void;
  diagnosticsOpen: boolean;
  setDiagnosticsOpen: (open: boolean) => void;
  newProjectOpen: boolean;
  setNewProjectOpen: (open: boolean) => void;
  /** The setup guide (starter stacks), reopened from Settings; the first run opens it itself. */
  setupGuideOpen: boolean;
  setSetupGuideOpen: (open: boolean) => void;
  /** The template editor: open on some YAML (a project's), and whether New project opened it. */
  templateEditor: { yaml?: string; fromWizard?: boolean } | null;
  openTemplateEditor: (editor: { yaml?: string; fromWizard?: boolean }) => void;
  closeTemplateEditor: () => void;
  /** The keyboard shortcuts help ("?"). */
  shortcutsOpen: boolean;
  setShortcutsOpen: (open: boolean) => void;
  /** How connection details are shown and copied: a URL or .env lines. */
  connMode: "url" | "env";
  setConnMode: (mode: "url" | "env") => void;
  /** The stack profile Start acts on; null means every installed service. */
  activeProfile: string | null;
  setActiveProfile: (name: string | null) => void;
  navPosition: NavEdge;
  setNavPosition: (edge: NavEdge) => void;
  logs: LogEntry[];
  /** Lines that fell out of the view because it keeps only the newest ones. */
  logsDropped: number;
  notices: Notice[];
  activity: Activity[];
  /** In-flight installs keyed by installKey(). */
  installs: Record<string, InstallProgress>;
  setInstall: (p: InstallProgress) => void;
  pushLog: (line: RawLogLine) => void;
  /** Replaces the view with history read from the log files on disk. */
  loadHistory: (history: LogHistory) => void;
  clearLogs: () => void;
  record: (text: string, source: string, tone: Activity["tone"]) => void;
  notify: (tone: Notice["tone"], text: string, detail?: string[]) => void;
  dismiss: (id: number) => void;
}

// View preferences remembered between launches. Storage can be unavailable; a choice
// then lasts for this session only.
export function readPref(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

export function writePref(key: string, value: string | null) {
  try {
    if (value === null) localStorage.removeItem(key);
    else localStorage.setItem(key, value);
  } catch {
    // see readPref
  }
}

let nextId = 1;
/** Live lines at or below this id are already part of the loaded history. */
let logCursor = 0;

export const useAppStore = create<AppStore>((set) => ({
  page: "overview",
  // Leaving or entering Logs counts as having seen its errors.
  navigate: (page) => set((s) => ({ page, logsSeenAt: page === "logs" || s.page === "logs" ? Date.now() : s.logsSeenAt })),
  siteFocus: null,
  openSite: (siteFocus) => set(siteFocus ? { siteFocus, page: "sites" } : { siteFocus }),
  logsSeenAt: Date.now(),
  logFilter: null,
  showLogs: (logFilter) => set({ page: "logs", logFilter }),
  diagnosticsOpen: false,
  setDiagnosticsOpen: (diagnosticsOpen) => set({ diagnosticsOpen }),
  newProjectOpen: false,
  setNewProjectOpen: (newProjectOpen) => set({ newProjectOpen }),
  setupGuideOpen: false,
  setSetupGuideOpen: (setupGuideOpen) => set({ setupGuideOpen }),
  templateEditor: null,
  openTemplateEditor: (templateEditor) => set({ templateEditor }),
  closeTemplateEditor: () => set({ templateEditor: null }),
  shortcutsOpen: false,
  setShortcutsOpen: (shortcutsOpen) => set({ shortcutsOpen }),
  connMode: readPref("connMode") === "env" ? "env" : "url",
  setConnMode: (connMode) => {
    writePref("connMode", connMode);
    set({ connMode });
  },
  activeProfile: readPref("activeProfile"),
  setActiveProfile: (activeProfile) => {
    writePref("activeProfile", activeProfile);
    set({ activeProfile });
  },
  navPosition: loadNavEdge(),
  setNavPosition: (navPosition) => {
    writePref("navPosition", navPosition);
    set({ navPosition });
  },
  logs: [],
  logsDropped: 0,
  notices: [],
  activity: [{ id: nextId++, at: new Date(), text: "Environment ready", source: "System", tone: "info" }],
  installs: {},
  setInstall: (p) =>
    set((s) => {
      const installs = { ...s.installs };
      const key = installKey(p.runtime, p.version);
      if (p.stage === "done" || p.stage === "error") delete installs[key];
      else installs[key] = p;
      return { installs };
    }),
  pushLog: (line) =>
    set((s) => (line.id <= logCursor ? s : { logs: [...s.logs.slice(-(MAX_LOG_LINES - 1)), parseLog(line)], logsDropped: s.logsDropped + (s.logs.length >= MAX_LOG_LINES ? 1 : 0) })),
  loadHistory: ({ lines, cursor }) =>
    set((s) => {
      logCursor = cursor;
      // History ids are 0; give them unique negative ids so they never collide with live ones.
      // Lines without their own timestamp (stack traces, MariaDB's "Version:" line) take the
      // time of the line before them in the same file.
      const lastTime: Record<string, Date> = {};
      const fileOf = (l: { service: string; stream: string; project?: string }) => `${l.service}/${l.project ?? ""}/${l.stream}`;
      const history = lines.map((l, i) => {
        const entry = parseLog({ ...l, id: -(i + 1) });
        const file = fileOf(l);
        if (entry.timed) lastTime[file] = entry.time;
        else if (lastTime[file]) entry.time = lastTime[file];
        else entry.time = new Date(NaN);
        return entry;
      });
      // Lines at the top of a file with nothing timed before them take the next timed line's
      // time; a file with no timestamps at all keeps "when read", which the page marks. They
      // used to all show the current time, so days-old errors looked like they just happened.
      const nextTime: Record<string, Date> = {};
      for (let i = history.length - 1; i >= 0; i--) {
        const entry = history[i];
        const file = fileOf(entry);
        if (entry.timed) nextTime[file] = entry.time;
        else if (Number.isNaN(entry.time.getTime())) entry.time = nextTime[file] ?? new Date();
      }
      const live = s.logs.filter((l) => l.id > cursor);
      const merged = [...history, ...live].sort((a, b) => a.time.getTime() - b.time.getTime());
      return { logs: merged.slice(-MAX_LOG_LINES), logsDropped: Math.max(0, merged.length - MAX_LOG_LINES) };
    }),
  clearLogs: () => set({ logs: [], logsDropped: 0 }),
  record: (text, source, tone) =>
    set((s) => ({
      activity: [{ id: nextId++, at: new Date(), text, source, tone }, ...s.activity].slice(0, MAX_ACTIVITY),
    })),
  notify: (tone, text, detail) => {
    const id = nextId++;
    const hint = tone === "error" ? explain([text, ...(detail ?? [])].join("\n")) : null;
    set((s) => ({ notices: [...s.notices, { id, tone, text, detail, hint }] }));
    // Ones with output or advice stay longer, to be read.
    setTimeout(() => set((s) => ({ notices: s.notices.filter((n) => n.id !== id) })), detail?.length || hint ? 15000 : 6000);
  },
  dismiss: (id) => set((s) => ({ notices: s.notices.filter((n) => n.id !== id) })),
}));
