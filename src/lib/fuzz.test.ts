import { describe, expect, it } from "vitest";
import fc from "fast-check";
import { parseLog } from "./logparse";
import { convertUrl, parseDiff } from "./git";
import type { LogService, LogStream } from "./api";

/**
 * Property tests: thousands of generated inputs instead of hand-picked ones. Logs and diffs
 * come from programs we don't control, so the parsers must never throw, never invent an
 * invalid date, and keep their counts consistent whatever they're fed.
 */

const levels = ["error", "warn", "notice", "info", "access", "debug"];

describe("parseLog never breaks", () => {
  it("returns a valid date and a known level for any line", () => {
    fc.assert(
      fc.property(fc.string({ unit: "binary", maxLength: 400 }), fc.constantFrom("mysql", "apache", "project", "system"), fc.constantFrom("output", "error", "access"), (line, service, stream) => {
        const e = parseLog({ id: 1, service: service as LogService, stream: stream as LogStream, line });
        expect(Number.isNaN(e.time.getTime())).toBe(false);
        expect(levels).toContain(e.level);
        expect(typeof e.message).toBe("string");
      }),
      { numRuns: 3000 },
    );
  });

  it("survives lines that look almost like a known format", () => {
    const prefix = fc.constantFrom("2026-11-01T", "2026-13-45 99:99:99 ", "{", '{"t":{"$date":', "1234:M 01 Nov 2026 ", "[Sat Oct 24 ", "time=", "E2026", "01-Nov-2026 ", "[2026-11-01T");
    fc.assert(
      fc.property(prefix, fc.string({ maxLength: 120 }), (p, rest) => {
        const e = parseLog({ id: 1, service: "mysql", stream: "output", line: p + rest });
        expect(Number.isNaN(e.time.getTime())).toBe(false);
      }),
      { numRuns: 3000 },
    );
  });

  it("reads back any MySQL timestamp it is given", () => {
    fc.assert(
      fc.property(fc.date({ min: new Date("2000-01-01T00:00:00Z"), max: new Date("2099-12-31T23:59:59Z"), noInvalidDate: true }), (d) => {
        const stamp = d.toISOString().replace(/\.\d{3}Z$/, ".123456Z");
        const e = parseLog({ id: 1, service: "mysql", stream: "output", line: `${stamp} 0 [Note] [MY-000000] [Server] hello` });
        expect(e.timed).toBe(true);
        expect(Math.floor(e.time.getTime() / 1000)).toBe(Math.floor(d.getTime() / 1000));
      }),
    );
  });
});

describe("parseDiff stays consistent", () => {
  // A random hunk: lines of context, removals and additions, with a matching header.
  const hunk = fc.array(fc.tuple(fc.constantFrom(" ", "-", "+"), fc.string({ maxLength: 30 }).map((s) => s.replace(/[\r\n]/g, ""))), { minLength: 1, maxLength: 60 });
  it("numbers lines exactly as the hunk header says", () => {
    fc.assert(
      fc.property(fc.integer({ min: 1, max: 5000 }), fc.integer({ min: 1, max: 5000 }), hunk, (oldStart, newStart, lines) => {
        const oldCount = lines.filter(([k]) => k !== "+").length;
        const newCount = lines.filter(([k]) => k !== "-").length;
        const text = `@@ -${oldStart},${oldCount} +${newStart},${newCount} @@\n${lines.map(([k, s]) => k + s).join("\n")}\n`;
        const rows = parseDiff(text).filter((r) => r.kind === "add" || r.kind === "del" || r.kind === "ctx");
        expect(rows.length).toBe(lines.length);
        const olds = rows.flatMap((r) => (r.kind !== "hunk" && r.kind !== "note" && r.old !== null ? [r.old] : []));
        const news = rows.flatMap((r) => (r.kind !== "hunk" && r.kind !== "note" && r.new !== null ? [r.new] : []));
        expect(olds).toEqual(Array.from({ length: oldCount }, (_, i) => oldStart + i));
        expect(news).toEqual(Array.from({ length: newCount }, (_, i) => newStart + i));
      }),
      { numRuns: 1000 },
    );
  });

  it("never throws on garbage", () => {
    fc.assert(fc.property(fc.string({ unit: "binary", maxLength: 2000 }), (t) => void parseDiff(t)), { numRuns: 2000 });
  });
});

describe("convertUrl round trips", () => {
  const part = fc.stringMatching(/^[a-z0-9][a-z0-9_-]{0,15}$/);
  it("HTTPS → SSH → HTTPS gives the same repository", () => {
    fc.assert(
      fc.property(fc.constantFrom("github.com", "gitlab.com", "git.example.org"), part, part, (host, owner, repo) => {
        const https = `https://${host}/${owner}/${repo}.git`;
        const ssh = convertUrl(https);
        expect(ssh?.to).toBe("SSH");
        expect(convertUrl(ssh!.url)).toEqual({ url: https, to: "HTTPS" });
      }),
    );
  });
});
