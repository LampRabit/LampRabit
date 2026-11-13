import { useRef, useState, type ReactNode } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { AlertTriangle, Box, Check, FileCode, Plus, Trash2 } from "lucide-react";
import clsx from "clsx";
import { api } from "../lib/api";
import { useAppStore } from "../lib/store";
import {
  blankForm,
  canServeDynamic,
  commandPresets,
  langs,
  presetsFor,
  problems,
  servePresets,
  slug,
  toYaml,
  type CustomStep,
  type Engine,
  type Form,
  type Lang,
  type StepKind,
  type Variable,
} from "../lib/templateYaml";
import Dialog from "./Dialog";
import { icons, tiles } from "./NewProject";
import { Button, Segmented, Switch, inputClass } from "./ui";

/**
 * Making a template without writing YAML by hand: a guided form for every part of the
 * format (languages, where the code comes from, setup steps per language and custom ones,
 * the database, how it's served, the wizard's questions, processes, subdomains, commands),
 * and the YAML itself to adjust anything else. "Save as template" on a project opens
 * straight on its generated YAML. Saved into the templates folder after the same checks
 * loading a template does.
 */
export default function TemplateEditor() {
  const queryClient = useQueryClient();
  const notify = useAppStore((s) => s.notify);
  const editor = useAppStore((s) => s.templateEditor);
  const close = useAppStore((s) => s.closeTemplateEditor);
  const setNewProjectOpen = useAppStore((s) => s.setNewProjectOpen);
  if (!editor) return null;
  return (
    <Editor
      initialYaml={editor.yaml}
      onClose={(saved) => {
        close();
        if (saved) queryClient.invalidateQueries({ queryKey: ["templates"] });
        // Back to the wizard it was opened from, with the new template in its list.
        if (editor.fromWizard) setNewProjectOpen(true);
        if (saved) notify("info", `Saved the template ${saved}. It's in New project's list.`);
      }}
    />
  );
}

const sections = [
  { id: "basics", label: "Basics" },
  { id: "languages", label: "Languages" },
  { id: "code", label: "Where the code comes from" },
  { id: "setup", label: "Setup steps" },
  { id: "database", label: "Database" },
  { id: "serving", label: "Serving" },
  { id: "questions", label: "Wizard questions" },
  { id: "running", label: "Processes and subdomains" },
  { id: "commands", label: "Commands" },
  { id: "finish", label: "When it's ready" },
] as const;

const engineLabels: Record<Engine, string> = { mariadb: "MariaDB", mysql: "MySQL", postgres: "PostgreSQL", sqlite: "SQLite" };
const stepKinds: { value: StepKind; label: string; a: string; b?: string; aHint: string; bHint?: string }[] = [
  { value: "run", label: "Shell command", a: "Command", aHint: "npm run build, {{venv.python}} manage.py collectstatic…" },
  { value: "php", label: "PHP", a: "Arguments", aHint: "artisan storage:link" },
  { value: "composer", label: "Composer", a: "Arguments", aHint: "require laravel/breeze --dev" },
  { value: "write", label: "Write a file", a: "Path", b: "Content", aHint: "config/local.php", bHint: "Placeholders like {{url}} and {{database}} work" },
  { value: "copy", label: "Copy a file", a: "From", b: "To", aHint: ".env.example", bHint: ".env" },
  { value: "env", label: "Set .env values", a: "File", b: "KEY=value, one per line", aHint: ".env", bHint: "APP_URL={{url}}" },
];

function Editor({ initialYaml, onClose }: { initialYaml?: string; onClose: (saved?: string) => void }) {
  const [form, setForm] = useState<Form>(blankForm);
  const [tab, setTab] = useState<"guided" | "yaml">(initialYaml ? "yaml" : "guided");
  const [yaml, setYaml] = useState(initialYaml ?? toYaml(blankForm));
  /** The YAML was changed by hand: the guided form would overwrite it. */
  const [yamlEdited, setYamlEdited] = useState(!!initialYaml);
  const [taken, setTaken] = useState(false);
  const [section, setSection] = useState<string>("basics");
  const body = useRef<HTMLDivElement>(null);
  const update = (patch: Partial<Form>) => {
    const next = { ...form, ...patch };
    setForm(next);
    setYaml(toYaml(next));
    setYamlEdited(false);
    setTaken(false);
  };
  const save = useMutation({
    mutationFn: (replace: boolean) => api.saveTemplate(yaml, replace),
    onSuccess: (id) => onClose(id),
    onError: (e) => setTaken(String(e).includes("already have a template")),
  });
  // The form's own checks; the YAML tab is checked by the backend when saving.
  const missing = tab === "guided" && !yamlEdited ? problems(form) : [];
  const go = (id: string) => {
    setSection(id);
    body.current?.querySelector(`[data-section="${id}"]`)?.scrollIntoView({ block: "start", behavior: "smooth" });
  };
  const toggleLang = (l: Lang) => {
    const on = form.langs.includes(l);
    const next = on ? form.langs.filter((x) => x !== l) : [...form.langs, l];
    // A language's usual first steps come on with it.
    const added = on ? [] : presetsFor({ ...form, langs: [l] }).filter((p) => p.lang === l && ["composer-install", "node-install", "python-venv", "go-download", "cargo-build"].includes(p.id)).map((p) => p.id);
    update({ langs: next, presets: [...new Set([...form.presets, ...added])], ...(next.some((x) => x !== "php") ? {} : { serve: form.serve === "dynamic" ? "files" : form.serve }) });
  };

  return (
    <Dialog
      open
      onClose={() => !save.isPending && onClose()}
      title={initialYaml ? "Save as a template" : "New template"}
      icon={<FileCode size={18} />}
      width="max-w-5xl"
      footer={
        <>
          {save.error ? (
            <span className="mr-auto max-w-md truncate text-xs text-danger-text" title={String(save.error)}>
              {String(save.error)}
            </span>
          ) : missing.length > 0 ? (
            <span className="mr-auto flex max-w-md items-center gap-1.5 truncate text-xs text-warn" title={missing.join("\n")}>
              <AlertTriangle size={13} className="shrink-0" /> {missing[0]}
            </span>
          ) : (
            <span className="mr-auto text-xs text-muted">Saved to your templates folder, then listed in New project.</span>
          )}
          <Button onClick={() => onClose()} disabled={save.isPending}>
            Cancel
          </Button>
          {taken && (
            <Button variant="danger" onClick={() => save.mutate(true)} disabled={save.isPending}>
              Replace it
            </Button>
          )}
          <Button variant="primary" onClick={() => save.mutate(false)} disabled={save.isPending || missing.length > 0}>
            {save.isPending ? "Saving…" : "Save template"}
          </Button>
        </>
      }
    >
      <div ref={body} className="space-y-4">
        <div className="flex flex-wrap items-center gap-3">
          <Segmented
            value={tab}
            onChange={setTab}
            options={[
              { value: "guided", label: "Guided" },
              { value: "yaml", label: "YAML" },
            ]}
            className="w-56"
          />
          {yamlEdited && tab === "guided" && <p className="rounded-lg bg-warn/10 px-3 py-1.5 text-xs text-warn">You changed the YAML by hand; changing anything here replaces those edits.</p>}
        </div>

        {tab === "yaml" ? (
          <div className="space-y-2">
            <p className="text-xs text-muted">The template as it's saved. Everything the format offers is described in docs/templates.md (variables, conditions, more step kinds).</p>
            <textarea
              className={clsx(inputClass, "h-[28rem] font-mono text-xs leading-relaxed")}
              value={yaml}
              spellCheck={false}
              onChange={(e) => {
                setYaml(e.target.value);
                setYamlEdited(true);
                setTaken(false);
              }}
              aria-label="Template YAML"
            />
          </div>
        ) : (
          <div className="grid gap-6 md:grid-cols-[11rem_minmax(0,1fr)]">
            <nav aria-label="Template sections" className="hidden self-start md:sticky md:top-0 md:block">
              <ul className="space-y-0.5">
                {sections.map((s) => (
                  <li key={s.id}>
                    <button
                      type="button"
                      onClick={() => go(s.id)}
                      className={clsx("w-full rounded-lg px-2.5 py-1.5 text-left text-sm", section === s.id ? "bg-accent-soft font-medium text-accent-text" : "text-muted hover:bg-surface-2 hover:text-fg")}
                    >
                      {s.label}
                    </button>
                  </li>
                ))}
              </ul>
            </nav>
            <div className="min-w-0 space-y-8 text-sm">
              <Basics form={form} update={update} />
              <Languages form={form} update={update} toggle={toggleLang} />
              <Code form={form} update={update} />
              <Setup form={form} update={update} />
              <Database form={form} update={update} />
              <Serving form={form} update={update} />
              <Questions form={form} update={update} />
              <Running form={form} update={update} />
              <Commands form={form} update={update} />
              <Section id="finish" title="When it's ready" hint="Shown when the project is made. Placeholders such as {{url}} work.">
                <textarea className={clsx(inputClass, "h-20")} value={form.done} placeholder="Start it from the Sites page; it's served at {{url}}." onChange={(e) => update({ done: e.target.value })} aria-label="Message when ready" />
                <label className="mt-3 flex items-center justify-between gap-4 rounded-xl bg-surface-2/60 px-4 py-3">
                  <span>
                    <span className="block font-medium">Offer to initialise Git</span>
                    <span className="block text-xs text-muted">A checkbox in the wizard, off by default: git init and a first commit.</span>
                  </span>
                  <Switch checked={form.gitInit} onChange={(gitInit) => update({ gitInit })} label="Offer to initialise Git" />
                </label>
              </Section>
            </div>
          </div>
        )}
      </div>
    </Dialog>
  );
}

type Part = { form: Form; update: (patch: Partial<Form>) => void };
const label = "mb-1 block text-xs font-semibold text-muted";
const narrow = (extra: string) => clsx(inputClass.replace("w-full", ""), extra);

function Section({ id, title, hint, children }: { id: string; title: string; hint?: string; children: ReactNode }) {
  return (
    <section data-section={id} aria-label={title} className="scroll-mt-2">
      <h3 className="text-base font-bold tracking-tight">{title}</h3>
      {hint && <p className="mt-0.5 text-xs text-muted">{hint}</p>}
      <div className="mt-3">{children}</div>
    </section>
  );
}

/** Rows that can be added and removed, each with its own fields. */
function Rows<T>({ items, render, onChange, blank, addLabel, empty }: { items: T[]; render: (item: T, set: (patch: Partial<T>) => void) => ReactNode; onChange: (items: T[]) => void; blank: T; addLabel: string; empty?: string }) {
  return (
    <div className="space-y-2">
      {items.length === 0 && empty && <p className="text-xs text-muted">{empty}</p>}
      {items.map((item, i) => (
        <div key={i} className="flex items-start gap-2 rounded-xl border border-line bg-surface p-3">
          <div className="min-w-0 flex-1">{render(item, (patch) => onChange(items.map((x, j) => (j === i ? { ...x, ...patch } : x))))}</div>
          <button type="button" onClick={() => onChange(items.filter((_, j) => j !== i))} className="rounded p-1 text-muted hover:text-danger-text" aria-label="Remove">
            <Trash2 size={14} />
          </button>
        </div>
      ))}
      <Button size="sm" variant="ghost" onClick={() => onChange([...items, blank])}>
        <Plus size={13} /> {addLabel}
      </Button>
    </div>
  );
}

function Basics({ form, update }: Part) {
  const Icon = icons[form.icon] ?? Box;
  return (
    <Section id="basics" title="Basics" hint="How it shows in New project's list.">
      <div className="grid gap-3 sm:grid-cols-2">
        <label className="block">
          <span className={label}>Name</span>
          <input className={inputClass} value={form.name} placeholder="Shop" onChange={(e) => update({ name: e.target.value })} data-autofocus />
        </label>
        <label className="block">
          <span className={label}>Id (its file name)</span>
          <input className={clsx(inputClass, "font-mono")} value={form.idTouched ? form.id : slug(form.name)} placeholder="shop" onChange={(e) => update({ id: slug(e.target.value), idTouched: true })} />
        </label>
        <label className="block sm:col-span-2">
          <span className={label}>Description</span>
          <input className={inputClass} value={form.description} placeholder="Our shop, ready to work on" onChange={(e) => update({ description: e.target.value })} />
        </label>
        <label className="block">
          <span className={label}>How long it takes (a hint)</span>
          <input className={inputClass} value={form.duration} placeholder="Composer · about a minute" onChange={(e) => update({ duration: e.target.value })} />
        </label>
        <div className="flex items-end gap-3">
          <span className={clsx("flex size-10 shrink-0 items-center justify-center rounded-lg", tiles[form.color] ?? "bg-surface-2 text-fg")} aria-hidden>
            <Icon size={20} />
          </span>
          <div className="min-w-0 flex-1 space-y-1.5">
            <div className="flex flex-wrap gap-1" role="radiogroup" aria-label="Icon">
              {Object.entries(icons).map(([id, I]) => (
                <button key={id} type="button" role="radio" aria-checked={form.icon === id} aria-label={id} onClick={() => update({ icon: id })} className={clsx("flex size-7 items-center justify-center rounded-md", form.icon === id ? "bg-accent-soft text-accent-text" : "text-muted hover:bg-surface-2")}>
                  <I size={14} />
                </button>
              ))}
            </div>
            <div className="flex flex-wrap gap-1" role="radiogroup" aria-label="Colour">
              {Object.entries(tiles).map(([id, cls]) => (
                <button key={id} type="button" role="radio" aria-checked={form.color === id} aria-label={id} onClick={() => update({ color: id })} className={clsx("size-5 rounded-full ring-offset-1 ring-offset-surface", cls, form.color === id && "ring-2 ring-accent")} />
              ))}
            </div>
          </div>
        </div>
      </div>
    </Section>
  );
}

function Languages({ form, update, toggle }: Part & { toggle: (l: Lang) => void }) {
  return (
    <Section id="languages" title="Languages" hint="Everything the project runs on. Pick several for mixed projects: Laravel with Vite is PHP and Node.js, a Tauri app is Rust and Node.js.">
      <div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-3">
        {langs.map((l) => {
          const on = form.langs.includes(l.id);
          return (
            <div key={l.id} className={clsx("rounded-xl border p-3 transition-colors", on ? "border-accent bg-accent-soft/30" : "border-line")}>
              <button type="button" aria-pressed={on} onClick={() => toggle(l.id)} className="flex w-full items-center gap-2 text-left">
                <span className={clsx("flex size-5 shrink-0 items-center justify-center rounded border", on ? "border-accent bg-accent text-accent-fg" : "border-line")} aria-hidden>
                  {on && <Check size={12} strokeWidth={3} />}
                </span>
                <span className="font-semibold">{l.label}</span>
              </button>
              <p className="mt-1 pl-7 text-xs text-muted">{l.hint}</p>
              {on && l.id !== "node" && (
                <label className="mt-2 flex items-center gap-2 pl-7 text-xs text-muted">
                  At least
                  <input
                    className={narrow("h-7 w-20 font-mono text-xs")}
                    value={form.versions[l.id] ?? ""}
                    placeholder={l.id === "php" ? "any" : l.placeholder}
                    onChange={(e) => update({ versions: { ...form.versions, [l.id]: e.target.value } })}
                    aria-label={`Lowest ${l.label} version`}
                  />
                </label>
              )}
              {on && l.id === "php" && (
                <label className="mt-2 block pl-7">
                  <input className={narrow("h-7 w-full font-mono text-xs")} value={form.extensions} placeholder="Extensions: intl, gd" onChange={(e) => update({ extensions: e.target.value })} aria-label="PHP extensions it needs" />
                </label>
              )}
            </div>
          );
        })}
      </div>
    </Section>
  );
}

function Code({ form, update }: Part) {
  const offered = commandPresets.filter((p) => p.langs.some((l) => form.langs.includes(l)));
  return (
    <Section id="code" title="Where the code comes from">
      <Segmented
        value={form.source}
        onChange={(source) => update({ source })}
        options={[
          { value: "git", label: "Git" },
          ...(form.langs.includes("php") ? [{ value: "composer" as const, label: "Composer" }] : []),
          { value: "command", label: "A command" },
          { value: "download", label: "A download" },
          { value: "empty", label: "Empty" },
        ]}
      />
      <div className="mt-3 space-y-3">
        {form.source === "git" && (
          <div className="grid gap-3 sm:grid-cols-[minmax(0,1fr)_10rem]">
            <label className="block">
              <span className={label}>Repository</span>
              <input className={clsx(inputClass, "font-mono")} value={form.gitUrl} placeholder="https://github.com/you/shop.git" onChange={(e) => update({ gitUrl: e.target.value })} />
            </label>
            <label className="block">
              <span className={label}>Branch (optional)</span>
              <input className={clsx(inputClass, "font-mono")} value={form.gitBranch} placeholder="main" onChange={(e) => update({ gitBranch: e.target.value })} />
            </label>
          </div>
        )}
        {form.source === "composer" && (
          <label className="block">
            <span className={label}>Package for composer create-project</span>
            <input className={clsx(inputClass, "font-mono")} value={form.composerPackage} placeholder="laravel/laravel" onChange={(e) => update({ composerPackage: e.target.value })} />
          </label>
        )}
        {form.source === "command" && (
          <>
            {offered.length > 0 && (
              <div className="flex flex-wrap items-center gap-1.5">
                <span className="text-xs text-muted">Starters:</span>
                {offered.map((p) => (
                  <button
                    key={p.label}
                    type="button"
                    onClick={() => update({ command: p.command, commandInWww: p.inWww, commandNode: p.node, ...(p.serve && canServeDynamic(form) ? { serve: "dynamic", serveCommand: p.serve } : {}) })}
                    className="rounded-full border border-line px-2.5 py-1 text-xs text-muted hover:border-accent hover:text-fg"
                  >
                    {p.label}
                  </button>
                ))}
              </div>
            )}
            <label className="block">
              <span className={label}>Command</span>
              <input className={clsx(inputClass, "font-mono text-xs")} value={form.command} placeholder='{{pm.create}} vite@latest "{{project}}"' onChange={(e) => update({ command: e.target.value })} />
            </label>
            <div className="flex flex-wrap gap-x-6 gap-y-2">
              <label className="flex items-center gap-2 text-xs">
                <Switch checked={form.commandInWww} onChange={(commandInWww) => update({ commandInWww })} label="Runs in www" />
                Runs in www and makes the {"{{project}}"} folder (create-* tools)
              </label>
              <label className="flex items-center gap-2 text-xs">
                <Switch checked={form.commandNode} onChange={(commandNode) => update({ commandNode })} label="Needs Node.js" />
                Needs Node.js (npm, npx)
              </label>
            </div>
          </>
        )}
        {form.source === "download" && (
          <div className="grid gap-3 sm:grid-cols-2">
            <label className="block sm:col-span-2">
              <span className={label}>Archive (.zip, .tar.gz)</span>
              <input className={clsx(inputClass, "font-mono")} value={form.downloadUrl} placeholder="https://wordpress.org/latest.zip" onChange={(e) => update({ downloadUrl: e.target.value })} />
            </label>
            <label className="block">
              <span className={label}>SHA-256 (recommended)</span>
              <input className={clsx(inputClass, "font-mono text-xs")} value={form.downloadSha256} onChange={(e) => update({ downloadSha256: e.target.value })} />
            </label>
            <label className="block">
              <span className={label}>Folder inside it to take</span>
              <input className={clsx(inputClass, "font-mono")} value={form.downloadStrip} placeholder="wordpress" onChange={(e) => update({ downloadStrip: e.target.value })} />
            </label>
            {!form.downloadSha256.trim() && <p className="text-xs text-warn sm:col-span-2">Without a checksum, people making a project from this template are asked to approve the download.</p>}
          </div>
        )}
        {form.source === "empty" && <p className="text-xs text-muted">The project starts with a README; add files with setup steps below.</p>}
      </div>
    </Section>
  );
}

function Setup({ form, update }: Part) {
  const presets = presetsFor(form);
  const blankStep: CustomStep = { name: "", kind: "run", a: "", b: "", node: false, when: "always", optional: false };
  return (
    <Section id="setup" title="Setup steps" hint="Run in order after the code is there. The suggestions follow the languages picked above.">
      <div className="divide-y divide-line rounded-xl border border-line">
        {presets.map((p) => {
          const on = form.presets.includes(p.id);
          return (
            <label key={p.id} className="flex cursor-pointer items-start gap-3 px-4 py-2.5">
              <input type="checkbox" className="mt-0.5 size-4 accent-(--accent)" checked={on} onChange={() => update({ presets: on ? form.presets.filter((x) => x !== p.id) : [...form.presets, p.id] })} />
              <span>
                {p.label}
                {p.hint && <span className="block text-xs text-muted">{p.hint}</span>}
              </span>
              <span className="ml-auto shrink-0 rounded bg-surface-2 px-1.5 py-0.5 text-[10px] font-medium text-muted uppercase">{p.lang === "any" ? "all" : langs.find((l) => l.id === p.lang)?.label}</span>
            </label>
          );
        })}
      </div>
      <h4 className="mt-5 mb-2 text-xs font-semibold text-muted">Your own steps</h4>
      <Rows
        items={form.steps}
        blank={blankStep}
        addLabel="Add a step"
        empty="Commands, files to write or copy, .env values: anything the suggestions don't cover."
        onChange={(steps) => update({ steps })}
        render={(s, set) => {
          const kind = stepKinds.find((k) => k.value === s.kind)!;
          return (
            <div className="space-y-2">
              <div className="flex flex-wrap gap-2">
                <select className={narrow("w-40")} value={s.kind} onChange={(e) => set({ kind: e.target.value as StepKind })} aria-label="Kind of step">
                  {stepKinds.map((k) => (
                    <option key={k.value} value={k.value}>
                      {k.label}
                    </option>
                  ))}
                </select>
                <input className={narrow("min-w-0 flex-1")} value={s.name} placeholder="Heading shown while it runs" onChange={(e) => set({ name: e.target.value })} aria-label="Step heading" />
              </div>
              <input className={clsx(inputClass, "font-mono text-xs")} value={s.a} placeholder={kind.aHint} onChange={(e) => set({ a: e.target.value })} aria-label={kind.a} />
              {kind.b &&
                (s.kind === "copy" ? (
                  <input className={clsx(inputClass, "font-mono text-xs")} value={s.b} placeholder={kind.bHint} onChange={(e) => set({ b: e.target.value })} aria-label={kind.b} />
                ) : (
                  <textarea className={clsx(inputClass, "h-20 font-mono text-xs")} value={s.b} placeholder={kind.bHint} onChange={(e) => set({ b: e.target.value })} aria-label={kind.b} />
                ))}
              <div className="flex flex-wrap items-center gap-x-5 gap-y-2 text-xs text-muted">
                <label className="flex items-center gap-2">
                  Runs
                  <select className={narrow("h-7 w-44 text-xs")} value={s.when} onChange={(e) => set({ when: e.target.value as CustomStep["when"] })} aria-label="When it runs">
                    <option value="always">always</option>
                    <option value="database">with a database</option>
                    <option value="nodatabase">without a database</option>
                  </select>
                </label>
                <label className="flex items-center gap-2">
                  <input type="checkbox" className="size-4 accent-(--accent)" checked={s.optional} onChange={(e) => set({ optional: e.target.checked })} /> A checkbox in the wizard
                </label>
                {s.kind === "run" && (
                  <label className="flex items-center gap-2">
                    <input type="checkbox" className="size-4 accent-(--accent)" checked={s.node} onChange={(e) => set({ node: e.target.checked })} /> Needs Node.js
                  </label>
                )}
              </div>
            </div>
          );
        }}
      />
    </Section>
  );
}

function Database({ form, update }: Part) {
  return (
    <Section id="database" title="Database">
      <div className="grid gap-3 sm:grid-cols-2">
        <label className="block">
          <span className={label}>A database for each project</span>
          <select className={inputClass} value={form.database} onChange={(e) => update({ database: e.target.value as Form["database"] })}>
            <option value="none">None</option>
            <option value="optional">Offered, on by default</option>
            <option value="required">Always</option>
          </select>
        </label>
        {form.database !== "none" && (
          <div>
            <span className={label}>Servers it works with</span>
            <div className="flex flex-wrap gap-1.5">
              {(Object.keys(engineLabels) as Engine[]).map((e) => {
                const on = form.engines.includes(e);
                return (
                  <button
                    key={e}
                    type="button"
                    aria-pressed={on}
                    onClick={() => update({ engines: on ? form.engines.filter((x) => x !== e) : [...form.engines, e] })}
                    className={clsx("flex items-center gap-1 rounded-lg border px-2.5 py-1 text-xs font-medium", on ? "border-accent bg-accent-soft text-accent-text" : "border-line text-muted hover:text-fg")}
                  >
                    {on && <Check size={12} />} {engineLabels[e]}
                  </button>
                );
              })}
            </div>
          </div>
        )}
        {form.database !== "none" && (
          <>
            <label className="block">
              <span className={label}>Write the connection into</span>
              <select className={inputClass} value={form.dbEnv} onChange={(e) => update({ dbEnv: e.target.value as Form["dbEnv"] })}>
                <option value="none">Nothing (steps do it)</option>
                <option value="laravel">DB_HOST, DB_PORT… (Laravel, CodeIgniter)</option>
                <option value="url">DATABASE_URL (Symfony, Django, Prisma)</option>
              </select>
            </label>
            {form.dbEnv !== "none" && (
              <label className="block">
                <span className={label}>File</span>
                <input className={clsx(inputClass, "font-mono")} value={form.envFile} placeholder=".env" onChange={(e) => update({ envFile: e.target.value })} />
              </label>
            )}
          </>
        )}
      </div>
    </Section>
  );
}

function Serving({ form, update }: Part) {
  const dynamic = canServeDynamic(form);
  const presets = servePresets.filter((p) => form.langs.includes(p.lang));
  return (
    <Section id="serving" title="Serving" hint="What the project's .test domain shows.">
      <Segmented
        value={form.serve}
        onChange={(serve) => update({ serve })}
        options={[
          { value: "files", label: "The files (PHP)" },
          { value: "static", label: "A static build" },
          ...(dynamic ? [{ value: "dynamic" as const, label: "A dev server" }] : []),
        ]}
      />
      <p className="mt-2 text-xs text-muted">
        {form.serve === "files"
          ? "The web server serves the project's files, with PHP for .php."
          : form.serve === "static"
            ? "The web server serves the build folder (dist, build, out or public, whichever has an index.html)."
            : "The domain forwards to a server the project runs. {port} is the port LampRabit picks; $PORT is set too."}
        {!dynamic && " A dev server needs Node.js, Python, Java, Go or Rust."}
      </p>
      {form.serve === "dynamic" && dynamic && (
        <div className="mt-3 space-y-2">
          {presets.length > 0 && (
            <div className="flex flex-wrap items-center gap-1.5">
              <span className="text-xs text-muted">Common:</span>
              {presets.map((p) => (
                <button key={p.label} type="button" onClick={() => update({ serveCommand: p.command })} className="rounded-full border border-line px-2.5 py-1 text-xs text-muted hover:border-accent hover:text-fg">
                  {p.label}
                </button>
              ))}
            </div>
          )}
          <input className={clsx(inputClass, "font-mono text-xs")} value={form.serveCommand} placeholder="{{pm.run}} dev {{pm.args}} --host 127.0.0.1 --port {port}" onChange={(e) => update({ serveCommand: e.target.value })} aria-label="Dev server command" />
        </div>
      )}
    </Section>
  );
}

function Questions({ form, update }: Part) {
  const blank: Variable = { id: "", label: "", type: "text", options: "", default: "", help: "", required: false };
  return (
    <Section id="questions" title="Wizard questions" hint="Extra fields in New project. Use the answer as {{vars.<id>}} in steps and commands.">
      <Rows
        items={form.variables}
        blank={blank}
        addLabel="Add a question"
        empty="None: the wizard asks only for the name, versions and database."
        onChange={(variables) => update({ variables })}
        render={(v, set) => (
          <div className="grid gap-2 sm:grid-cols-[8rem_minmax(0,1fr)_8rem]">
            <input className={clsx(inputClass, "font-mono text-xs")} value={v.id} placeholder="id" onChange={(e) => set({ id: e.target.value })} aria-label="Question id" />
            <input className={inputClass} value={v.label} placeholder="Label" onChange={(e) => set({ label: e.target.value })} aria-label="Question label" />
            <select className={inputClass} value={v.type} onChange={(e) => set({ type: e.target.value as Variable["type"] })} aria-label="Answer type">
              <option value="text">Text</option>
              <option value="choice">Choice</option>
              <option value="bool">Yes / no</option>
            </select>
            {v.type === "choice" && <input className={clsx(inputClass, "sm:col-span-3")} value={v.options} placeholder="Options, comma-separated: blade, react, vue" onChange={(e) => set({ options: e.target.value })} aria-label="Options" />}
            <input className={inputClass} value={v.default} placeholder={v.type === "bool" ? "true / false" : "Default"} onChange={(e) => set({ default: e.target.value })} aria-label="Default" />
            <input className={clsx(inputClass, "sm:col-span-2")} value={v.help} placeholder="Help under the field" onChange={(e) => set({ help: e.target.value })} aria-label="Help" />
            {v.type === "text" && (
              <label className="flex items-center gap-2 text-xs text-muted sm:col-span-3">
                <input type="checkbox" className="size-4 accent-(--accent)" checked={v.required} onChange={(e) => set({ required: e.target.checked })} /> Required
              </label>
            )}
          </div>
        )}
      />
    </Section>
  );
}

function Running({ form, update }: Part) {
  return (
    <Section id="running" title="Processes and subdomains" hint="Commands that keep running (queues, workers, a second server), and subdomains of the project's domain.">
      <Rows
        items={form.processes}
        blank={{ name: "", command: "", cwd: "", port: false, autostart: true }}
        addLabel="Add a process"
        onChange={(processes) => update({ processes })}
        render={(p, set) => (
          <div className="space-y-2">
            <div className="flex flex-wrap gap-2">
              <input className={narrow("w-28 font-mono")} value={p.name} placeholder="queue" onChange={(e) => set({ name: e.target.value })} aria-label="Process name" />
              <input className={narrow("min-w-0 flex-1 font-mono text-xs")} value={p.command} placeholder="php artisan queue:work" onChange={(e) => set({ command: e.target.value })} aria-label="Command" />
            </div>
            <div className="flex flex-wrap items-center gap-x-5 gap-y-2 text-xs text-muted">
              <input className={narrow("h-7 w-32 font-mono text-xs")} value={p.cwd} placeholder="Folder (optional)" onChange={(e) => set({ cwd: e.target.value })} aria-label="Folder" />
              <label className="flex items-center gap-1.5" title="A free port, as {port} and $PORT">
                <Switch checked={p.port} onChange={(port) => set({ port })} label="Gets a port" /> Gets a port
              </label>
              <label className="flex items-center gap-1.5">
                <Switch checked={p.autostart} onChange={(autostart) => set({ autostart })} label="Starts with the project" /> Starts with the project
              </label>
            </div>
          </div>
        )}
      />
      <h4 className="mt-5 mb-2 text-xs font-semibold text-muted">Subdomains</h4>
      <Rows
        items={form.domains}
        blank={{ name: "", root: "" }}
        addLabel="Add a subdomain"
        onChange={(domains) => update({ domains })}
        render={(d, set) => (
          <div className="flex flex-wrap items-center gap-2">
            <input className={narrow("w-28 font-mono")} value={d.name} placeholder="admin" onChange={(e) => set({ name: e.target.value })} aria-label="Subdomain" />
            <span className="font-mono text-xs text-muted">.project.test →</span>
            <input className={narrow("min-w-0 flex-1 font-mono")} value={d.root} placeholder="admin/public (empty: like the main domain)" onChange={(e) => set({ root: e.target.value })} aria-label="Folder" />
          </div>
        )}
      />
    </Section>
  );
}

function Commands({ form, update }: Part) {
  return (
    <Section id="commands" title="Commands" hint="What the project's Commands tab offers, next to the ones found in package.json, composer.json, artisan and the like.">
      <Rows
        items={form.commands}
        blank={{ name: "", run: "", description: "" }}
        addLabel="Add a command"
        onChange={(commands) => update({ commands })}
        render={(c, set) => (
          <div className="grid gap-2 sm:grid-cols-[10rem_minmax(0,1fr)]">
            <input className={inputClass} value={c.name} placeholder="Run the tests" onChange={(e) => set({ name: e.target.value })} aria-label="Command name" />
            <input className={clsx(inputClass, "font-mono text-xs")} value={c.run} placeholder="php artisan test" onChange={(e) => set({ run: e.target.value })} aria-label="Command line" />
            <input className={clsx(inputClass, "sm:col-span-2")} value={c.description} placeholder="What it does (optional)" onChange={(e) => set({ description: e.target.value })} aria-label="Command description" />
          </div>
        )}
      />
      <label className="mt-3 flex items-center gap-2 text-xs text-muted">
        <Switch checked={form.discover} onChange={(discover) => update({ discover })} label="Also find commands in the project's files" /> Also find commands in the project's files
      </label>
    </Section>
  );
}
