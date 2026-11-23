// @vitest-environment jsdom
import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import type { PortStatus, ServiceInfo } from "../lib/api";
import { NeedsAttention, conflictFor } from "./HomeStatus";

afterEach(cleanup);

const redis = { kind: "redis", port: 6379, state: "stopped" } as ServiceInfo;
const held: PortStatus = { key: "redis" as PortStatus["key"], port: 6379, free: false, usedByService: null, owner: { pid: 1, name: "redis-server", note: null, stoppable: true } };

describe("Home's status", () => {
  it("counts a port as a conflict only when another program holds it before the service starts", () => {
    expect(conflictFor(redis, [held])).toBe(held);
    expect(conflictFor({ ...redis, state: "running" }, [held])).toBeNull();
    expect(conflictFor(redis, [{ ...held, usedByService: "redis" }])).toBeNull();
    expect(conflictFor(redis, [{ ...held, free: true }])).toBeNull();
  });

  it("shows nothing when all is well, and the most serious issue first", () => {
    const { container } = render(<NeedsAttention issues={[]} />);
    expect(container.innerHTML).toBe("");
    render(
      <NeedsAttention
        issues={[
          { id: "a", tone: "warn", title: "Port 6379 is in use", actions: null },
          { id: "b", tone: "danger", title: "MySQL isn't answering", actions: null },
        ]}
      />,
    );
    const titles = screen.getAllByRole("listitem").map((li) => li.textContent);
    expect(titles).toEqual(["MySQL isn't answering", "Port 6379 is in use"]);
  });
});
