import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { useQuery } from "@tanstack/react-query";
import { create } from "zustand";
import { api } from "./api";

interface UpdateStore {
  /** A newer LampRabit release, once a check found one. */
  available: Update | null;
  checking: boolean;
  error: string | null;
  checked: boolean;
  checkNow: () => Promise<void>;
}

export const useAppUpdate = create<UpdateStore>((set) => ({
  available: null,
  checking: false,
  error: null,
  checked: false,
  checkNow: async () => {
    set({ checking: true, error: null });
    try {
      set({ available: await check(), checked: true });
    } catch (e) {
      // Before the first release there's no latest.json to read; that isn't worth alarming anyone.
      const message = String(e);
      set({ error: /404|Not Found|Could not fetch/i.test(message) ? "No published releases yet." : message, checked: true });
    } finally {
      set({ checking: false });
    }
  },
}));

export async function installAppUpdate(update: Update, onProgress: (pct: number | null) => void) {
  let total = 0;
  let done = 0;
  await update.download((event) => {
    if (event.event === "Started") total = event.data.contentLength ?? 0;
    if (event.event === "Progress") {
      done += event.data.chunkLength;
      onProgress(total ? Math.round((done / total) * 100) : null);
    }
  });
  // Only once the download is complete: services stop cleanly (databases flush) before the
  // installer takes over, which on Windows ends the app without its usual shutdown.
  await api.prepareForUpdate().catch(() => {});
  await update.install();
  await relaunch();
}

/** Newer patch releases for installed runtimes; refreshed every six hours. */
export function useRuntimeUpdates() {
  return useQuery({ queryKey: ["runtime-updates"], queryFn: api.runtimeUpdates, staleTime: 6 * 60 * 60 * 1000 });
}
