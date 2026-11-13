import { describe, expect, it } from "vitest";
import { blankForm, problems, toYaml, type Form } from "./templateYaml";

/**
 * The template editor's output for a spread of forms. Each is kept in __fixtures__/templates,
 * which the backend parses with its own checks (templates.rs, editor_samples_are_valid), so a
 * form that writes a template LampRabit would refuse fails one test or the other.
 * UPDATE_TEMPLATE_SAMPLES=1 npm test rewrites the files after a deliberate change.
 */

const form = (patch: Partial<Form>): Form => ({ ...blankForm, ...patch });

const samples: Record<string, Form> = {
  "laravel-git": form({
    name: "Shop",
    description: "Our shop, ready to work on",
    icon: "flame",
    color: "red",
    langs: ["php", "node"],
    versions: { php: "8.3" },
    extensions: "intl, gd",
    gitUrl: "https://github.com/acme/shop.git",
    gitBranch: "main",
    presets: ["composer-install", "copy-env", "laravel", "node-install", "node-build"],
    dbEnv: "laravel",
    processes: [{ name: "queue", command: "php artisan queue:work", cwd: "", port: false, autostart: false }],
    commands: [{ name: "Seed", run: "php artisan db:seed", description: "Fills the database" }],
  }),
  tauri: form({
    name: "Tauri app",
    langs: ["rust", "node"],
    versions: { rust: "1.80" },
    source: "command",
    command: '{{pm.create}} tauri-app@latest "{{project}}" {{pm.args}} --template react-ts --manager {{pm}} --yes',
    commandNode: true,
    presets: ["node-install", "cargo-build"],
    database: "none",
    serve: "dynamic",
    serveCommand: "{{pm.run}} dev {{pm.args}} --host 127.0.0.1 --port {port} --strictPort",
  }),
  django: form({
    name: "Django + Postgres",
    langs: ["python"],
    versions: { python: "3.11" },
    source: "command",
    commandInWww: false,
    command: "python -m venv .venv && {{venv.python}} -m pip install django && {{venv.python}} -m django startproject config .",
    presets: ["django-migrate"],
    database: "required",
    engines: ["postgres"],
    dbEnv: "url",
    serve: "dynamic",
    serveCommand: "{{venv.python}} manage.py runserver 127.0.0.1:{port}",
    variables: [
      { id: "admin", label: "Admin user", type: "text", options: "", default: "admin", help: "Made with createsuperuser", required: true },
      { id: "css", label: "CSS", type: "choice", options: "plain, tailwind", default: "", help: "", required: false },
      { id: "api", label: "REST API", type: "bool", options: "", default: "", help: "", required: false },
    ],
  }),
  "go-service": form({ name: "Go service", langs: ["go"], versions: {}, source: "command", commandInWww: false, command: "go mod init {{project}}", presets: ["go-download"], database: "none", serve: "dynamic", serveCommand: "go run .", gitInit: false }),
  "spring-git": form({ name: "Spring", langs: ["java"], versions: { java: "21" }, gitUrl: "https://github.com/acme/api.git", presets: ["maven-build"], database: "optional", engines: ["mysql", "postgres"], dbEnv: "url", envFile: ".env.local", serve: "dynamic", serveCommand: "{{mvnw}} spring-boot:run" }),
  "download-custom": form({
    name: "CMS from a zip",
    source: "download",
    downloadUrl: "https://example.com/cms.zip",
    downloadSha256: "ABCDEF0123456789abcdef0123456789abcdef0123456789abcdef0123456789",
    downloadStrip: "cms",
    presets: [],
    steps: [
      { name: "Writing the config", kind: "write", a: "config.php", b: "<?php\nreturn ['url' => '{{url}}'];\n", node: false, when: "always", optional: false },
      { name: "Settings", kind: "env", a: ".env", b: "APP_NAME={{project}}\n# a comment\nSECRET={{random:32}}", node: false, when: "always", optional: false },
      { name: "Installer", kind: "php", a: "install.php --quiet", b: "", node: false, when: "database", optional: true },
      { name: "", kind: "composer", a: "require predis/predis", b: "", node: false, when: "nodatabase", optional: false },
      { name: "Sample", kind: "copy", a: "sample.htaccess", b: ".htaccess", node: false, when: "always", optional: false },
    ],
    domains: [{ name: "admin", root: "admin" }],
    discover: false,
    done: "Open {{url}}/install to finish.",
  }),
  "static-site": form({ name: "Landing page", langs: ["node"], versions: {}, source: "empty", presets: [], database: "none", serve: "static" }),
};

// The app's tsconfig has no Node types; Vitest runs in Node, so fs is there at run time.
type Fs = { existsSync(p: URL): boolean; mkdirSync(p: URL, o: object): void; readFileSync(p: URL, e: string): string; writeFileSync(p: URL, s: string): void };
const fs: Fs = await import(/* @vite-ignore */ "node:fs" as string);
const update = !!(globalThis as { process?: { env: Record<string, string | undefined> } }).process?.env.UPDATE_TEMPLATE_SAMPLES;
const dir = new URL("./__fixtures__/templates/", import.meta.url);

describe("template editor output", () => {
  for (const [name, f] of Object.entries(samples)) {
    it(`writes ${name}`, () => {
      expect(problems(f)).toEqual([]);
      const yaml = toYaml(f);
      const file = new URL(`${name}.yaml`, dir);
      if (update || !fs.existsSync(file)) {
        fs.mkdirSync(dir, { recursive: true });
        fs.writeFileSync(file, yaml);
      }
      expect(yaml).toBe(fs.readFileSync(file, "utf8").replace(/\r\n/g, "\n"));
    });
  }

  it("says what's missing", () => {
    expect(problems(form({ name: "", gitUrl: "" }))).toEqual(["Give the template a name.", "Say where the code comes from, or pick An empty folder."]);
    expect(problems(form({ name: "x", source: "empty", serve: "dynamic", serveCommand: "php -S" }))).toContain("A dev server needs Node.js, Python, Java, Go or Rust: PHP projects are served by PHP.");
  });

  it("leaves out what doesn't apply to the languages", () => {
    // Composer and Laravel presets stay off for a Rust template even if ticked earlier.
    const yaml = toYaml(form({ name: "r", langs: ["rust"], source: "empty", presets: ["composer-install", "laravel", "cargo-build"], database: "none" }));
    expect(yaml).not.toContain("composer");
    expect(yaml).not.toContain("artisan");
    expect(yaml).toContain("run: cargo build");
    expect(yaml).toContain("rust: '1.80'");
  });
});
