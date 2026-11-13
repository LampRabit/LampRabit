/**
 * The template editor's guided form, and the template YAML it becomes (docs/templates.md
 * has the format). Kept apart from the component so the output can be checked: the
 * samples in __fixtures__ are generated from here and parsed by the backend's own checks
 * (templates.rs, `editor_samples_are_valid`).
 */

export type Lang = "php" | "node" | "python" | "java" | "go" | "rust";
export const langs: { id: Lang; label: string; placeholder: string; hint: string }[] = [
  { id: "php", label: "PHP", placeholder: "8.3", hint: "Composer, artisan and the project's PHP version" },
  { id: "node", label: "Node.js", placeholder: "", hint: "Asks for a Node.js version and package manager" },
  { id: "python", label: "Python", placeholder: "3.10", hint: "pip and a virtual environment (.venv)" },
  { id: "java", label: "Java", placeholder: "17", hint: "JAVA_HOME and the Maven wrapper" },
  { id: "go", label: "Go", placeholder: "1.22", hint: "go on PATH in steps and the dev server" },
  { id: "rust", label: "Rust", placeholder: "1.80", hint: "cargo and rustc with LampRabit's rustup" },
];

export type Source = "git" | "composer" | "command" | "download" | "empty";
export type StepKind = "run" | "php" | "composer" | "write" | "copy" | "env";
export type When = "always" | "database" | "nodatabase";
export type Engine = "mariadb" | "mysql" | "postgres" | "sqlite";

export interface CustomStep {
  name: string;
  kind: StepKind;
  /** run: the command; php/composer: the arguments; write: the path; copy: from; env: the file. */
  a: string;
  /** write: the content; copy: to; env: KEY=value lines. */
  b: string;
  /** Needs Node.js (npm, npx) on PATH. */
  node: boolean;
  when: When;
  /** A checkbox in the new-project wizard. */
  optional: boolean;
}

export interface Variable {
  id: string;
  label: string;
  type: "text" | "choice" | "bool";
  /** choice: comma-separated values. */
  options: string;
  default: string;
  help: string;
  required: boolean;
}

export interface Process {
  name: string;
  command: string;
  cwd: string;
  port: boolean;
  autostart: boolean;
}

export interface Form {
  name: string;
  /** Set by hand; otherwise follows the name. */
  id: string;
  idTouched: boolean;
  description: string;
  icon: string;
  color: string;
  duration: string;
  langs: Lang[];
  /** Lowest version per language ("" = any); Node.js has none. */
  versions: Partial<Record<Lang, string>>;
  extensions: string;

  source: Source;
  gitUrl: string;
  gitBranch: string;
  composerPackage: string;
  command: string;
  /** The command makes the {{project}} folder in www (create-* tools) or runs inside it. */
  commandInWww: boolean;
  commandNode: boolean;
  downloadUrl: string;
  downloadSha256: string;
  downloadStrip: string;

  /** Ids of the preset steps that are on (see presetSteps). */
  presets: string[];
  steps: CustomStep[];

  database: "none" | "optional" | "required";
  engines: Engine[];
  /** How the connection reaches the project's config. */
  dbEnv: "none" | "laravel" | "url";
  envFile: string;

  serve: "files" | "static" | "dynamic";
  serveCommand: string;

  variables: Variable[];
  processes: Process[];
  domains: { name: string; root: string }[];
  commands: { name: string; run: string; description: string }[];
  discover: boolean;
  gitInit: boolean;
  done: string;
}

export const blankForm: Form = {
  name: "",
  id: "",
  idTouched: false,
  description: "",
  icon: "box",
  color: "zinc",
  duration: "",
  langs: ["php"],
  versions: { php: "8.3" },
  extensions: "",
  source: "git",
  gitUrl: "",
  gitBranch: "",
  composerPackage: "",
  command: "",
  commandInWww: true,
  commandNode: false,
  downloadUrl: "",
  downloadSha256: "",
  downloadStrip: "",
  presets: ["composer-install", "copy-env"],
  steps: [],
  database: "optional",
  engines: ["mariadb", "mysql"],
  dbEnv: "none",
  envFile: ".env",
  serve: "files",
  serveCommand: "",
  variables: [],
  processes: [],
  domains: [],
  commands: [],
  discover: true,
  gitInit: true,
  done: "",
};

/**
 * Steps the form offers per language, in the order they run. `step` is the YAML (without
 * the leading "- "), its lines indented by four spaces under the list item.
 */
export const presetSteps: { id: string; lang: Lang | "any"; label: string; hint?: string; database?: boolean; yaml: string[] }[] = [
  { id: "composer-install", lang: "php", label: "composer install", yaml: ["name: Installing PHP packages", "composer: [install, --no-interaction]"] },
  { id: "copy-env", lang: "any", label: "Copy .env.example to .env", yaml: ["name: Creating .env", "copy: { from: .env.example, to: .env }"] },
  {
    id: "laravel",
    lang: "php",
    label: "Laravel: APP_URL and mail in .env, the app key, migrations",
    hint: "Migrations run only when a database was chosen.",
    yaml: ["name: Configuring the environment", "env:", "  values:", '    APP_URL: "{{url}}"', '    MAIL_HOST: "{{mail.host}}"', '    MAIL_PORT: "{{mail.port}}"'],
  },
  { id: "node-install", lang: "node", label: "Install the Node packages", yaml: ["name: Installing Node packages", "needs: [node]", 'run: "{{pm.install}}"'] },
  { id: "node-build", lang: "node", label: "Build the front end once", hint: "{{pm.run}} build", yaml: ["name: Building the front end", "needs: [node]", 'run: "{{pm.run}} build"'] },
  { id: "python-venv", lang: "python", label: "Create a virtual environment (.venv)", yaml: ["name: Creating a virtual environment (.venv)", "run: python -m venv .venv"] },
  {
    id: "python-requirements",
    lang: "python",
    label: "pip install -r requirements.txt",
    hint: "Into .venv when it's made above.",
    yaml: ["name: Installing Python packages", 'run: "{{venv.python}} -m pip install --disable-pip-version-check -r requirements.txt"'],
  },
  { id: "django-migrate", lang: "python", label: "Django: run the migrations", yaml: ["name: Running the migrations", 'run: "{{venv.python}} manage.py migrate"'] },
  { id: "maven-build", lang: "java", label: "Build with the Maven wrapper", hint: "{{mvnw}} package -DskipTests", yaml: ["name: Building with Maven", 'run: "{{mvnw}} -q package -DskipTests"'] },
  { id: "go-download", lang: "go", label: "go mod download", yaml: ["name: Downloading Go modules", "run: go mod download"] },
  { id: "cargo-build", lang: "rust", label: "cargo build once (later runs are quick)", yaml: ["name: Building it once", "run: cargo build"] },
];

/** Starter commands for the "a command creates it" source. */
export const commandPresets: { label: string; langs: Lang[]; command: string; inWww: boolean; node: boolean; serve?: string }[] = [
  { label: "Vite (React + TypeScript)", langs: ["node"], command: '{{pm.create}} vite@latest "{{project}}" {{pm.args}} --template react-ts', inWww: true, node: true, serve: "{{pm.run}} dev {{pm.args}} --host 127.0.0.1 --port {port} --strictPort" },
  { label: "Next.js", langs: ["node"], command: '{{pm.exec}} create-next-app@latest "{{project}}" --yes', inWww: true, node: true, serve: "{{pm.run}} dev {{pm.args}} --hostname 127.0.0.1 --port {port}" },
  {
    label: "Tauri (Rust + React)",
    langs: ["rust", "node"],
    command: '{{pm.create}} tauri-app@latest "{{project}}" {{pm.args}} --template react-ts --manager {{pm}} --yes',
    inWww: true,
    node: true,
    serve: "{{pm.run}} dev {{pm.args}} --host 127.0.0.1 --port {port} --strictPort",
  },
  { label: "Cargo (Rust binary)", langs: ["rust"], command: "cargo init --name {{project}} --vcs none", inWww: false, node: false, serve: "cargo run" },
  { label: "Go module", langs: ["go"], command: "go mod init {{project}}", inWww: false, node: false, serve: "go run ." },
  {
    label: "Django",
    langs: ["python"],
    command: "python -m venv .venv && {{venv.python}} -m pip install --disable-pip-version-check django && {{venv.python}} -m django startproject config .",
    inWww: false,
    node: false,
    serve: "{{venv.python}} manage.py runserver 127.0.0.1:{port}",
  },
];

/** Dev-server commands per language, for the dynamic serve mode. */
export const servePresets: { label: string; lang: Lang; command: string }[] = [
  { label: "Vite / most Node dev servers", lang: "node", command: "{{pm.run}} dev {{pm.args}} --host 127.0.0.1 --port {port} --strictPort" },
  { label: "npm start", lang: "node", command: "{{pm.run}} start" },
  { label: "Django", lang: "python", command: "{{venv.python}} manage.py runserver 127.0.0.1:{port}" },
  { label: "FastAPI (uvicorn)", lang: "python", command: "{{venv.python}} -m uvicorn main:app --host 127.0.0.1 --port {port} --reload" },
  { label: "Flask", lang: "python", command: "{{venv.python}} -m flask run --host 127.0.0.1 --port {port} --debug" },
  { label: "Spring Boot", lang: "java", command: "{{mvnw}} spring-boot:run" },
  { label: "go run .", lang: "go", command: "go run ." },
  { label: "cargo run", lang: "rust", command: "cargo run" },
];

export const slug = (text: string) =>
  text
    .toLowerCase()
    .replace(/[^a-z0-9_]+/g, "-")
    .replace(/^-+|-+$/g, "");
/** A YAML single-quoted string. */
const q = (text: string) => `'${text.replace(/'/g, "''")}'`;
const list = (text: string) =>
  text
    .split(",")
    .map((s) => s.trim())
    .filter(Boolean);
/** A dynamic server needs a language other than PHP to run it (templates.rs checks the same). */
export const canServeDynamic = (f: Form) => f.langs.some((l) => l !== "php");
const has = (f: Form, lang: Lang) => f.langs.includes(lang);
/** Preset steps that apply to the form's languages. */
export const presetsFor = (f: Form) => presetSteps.filter((p) => p.lang === "any" || has(f, p.lang));

function pushStep(lines: string[], body: string[]) {
  lines.push(`  - ${body[0]}`, ...body.slice(1).map((l) => `    ${l}`));
}

function customStep(s: CustomStep): string[] | null {
  const a = s.a.trim();
  const body: string[] = [];
  if (s.name.trim()) body.push(`name: ${q(s.name.trim())}`);
  if (s.optional) {
    body.push("optional: true", `label: ${q(s.name.trim() || a)}`);
  }
  if (s.when === "database") body.push("if: database");
  if (s.when === "nodatabase") body.push("if: '!database'");
  if (s.node) body.push("needs: [node]");
  switch (s.kind) {
    case "run":
      if (!a) return null;
      body.push(`run: ${q(a)}`);
      break;
    case "php":
    case "composer": {
      const args = a.split(/\s+/).filter(Boolean);
      if (!args.length) return null;
      body.push(`${s.kind}: [${args.map(q).join(", ")}]`);
      break;
    }
    case "write":
      if (!a) return null;
      body.push("write:", `  path: ${q(a)}`, "  content: |", ...(s.b || "\n").replace(/\n$/, "").split("\n").map((l) => `    ${l}`));
      break;
    case "copy":
      if (!a || !s.b.trim()) return null;
      body.push(`copy: { from: ${q(a)}, to: ${q(s.b.trim())} }`);
      break;
    case "env": {
      const values = s.b
        .split("\n")
        .map((l) => l.trim())
        .filter((l) => l.includes("=") && !l.startsWith("#"))
        .map((l) => [l.slice(0, l.indexOf("=")).trim(), l.slice(l.indexOf("=") + 1).trim()] as const)
        .filter(([k]) => k);
      if (!values.length) return null;
      body.push("env:");
      if (a && a !== ".env") body.push(`  file: ${q(a)}`);
      body.push("  values:", ...values.map(([k, v]) => `    ${k}: ${q(v)}`));
      break;
    }
  }
  // Without a name the step body must still start with its action for the "- " line.
  return body;
}

/** The form as template YAML. */
export function toYaml(f: Form): string {
  const id = (f.idTouched ? f.id : slug(f.name)) || "my-template";
  const lines = ["schema: 1", `id: ${id}`, `name: ${q(f.name.trim() || "My template")}`];
  if (f.description.trim()) lines.push(`description: ${q(f.description.trim())}`);
  if (f.icon && f.icon !== "box") lines.push(`icon: ${f.icon}`);
  if (f.color && f.color !== "zinc") lines.push(`color: ${f.color}`);
  if (f.duration.trim()) lines.push(`duration: ${q(f.duration.trim())}`);

  const requires: string[] = [];
  for (const l of f.langs) {
    if (l === "node") requires.push("  node: true");
    else requires.push(`  ${l}: ${q(f.versions[l]?.trim() || (l === "php" ? "any" : defaultVersion(l)))}`);
  }
  const extensions = list(f.extensions);
  if (has(f, "php") && extensions.length) requires.push(`  extensions: [${extensions.join(", ")}]`);
  if (requires.length) lines.push("requires:", ...requires);

  if (f.database !== "none") {
    lines.push(`database: ${f.database}`);
    const engines = f.engines.length ? f.engines : ["mariadb", "mysql"];
    lines.push(`engines: [${engines.join(", ")}]`);
  }

  const variables = f.variables.filter((v) => slug(v.id) && v.label.trim());
  if (variables.length) {
    lines.push("variables:");
    for (const v of variables) {
      lines.push(`  - id: ${slug(v.id).replace(/-/g, "_")}`, `    label: ${q(v.label.trim())}`);
      if (v.type !== "text") lines.push(`    type: ${v.type}`);
      const options = list(v.options);
      if (v.type === "choice" && options.length) lines.push(`    options: [${options.map(q).join(", ")}]`);
      const fallback = v.type === "bool" ? "false" : v.type === "choice" ? (options[0] ?? "") : "";
      const def = v.default.trim() || fallback;
      if (def) lines.push(`    default: ${q(def)}`);
      if (v.help.trim()) lines.push(`    help: ${q(v.help.trim())}`);
      if (v.required && v.type === "text") lines.push("    required: true");
    }
  }

  lines.push("steps:");
  // First: a database name that's taken then stops everything before any long download.
  if (f.database !== "none") lines.push("  - create_database: {}");
  switch (f.source) {
    case "git":
      if (f.gitUrl.trim()) {
        const branch = f.gitBranch.trim() ? `--branch ${f.gitBranch.trim()} ` : "";
        pushStep(lines, ["name: Getting the code", "cwd: www", "needs: [git]", `run: ${q(`git clone ${branch}${f.gitUrl.trim()} "{{project}}"`)}`]);
      }
      break;
    case "composer":
      if (f.composerPackage.trim()) pushStep(lines, ["name: Creating the project", `composer: [create-project, ${q(f.composerPackage.trim())}, '{{project}}', --no-interaction]`]);
      break;
    case "command":
      if (f.command.trim()) {
        pushStep(lines, ["name: Creating the project", ...(f.commandInWww ? ["cwd: www"] : []), ...(f.commandNode ? ["needs: [node]"] : []), `run: ${q(f.command.trim())}`]);
      }
      break;
    case "download":
      if (f.downloadUrl.trim()) {
        const d = ["name: Downloading the code", "download:", `  url: ${q(f.downloadUrl.trim())}`];
        if (f.downloadSha256.trim()) d.push(`  sha256: ${f.downloadSha256.trim().toLowerCase()}`);
        d.push("  extract: true");
        if (f.downloadStrip.trim()) d.push(`  strip: ${q(f.downloadStrip.trim())}`);
        pushStep(lines, d);
      }
      break;
    case "empty":
      break;
  }
  if (f.source === "empty" || !sourceGiven(f)) pushStep(lines, ["name: Writing a README", "write:", "  path: README.md", '  content: "# {{project}}\\n"']);

  const presets = presetsFor(f).filter((p) => f.presets.includes(p.id));
  for (const p of presets) pushStep(lines, p.yaml);
  // The connection goes into the config file after it exists (.env from .env.example above).
  if (f.database !== "none" && f.dbEnv !== "none") {
    const file = f.envFile.trim() && f.envFile.trim() !== ".env" ? [`  file: ${q(f.envFile.trim())}`] : [];
    const values =
      f.dbEnv === "laravel"
        ? ['    DB_CONNECTION: "{{db.engine}}"', '    DB_HOST: "{{db.host}}"', '    DB_PORT: "{{db.port}}"', '    DB_DATABASE: "{{database}}"', '    DB_USERNAME: "{{db.user}}"', '    DB_PASSWORD: "{{db.password}}"']
        : ['    DATABASE_URL: "{{db.scheme}}://{{db.user}}:{{db.password}}@{{db.host}}:{{db.port}}/{{database}}?charset={{db.charset}}"'];
    pushStep(lines, ["name: Connecting the database", "if: database", "env:", ...file, "  values:", ...values]);
  }
  if (presets.some((p) => p.id === "laravel")) {
    pushStep(lines, ["name: Generating the app key", "php: [artisan, key:generate]"]);
    pushStep(lines, ["name: Running database migrations", "if: database", "php: [artisan, migrate, --force]"]);
  }
  for (const s of f.steps) {
    const body = customStep(s);
    if (body) pushStep(lines, body);
  }
  if (f.gitInit) pushStep(lines, ["id: git", "optional: true", "default: false", "label: Initialise a Git repository", "needs: [git]", "git_init:", `  commit: ${q(`New ${f.name.trim() || "project"}`)}`]);

  if (f.serve === "static") lines.push("serve:", "  mode: static");
  if (f.serve === "dynamic" && canServeDynamic(f) && f.serveCommand.trim()) lines.push("serve:", "  mode: dynamic", `  command: ${q(f.serveCommand.trim())}`);

  const commands = f.commands.filter((c) => c.name.trim() && c.run.trim());
  if (commands.length || !f.discover) {
    lines.push("commands:");
    if (!f.discover) lines.push("  discover: []");
    if (commands.length) {
      lines.push("  list:");
      for (const c of commands) {
        lines.push(`    - name: ${q(c.name.trim())}`, `      run: ${q(c.run.trim())}`);
        if (c.description.trim()) lines.push(`      description: ${q(c.description.trim())}`);
      }
    }
  }

  const processes = f.processes.filter((p) => slug(p.name) && p.command.trim());
  if (processes.length) {
    lines.push("processes:");
    for (const p of processes) {
      lines.push(`  - name: ${slug(p.name)}`, `    command: ${q(p.command.trim())}`);
      if (p.cwd.trim()) lines.push(`    cwd: ${q(p.cwd.trim())}`);
      if (p.port) lines.push("    port: auto");
      if (!p.autostart) lines.push("    autostart: false");
    }
  }
  const domains = f.domains.filter((d) => slug(d.name));
  if (domains.length) {
    lines.push("domains:");
    for (const d of domains) {
      lines.push(`  - name: ${slug(d.name)}`);
      if (d.root.trim()) lines.push(`    root: ${q(d.root.trim())}`);
    }
  }
  if (f.done.trim()) lines.push(`done: ${q(f.done.trim())}`);
  return lines.join("\n") + "\n";
}

function sourceGiven(f: Form) {
  return (
    (f.source === "git" && !!f.gitUrl.trim()) ||
    (f.source === "composer" && !!f.composerPackage.trim()) ||
    (f.source === "command" && !!f.command.trim()) ||
    (f.source === "download" && !!f.downloadUrl.trim())
  );
}

function defaultVersion(l: Lang) {
  return langs.find((x) => x.id === l)?.placeholder || "any";
}

/** What's missing before the template can be saved, in the form's words. */
export function problems(f: Form): string[] {
  const out: string[] = [];
  if (!f.name.trim()) out.push("Give the template a name.");
  if (f.langs.length === 0) out.push("Pick at least one language, or PHP for plain files.");
  if (f.source !== "empty" && !sourceGiven(f)) out.push("Say where the code comes from, or pick An empty folder.");
  if (f.serve === "dynamic" && !canServeDynamic(f)) out.push("A dev server needs Node.js, Python, Java, Go or Rust: PHP projects are served by PHP.");
  if (f.serve === "dynamic" && canServeDynamic(f) && !f.serveCommand.trim()) out.push("Give the dev server's command.");
  if (f.database !== "none" && f.engines.length === 0) out.push("Pick at least one database server.");
  return out;
}
