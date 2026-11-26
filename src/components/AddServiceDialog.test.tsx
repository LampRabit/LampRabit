// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ServiceFile } from "../lib/api";

/** Adding a service from a file shows what it downloads and runs, and only adds on approval. */

const added: unknown[][] = [];
vi.mock("../lib/api", async (importOriginal) => {
  const original = await importOriginal<typeof import("../lib/api")>();
  const api = { addServiceFile: (...args: unknown[]) => (added.push(args), Promise.resolve("echo")) };
  return { ...original, api };
});

const { default: AddServiceDialog } = await import("./AddServiceDialog");

const file: ServiceFile = {
  replaces: false,
  review: {
    id: "echo",
    name: "Echo",
    blurb: "Says things back",
    category: "tools",
    homepage: null,
    revision: 1,
    unavailable: null,
    downloads: "GitHub releases of someone/echo",
    checksum: "The SHA-256 GitHub reports for each file",
    program: "{{home}}/echo{{exe}}",
    args: ["--port", "{{port.http}}"],
    commands: [],
    files: [],
    ports: [["Web", 7000]],
    needs: ["java 21 or newer"],
  },
};

afterEach(() => {
  cleanup();
  added.length = 0;
});

describe("adding a service from a file", () => {
  it("shows what it downloads and runs, and adds it with approval", async () => {
    const onAdded = vi.fn();
    render(<AddServiceDialog path="C:/services/echo/service.yaml" file={file} onClose={() => {}} onAdded={onAdded} />);
    expect(screen.getByText(/hasn't been reviewed/)).toBeTruthy();
    expect(screen.getByText("GitHub releases of someone/echo")).toBeTruthy();
    expect(screen.getByText("{{home}}/echo{{exe}} --port {{port.http}}")).toBeTruthy();
    expect(screen.getByText("Web 7000")).toBeTruthy();
    expect(screen.getByText("java 21 or newer")).toBeTruthy();
    await userEvent.click(screen.getByRole("button", { name: "Add service" }));
    await vi.waitFor(() => expect(onAdded).toHaveBeenCalledWith("echo"));
    expect(added).toEqual([["C:/services/echo/service.yaml", true]]);
  });

  it("says when it updates a service added before", () => {
    render(<AddServiceDialog path="x.yaml" file={{ ...file, replaces: true }} onClose={() => {}} onAdded={() => {}} />);
    expect(screen.getByRole("button", { name: "Update service" })).toBeTruthy();
  });
});
