import React from "react";
import ReactDOM from "react-dom/client";
import { MutationCache, QueryClient, QueryClientProvider } from "@tanstack/react-query";
import App from "./App";
import { loadLabels } from "./lib/api";
import "./lib/motion";
import "./index.css";

// The webview's own right-click menu (Back, Reload, Inspect…) is a browser's, not the
// app's; it stays in development for inspecting the UI. Text fields keep it, for
// cut, copy and paste.
if (!import.meta.env.DEV) {
  document.addEventListener("contextmenu", (e) => {
    const target = e.target as HTMLElement | null;
    if (!target?.closest("input, textarea, [contenteditable='true']")) e.preventDefault();
  });
}

// Data that is slow to fetch (downloads catalogs from the internet) and changes on its own clock.
const SLOW = new Set(["runtime-info", "runtime-updates", "runtime-catalog", "settings-index", "first-run", "starter-plan", "starter-item", "git-version"]);

// Real time: any change made in the app refreshes every query on screen, so nothing waits for
// a restart; coming back to the window picks up changes made outside it.
const queryClient: QueryClient = new QueryClient({
  defaultOptions: { queries: { refetchOnWindowFocus: true, retry: false } },
  mutationCache: new MutationCache({
    onSettled: () => queryClient.invalidateQueries({ predicate: (q) => !SLOW.has(String(q.queryKey[0])) }),
  }),
});

// Runtime and service names come from the backend; without them ids show until a reload.
loadLabels()
  .catch(() => {})
  .finally(() =>
    ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
      <React.StrictMode>
        <QueryClientProvider client={queryClient}>
          <App />
        </QueryClientProvider>
      </React.StrictMode>,
    ),
  );
