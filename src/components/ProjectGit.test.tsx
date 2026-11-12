// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { GitStatus, Site } from "../lib/api";

/**
 * The Git tab against a fake backend: each test sets the repository's state and checks what
 * the user sees and which commands the buttons send.
 */

const calls: { cmd: string; args: unknown[] }[] = [];
let state: GitStatus;
const record =
  (cmd: string, result: unknown = undefined) =>
  (...args: unknown[]) => {
    calls.push({ cmd, args });
    return Promise.resolve(typeof result === "function" ? (result as (...a: unknown[]) => unknown)(...args) : result);
  };

vi.mock("../lib/api", () => ({
  api: new Proxy(
    {},
    {
      get: (_, name: string) => {
        const table: Record<string, unknown> = {
          gitVersion: record("gitVersion", "2.47.0"),
          gitStatus: record("gitStatus", () => state),
          gitDiff: record("gitDiff", "@@ -0,0 +1,2 @@\n+hello\n+world\n"),
          gitCommit: record("gitCommit", ""),
          gitSync: record("gitSync", ""),
          gitStage: record("gitStage"),
          gitRemote: record("gitRemote"),
          gitRepoSettings: record("gitRepoSettings", { pull: null, prune: false, autoSetupRemote: false }),
          gitLog: record("gitLog", [{ hash: "a".repeat(40), short: "aaaaaaa", author: "Ada", email: "ada@x.io", time: 1793500000, refs: "HEAD -> main, origin/main, tag: v1.0", subject: "Add the shop" }]),
          gitCommitDetails: record("gitCommitDetails", {
            hash: "a".repeat(40),
            parents: ["b".repeat(40)],
            author: "Ada",
            authorEmail: "ada@x.io",
            authorTime: 1793500000,
            committer: "Ada",
            committerEmail: "ada@x.io",
            committerTime: 1793500000,
            refs: "HEAD -> main",
            subject: "Add the shop",
            body: "With a cart.",
            files: [
              { path: "src/Cart.php", from: null, status: "A", additions: 40, deletions: 0, binary: false },
              { path: "logo.png", from: null, status: "M", additions: 0, deletions: 0, binary: true },
            ],
          }),
          gitCommitFileDiff: record("gitCommitFileDiff", "@@ -1 +1 @@\n-old\n+new\n"),
          gitBranches: record("gitBranches", []),
          gitStashes: record("gitStashes", [{ index: 0, message: "On main: half-done cart", time: 1793500000 }]),
          gitTags: record("gitTags", []),
          gitStash: record("gitStash", ""),
          gitResolve: record("gitResolve"),
        };
        return table[name] ?? record(name);
      },
    },
  ),
}));

const { default: ProjectGit } = await import("./ProjectGit");

const site = { name: "shop", path: "C:/www/shop" } as Site;
const base = (over: Partial<GitStatus> = {}): GitStatus => ({
  repo: true,
  branch: "main",
  head: "abc",
  upstream: "origin/main",
  ahead: 0,
  behind: 0,
  files: [],
  operation: null,
  identity: { name: "Ada", email: "ada@x.io", local: false },
  remotes: [{ name: "origin", url: "https://github.com/acme/shop.git", pushUrl: null }],
  ...over,
});
const untracked = (path: string) => ({ path, from: null, staged: "?", unstaged: "?", conflicted: false });

function show() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={client}>
      <ProjectGit site={site} />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  calls.length = 0;
});
afterEach(cleanup);

describe("Changes", () => {
  it("commits everything when nothing is staged", async () => {
    state = base({ files: [untracked("a.txt"), untracked("src/b.txt")] });
    show();
    const box = await screen.findByLabelText("Commit message");
    await userEvent.type(box, "First pass");
    const button = screen.getByRole("button", { name: /Commit all 2/ });
    await userEvent.click(button);
    await waitFor(() => expect(calls.find((c) => c.cmd === "gitCommit")?.args).toEqual(["shop", "First pass", false, true]));
  });

  it("names a file first and its folder after", async () => {
    state = base({ files: [untracked("src/deep/Cart.php")] });
    show();
    expect(await screen.findByText("Cart.php")).toBeTruthy();
    expect(screen.getByText("src/deep")).toBeTruthy();
  });

  it("shows a diff with line numbers and no +/- marks", async () => {
    state = base({ files: [untracked("a.txt")] });
    show();
    await userEvent.click(await screen.findByText("a.txt"));
    const line = await screen.findByText("hello");
    expect(line.textContent).toBe("hello");
    expect(line.parentElement?.textContent).toContain("1");
  });

  it("offers to push when everything is committed but not pushed", async () => {
    state = base({ ahead: 2 });
    show();
    await userEvent.click(await screen.findByRole("button", { name: /Push 2 commits/ }));
    // The third argument is the run's id, for its progress events and Cancel.
    await waitFor(() => expect(calls.find((c) => c.cmd === "gitSync")?.args.slice(0, 2)).toEqual(["shop", "push"]));
    expect(String(calls.find((c) => c.cmd === "gitSync")?.args[2])).toMatch(/^shop:push:\d+$/);
  });

  it("offers to publish a branch that has no upstream", async () => {
    state = base({ upstream: null });
    show();
    expect(await screen.findAllByRole("button", { name: /Publish branch/ })).toHaveLength(2);
  });

  it("asks for a remote instead of showing sync buttons that can't work", async () => {
    state = base({ remotes: [], upstream: null });
    show();
    expect(await screen.findByRole("button", { name: /Add a remote$/ })).toBeTruthy();
    expect(screen.queryByRole("button", { name: /Fetch/ })).toBeNull();
    expect(screen.queryByRole("button", { name: /^Pull/ })).toBeNull();
  });

  it("highlights pull when the remote is ahead", async () => {
    state = base({ behind: 3 });
    show();
    const pull = await screen.findByRole("button", { name: /Pull 3/ });
    expect(pull.className).toContain("bg-accent");
    expect(screen.getByRole("button", { name: /Fetch/ }).className).not.toContain("bg-accent");
  });
});

describe("Stash and conflicts", () => {
  it("stashes everything and lists stashes to restore", async () => {
    state = base({ files: [untracked("a.txt")] });
    show();
    await userEvent.click(await screen.findByRole("button", { name: "Stash" }));
    await waitFor(() => expect(calls.find((c) => c.cmd === "gitStash")?.args).toEqual(["shop", { kind: "save", message: "", untracked: true }]));
    expect(await screen.findByText("half-done cart")).toBeTruthy();
    await userEvent.click(screen.getByRole("button", { name: "Restore" }));
    await waitFor(() => expect(calls.filter((c) => c.cmd === "gitStash").slice(-1)[0]?.args).toEqual(["shop", { kind: "pop", index: 0 }]));
  });

  it("settles a conflict with one side's version", async () => {
    state = base({ operation: "merge", files: [{ path: "app.php", from: null, staged: "U", unstaged: "U", conflicted: true }] });
    show();
    expect(await screen.findByText(/A merge is in progress/)).toBeTruthy();
    await userEvent.click(screen.getByText("app.php"));
    await userEvent.click(await screen.findByRole("button", { name: "Take the incoming version" }));
    await waitFor(() => expect(calls.find((c) => c.cmd === "gitResolve")?.args).toEqual(["shop", "app.php", true]));
  });
});

describe("History", () => {
  it("shows the commit's metadata, its files with counts, and a file's diff", async () => {
    state = base();
    show();
    await userEvent.click(await screen.findByRole("tab", { name: /History/ }));
    expect(await screen.findByText("With a cart.")).toBeTruthy();
    expect(screen.getByText("a".repeat(40))).toBeTruthy();
    expect(screen.getByText("2 files changed")).toBeTruthy();
    expect(screen.getByText("binary")).toBeTruthy();
    expect(screen.getAllByText("tag v1.0").length).toBeGreaterThan(0);
    await userEvent.click(screen.getByText("Cart.php"));
    expect(await screen.findByText("new")).toBeTruthy();
    await waitFor(() => expect(calls.some((c) => c.cmd === "gitCommitFileDiff" && c.args[2] === "src/Cart.php")).toBe(true));
  });
});

describe("Remotes", () => {
  it("builds an SSH URL from host and owner/repository", async () => {
    state = base({ remotes: [] , upstream: null});
    show();
    await userEvent.click(await screen.findByRole("tab", { name: /Remotes/ }));
    const form = (await screen.findByText("Add a remote", { selector: "span" })).closest("form")!;
    await userEvent.click(within(form).getByRole("radio", { name: "SSH" }));
    await userEvent.type(within(form).getByLabelText("Owner and repository"), "acme/shop");
    expect(within(form).getByText("git@github.com:acme/shop.git")).toBeTruthy();
    await userEvent.click(within(form).getByRole("button", { name: /Add/ }));
    await waitFor(() => expect(calls.find((c) => c.cmd === "gitRemote")?.args).toEqual(["shop", { kind: "add", name: "origin", url: "git@github.com:acme/shop.git" }]));
  });

  it("switches an HTTPS remote to SSH in one click", async () => {
    state = base();
    show();
    await userEvent.click(await screen.findByRole("tab", { name: /Remotes/ }));
    await userEvent.click(await screen.findByRole("button", { name: /Use SSH/ }));
    await waitFor(() => expect(calls.find((c) => c.cmd === "gitRemote")?.args).toEqual(["shop", { kind: "setUrl", name: "origin", url: "git@github.com:acme/shop.git" }]));
  });
});
