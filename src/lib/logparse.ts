import type { LogService, LogStream, RawLogLine } from "./api";

export type Level = "error" | "warn" | "notice" | "info" | "access" | "debug";

export interface LogEntry {
  id: number;
  service: LogService;
  stream: LogStream;
  /** For service "project". */
  project?: string;
  time: Date;
  /** False when the line carried no timestamp and `time` is when it was read. */
  timed: boolean;
  level: Level;
  message: string;
}

const apacheLevels: Record<string, Level> = {
  emerg: "error",
  alert: "error",
  crit: "error",
  error: "error",
  warn: "warn",
  notice: "notice",
  info: "info",
};

function validDate(d: Date | null): Date | null {
  return d && !Number.isNaN(d.getTime()) ? d : null;
}

function levelFromText(text: string): Level {
  // Summaries like "0 warnings and 0 errors" or "0 failed" are good news.
  text = text.replace(/(no|0) (errors?|warnings?|failed|failures?)/gi, "");
  if (/\b(fatal|error|failed|crit)/i.test(text)) return "error";
  if (/\bwarn/i.test(text)) return "warn";
  if (/\b(notice|deprecated)/i.test(text)) return "notice";
  return "info";
}

/** Extracts time, level and message from the formats Apache, MariaDB and PHP write. */
export function parseLog(raw: RawLogLine, received = new Date()): LogEntry {
  const base = { id: raw.id, service: raw.service, stream: raw.stream, project: raw.project, timed: true };
  const line = raw.line;
  let m: RegExpMatchArray | null;

  // A project's output: [2026-10-31 12:00:00] output|error|command: text
  if ((m = line.match(/^\[(\d{4}-\d{2}-\d{2}) (\d{2}:\d{2}:\d{2})\] (output|error|command): (.*)$/))) {
    const message = m[4];
    return {
      ...base,
      stream: m[3] as LogStream,
      time: validDate(new Date(`${m[1]}T${m[2]}`)) ?? received,
      level: m[3] === "command" ? "notice" : levelFromText(message),
      message,
    };
  }

  // LampRabit system log: [2026-10-24 18:11:14] info: Apache started
  if ((m = line.match(/^\[(\d{4}-\d{2}-\d{2}) (\d{2}:\d{2}:\d{2})\] (info|warn|error): (.*)$/))) {
    return {
      ...base,
      time: validDate(new Date(`${m[1]}T${m[2]}`)) ?? received,
      level: m[3] as Level,
      message: m[4],
    };
  }

  // Apache access: 127.0.0.1 - - [24/Oct/2026:17:56:01 +0530] "GET / HTTP/1.1" 200 1234
  if (raw.stream === "access" && (m = line.match(/\[(\d{2})\/(\w{3})\/(\d{4}):(\d{2}:\d{2}:\d{2}) ([+-]\d{4})\]/))) {
    const status = Number(line.match(/" (\d{3}) /)?.[1]);
    return {
      ...base,
      time: validDate(new Date(`${m[1]} ${m[2]} ${m[3]} ${m[4]} ${m[5]}`)) ?? received,
      level: status >= 500 ? "error" : status >= 400 ? "warn" : "access",
      message: line,
    };
  }

  // Apache error: [Sat Oct 24 17:55:59.123456 2026] [core:warn] [pid 1:tid 2] AH00098: ...
  if ((m = line.match(/^\[\w{3} (\w{3}) +(\d{1,2}) (\d{2}:\d{2}:\d{2})(?:\.\d+)? (\d{4})\] \[[\w-]*:(\w+)\](?: \[pid [^\]]*\])?(?: \[client [^\]]*\])? ?(.*)$/))) {
    return {
      ...base,
      time: validDate(new Date(`${m[1]} ${m[2]} ${m[4]} ${m[3]}`)) ?? received,
      level: apacheLevels[m[5]] ?? (m[5].startsWith("trace") || m[5] === "debug" ? "debug" : "info"),
      message: m[6],
    };
  }

  // Nginx: 2026/10/24 19:00:00 [error] 1234#5678: *1 message
  if ((m = line.match(/^(\d{4})\/(\d{2})\/(\d{2}) (\d{2}:\d{2}:\d{2}) \[(\w+)\] \d+#\d+: (?:\*\d+ )?(.*)$/))) {
    const kind = m[5];
    return {
      ...base,
      time: validDate(new Date(`${m[1]}-${m[2]}-${m[3]}T${m[4]}`)) ?? received,
      level: ["emerg", "alert", "crit", "error"].includes(kind) ? "error" : kind === "warn" ? "warn" : kind === "notice" ? "notice" : "info",
      message: m[6],
    };
  }

  // MariaDB: 2026-10-24 17:43:11 0 [Note] InnoDB: ...
  if ((m = line.match(/^(\d{4}-\d{2}-\d{2}) +(\d{1,2}):(\d{2}:\d{2}) +\d+ \[(\w+)\] (.*)$/))) {
    const kind = m[4].toLowerCase();
    return {
      ...base,
      time: validDate(new Date(`${m[1]}T${m[2].padStart(2, "0")}:${m[3]}`)) ?? received,
      level: kind === "error" ? "error" : kind === "warning" ? "warn" : "info",
      message: `[${m[4]}] ${m[5]}`,
    };
  }

  // PHP: [24-Oct-2026 12:00:00 UTC] PHP Warning:  ...   /   php-fpm: [24-Oct-2026 12:00:00] NOTICE: ...
  if ((m = line.match(/^\[(\d{2})-(\w{3})-(\d{4}) (\d{2}:\d{2}:\d{2})(?: ([\w/+-]+))?\] (.*)$/))) {
    const zone = m[5] === "UTC" ? " GMT" : "";
    const message = m[6].replace(/^PHP +/, "");
    return {
      ...base,
      time: validDate(new Date(`${m[1]} ${m[2]} ${m[3]} ${m[4]}${zone}`)) ?? received,
      level: levelFromText(message.split(":")[0]),
      message,
    };
  }

  const other = parseOther(line);
  if (other) return { ...base, ...other };
  return { ...base, timed: false, time: received, level: levelFromText(line), message: line };
}

const months: Record<string, string> = { Jan: "01", Feb: "02", Mar: "03", Apr: "04", May: "05", Jun: "06", Jul: "07", Aug: "08", Sep: "09", Oct: "10", Nov: "11", Dec: "12" };

/** Level words as the other services write them (MySQL, PostgreSQL, Go and Rust loggers…). */
function levelWord(word: string, message: string): Level {
  const w = word.toLowerCase();
  if (/^(error|err|fatal|panic|crit|critical|emerg|alert|severe|e|f)$/.test(w)) return "error";
  if (/^(warn|warning|w)$/.test(w)) return "warn";
  if (/^(notice|n)$/.test(w)) return "notice";
  if (/^(debug|trace|d|fine|finer|finest)$/.test(w)) return "debug";
  // These name the level outright, so words like "error" in the text don't change it.
  if (/^(log|system|note|info|i|statement|detail|hint|context)$/.test(w)) return "info";
  return levelFromText(message);
}

/**
 * The formats of the services beyond Apache, Nginx, MariaDB and PHP. A line none of these
 * match keeps the time it was read, so its own date must be found here: MySQL 8's lines
 * all showed "now" when the Logs page loaded a log written days earlier.
 */
function parseOther(line: string): { time: Date; level: Level; message: string } | null {
  let m: RegExpMatchArray | null;

  // JSON lines: MongoDB {"t":{"$date":…},"s":"I","msg":…}, Caddy/FrankenPHP {"level","ts","msg"}.
  if (line.startsWith("{") && line.endsWith("}")) {
    try {
      const j = JSON.parse(line);
      const t = j.t?.$date ?? j.ts ?? j.time ?? j.timestamp;
      const time = validDate(typeof t === "number" ? new Date(t > 1e12 ? t : t * 1000) : typeof t === "string" ? new Date(t) : null);
      if (time) {
        const msg = String(j.msg ?? j.message ?? "");
        const attr = j.attr ?? j.error ?? j.err;
        const message = [j.logger ?? j.c, msg, attr && typeof attr === "object" ? JSON.stringify(attr) : attr].filter(Boolean).join(" · ");
        return { time, level: levelWord(String(j.s ?? j.level ?? ""), msg), message: message || line };
      }
    } catch {
      // Not JSON after all; the patterns below may still match.
    }
  }

  // ISO 8601 first, with a level word somewhere in the next few fields:
  // MySQL 8:       2026-11-01T04:39:12.345678Z 0 [Warning] [MY-010068] [Server] …
  // RabbitMQ:      2026-11-01 10:00:00.123456+05:30 [info] <0.1.0> …
  // Rust tracing:  2026-11-01T10:00:00.123Z  INFO meilisearch: …
  // PostgreSQL:    2026-11-01 10:00:00.123 IST [1234] LOG:  …
  if ((m = line.match(/^\[?(\d{4}-\d{2}-\d{2})[T ](\d{2}:\d{2}:\d{2})(?:[.,]\d+)?(Z|[+-]\d{2}:?\d{2})?\]? *(?:[A-Z]{2,5}(?= \[\d+\]) )?(.*)$/))) {
    const zone = m[3] ? (m[3] === "Z" ? "Z" : m[3].length === 5 ? `${m[3].slice(0, 3)}:${m[3].slice(3)}` : m[3]) : "";
    const time = validDate(new Date(`${m[1]}T${m[2]}${zone}`));
    if (time) {
      const rest = m[4];
      const word = rest.match(/^(?:\d+ |\[\d+\] )?\[?(\w+)\]?[: ]/)?.[1] ?? "";
      return { time, level: levelWord(word, rest), message: rest.replace(/^\d+ (?=\[)/, "") };
    }
  }

  // Elasticsearch / OpenSearch: [2026-11-01T10:00:00,123][INFO ][o.e.n.Node] …
  if ((m = line.match(/^\[(\d{4}-\d{2}-\d{2})T(\d{2}:\d{2}:\d{2})[,.]\d+\]\[(\w+) *\](.*)$/))) {
    const time = validDate(new Date(`${m[1]}T${m[2]}`));
    if (time) return { time, level: levelWord(m[3], m[4]), message: m[4].trim() };
  }

  // Redis: 1234:M 01 Nov 2026 10:00:00.123 * Ready to accept connections
  if ((m = line.match(/^\d+:[A-Z] (\d{2}) (\w{3}) (\d{4}) (\d{2}:\d{2}:\d{2})(?:\.\d+)? ([.\-*#]) (.*)$/)) && months[m[2]]) {
    const time = validDate(new Date(`${m[3]}-${months[m[2]]}-${m[1]}T${m[4]}`));
    if (time) return { time, level: m[5] === "#" ? "warn" : m[5] === "." ? "debug" : levelFromText(m[6]), message: m[6] };
  }

  // Go's slog/logrus text: time=2026-11-01T10:00:00.000+05:30 level=INFO msg="…" (Ollama, Mailpit, n8n)
  if ((m = line.match(/^time="?([^" ]+(?: [^" ]+)?)"? level=(\w+)(?: source=\S+)? msg=(?:"((?:[^"\\]|\\.)*)"|(\S+))(.*)$/))) {
    const time = validDate(new Date(m[1].replace(/\//g, "-").replace(" ", "T")));
    const msg = m[3] ?? m[4];
    if (time) return { time, level: levelWord(m[2], msg), message: `${msg}${m[5]}` };
  }

  // Go's log package: 2026/11/01 10:00:00 message (Mailpit, MinIO, cloudflared)
  if ((m = line.match(/^(\d{4})\/(\d{2})\/(\d{2}) (\d{2}:\d{2}:\d{2})(?:\.\d+)? (.*)$/))) {
    const time = validDate(new Date(`${m[1]}-${m[2]}-${m[3]}T${m[4]}`));
    if (time) {
      const word = m[5].match(/^\[?(\w+)\]?[: ]/)?.[1] ?? "";
      return { time, level: levelWord(word, m[5]), message: m[5] };
    }
  }

  // Tomcat/Java: 01-Nov-2026 10:00:00.123 INFO [main] …
  if ((m = line.match(/^(\d{2})-(\w{3})-(\d{4}) (\d{2}:\d{2}:\d{2})(?:\.\d+)? (\w+) (.*)$/)) && months[m[2]]) {
    const time = validDate(new Date(`${m[3]}-${months[m[2]]}-${m[1]}T${m[4]}`));
    if (time) return { time, level: levelWord(m[5], m[6]), message: m[6] };
  }

  // Typesense (glog): I20261101 10:00:00.123456 12345 file.cpp:12] message
  if ((m = line.match(/^([IWEF])(\d{4})(\d{2})(\d{2}) (\d{2}:\d{2}:\d{2})\.\d+ +\d+ [^\]]*\] (.*)$/))) {
    const time = validDate(new Date(`${m[2]}-${m[3]}-${m[4]}T${m[5]}`));
    if (time) return { time, level: levelWord(m[1], m[6]), message: m[6] };
  }
  return null;
}
