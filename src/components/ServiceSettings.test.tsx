// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ServiceOption, Settings } from "../lib/api";

/**
 * The options a service's manifest declares show in its Settings card, by type, and
 * Apply sends each changed one to the backend (which checks it) and restarts the service
 * when it's running.
 */

const calls: { cmd: string; args: unknown[] }[] = [];
let running = true;
const settings = { ports: { redis: 6379 }, storage: {}, webServers: ["apache"], phpWorkers: 4, routeMail: true, defaultDatabase: "mariadb", serviceOptions: {} } as unknown as Settings;
const options: ServiceOption[] = [
  { name: "appendonly", label: "Write every change to disk", description: "Append-only persistence.", kind: "toggle", options: [], min: null, max: null, unit: null, default: "false", value: "false" },
  { name: "maxmemory", label: "Memory limit", description: null, kind: "number", options: [], min: 0, max: 4096, unit: "MB", default: "0", value: "0" },
  { name: "policy", label: "When full", description: null, kind: "choice", options: ["noeviction", "allkeys-lru"], min: null, max: null, unit: null, default: "noeviction", value: "noeviction" },
];
const record =
  (cmd: string, result: unknown = undefined) =>
  (...args: unknown[]) => {
    calls.push({ cmd, args });
    return Promise.resolve(typeof result === "function" ? (result as (...a: unknown[]) => unknown)(...args) : result);
  };

vi.mock("../lib/api", async (importOriginal) => {
  const original = await importOriginal<typeof import("../lib/api")>();
  Object.assign(original.runtimeLabels, { redis: "Redis", php: "PHP" });
  const table: Record<string, unknown> = {
    getSettings: record("getSettings", () => settings),
    envPaths: record("envPaths", () => ({ os: "windows" })),
    listServices: record("listServices", () => [{ kind: "redis", state: running ? "running" : "stopped", installed: true }]),
    serviceOptions: record("serviceOptions", () => options),
  };
  return { ...original, api: new Proxy({}, { get: (_, name: string) => table[name] ?? record(name) }) };
});

const { default: ServiceSettings } = await import("./ServiceSettings");

function show(runtime: "redis" | "php") {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={client}>
      <ServiceSettings runtime={runtime} />
    </QueryClientProvider>,
  );
}

afterEach(() => {
  cleanup();
  calls.length = 0;
  running = true;
});

describe("manifest options in a service's settings", () => {
  it("shows each option with the input its type needs", async () => {
    show("redis");
    expect(await screen.findByRole("switch", { name: "Write every change to disk" })).toBeTruthy();
    expect(screen.getByText("Append-only persistence.")).toBeTruthy();
    expect((screen.getByRole("spinbutton", { name: "Memory limit" }) as HTMLInputElement).value).toBe("0");
    expect(screen.getByText("MB")).toBeTruthy();
    expect(screen.getByRole("radio", { name: "allkeys-lru" })).toBeTruthy();
  });

  it("applies only changed options, then restarts the running service", async () => {
    show("redis");
    await userEvent.click(await screen.findByRole("switch", { name: "Write every change to disk" }));
    await userEvent.click(await screen.findByRole("button", { name: /Apply changes/ }));
    await vi.waitFor(() => expect(calls.some((c) => c.cmd === "restartService")).toBe(true));
    const set = calls.filter((c) => c.cmd === "setServiceOption");
    expect(set.map((c) => c.args)).toEqual([["redis", "appendonly", "true"]]);
    // The options are saved by the backend's check, not overwritten by the settings save.
    const saved = calls.find((c) => c.cmd === "saveSettings")?.args[0] as Settings;
    expect(saved.serviceOptions).toEqual({});
  });

  it("doesn't restart a stopped service", async () => {
    running = false;
    show("redis");
    await userEvent.click(await screen.findByRole("switch", { name: "Write every change to disk" }));
    await userEvent.click(await screen.findByRole("button", { name: /Apply changes/ }));
    await vi.waitFor(() => expect(calls.some((c) => c.cmd === "saveSettings")).toBe(true));
    expect(calls.some((c) => c.cmd === "restartService")).toBe(false);
  });

  it("asks nothing for runtimes that aren't services", async () => {
    show("php");
    await vi.waitFor(() => expect(calls.some((c) => c.cmd === "listServices")).toBe(true));
    expect(calls.some((c) => c.cmd === "serviceOptions")).toBe(false);
  });
});
