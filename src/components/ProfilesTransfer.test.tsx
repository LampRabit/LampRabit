// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ServiceKind, Settings, StackProfile } from "../lib/api";

/**
 * Importing stack profiles against a fake backend: services that aren't installed are
 * reviewed first, and each is installed or taken out of the profiles.
 */

const calls: { cmd: string; args: unknown[] }[] = [];
let installed: ServiceKind[] = [];
let file: { profiles: StackProfile[] };
let settings: Settings;
const record =
  (cmd: string, result: unknown = undefined) =>
  (...args: unknown[]) => {
    calls.push({ cmd, args });
    return Promise.resolve(typeof result === "function" ? (result as (...a: unknown[]) => unknown)(...args) : result);
  };

vi.mock("@tauri-apps/plugin-dialog", () => ({ open: () => Promise.resolve("C:/profiles.json"), save: () => Promise.resolve(null) }));
vi.mock("../lib/api", async (importOriginal) => {
  const original = await importOriginal<typeof import("../lib/api")>();
  // The app loads these from the backend's catalog before the first render.
  Object.assign(original.serviceLabels, { apache: "Apache", mariadb: "MariaDB", redis: "Redis", mailpit: "Mailpit" });
  const table: Record<string, unknown> = {
    getSettings: record("getSettings", () => settings),
    saveSettings: record("saveSettings", (s: Settings) => (settings = s)),
    readText: record("readText", () => JSON.stringify({ lamprabitProfiles: 1, ...file })),
    listServices: record("listServices", () => (["apache", "mariadb", "redis", "mailpit"] as ServiceKind[]).map((kind) => ({ kind, installed: installed.includes(kind), external: false }))),
    starterPlan: record("starterPlan", (ids: string[]) =>
      ids.map((runtime) =>
        runtime === "mailpit"
          ? { runtime, label: "Mailpit", blurb: "", action: { kind: "unavailable", reason: "Not built for this computer." }, size: null }
          : { runtime, label: original.serviceLabels[runtime as ServiceKind], blurb: "", action: { kind: "install", release: { runtime, version: "8.2.1" } }, size: 5_000_000 },
      ),
    ),
  };
  return { ...original, api: new Proxy({}, { get: (_, name: string) => table[name] ?? record(name) }) };
});

const { ProfilesTransfer } = await import("./ProfilesDialog");

function show() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={client}>
      <ProfilesTransfer />
    </QueryClientProvider>,
  );
}

/** Clicks Import once settings have loaded (it's disabled until then). */
async function clickImport() {
  const button = await screen.findByRole("button", { name: /Import/ });
  await waitFor(() => expect(button).toHaveProperty("disabled", false));
  await userEvent.click(button);
}

const saved = () => (calls.filter((c) => c.cmd === "saveSettings").slice(-1)[0]?.args[0] as Settings | undefined)?.profiles;

beforeEach(() => {
  calls.length = 0;
  installed = ["apache", "mariadb"];
  settings = { profiles: [{ name: "Old", services: ["apache"], processes: [] }], sites: {} } as unknown as Settings;
});
afterEach(cleanup);

describe("importing profiles", () => {
  it("saves straight away when every service is installed", async () => {
    file = { profiles: [{ name: "Laravel", services: ["apache", "mariadb"], processes: [] }] };
    show();
    await clickImport();
    await waitFor(() => expect(saved()).toEqual([settings.profiles[0], { name: "Laravel", services: ["apache", "mariadb"], processes: [] }].slice(-2)));
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("takes a missing service out of the profile when asked", async () => {
    file = { profiles: [{ name: "Laravel", services: ["apache", "mariadb", "redis"], processes: [] }] };
    show();
    await clickImport();
    expect(await screen.findByText("A service isn't installed")).toBeTruthy();
    expect(screen.getByText("In Laravel")).toBeTruthy();
    expect(await screen.findByText(/Downloads 8.2.1/)).toBeTruthy();
    await userEvent.click(screen.getByRole("radio", { name: "Remove" }));
    await userEvent.click(within(screen.getByRole("dialog")).getByRole("button", { name: "Import" }));
    await waitFor(() => expect(saved()?.find((p) => p.name === "Laravel")?.services).toEqual(["apache", "mariadb"]));
    expect(calls.some((c) => c.cmd === "installRuntime")).toBe(false);
  });

  it("saves the profile, then installs the missing service", async () => {
    file = { profiles: [{ name: "Laravel", services: ["apache", "redis"], processes: [] }] };
    show();
    await clickImport();
    await userEvent.click(await screen.findByRole("button", { name: "Install 1 and import" }));
    await waitFor(() => expect(saved()?.find((p) => p.name === "Laravel")?.services).toEqual(["apache", "redis"]));
    await waitFor(() => expect(calls.find((c) => c.cmd === "installRuntime")?.args[0]).toEqual({ runtime: "redis", version: "8.2.1" }));
    expect(await screen.findByText("Installed")).toBeTruthy();
  });

  it("can't install what isn't offered here, and drops a profile left empty", async () => {
    file = {
      profiles: [
        { name: "Mail", services: ["mailpit"], processes: [] },
        { name: "Cache", services: ["apache", "redis"], processes: [] },
      ],
    };
    show();
    await clickImport();
    expect(await screen.findByText("Not built for this computer.")).toBeTruthy();
    expect(screen.getAllByRole("radio", { name: "Install" })[0]).toHaveProperty("disabled", true);
    expect(screen.getByText(/so not imported: Mail/)).toBeTruthy();
    await userEvent.click(screen.getByRole("button", { name: "Install 1 and import" }));
    await waitFor(() => expect(saved()?.map((p) => p.name)).toEqual(["Old", "Cache"]));
  });
});
