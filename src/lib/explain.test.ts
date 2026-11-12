import { describe, expect, it } from "vitest";
import { explain } from "./explain";

describe("explain", () => {
  it("turns common raw errors into what to do", () => {
    expect(explain("git@github.com: Permission denied (publickey).\nfatal: Could not read from remote repository.")).toMatch(/SSH key/);
    expect(explain("fatal: could not read Username for 'https://github.com': terminal prompts disabled")).toMatch(/sign in/);
    expect(explain("! [rejected]        main -> main (fetch first)")).toMatch(/Pull first/);
    expect(explain("bind: An attempt was made... Only one usage of each socket address (protocol/network address/port) is normally permitted.")).toMatch(/port/);
    expect(explain("fatal: unable to access 'https://x/': Could not resolve host: x")).toMatch(/internet/);
  });
  it("prefers the specific SSH hint over the generic permission one", () => {
    expect(explain("Permission denied (publickey)")).not.toMatch(/system refused/);
  });
  it("stays quiet about errors it doesn't know", () => {
    expect(explain("Write a commit message first.")).toBe(null);
  });
});
