// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";

/** The template editor's guided form: languages beyond PHP, starters, and saving. */

const saved: string[] = [];
vi.mock("../lib/api", async (importOriginal) => {
  const original = await importOriginal<typeof import("../lib/api")>();
  const table: Record<string, unknown> = {
    saveTemplate: (yaml: string) => {
      saved.push(yaml);
      return Promise.resolve("tauri-desk");
    },
  };
  return { ...original, api: new Proxy({}, { get: (_, name: string) => table[name] ?? (() => Promise.resolve(null)) }) };
});

const { default: TemplateEditor } = await import("./TemplateEditor");
const { useAppStore } = await import("../lib/store");

beforeEach(() => {
  saved.length = 0;
  act(() => useAppStore.getState().openTemplateEditor({}));
  render(
    <QueryClientProvider client={new QueryClient()}>
      <TemplateEditor />
    </QueryClientProvider>,
  );
});
afterEach(() => {
  cleanup();
  act(() => useAppStore.getState().closeTemplateEditor());
});

describe("template editor", () => {
  it("makes a Rust and Node.js template from the Tauri starter", async () => {
    const save = screen.getByRole("button", { name: "Save template" });
    expect(save).toHaveProperty("disabled", true);
    await userEvent.type(screen.getByPlaceholderText("Shop"), "Tauri desk");

    // A Tauri app is Rust plus Node.js, without PHP.
    const languages = screen.getByRole("region", { name: "Languages" });
    await userEvent.click(within(languages).getByRole("button", { name: "PHP" }));
    await userEvent.click(within(languages).getByRole("button", { name: "Rust" }));
    await userEvent.click(within(languages).getByRole("button", { name: "Node.js" }));

    const code = screen.getByRole("region", { name: "Where the code comes from" });
    // Composer is offered only for PHP templates.
    expect(within(code).queryByRole("radio", { name: "Composer" })).toBeNull();
    await userEvent.click(within(code).getByRole("radio", { name: "A command" }));
    await userEvent.click(within(code).getByRole("button", { name: "Tauri (Rust + React)" }));

    // The setup suggestions follow the languages: Cargo and the Node packages, no Composer.
    const setup = screen.getByRole("region", { name: "Setup steps" });
    expect(within(setup).queryByText("composer install")).toBeNull();
    expect(within(setup).getByText("cargo build once (later runs are quick)")).toBeTruthy();

    await userEvent.click(screen.getByRole("radio", { name: "YAML" }));
    const yaml = (screen.getByRole("textbox", { name: "Template YAML" }) as HTMLTextAreaElement).value;
    expect(yaml).toContain("id: tauri-desk");
    expect(yaml).toContain("  rust: '1.80'\n  node: true");
    expect(yaml).not.toContain("php:");
    expect(yaml).toContain("tauri-app@latest");
    expect(yaml).toContain("mode: dynamic");

    await userEvent.click(save);
    expect(saved[0]).toBe(yaml);
  });
});
