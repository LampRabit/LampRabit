// Captures every main view of the app with a fake backend (harness.tsx) in headless Edge
// or Chrome, into scripts/screenshots/out/. Start `npx vite --port 1430` first, or pass
// --serve to have this script start it. Usage: node scripts/screenshots/shoot.mjs [name…]
import { spawn, execFileSync } from "node:child_process";
import { existsSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const out = join(here, "out");
mkdirSync(out, { recursive: true });

const browsers = [
  "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe",
  "C:/Program Files/Google/Chrome/Application/chrome.exe",
  "/usr/bin/google-chrome",
  "/usr/bin/chromium",
  "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
];
const browser = browsers.find(existsSync);
if (!browser) throw new Error("No Edge or Chrome found for screenshots.");

const views = {
  home: "page=overview",
  "home-dark": "page=overview&theme=dark",
  sites: "page=sites",
  "git-changes": "page=sites&tab=git&file=README.md",
  "git-changes-dark": "page=sites&tab=git&file=README.md&theme=dark",
  "git-clean-ahead": "page=sites&tab=git&clean=1&ahead=2",
  "git-no-remote": "page=sites&tab=git&clean=1&noRemote=1",
  "git-history": "page=sites&tab=git&gitView=history",
  "git-history-file": "page=sites&tab=git&gitView=history&file=.editorconfig",
  "git-branches": "page=sites&tab=git&gitView=branches",
  "git-remotes": "page=sites&tab=git&gitView=remotes",
  services: "page=services",
  logs: "page=logs",
  "logs-filters": "page=logs&filters=1",
  "logs-dark": "page=logs&theme=dark",
  settings: "page=settings",
  assistants: "page=settings&mcpOn=1",
  "doctor-issues": "page=sites",
  "doctor-healthy": "page=sites&healthy=1",
  "first-run": "page=overview&firstRun=1",
  "narrow-sites": "page=sites&tab=git&width=900",
};

let server;
if (process.argv.includes("--serve")) {
  server = spawn(process.platform === "win32" ? "npx.cmd" : "npx", ["vite", "--port", "1430", "--strictPort"], { stdio: "ignore", shell: process.platform === "win32" });
  await new Promise((r) => setTimeout(r, 4000));
}

const only = process.argv.slice(2).filter((a) => !a.startsWith("--"));
for (const [name, query] of Object.entries(views)) {
  if (only.length && !only.includes(name)) continue;
  const width = /width=(\d+)/.exec(query)?.[1] ?? "1440";
  const file = join(out, `${name}.png`);
  execFileSync(browser, ["--headless=new", "--disable-gpu", "--hide-scrollbars", `--window-size=${width},900`, "--virtual-time-budget=6000", `--screenshot=${file}`, `http://localhost:1430/scripts/screenshots/index.html?${query}`], { stdio: "ignore" });
  console.log(file);
}
server?.kill();
