/** One setting on the Settings page, for the command palette. */
export interface SettingRow {
  section: string;
  label: string;
  hint: string;
}

/**
 * The settings rows in Settings.tsx's source: each `<Row label="…" hint="…">` (or ToggleRow)
 * with the `<Section id>` it sits in. Read from the source itself, so the palette's list can't
 * drift from the page. A row in a helper component below the sections (after a `function`)
 * can't be placed, so it's left out; its section is still found.
 */
export function parseSettings(source: string): SettingRow[] {
  const sections = [...source.matchAll(/<Section\s+id="([a-z]+)"/g)].map((m) => ({ id: m[1], at: m.index ?? 0 }));
  const rows = [...source.matchAll(/<(?:Toggle)?Row\b[\s\S]{0,120}?\blabel="([^"]+)"/g)];
  const out: SettingRow[] = [];
  const seen = new Set<string>();
  rows.forEach((m, i) => {
    const at = m.index ?? 0;
    const section = [...sections].reverse().find((s) => s.at < at);
    if (!section || /\n\s*(?:export )?function [A-Z]/.test(source.slice(section.at, at))) return;
    // Its own attributes only: up to the next row.
    const own = source.slice(at, rows[i + 1]?.index ?? at + 800);
    const hint = own.match(/\bhint=(?:"([^"]*)"|'([^']*)'|\{`([^`]*)`\})/);
    const text = (hint?.[1] ?? hint?.[2] ?? hint?.[3] ?? "").replace(/\$\{[^}]*\}/g, "").replace(/\s+/g, " ").trim();
    const key = `${section.id}#${m[1]}`;
    if (seen.has(key)) return;
    seen.add(key);
    out.push({ section: section.id, label: m[1], hint: text });
  });
  return out;
}

/** The Settings page's rows, read when the palette first opens (a separate small chunk). */
export const loadSettingsIndex = () => import("../pages/Settings.tsx?raw").then((m) => parseSettings(m.default));
