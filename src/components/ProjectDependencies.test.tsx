// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { DependencyEcosystem, Site } from "../lib/api";

/**
 * The Dependencies tab against a fake backend: what it lists, and that every change is
 * planned (shown) first and only runs once confirmed.
 */

const calls: { cmd: string; args: unknown[] }[] = [];
let ecosystems: DependencyEcosystem[];
const record =
  (cmd: string, result: unknown = undefined) =>
  (...args: unknown[]) => {
    calls.push({ cmd, args });
    return Promise.resolve(typeof result === "function" ? (result as (...a: unknown[]) => unknown)(...args) : result);
  };

vi.mock("@tauri-apps/api/event", () => ({ listen: () => Promise.resolve(() => {}) }));
vi.mock("../lib/api", () => ({
  api: new Proxy(
    {},
    {
      get: (_, name: string) => {
        const table: Record<string, unknown> = {
          projectDependencies: record("projectDependencies", () => ecosystems),
          appLogs: record("appLogs", []),
          dependenciesOutdated: record("dependenciesOutdated", [{ name: "laravel/framework", current: "v11.30.0", latest: "12.1.0" }]),
          dependencyPlan: record("dependencyPlan", (_: string, r: { action: string; package?: string; version?: string }) => ({
            label: `Doing ${r.action}`,
            command: r.action === "upgrade" ? `composer require ${r.package}:^${r.version}` : `composer ${r.action} ${r.package ?? ""}`.trim(),
            edit: null,
          })),
          runDependencyAction: record("runDependencyAction"),
        };
        return table[name] ?? record(name);
      },
    },
  ),
}));

const { default: ProjectDependencies } = await import("./ProjectDependencies");

const site = { name: "shop", path: "C:/www/shop" } as Site;
const composer = (over: Partial<DependencyEcosystem> = {}): DependencyEcosystem => ({
  manager: "composer",
  label: "Composer",
  manifest: "composer.json",
  lock: "composer.lock",
  installed: true,
  packages: [
    { name: "laravel/framework", wanted: "^11.0", installed: "v11.30.0", dev: false },
    { name: "phpunit/phpunit", wanted: "^11", installed: "11.4.0", dev: true },
  ],
  canEdit: true,
  install: "composer install",
  updateAll: "composer update",
  missing: null,
  note: null,
  ...over,
});

function show() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={client}>
      <ProjectDependencies site={site} />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  calls.length = 0;
  ecosystems = [composer()];
});
afterEach(cleanup);

describe("Dependencies", () => {
  it("lists the manifest's packages with their installed versions", async () => {
    show();
    expect(await screen.findByText("laravel/framework")).toBeTruthy();
    expect(screen.getByText("v11.30.0")).toBeTruthy();
    expect(screen.getByText("dev")).toBeTruthy();
    expect(screen.getAllByText("Installed")).toHaveLength(2); // the column and the status
  });

  it("shows the upgrade's command and runs it only when confirmed", async () => {
    show();
    await userEvent.click(await screen.findByRole("button", { name: /Check for updates/ }));
    await userEvent.click(await screen.findByRole("button", { name: "To 12.1.0" }));
    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText("composer require laravel/framework:^12.1.0")).toBeTruthy();
    expect(calls.some((c) => c.cmd === "runDependencyAction")).toBe(false);
    await userEvent.click(within(dialog).getByRole("button", { name: "Run" }));
    await waitFor(() =>
      expect(calls.find((c) => c.cmd === "runDependencyAction")?.args).toEqual(["shop", { manager: "composer", action: "upgrade", package: "laravel/framework", dev: false, version: "12.1.0" }]),
    );
  });

  it("adds a development package", async () => {
    show();
    await userEvent.type(await screen.findByLabelText("Add a Composer package"), "pestphp/pest");
    await userEvent.click(screen.getByLabelText("Development only"));
    await userEvent.click(screen.getByRole("button", { name: /Add/ }));
    await waitFor(() => expect(calls.find((c) => c.cmd === "dependencyPlan")?.args).toEqual(["shop", { manager: "composer", action: "add", package: "pestphp/pest", dev: true }]));
  });

  it("says what's missing and keeps the buttons off", async () => {
    ecosystems = [composer({ missing: "Composer isn't installed. Install it on the Services page.", canEdit: false })];
    show();
    expect(await screen.findByText(/Composer isn't installed/)).toBeTruthy();
    expect((screen.getByRole("button", { name: /Install/ }) as HTMLButtonElement).disabled).toBe(true);
  });
});
