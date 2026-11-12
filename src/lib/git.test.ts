import { describe, expect, it } from "vitest";
import { convertUrl, isSsh, nextStep, parseDiff } from "./git";
import type { GitStatus } from "./api";

const status = (over: Partial<GitStatus>): GitStatus => ({
  repo: true,
  branch: "main",
  head: "abc123",
  upstream: "origin/main",
  ahead: 0,
  behind: 0,
  files: [],
  operation: null,
  identity: { name: "A", email: "a@b.c", local: false },
  remotes: [{ name: "origin", url: "https://github.com/a/b.git", pushUrl: null }],
  ...over,
});

describe("nextStep", () => {
  const file = { path: "a", from: null, staged: " ", unstaged: "M", conflicted: false };
  it("puts a merge in progress first, then changes, pulling, a remote, publishing, pushing", () => {
    expect(nextStep(status({ operation: "merge", files: [file] }))).toBe("resolve");
    expect(nextStep(status({ files: [file], behind: 2 }))).toBe("commit");
    expect(nextStep(status({ behind: 2, ahead: 1 }))).toBe("pull");
    expect(nextStep(status({ remotes: [] }))).toBe("remote");
    expect(nextStep(status({ upstream: null }))).toBe("publish");
    expect(nextStep(status({ ahead: 3 }))).toBe("push");
    expect(nextStep(status({}))).toBe(null);
  });
  it("asks for nothing in a repository with no commit and no changes", () => {
    expect(nextStep(status({ head: null, remotes: [] }))).toBe(null);
  });
});

describe("parseDiff", () => {
  const diff = ["diff --git a/x b/x", "index 1..2 100644", "--- a/x", "+++ b/x", "@@ -10,3 +10,4 @@ function boot()", " keep", "-old", "+new", "+more", " end", "\\ No newline at end of file"].join("\n");
  it("numbers old and new lines like an editor, without git's headers", () => {
    const rows = parseDiff(diff);
    expect(rows[0]).toEqual({ kind: "hunk", text: "function boot()" });
    expect(rows.slice(1, 6)).toEqual([
      { kind: "ctx", old: 10, new: 10, text: "keep" },
      { kind: "del", old: 11, new: null, text: "old" },
      { kind: "add", old: null, new: 11, text: "new" },
      { kind: "add", old: null, new: 12, text: "more" },
      { kind: "ctx", old: 12, new: 13, text: "end" },
    ]);
    expect(rows[6]).toEqual({ kind: "note", text: "No newline at end of file" });
  });
  it("shows a new file from line 1", () => {
    const rows = parseDiff("diff --git a/n b/n\nnew file mode 100644\n--- /dev/null\n+++ b/n\n@@ -0,0 +1,2 @@\n+a\n+b\n");
    expect(rows.filter((r) => r.kind === "add").map((r) => r.kind === "add" && r.new)).toEqual([1, 2]);
  });
  it("says so for a binary file", () => {
    expect(parseDiff("diff --git a/i.png b/i.png\nBinary files /dev/null and b/i.png differ\n")).toEqual([{ kind: "note", text: "Binary file: no text to show." }]);
  });
  it("ignores git warnings printed before the diff", () => {
    const rows = parseDiff("warning: in the working copy of 'README.md', LF will be replaced by CRLF\n@@ -0,0 +1 @@\n+# Title\n");
    expect(rows).toEqual([
      { kind: "hunk", text: "" },
      { kind: "add", old: null, new: 1, text: "# Title" },
    ]);
  });
});

describe("convertUrl and isSsh", () => {
  it("switches GitHub-style URLs both ways", () => {
    expect(convertUrl("https://github.com/acme/shop.git")).toEqual({ url: "git@github.com:acme/shop.git", to: "SSH" });
    expect(convertUrl("https://github.com/acme/shop")).toEqual({ url: "git@github.com:acme/shop.git", to: "SSH" });
    expect(convertUrl("git@gitlab.com:group/sub/repo.git")).toEqual({ url: "https://gitlab.com/group/sub/repo.git", to: "HTTPS" });
    expect(convertUrl("ssh://git@host.example/team/repo.git")).toEqual({ url: "https://host.example/team/repo.git", to: "HTTPS" });
  });
  it("leaves alone what it can't switch", () => {
    expect(convertUrl("/srv/git/repo.git")).toBe(null);
    expect(convertUrl("file:///srv/git/repo.git")).toBe(null);
  });
  it("tells SSH from HTTPS", () => {
    expect(isSsh("git@github.com:a/b.git")).toBe(true);
    expect(isSsh("ssh://git@h/a.git")).toBe(true);
    expect(isSsh("https://github.com/a/b.git")).toBe(false);
  });
});
