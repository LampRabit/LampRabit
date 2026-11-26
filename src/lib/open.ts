import { openUrl as openInBrowser } from "@tauri-apps/plugin-opener";
import { api } from "./api";
import { useAppStore } from "./store";

/**
 * Opens a page in the browser. A .test address gets what answers it ready first: the web
 * server (started, or told about the domain) and, for a tool's domain, the service behind it
 * (KarotX, pgAdmin and its PostgreSQL, a database for phpMyAdmin). Without that, a link
 * that looks fine lands on "service unavailable".
 */
export const openUrl = (url: string) => {
  // A service that has to start first can take a while (pgAdmin, Neo4j); say so instead of nothing happening.
  const slow = setTimeout(() => useAppStore.getState().notify("info", `Getting ${hostOf(url)} ready…`), 700);
  return Promise.resolve()
    .then(() => api.prepareUrl(url))
    .then(
      (started) => {
        if (started?.length) useAppStore.getState().notify("info", `Started ${started.join(" and ")} to open ${hostOf(url)}`);
      },
      () => {},
    )
    .finally(() => clearTimeout(slow))
    .then(() => openInBrowser(url));
};

const hostOf = (url: string) => url.replace(/^[a-z]+:\/\//, "").split(/[/?#]/)[0];
