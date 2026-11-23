import { readPref, writePref } from "./store";

/** One thing the command palette can do. Every feature adds its actions here, by building items. */
export interface PaletteItem {
  id: string;
  title: string;
  /** Where it belongs: "Actions", "Projects", "Services", "Settings", "Config files", "Docs". */
  group: string;
  /** Other words it's found by (a service's manifest keywords, a section's settings). */
  keywords?: readonly string[];
  /** A path, a URL or a status, shown quietly after the title. */
  hint?: string;
  /** Its keyboard shortcut, from SHORTCUTS. */
  keys?: string[];
  run: () => void;
}

/** Words that mean the same thing here: a search for one finds the others too. */
const SYNONYMS: string[][] = [
  ["ssl", "https", "certificate", "cert", "tls"],
  ["db", "database", "mysql", "mariadb", "postgres", "sql"],
  ["mail", "email", "smtp", "mailpit", "inbox"],
  ["web server", "apache", "nginx", "httpd", "vhost"],
  ["port", "ports", "conflict"],
  ["start", "run", "launch", "up"],
  ["stop", "kill", "shut down", "halt"],
  ["project", "site", "app", "website"],
  ["terminal", "shell", "console", "cmd", "command line"],
  ["config", "configuration", "ini", "conf"],
  ["share", "tunnel", "ngrok", "public url"],
  ["log", "logs", "errors", "output"],
  ["cache", "redis", "memcached"],
  ["node", "npm", "javascript", "js"],
  ["theme", "dark mode", "light mode", "appearance"],
  ["phone", "mobile", "device", "adb"],
  ["java", "jdk", "java_home", "gradle", "maven"],
  ["git", "branch", "commit", "repository"],
  ["antivirus", "defender", "windows security", "slow"],
];

/** A word and what it also means: whole synonyms, or ones it starts (3 letters or more). */
export function expand(word: string): string[] {
  const out = new Set([word]);
  for (const group of SYNONYMS) {
    if (group.some((w) => w === word || (word.length >= 3 && w.startsWith(word)))) group.forEach((w) => out.add(w));
  }
  return [...out];
}

/** How well an item matches every word of the query; null when one word doesn't match at all. */
export function score(item: PaletteItem, query: string): number | null {
  const words = query.trim().toLowerCase().split(/\s+/).filter(Boolean);
  if (words.length === 0) return 0;
  const title = item.title.toLowerCase();
  const titleWords = title.split(/[\s:·/._-]+/);
  const rest = [item.group, item.hint ?? "", ...(item.keywords ?? [])].join(" ").toLowerCase();
  let total = 0;
  for (const word of words) {
    let best = -1;
    for (const alt of expand(word)) {
      const s = title.startsWith(alt) ? 100 : titleWords.some((w) => w.startsWith(alt)) ? 80 : title.includes(alt) ? 50 : rest.includes(alt) ? 30 : -1;
      // The word itself beats a synonym of it.
      best = Math.max(best, s < 0 ? s : alt === word ? s : s * 0.6);
    }
    if (best < 0) return null;
    total += best;
  }
  return total;
}

const RECENT = "palette-recent";

export function readRecent(): string[] {
  try {
    const list = JSON.parse(readPref(RECENT) ?? "[]");
    return Array.isArray(list) ? list.filter((x) => typeof x === "string") : [];
  } catch {
    return [];
  }
}

export function pushRecent(id: string) {
  writePref(RECENT, JSON.stringify([id, ...readRecent().filter((x) => x !== id)].slice(0, 20)));
}

/**
 * The items for a query, best first. Recently used ones get a small lift; with no query,
 * the recent ones come first and the rest keep their order.
 */
export function search(items: PaletteItem[], query: string, recent: string[], limit = 60): PaletteItem[] {
  const lift = (id: string) => {
    const i = recent.indexOf(id);
    return i < 0 ? 0 : 20 - i;
  };
  if (!query.trim()) {
    const used = recent.map((id) => items.find((i) => i.id === id)).filter((i): i is PaletteItem => !!i);
    return [...used, ...items.filter((i) => !recent.includes(i.id))].slice(0, limit);
  }
  return items
    .map((item, order) => ({ item, order, s: score(item, query) }))
    .filter((r): r is { item: PaletteItem; order: number; s: number } => r.s !== null)
    .sort((a, b) => b.s + lift(b.item.id) - (a.s + lift(a.item.id)) || a.order - b.order)
    .slice(0, limit)
    .map((r) => r.item);
}
