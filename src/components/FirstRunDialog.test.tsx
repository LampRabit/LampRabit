// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ImportSource, Settings } from "../lib/api";

/**
 * The setup guide against a fake backend: usages bring their components by category,
 * projects from other tools are copied under the names chosen, and the name asked last
 * ends up in the settings and the closing greeting.
 */

const calls: { cmd: string; args: unknown[] }[] = [];
let settings: Settings;
let sources: ImportSource[];
const record =
  (cmd: string, result: unknown = undefined) =>
  (...args: unknown[]) => {
    calls.push({ cmd, args });
    return Promise.resolve(typeof result === "function" ? (result as (...a: unknown[]) => unknown)(...args) : result);
  };

vi.mock("@tauri-apps/plugin-dialog", () => ({ open: () => Promise.resolve(null) }));
vi.mock("@tauri-apps/api/event", () => ({ listen: () => Promise.resolve(() => {}) }));
vi.mock("../lib/api", async (importOriginal) => {
  const original = await importOriginal<typeof import("../lib/api")>();
  Object.assign(original.serviceLabels, { apache: "Apache", mariadb: "MariaDB", mailpit: "Mailpit", redis: "Redis", postgres: "PostgreSQL" });
  const labels: Record<string, string> = { apache: "Apache", php: "PHP", composer: "Composer", mariadb: "MariaDB", mailpit: "Mailpit", phpmyadmin: "phpMyAdmin", git: "Git", python: "Python", postgres: "PostgreSQL", redis: "Redis", meilisearch: "Meilisearch" };
  const table: Record<string, unknown> = {
    firstRun: record("firstRun", false),
    getSettings: record("getSettings", () => settings),
    saveSettings: record("saveSettings", (s: Settings) => (settings = s)),
    nodeStatus: record("nodeStatus", { manager: "nvm", current: "22.11.0" }),
    runtimeInfo: record("runtimeInfo", {
      groups: [
        { id: "web", title: "Web and app servers", blurb: "", items: ["apache", "nginx"] },
        { id: "languages", title: "Languages", blurb: "", items: ["php", "python", "node"] },
        { id: "databases", title: "Databases", blurb: "", items: ["mariadb", "postgres", "sqlite"] },
        { id: "search", title: "Search", blurb: "", items: ["meilisearch"] },
        { id: "tools", title: "Developer tools", blurb: "", items: ["git", "mailpit", "phpmyadmin", "composer", "redis"] },
      ],
      runtimes: Object.entries(labels).map(([id, label]) => ({ id, label, blurb: "", source: "", unavailable: null, needs: [], suggests: [] })),
    }),
    starterPlan: record("starterPlan", (ids: string[]) =>
      ids.map((runtime) => ({ runtime, label: labels[runtime] ?? runtime, blurb: "", action: { kind: "install", release: { runtime, version: "1.0.0" } }, size: 1_000_000 })),
    ),
    importSources: record("importSources", () => sources),
    importProject: record("importProject", (_: string, name: string) => ({ name, files: 12, skipped: ["vendor"] })),
  };
  return { ...original, api: new Proxy({}, { get: (_, name: string) => table[name] ?? record(name) }) };
});

const { default: FirstRunDialog } = await import("./FirstRunDialog");
const { useAppStore } = await import("../lib/store");

function show(open: "guide" | "import") {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  act(() => useAppStore.getState().setSetupGuideOpen(open));
  return render(
    <QueryClientProvider client={client}>
      <FirstRunDialog />
    </QueryClientProvider>,
  );
}

const xampp = (): ImportSource => ({
  id: "xampp",
  name: "XAMPP",
  folders: ["C:\\xampp\\htdocs"],
  projects: [
    { name: "shop", path: "C:\\xampp\\htdocs\\shop", framework: "Laravel", database: "shop_db", target: "shop" },
    { name: "blog", path: "C:\\xampp\\htdocs\\blog", framework: "WordPress", database: null, target: "blog" },
  ],
  databases: ["shop_db"],
});

beforeEach(() => {
  calls.length = 0;
  sources = [];
  settings = { profiles: [], sites: {}, versions: {}, rootDir: "C:\\lamprabit", developerName: "" } as unknown as Settings;
});
afterEach(() => {
  cleanup();
  act(() => useAppStore.getState().setSetupGuideOpen(false));
});

describe("what you build", () => {
  it("shows each usage's components by category, and adds or skips them", async () => {
    show("guide");
    expect(await screen.findByText("What do you build?")).toBeTruthy();
    // PHP websites is picked to begin with.
    expect(await screen.findByRole("region", { name: "Web and app servers" })).toBeTruthy();
    expect(within(screen.getByRole("region", { name: "Databases" })).getByText("MariaDB")).toBeTruthy();
    expect(within(screen.getByRole("region", { name: "Databases" })).queryByText("PostgreSQL")).toBeNull();

    await userEvent.click(within(screen.getByRole("group", { name: "What you build" })).getByRole("button", { name: /Python/ }));
    const databases = screen.getByRole("region", { name: "Databases" });
    expect(await within(databases).findByText("PostgreSQL")).toBeTruthy();

    // Anything else comes from its category; SQLite isn't offered (PHP has it built in).
    const more = screen.getByRole("region", { name: "Add more" });
    expect(within(more).queryByText(/SQLite|sqlite/)).toBeNull();
    await userEvent.click(within(more).getByRole("button", { name: /Meilisearch/ }));
    expect(await within(screen.getByRole("region", { name: "Search" })).findByText("Meilisearch")).toBeTruthy();

    await userEvent.click(screen.getByRole("checkbox", { name: "Include phpMyAdmin" }));
    await waitFor(() => expect(screen.getByRole("button", { name: "Install" })).toHaveProperty("disabled", false));
    await userEvent.click(screen.getByRole("button", { name: "Install" }));
    await waitFor(() => expect(calls.filter((c) => c.cmd === "installRuntime").map((c) => (c.args[0] as { runtime: string }).runtime)).toContain("meilisearch"));
    const installed = calls.filter((c) => c.cmd === "installRuntime").map((c) => (c.args[0] as { runtime: string }).runtime);
    expect(installed).not.toContain("phpmyadmin");
    expect(installed).toContain("postgres");
  });
});

describe("importing projects", () => {
  it("copies the picked projects under the names chosen", async () => {
    sources = [xampp()];
    show("import");
    expect(await screen.findByText("XAMPP")).toBeTruthy();
    expect(screen.getByText(/Its databases stay in XAMPP/)).toBeTruthy();

    await userEvent.click(screen.getByRole("checkbox", { name: "Import shop" }));
    const field = screen.getByRole("textbox", { name: "Folder name for shop" });
    await userEvent.clear(field);
    await userEvent.type(field, "old-shop");
    await userEvent.click(screen.getByRole("button", { name: "Copy 1 into www" }));
    await waitFor(() => expect(calls.find((c) => c.cmd === "importProject")?.args).toEqual(["C:\\xampp\\htdocs\\shop", "old-shop"]));
    expect(await screen.findByText(/Copied 1 project into www/)).toBeTruthy();
    expect(screen.getByText(/each project's Overview offers to install them/)).toBeTruthy();
  });

  it("won't copy two projects to one name", async () => {
    sources = [xampp()];
    show("import");
    await userEvent.click(await screen.findByRole("button", { name: "Select all" }));
    const field = screen.getByRole("textbox", { name: "Folder name for blog" });
    await userEvent.clear(field);
    await userEvent.type(field, "shop");
    expect(screen.getAllByText("Two projects would get this name.").length).toBe(2);
    expect(screen.getByRole("button", { name: "Copy 2 into www" })).toHaveProperty("disabled", true);
  });
});

describe("the name", () => {
  it("is asked last and greets the developer", async () => {
    show("guide");
    await userEvent.click(await screen.findByRole("button", { name: "Skip, I'll use Services" }));
    await userEvent.click(await screen.findByRole("button", { name: "Skip" }));
    const input = await screen.findByPlaceholderText("Your name");
    await userEvent.type(input, "Sam{Enter}");
    await waitFor(() => expect(settings.developerName).toBe("Sam"));
    expect(await screen.findByText("You're all set, Sam")).toBeTruthy();
  });
});
