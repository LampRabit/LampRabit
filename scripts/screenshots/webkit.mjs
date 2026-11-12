// The same views as shoot.mjs, rendered by WebKit (Playwright's build), the engine of the
// macOS app's WKWebView, so Safari-only layout problems show up without a Mac. Needs the
// harness served on port 1430 (`npx vite --port 1430`) and `npx playwright install webkit`.
// Usage: node scripts/screenshots/webkit.mjs [name…]  ->  scripts/screenshots/out/webkit-<name>.png
import { mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { webkit } from "playwright";

const here = dirname(fileURLToPath(import.meta.url));
const out = join(here, "out");
mkdirSync(out, { recursive: true });

const views = {
  home: "page=overview",
  sites: "page=sites",
  "git-changes": "page=sites&tab=git&file=README.md",
  "git-history": "page=sites&tab=git&gitView=history",
  "git-remotes": "page=sites&tab=git&gitView=remotes",
  services: "page=services",
  logs: "page=logs",
  settings: "page=settings",
  "first-run": "page=overview&firstRun=1",
  "home-dark": "page=overview&theme=dark",
};

const only = process.argv.slice(2);
const browser = await webkit.launch();
const errors = [];
for (const [name, query] of Object.entries(views)) {
  if (only.length && !only.includes(name)) continue;
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  // Script errors are what breaks in one engine and not another; collect them.
  page.on("pageerror", (e) => errors.push(`${name}: ${e.message}`));
  await page.goto(`http://localhost:1430/scripts/screenshots/index.html?${query}`);
  await page.waitForSelector("body[data-ready='1']", { timeout: 15000 }).catch(() => errors.push(`${name}: never got ready`));
  await page.waitForTimeout(800);
  const file = join(out, `webkit-${name}.png`);
  await page.screenshot({ path: file });
  console.log(file);
  await page.close();
}
await browser.close();
if (errors.length) {
  console.error(`\n${errors.length} problem(s) in WebKit:\n${errors.join("\n")}`);
  process.exit(1);
}
