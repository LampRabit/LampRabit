import { describe, expect, it } from "vitest";
import { expand, score, search, type PaletteItem } from "./palette";

const item = (id: string, title: string, extra: Partial<PaletteItem> = {}): PaletteItem => ({ id, title, group: "Actions", run: () => {}, ...extra });

const items = [
  item("start-mariadb", "Start MariaDB", { group: "Services", keywords: ["database"] }),
  item("settings-ports", "Ports", { group: "Settings", keywords: ["3306", "conflict"] }),
  item("new-project", "New project"),
  item("site-blog", "blog", { group: "Projects", hint: "https://blog.test" }),
  item("settings-sharing", "Sharing", { group: "Settings", keywords: ["ngrok", "tunnel"] }),
];

describe("the command palette's search", () => {
  it("expands a word to its synonyms, and a prefix of one", () => {
    expect(expand("ssl")).toContain("https");
    expect(expand("data")).toContain("mariadb");
    expect(expand("zz")).toEqual(["zz"]);
  });

  it("needs every word to match, and ranks title matches over keyword matches", () => {
    expect(score(items[0], "start maria")).not.toBeNull();
    expect(score(items[0], "start nginx")).toBeNull();
    expect(score(items[0], "start")! > score(items[0], "database")!).toBe(true);
  });

  it("finds things by synonym and keyword", () => {
    expect(search(items, "db", []).map((i) => i.id)).toContain("start-mariadb");
    expect(search(items, "tunnel", [])[0].id).toBe("settings-sharing");
    expect(search(items, "3306", [])[0].id).toBe("settings-ports");
    expect(search(items, "blog.test", [])[0].id).toBe("site-blog");
  });

  it("puts recent items first with no query, and lifts them a little otherwise", () => {
    expect(search(items, "", ["site-blog"])[0].id).toBe("site-blog");
    const tie = [item("a", "Open alpha"), item("b", "Open beta")];
    expect(search(tie, "open", ["b"])[0].id).toBe("b");
    expect(search(tie, "open", [])[0].id).toBe("a");
  });
});
