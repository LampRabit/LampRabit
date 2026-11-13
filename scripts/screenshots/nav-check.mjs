// The page bar in WebKit (the macOS app's engine): docked at each edge it must stay inside the
// room the page pads for it, a real drag to the left edge must dock it there, and a click on a tab must still
// navigate. Needs the harness on port 1430 (npx vite --port 1430). Exits 1 on a problem.
import { mkdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { webkit } from "playwright";
const out = fileURLToPath(new URL("./out", import.meta.url));
mkdirSync(out, { recursive: true });
const browser = await webkit.launch();
const errors = [];
for (const edge of ["bottom-center", "bottom-start", "left-start", "left-end", "right-center", "top-end"]) {
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  page.on("pageerror", (e) => errors.push(`${edge}: ${e.message}`));
  await page.goto(`http://localhost:1430/scripts/screenshots/index.html?page=sites&nav=${edge}`);
  await page.waitForSelector("body[data-ready='1']");
  await page.waitForTimeout(600);
  // The bar floats over the page, which pads that edge for it: does the bar overlap the
  // page's content box (its rect less that padding)?
  const r = await page.evaluate(() => {
    const b = document.querySelector("[data-nav-bar]").getBoundingClientRect();
    const el = document.querySelector("[data-page-scroller]");
    const s = getComputedStyle(el);
    const o = el.getBoundingClientRect();
    const p = { left: o.left + parseFloat(s.paddingLeft), right: o.right - parseFloat(s.paddingRight), top: o.top + parseFloat(s.paddingTop), bottom: o.bottom - parseFloat(s.paddingBottom) };
    p.width = p.right - p.left;
    p.height = p.bottom - p.top;
    const overlap = !(b.right <= p.left || b.left >= p.right || b.bottom <= p.top || b.top >= p.bottom);
    return { bar: [b.left, b.top, b.width, b.height].map(Math.round), page: [p.left, p.top, p.width, p.height].map(Math.round), overlap };
  });
  console.log(edge, JSON.stringify(r));
  if (r.overlap) errors.push(`${edge}: the bar overlaps the page`);
  await page.screenshot({ path: `${out}/nav-${edge}.png` });
  await page.close();
}
// A real drag: from the bottom bar's grip to the left edge, screenshot mid-drag and after.
const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
page.on("pageerror", (e) => errors.push(`drag: ${e.message}`));
await page.goto(`http://localhost:1430/scripts/screenshots/index.html?page=overview&nav=bottom`);
await page.waitForSelector("body[data-ready='1']");
const grip = await page.locator("[data-nav-bar] button").first().boundingBox();
await page.mouse.move(grip.x + grip.width / 2, grip.y + grip.height / 2);
await page.mouse.down();
for (let i = 1; i <= 20; i++) await page.mouse.move(grip.x - (grip.x - 30) * (i / 20), grip.y - (grip.y - 400) * (i / 20));
await page.screenshot({ path: `${out}/nav-dragging.png` });
await page.mouse.up();
await page.waitForTimeout(100);
await page.screenshot({ path: `${out}/nav-dropping.png` });
await page.waitForTimeout(600);
const saved = await page.evaluate(() => localStorage.getItem("navPosition"));
console.log("after drop, saved edge:", saved);
if (saved !== "left-center") errors.push(`drag: docked at ${saved}, not left-center`);
// A click on a tab still navigates (no drag).
await page.locator("[data-nav-bar] button", { hasText: "Logs" }).click();
await page.waitForTimeout(300);
const navigated = await page.evaluate(() => !!document.querySelector('[aria-current="page"]')?.textContent?.includes("Logs"));
console.log("click navigates:", navigated);
if (!navigated) errors.push("click: a tab click didn't navigate");
await page.screenshot({ path: `${out}/nav-after.png` });
// The floating notices keep clear of the bar wherever it is.
const clear = await page.evaluate(() => getComputedStyle(document.documentElement).getPropertyValue("--nav-left"));
console.log("room published for a left bar:", clear);
if (!clear || clear.trim() === "0px") errors.push("the bar didn't publish its size for floating bars");
await browser.close();
// The very first module load can race Vite's dependency optimizer; that isn't the bar's fault.
const real = errors.filter((e) => !e.includes("Importing a module script failed"));
console.log(real.length ? real.join("\n") : "page bar OK");
process.exit(real.length ? 1 : 0);
