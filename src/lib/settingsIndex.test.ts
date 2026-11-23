import { describe, expect, it } from "vitest";
import { parseSettings } from "./settingsIndex";
import source from "../pages/Settings.tsx?raw";

describe("the settings index", () => {
  it("places rows in their section, with their hints, and skips rows it can't place", () => {
    const rows = parseSettings(`
      <Section id="general" title="General">
        <Row label="Editor command" hint='Used by "Open in editor"'>
        <Row label="Site domain suffix" hint={\`Folders in www are served as <name>.\${draft.tld}\`}>
        <ToggleRow label="HTTPS by default" hint="Sites use HTTPS" checked={x} />
        <Row label="No hint here">
      </Section>
      <Section id="ports" title="Ports">
        <Row
          label="SMTP port"
          hint="Where apps send mail.">
      </Section>
      function Helper() { return <Row label="Somewhere" hint="?" /> }
    `);
    expect(rows).toEqual([
      { section: "general", label: "Editor command", hint: 'Used by "Open in editor"' },
      { section: "general", label: "Site domain suffix", hint: "Folders in www are served as <name>." },
      { section: "general", label: "HTTPS by default", hint: "Sites use HTTPS" },
      { section: "general", label: "No hint here", hint: "" },
      { section: "ports", label: "SMTP port", hint: "Where apps send mail." },
    ]);
  });

  it("finds the real Settings page's rows", () => {
    const rows = parseSettings(source);
    const find = (label: string) => rows.find((r) => r.label === label);
    expect(find("Editor command")?.section).toBe("general");
    expect(find("SMTP port")?.section).toBe("email");
    expect(find("Windows Security scanning")?.section).toBe("help");
    expect(rows.length).toBeGreaterThan(20);
  });
});
