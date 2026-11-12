import { describe, expect, it } from "vitest";
import { parseLog } from "./logparse";
import type { LogService, LogStream } from "./api";

const parse = (line: string, service = "mysql", stream = "output") => parseLog({ id: 1, service: service as LogService, stream: stream as LogStream, line });
const iso = (line: string) => parse(line).time.toISOString();

describe("parseLog: timestamps every service writes", () => {
  // Each format with the instant it names; lines without a zone are local time.
  const local = (s: string) => new Date(s).toISOString();
  const cases: [string, string, string, string][] = [
    ["MySQL 8 (UTC)", "2026-11-01T12:17:12.188184Z 0 [System] [MY-015015] [Server] MySQL Server - start.", "2026-11-01T12:17:12.000Z", "info"],
    ["MySQL 8 (offset)", "2026-11-01T10:00:00.5+05:30 1 [ERROR] [MY-012574] [InnoDB] Unable to lock", "2026-11-01T04:30:00.000Z", "error"],
    ["MySQL warning", "2026-11-01T04:39:12Z 0 [Warning] [MY-010068] [Server] CA certificate is self signed.", "2026-11-01T04:39:12.000Z", "warn"],
    ["MariaDB", "2026-10-24 17:43:11 0 [Note] InnoDB: ok", local("2026-10-24T17:43:11"), "info"],
    ["PostgreSQL", "2026-11-01 10:00:00.123 IST [1234] LOG:  database system is ready", local("2026-11-01T10:00:00"), "info"],
    ["PostgreSQL fatal", "2026-11-01 10:00:00.123 IST [1234] FATAL:  role \"x\" does not exist", local("2026-11-01T10:00:00"), "error"],
    ["MongoDB JSON", '{"t":{"$date":"2026-11-01T10:00:00.123+05:30"},"s":"W","c":"CONTROL","msg":"Access control is not enabled"}', "2026-11-01T04:30:00.123Z", "warn"],
    ["Caddy JSON", '{"level":"error","ts":1793500000.5,"logger":"http","msg":"listen failed"}', new Date(1793500000500).toISOString(), "error"],
    ["Redis", "1234:M 01 Nov 2026 10:00:00.123 # Warning: no config file specified", local("2026-11-01T10:00:00"), "warn"],
    ["Elasticsearch", "[2026-11-01T10:00:00,123][WARN ][o.e.n.Node] [node] pre-release", local("2026-11-01T10:00:00"), "warn"],
    ["Ollama slog", 'time=2026-11-01T10:00:00.000+05:30 level=INFO source=routes.go:1 msg="Listening on 127.0.0.1:11434"', "2026-11-01T04:30:00.000Z", "info"],
    ["Go log", "2026/11/01 10:00:00 [error] something broke", local("2026-11-01T10:00:00"), "error"],
    ["Tomcat", "01-Nov-2026 10:00:00.123 SEVERE [main] Failed to start", local("2026-11-01T10:00:00"), "error"],
    ["Typesense", "E20261101 10:00:00.123456 12345 main.cpp:12] Could not bind", local("2026-11-01T10:00:00"), "error"],
    ["RabbitMQ", "2026-11-01 10:00:00.123456+05:30 [info] <0.1.0> Starting RabbitMQ", "2026-11-01T04:30:00.000Z", "info"],
    ["Nginx", "2026/10/24 19:00:00 [error] 1234#5678: *1 connect() failed", local("2026-10-24T19:00:00"), "error"],
    ["Apache", "[Sat Oct 24 17:55:59.123456 2026] [core:warn] [pid 1:tid 2] AH00098: pid file overwritten", local("2026-10-24T17:55:59"), "warn"],
    ["PHP", "[24-Oct-2026 12:00:00 UTC] PHP Warning:  Undefined variable $x", "2026-10-24T12:00:00.000Z", "warn"],
    ["LampRabit", "[2026-10-24 18:11:14] error: Apache failed", local("2026-10-24T18:11:14"), "error"],
  ];
  for (const [name, line, time, level] of cases) {
    it(name, () => {
      const e = parse(line);
      expect(e.timed).toBe(true);
      expect(e.time.toISOString()).toBe(time);
      expect(e.level).toBe(level);
    });
  }

  it("keeps a quoted slog message whole, without the closing quote", () => {
    expect(parse('time=2026-11-01T10:00:00Z level=INFO msg="a \\"quoted\\" word"').message).toBe('a \\"quoted\\" word');
  });

  it("drops what the columns already show from MySQL and MariaDB messages", () => {
    expect(parse("2026-11-01T04:39:12Z 0 [Warning] [MY-010068] [Server] CA certificate is self signed.").message).toBe("Server: CA certificate is self signed.");
    expect(parse("2026-10-24 17:43:11 0 [Note] InnoDB: ok").message).toBe("InnoDB: ok");
  });

  it("marks lines without a time of their own", () => {
    const e = parse("   at Illuminate\\Foundation\\Application->boot()");
    expect(e.timed).toBe(false);
  });

  it("reads access log status codes as levels", () => {
    const line = '127.0.0.1 - - [24/Oct/2026:17:56:01 +0530] "GET / HTTP/1.1" 500 1234';
    expect(parse(line, "apache", "access").level).toBe("error");
    expect(iso('127.0.0.1 - - [24/Oct/2026:17:56:01 +0000] "GET / HTTP/1.1" 200 1')).toBeTruthy();
  });

  it("doesn't call good news an error", () => {
    expect(parse("Build finished: 0 errors, 0 warnings").level).toBe("info");
  });
});
