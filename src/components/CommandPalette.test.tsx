// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";

/** The command palette finds services, projects and settings by name or synonym, and runs them. */

const started: string[] = [];
vi.mock("../lib/api", async (importOriginal) => {
  const original = await importOriginal<typeof import("../lib/api")>();
  const api = {
    listServices: () =>
      Promise.resolve([
        { kind: "mariadb", label: "MariaDB", installed: true, state: "stopped", external: false },
        { kind: "redis", label: "Redis", installed: false, state: "stopped", external: false },
      ]),
    installedRuntimes: () => Promise.resolve([{ runtime: "mariadb", active: "12.3.4", versions: ["12.3.4"] }]),
    listSites: () => Promise.resolve({ sites: [{ name: "blog", domain: "blog.test", path: "C:/www/blog", url: "https://blog.test", framework: "Laravel", language: "php" }], tools: [] }),
    runtimeInfo: () => Promise.resolve({ runtimes: [{ id: "mariadb", label: "MariaDB", blurb: "Database", unavailable: null, keywords: ["sql"] }], groups: [] }),
    serviceConfigFiles: () => Promise.resolve([{ label: "my.ini", path: "C:/lampRabit/etc/mariadb/my.ini", exists: true }]),
    getSettings: () => Promise.resolve({ profiles: [], webServers: ["apache"] }),
    startService: (kind: string) => (started.push(kind), Promise.resolve()),
  };
  return { ...original, api };
});

const { default: CommandPalette } = await import("./CommandPalette");
const { useAppStore } = await import("../lib/store");

const show = () => {
  useAppStore.setState({ paletteOpen: true });
  render(
    <QueryClientProvider client={new QueryClient()}>
      <CommandPalette />
    </QueryClientProvider>,
  );
};

afterEach(() => {
  cleanup();
  started.length = 0;
  localStorage.clear();
});

describe("the command palette", () => {
  it("starts a service found by a synonym, and closes", async () => {
    show();
    await screen.findByText("Start MariaDB");
    await userEvent.type(screen.getByRole("combobox"), "start db");
    expect(screen.getAllByRole("option")[0].textContent).toContain("Start MariaDB");
    await userEvent.keyboard("{Enter}");
    await vi.waitFor(() => expect(started).toEqual(["mariadb"]));
    expect(useAppStore.getState().paletteOpen).toBe(false);
  });

  it("opens a project, a settings section and a config file by name", async () => {
    show();
    const input = await screen.findByRole("combobox");
    await userEvent.type(input, "blog.test");
    expect(screen.getAllByRole("option")[0].textContent).toContain("blog");
    await userEvent.keyboard("{Enter}");
    expect(useAppStore.getState().siteFocus).toBe("C:/www/blog");

    cleanup();
    show();
    await userEvent.type(await screen.findByRole("combobox"), "ngrok");
    await userEvent.keyboard("{Enter}");
    expect(useAppStore.getState().settingsFocus).toBe("sharing");

    cleanup();
    show();
    await userEvent.type(await screen.findByRole("combobox"), "my.ini");
    expect(await screen.findByText("my.ini")).toBeTruthy();
  });

  it("says when nothing matches", async () => {
    show();
    await userEvent.type(await screen.findByRole("combobox"), "zzzz");
    expect(screen.getByText(/Nothing matches/)).toBeTruthy();
  });
});
