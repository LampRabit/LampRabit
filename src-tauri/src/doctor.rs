//! The project doctor: what stands between a project and working, each problem with why it
//! matters and, where LampRabit can, a fix that's previewed before it runs. Checks read
//! what's already known (the project's files, its database's state, the services, the
//! hosts file and certificate state) plus `php -m` for the extensions. Each language gets its
//! own runtime checks: PHP against composer.json, Node against .nvmrc and `engines.node`,
//! Python, Java and Go against their version files, and a dev server that should be running.
//! Nothing is changed here; lib.rs applies a fix by its issue id, after recomputing the issues.

use std::{path::Path, process::Stdio, time::Duration};

use serde::Serialize;
use serde_json::Value;

use crate::{layout::{exe, Layout}, manage, procgroup};

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// The project won't work until it's fixed.
    Error,
    /// It works, but something will go wrong or look wrong.
    Warning,
    /// Worth knowing.
    Info,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    /// Stable for a project's state, so a fix can be asked for by id.
    pub id: String,
    pub severity: Severity,
    pub title: String,
    /// Why it matters, in plain words.
    pub detail: String,
    pub fix: Option<Fix>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Fix {
    /// The button: "Use PHP 8.3".
    pub label: String,
    /// What it will do, shown before it runs: "Sets shop's PHP to 8.3 and restarts the web server".
    pub preview: String,
    pub action: FixAction,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum FixAction {
    SetPhp { version: String },
    EnableExtension { php: String, name: String },
    RunCommand { label: String, command: String },
    CopyFile { from: String, to: String },
    StartService { service: String },
    CreateDatabase,
    SwitchDatabase { engine: manage::Engine },
    SetEnv { key: String, value: String },
    ApplySites,
    TrustCa,
    /// Installs a Node version with NVM.
    InstallNode { version: String },
    /// Runs the project's processes on this installed Node version.
    SetNode { version: String },
    /// Starts the project's dev server.
    StartApp,
}

/// A Node project's runtime.
pub struct NodeInput {
    /// Installed Node versions, newest first.
    pub installed: Vec<String>,
    pub nvm: bool,
    /// The version asked for, and where: (spec, "its .nvmrc" / "its settings").
    pub wanted: Option<(String, &'static str)>,
    /// The version it runs on, when that resolves.
    pub version: Option<String>,
    /// package.json's `engines.node`.
    pub engines: Option<String>,
}

/// The runtime of a Python, Java or Go project.
pub struct RuntimeInput {
    pub label: &'static str,
    pub installed: bool,
    /// What its version file asks for, and that file.
    pub request: Option<(String, &'static str)>,
    /// Whether an installed version matches the request.
    pub matched: bool,
    /// The version it runs on instead.
    pub version: Option<String>,
}

/// A project served by its own dev server.
pub struct AppInput {
    /// stopped, starting, running, crashed.
    pub state: &'static str,
    pub exit_code: Option<i32>,
    /// Something listens for the domain anyway (a dev server started in a terminal).
    pub routed: bool,
}

/// What lib.rs knows about a project, for the checks.
pub struct Input<'a> {
    pub name: &'a str,
    pub dir: &'a Path,
    pub layout: &'a Layout,
    /// "php", "node"…
    pub language: &'a str,
    pub framework: Option<&'a str>,
    pub url: &'a str,
    pub ssl: bool,
    /// The PHP version the project runs on, and every installed one.
    pub php: Option<&'a str>,
    pub php_installed: Vec<String>,
    pub php_requirement: Option<&'a str>,
    /// (manager, installed, outdated) from the project's facts.
    pub dependencies: Vec<(String, bool, bool)>,
    pub package_manager: &'a str,
    pub database: Option<manage::DbStatus>,
    /// The web server that serves it: its id, label, whether it's installed and running.
    pub server: (&'a str, &'a str, bool, bool),
    pub hosts_ok: bool,
    pub needs_apply: bool,
    pub ca_trusted: Option<bool>,
    /// Some when it's a Git repository: whether Git knows a name and email to commit with.
    pub git_identity: Option<bool>,
    /// Set for Node projects.
    pub node: Option<NodeInput>,
    /// Set for Python, Java and Go projects.
    pub runtime: Option<RuntimeInput>,
    /// Set when the domain is served by the project's own dev server.
    pub app: Option<AppInput>,
}

pub async fn check(input: &Input<'_>) -> Vec<Issue> {
    let mut issues = Vec::new();
    let mut add = |id: &str, severity, title: String, detail: String, fix: Option<Fix>| issues.push(Issue { id: id.to_string(), severity, title, detail, fix });

    // The web server: nothing opens while it's stopped.
    let (server_id, server_label, server_installed, server_running) = input.server;
    if !server_installed {
        add("server-missing", Severity::Error, format!("{server_label} isn't installed"), format!("{} is served by {server_label}. Install it on the Services page.", input.name), None);
    } else if !server_running {
        add(
            "server-stopped",
            Severity::Warning,
            format!("{server_label} is stopped"),
            format!("{} doesn't open until {server_label} runs.", input.url),
            Some(Fix { label: format!("Start {server_label}"), preview: format!("Starts {server_label}."), action: FixAction::StartService { service: server_id.to_string() } }),
        );
    }

    if input.language == "php" {
        php_checks(input, &mut add).await;
    }
    if let Some(node) = &input.node {
        node_checks(node, &mut add);
    }
    if let Some(rt) = &input.runtime {
        runtime_checks(rt, &mut add);
    }
    if let Some(app) = &input.app {
        app_checks(input, app, &mut add);
    }

    // Dependencies: vendor/ or node_modules missing, or older than the lock file.
    for (manager, installed, outdated) in &input.dependencies {
        let command = install_command(manager, input.package_manager);
        let folder = if manager == "composer" { "vendor" } else { "node_modules" };
        if !installed {
            add(
                &format!("deps-missing-{manager}"),
                Severity::Error,
                format!("{} dependencies aren't installed", title_case(manager)),
                format!("There's no {folder}/ folder, so the project can't load its packages."),
                Some(Fix { label: format!("Run {command}"), preview: format!("Runs {command} in the project (its output shows under Commands)."), action: FixAction::RunCommand { label: command.clone(), command } }),
            );
        } else if *outdated {
            add(
                &format!("deps-outdated-{manager}"),
                Severity::Warning,
                format!("{} dependencies are out of date", title_case(manager)),
                format!("The lock file changed since {folder}/ was installed (a pull or a branch switch), so the code may expect packages that aren't there."),
                Some(Fix { label: format!("Run {command}"), preview: format!("Runs {command} in the project (its output shows under Commands)."), action: FixAction::RunCommand { label: command.clone(), command } }),
            );
        }
    }

    // .env: frameworks read their settings from it; the example is the template.
    let has_env = input.dir.join(".env").is_file();
    if !has_env && input.dir.join(".env.example").is_file() {
        add(
            "env-missing",
            Severity::Error,
            ".env is missing".into(),
            "The project reads its settings (database, app URL, keys) from .env, and there's only .env.example.".into(),
            Some(Fix { label: "Create .env from .env.example".into(), preview: "Copies .env.example to .env; nothing is overwritten.".into(), action: FixAction::CopyFile { from: ".env.example".into(), to: ".env".into() } }),
        );
    }

    // The database the config names.
    if let Some(db) = &input.database {
        if !db.installed {
            let fix = db.alternative.as_ref().map(|alt| Fix {
                label: format!("Use {}", alt.label),
                preview: format!("Points the project at {} (port {}) in its config.", alt.label, alt.port),
                action: FixAction::SwitchDatabase { engine: alt.engine },
            });
            add("db-missing", Severity::Error, format!("{} isn't installed", db.label), db.reason.clone().unwrap_or_else(|| format!("The project's database lives on {}.", db.label)), fix);
        } else if !db.running {
            add(
                "db-stopped",
                Severity::Error,
                format!("{} is stopped", db.label),
                format!("The project can't reach its database {} until {} runs.", db.database, db.label),
                db.engine.service().map(|s| Fix { label: format!("Start {}", db.label), preview: format!("Starts {}.", db.label), action: FixAction::StartService { service: s.id().to_string() } }),
            );
        } else if db.exists == Some(false) {
            add(
                "db-not-created",
                Severity::Error,
                format!("The database {} doesn't exist yet", db.database),
                format!("{} is running, but has no database called {}.", db.label, db.database),
                Some(Fix { label: "Create it".into(), preview: format!("Creates the empty database {} in {} (utf8mb4 for MySQL-family servers).", db.database, db.label), action: FixAction::CreateDatabase }),
            );
        }
    }

    // Laravel (Statamic is a Laravel app); framework names are as sites.rs shows them.
    let laravel = input.framework.is_some_and(|f| f.eq_ignore_ascii_case("laravel") || f.eq_ignore_ascii_case("statamic"));
    if laravel {
        laravel_checks(input.dir, input.package_manager, has_env, &mut add);
    }
    // Laravel builds links and redirects from APP_URL.
    if has_env && laravel {
        let env = std::fs::read_to_string(input.dir.join(".env")).unwrap_or_default();
        let app_url = env.lines().find_map(|l| l.strip_prefix("APP_URL=")).map(|v| v.trim().trim_matches('"').trim_end_matches('/').to_string());
        let want = input.url.trim_end_matches('/');
        if let Some(have) = app_url.filter(|h| !h.is_empty() && h != want) {
            add(
                "app-url",
                Severity::Warning,
                "APP_URL doesn't match the site".into(),
                format!("APP_URL is {have}, but the site is {want}: links, redirects and assets may point at the wrong place."),
                Some(Fix { label: format!("Set APP_URL to {want}"), preview: format!("Changes APP_URL in .env to {want}."), action: FixAction::SetEnv { key: "APP_URL".into(), value: want.to_string() } }),
            );
        }
    }

    // The domain: the hosts file and the web server's config follow the projects.
    if !input.hosts_ok {
        add(
            "hosts",
            Severity::Error,
            "The hosts file doesn't list this site".into(),
            format!("{} won't resolve to this computer until the hosts file is updated (your system asks for approval).", input.url),
            Some(Fix { label: "Update the hosts file".into(), preview: "Writes LampRabit's block in the hosts file; your system asks for administrator approval.".into(), action: FixAction::ApplySites }),
        );
    } else if input.needs_apply {
        add(
            "config-stale",
            Severity::Warning,
            "The web server's config is out of date".into(),
            "Projects or domains changed since the web server started; it serves the old list until it reloads.".into(),
            Some(Fix { label: "Apply".into(), preview: "Rewrites the web server's sites and reloads it.".into(), action: FixAction::ApplySites }),
        );
    }
    if input.ssl && input.ca_trusted == Some(false) {
        add(
            "ca-untrusted",
            Severity::Warning,
            "Browsers will warn about this site's certificate".into(),
            "LampRabit's local certificate authority isn't trusted on this computer yet, so https:// shows a security warning.".into(),
            Some(Fix { label: "Trust the certificate".into(), preview: "Adds LampRabit's local certificate authority to your trust store; your system asks to confirm.".into(), action: FixAction::TrustCa }),
        );
    }

    if input.git_identity == Some(false) {
        add("git-identity", Severity::Info, "Git doesn't know who you are".into(), "Commits will fail until Git has a name and email: set them in the Git tab (for this repository only, or globally).".into(), None);
    }

    issues.sort_by_key(|i| i.severity);
    issues
}

/// PHP version against composer.json, and the extensions it asks for.
async fn php_checks(input: &Input<'_>, add: &mut impl FnMut(&str, Severity, String, String, Option<Fix>)) {
    let Some(php) = input.php else {
        add("php-missing", Severity::Error, "PHP isn't installed".into(), "The project is PHP, and no PHP version is installed. Install one on the Services page.".into(), None);
        return;
    };
    // FrankenPHP runs its own PHP, whatever version the project picks.
    if let Some(req) = input.php_requirement.filter(|_| input.server.0 != "frankenphp") {
        if satisfies(php, req) == Some(false) {
            // The newest installed version that fits, if any.
            let mut fitting: Vec<&String> = input.php_installed.iter().filter(|v| satisfies(v, req) == Some(true)).collect();
            fitting.sort_by_key(|v| version_key(v));
            let fix = fitting.last().map(|v| Fix {
                label: format!("Use PHP {v}"),
                preview: format!("Sets {}'s PHP to {v} (other projects keep theirs) and restarts the web server.", input.name),
                action: FixAction::SetPhp { version: v.to_string() },
            });
            let hint = if fix.is_some() { "" } else { " Install a version that does on the Services page." };
            add("php-version", Severity::Error, format!("PHP {php} doesn't match the project"), format!("composer.json asks for PHP {req}.{hint}"), fix);
        }
    }
    let wanted = composer_extensions(input.dir);
    if wanted.is_empty() {
        return;
    }
    let Some(loaded) = php_modules(input.layout, php).await else { return };
    for ext in wanted.iter().filter(|e| !loaded.contains(*e)) {
        add(
            &format!("ext-{ext}"),
            Severity::Error,
            format!("The PHP extension {ext} isn't enabled"),
            format!("composer.json requires ext-{ext}, and PHP {php} doesn't load it."),
            cfg!(windows).then(|| Fix {
                label: format!("Enable {ext}"),
                preview: format!("Turns on {ext} for PHP {php} (downloading it when it isn't bundled) and restarts PHP."),
                action: FixAction::EnableExtension { php: php.to_string(), name: ext.clone() },
            }),
        );
    }
}

/// Node installed, the version the project asks for, and that version against `engines.node`.
fn node_checks(node: &NodeInput, add: &mut impl FnMut(&str, Severity, String, String, Option<Fix>)) {
    // A spec NVM can install by number ("20", "20.11.1"); "lts/*" and names aren't.
    let installable = |spec: &str| node.nvm && spec.split('.').all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()));
    let install_fix = |spec: &str| {
        installable(spec).then(|| Fix {
            label: format!("Install Node {spec}"),
            preview: format!("Installs Node {spec} with NVM; the project picks it up the next time it starts."),
            action: FixAction::InstallNode { version: spec.to_string() },
        })
    };
    if node.installed.is_empty() {
        let (detail, fix) = match &node.wanted {
            Some((spec, _)) => (format!("The project is Node, and no Node version is installed. It asks for {spec}."), install_fix(spec)),
            None => ("The project is Node, and no Node version is installed. Install one on Services → Node.js.".to_string(), None),
        };
        add("node-missing", Severity::Error, "Node.js isn't installed".into(), detail, fix);
        return;
    }
    if let (Some((spec, source)), None) = (&node.wanted, &node.version) {
        add(
            "node-version-missing",
            Severity::Error,
            format!("Node {spec} isn't installed"),
            format!("{} asks for Node {spec}, so its dev server and commands won't start until it's installed (or another version is picked in its Settings).", title_case(source)),
            install_fix(spec),
        );
        return;
    }
    let (Some(version), Some(engines)) = (&node.version, &node.engines) else { return };
    if satisfies(version, engines) != Some(false) {
        return;
    }
    let fitting = node.installed.iter().find(|v| satisfies(v, engines) == Some(true));
    let hint = if fitting.is_some() { "" } else { " None of the installed versions does: install one on Services → Node.js." };
    add(
        "node-engines",
        Severity::Warning,
        format!("Node {version} doesn't match the project"),
        format!("package.json's engines asks for Node {engines}; packages may refuse to install or behave differently.{hint}"),
        fitting.map(|v| Fix {
            label: format!("Use Node {v}"),
            preview: format!("Runs the project's dev server and commands on Node {v} (saved in its settings; .nvmrc isn't changed)."),
            action: FixAction::SetNode { version: v.clone() },
        }),
    );
}

/// Python, Java and Go: installed at all, and the version the project's file asks for.
fn runtime_checks(rt: &RuntimeInput, add: &mut impl FnMut(&str, Severity, String, String, Option<Fix>)) {
    let label = rt.label;
    if !rt.installed {
        add("runtime-missing", Severity::Error, format!("{label} isn't installed"), format!("The project is {label}, and no {label} version is installed. Install one on the Services page."), None);
        return;
    }
    if let (Some((want, file)), false) = (&rt.request, rt.matched) {
        let runs_on = rt.version.as_deref().map(|v| format!(" It runs on {label} {v} instead.")).unwrap_or_default();
        add("runtime-version", Severity::Warning, format!("{label} {want} isn't installed"), format!("{file} asks for {label} {want}.{runs_on} Install it on the Services page."), None);
    }
}

/// The dev server that serves the domain: crashed, or not started.
fn app_checks(input: &Input<'_>, app: &AppInput, add: &mut impl FnMut(&str, Severity, String, String, Option<Fix>)) {
    if app.routed || matches!(app.state, "running" | "starting") {
        return;
    }
    let start = |label: &str| Some(Fix { label: label.into(), preview: format!("Starts {}'s dev server; its output shows under Commands.", input.name), action: FixAction::StartApp });
    if app.state == "crashed" {
        let code = app.exit_code.map(|c| format!(" (exit code {c})")).unwrap_or_default();
        add("app-crashed", Severity::Error, "The dev server stopped with an error".into(), format!("It exited{code}, so {} shows a \"not running\" page. Its last lines are under Commands.", input.url), start("Start it again"));
    } else {
        // Dev servers never start on their own (they're started by hand, with a profile or at
        // launch), so this is worth knowing rather than a fault.
        add("app-stopped", Severity::Info, "The dev server isn't running".into(), format!("{} shows a \"not running\" page until it starts.", input.url), start("Start the dev server"));
    }
}

/// What a fresh clone of a Laravel app is missing: its encryption key, the built Vite assets,
/// and the public/storage link.
fn laravel_checks(dir: &Path, package_manager: &str, has_env: bool, add: &mut impl FnMut(&str, Severity, String, String, Option<Fix>)) {
    let run = |label: &str, command: &str| Some(Fix { label: label.into(), preview: format!("Runs {command} in the project (its output shows under Commands)."), action: FixAction::RunCommand { label: command.into(), command: command.into() } });
    if has_env {
        let env = std::fs::read_to_string(dir.join(".env")).unwrap_or_default();
        let key = env.lines().find_map(|l| l.trim().strip_prefix("APP_KEY=")).map(|v| v.trim().trim_matches('"').trim_matches('\''));
        if key == Some("") {
            add("app-key", Severity::Error, "APP_KEY is empty".into(), "Laravel can't encrypt sessions or cookies without it, so every page fails with \"No application encryption key has been specified\".".into(), run("Generate a key", "php artisan key:generate"));
        }
    }
    // @vite() reads public/build/manifest.json, or public/hot while `vite` runs.
    let vite = ["vite.config.js", "vite.config.ts", "vite.config.mjs"].iter().any(|f| dir.join(f).is_file());
    let public = dir.join("public");
    if vite && !public.join("build").join("manifest.json").is_file() && !public.join("hot").is_file() {
        let pm = match package_manager {
            "pnpm" | "yarn" | "bun" => package_manager,
            _ => "npm",
        };
        let command = format!("{pm} run build");
        add(
            "vite-manifest",
            Severity::Error,
            "The front-end assets aren't built".into(),
            "There's no public/build/manifest.json and Vite's dev server isn't running, so pages that use @vite fail with \"Vite manifest not found\". Build them, or run the dev script under Commands.".into(),
            Some(Fix { label: format!("Run {command}"), preview: format!("Runs {command} in the project (its output shows under Commands)."), action: FixAction::RunCommand { label: command.clone(), command } }),
        );
    }
    // Uploads saved to the "public" disk are served through public/storage.
    if dir.join("storage").join("app").join("public").is_dir() && std::fs::symlink_metadata(public.join("storage")).is_err() {
        add("storage-link", Severity::Warning, "public/storage isn't linked".into(), "Files saved to the public disk (uploads, avatars) return 404 until public/storage points at storage/app/public.".into(), run("Create the link", "php artisan storage:link"));
    }
}

fn install_command(manager: &str, package_manager: &str) -> String {
    if manager == "composer" {
        "composer install".into()
    } else {
        match package_manager {
            "pnpm" => "pnpm install".into(),
            "yarn" => "yarn install".into(),
            "bun" => "bun install".into(),
            _ => "npm install".into(),
        }
    }
}

fn title_case(s: &str) -> String {
    let mut c = s.chars();
    c.next().map(|f| f.to_uppercase().chain(c).collect()).unwrap_or_default()
}

/// The ext-* packages composer.json requires (not require-dev: production code needs them).
pub fn composer_extensions(dir: &Path) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(dir.join("composer.json")) else { return Vec::new() };
    let Ok(json) = serde_json::from_str::<Value>(&text) else { return Vec::new() };
    let mut out: Vec<String> = json["require"].as_object().into_iter().flatten().filter_map(|(k, _)| k.strip_prefix("ext-")).map(|e| e.to_ascii_lowercase()).collect();
    out.sort();
    out.dedup();
    out
}

/// What `php -m` lists, lower-cased (Zend OPcache as "zend opcache" and "opcache").
/// `php -m` per PHP, kept while its program and php.ini stay the same (turning an extension
/// on or off rewrites php.ini). The Health panel checks every 15 seconds; without this,
/// each check started PHP.
type ModulesKey = (std::path::PathBuf, Option<std::time::SystemTime>, Option<std::time::SystemTime>);
static MODULES: std::sync::Mutex<Option<std::collections::HashMap<String, (ModulesKey, Vec<String>)>>> = std::sync::Mutex::new(None);

async fn php_modules(layout: &Layout, version: &str) -> Option<Vec<String>> {
    let home = layout.bin("php", version);
    let exe = home.join(exe("php"));
    let ini = layout.etc("php").join(version).join("php.ini");
    let modified = |p: &Path| std::fs::metadata(p).and_then(|m| m.modified()).ok();
    let key: ModulesKey = (exe.clone(), modified(&exe), modified(&ini));
    if let Some((k, mods)) = MODULES.lock().ok()?.get_or_insert_with(Default::default).get(version) {
        if *k == key {
            return Some(mods.clone());
        }
    }
    let mods = run_php_m(&exe, &ini).await?;
    MODULES.lock().ok()?.get_or_insert_with(Default::default).insert(version.to_string(), (key, mods.clone()));
    Some(mods)
}

async fn run_php_m(exe: &Path, ini: &Path) -> Option<Vec<String>> {
    let mut cmd = tokio::process::Command::new(exe);
    if ini.exists() {
        cmd.arg("-c").arg(ini);
    }
    cmd.arg("-m").stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null()).kill_on_drop(true);
    procgroup::prepare(&mut cmd);
    let out = tokio::time::timeout(Duration::from_secs(10), cmd.output()).await.ok()?.ok()?;
    let mut mods: Vec<String> = String::from_utf8_lossy(&out.stdout).lines().map(|l| l.trim().to_ascii_lowercase()).filter(|l| !l.is_empty() && !l.starts_with('[')).collect();
    if mods.iter().any(|m| m == "zend opcache") {
        mods.push("opcache".into());
    }
    Some(mods)
}

fn version_key(v: &str) -> Vec<u64> {
    v.split(['.', '-', '+']).map(|p| p.parse().unwrap_or(0)).collect()
}

/// Whether a version meets a Composer constraint ("^8.2", "~8.1.0", ">=8.1 <8.4",
/// "^8.2 || ^8.3", "8.3.*", "*"); None when the constraint can't be read.
pub fn satisfies(version: &str, constraint: &str) -> Option<bool> {
    let v = pad(&version_key(version));
    let mut any = false;
    for alternative in constraint.split("||").flat_map(|a| a.split('|')) {
        let parts: Vec<&str> = alternative.split([' ', ',']).map(str::trim).filter(|p| !p.is_empty()).collect();
        if parts.is_empty() {
            continue;
        }
        let mut all = true;
        for part in parts {
            all &= one(&v, part)?;
        }
        any |= all;
    }
    Some(any)
}

fn pad(v: &[u64]) -> [u64; 3] {
    [v.first().copied().unwrap_or(0), v.get(1).copied().unwrap_or(0), v.get(2).copied().unwrap_or(0)]
}

fn one(v: &[u64; 3], part: &str) -> Option<bool> {
    if part == "*" {
        return Some(true);
    }
    let (op, rest) = ["^", "~", ">=", "<=", ">", "<", "!=", "==", "="].iter().find_map(|op| part.strip_prefix(op).map(|r| (*op, r))).unwrap_or(("", part));
    let rest = rest.trim().trim_start_matches('v');
    // Wildcards: 8.3.* is >= 8.3.0 and < 8.4.0.
    if rest.contains('*') || rest.contains('x') {
        let fixed: Vec<u64> = rest.split('.').take_while(|p| *p != "*" && *p != "x").map(|p| p.parse().ok()).collect::<Option<_>>()?;
        return Some(fixed.iter().enumerate().all(|(i, n)| v[i] == *n));
    }
    let parsed: Vec<u64> = rest.split(['.', '-']).take(3).map(|p| p.parse().ok()).collect::<Option<_>>()?;
    let given = parsed.len();
    let c = pad(&parsed);
    Some(match op {
        "^" => {
            // Up to the next change of the first non-zero part.
            let upper = if c[0] > 0 { [c[0] + 1, 0, 0] } else if c[1] > 0 { [0, c[1] + 1, 0] } else { [0, 0, c[2] + 1] };
            *v >= c && *v < upper
        }
        "~" => {
            // ~8.1 allows 8.x from 8.1; ~8.1.2 allows 8.1.x from 8.1.2.
            let upper = if given <= 2 { [c[0] + 1, 0, 0] } else { [c[0], c[1] + 1, 0] };
            *v >= c && *v < upper
        }
        ">=" => *v >= c,
        "<=" => *v <= c,
        ">" => *v > c,
        "<" => *v < c,
        "!=" => *v != c,
        // A bare version matches as far as it's written (8.3 is any 8.3.x).
        _ => v.iter().zip(c.iter()).take(given).all(|(a, b)| a == b),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_composer_constraints() {
        assert_eq!(satisfies("8.3.14", "^8.2"), Some(true));
        assert_eq!(satisfies("9.0.0", "^8.2"), Some(false));
        assert_eq!(satisfies("8.1.0", "^8.2"), Some(false));
        assert_eq!(satisfies("8.4.1", ">=8.1 <8.4"), Some(false));
        assert_eq!(satisfies("8.3.9", ">=8.1 <8.4"), Some(true));
        assert_eq!(satisfies("8.3.9", ">=8.1,<8.4"), Some(true));
        assert_eq!(satisfies("8.2.0", "^7.4 || ^8.0"), Some(true));
        assert_eq!(satisfies("8.2.0", "^7.4|^8.0"), Some(true));
        assert_eq!(satisfies("8.2.5", "~8.1"), Some(true));
        assert_eq!(satisfies("8.2.5", "~8.1.0"), Some(false));
        assert_eq!(satisfies("8.3.12", "8.3.*"), Some(true));
        assert_eq!(satisfies("8.4.0", "8.3.*"), Some(false));
        assert_eq!(satisfies("8.3.12", "8.3"), Some(true));
        assert_eq!(satisfies("8.3.12", "*"), Some(true));
        assert_eq!(satisfies("8.3.12", "dev-main"), None);
    }

    #[test]
    fn finds_required_extensions() {
        let dir = std::env::temp_dir().join(format!("lamprabit-doctor-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("composer.json"), r#"{"require":{"php":"^8.2","ext-intl":"*","ext-PDO":"*","laravel/framework":"^11"},"require-dev":{"ext-xdebug":"*"}}"#).unwrap();
        assert_eq!(composer_extensions(&dir), ["intl", "pdo"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn node_issues(node: NodeInput) -> Vec<(String, Option<FixAction>)> {
        let mut out = Vec::new();
        node_checks(&node, &mut |id: &str, _, _, _, fix: Option<Fix>| out.push((id.to_string(), fix.map(|f| f.action))));
        out
    }

    fn node(installed: &[&str], wanted: Option<&str>, version: Option<&str>, engines: Option<&str>) -> NodeInput {
        NodeInput {
            installed: installed.iter().map(|s| s.to_string()).collect(),
            nvm: true,
            wanted: wanted.map(|w| (w.to_string(), "its .nvmrc")),
            version: version.map(str::to_string),
            engines: engines.map(str::to_string),
        }
    }

    #[test]
    fn node_projects() {
        // Nothing installed: offer the .nvmrc version, but not "lts/*", which NVM names differently.
        let issues = node_issues(node(&[], Some("20"), None, None));
        assert!(matches!(&issues[..], [(id, Some(FixAction::InstallNode { version }))] if id == "node-missing" && version == "20"));
        assert!(matches!(&node_issues(node(&[], Some("lts/*"), None, None))[..], [(_, None)]));
        // The .nvmrc version isn't installed.
        let issues = node_issues(node(&["22.11.0"], Some("18"), None, Some(">=18")));
        assert!(matches!(&issues[..], [(id, Some(FixAction::InstallNode { .. }))] if id == "node-version-missing"));
        // engines.node: npm ranges, and the newest installed version that fits.
        let issues = node_issues(node(&["22.11.0", "20.18.0", "18.20.4"], None, Some("22.11.0"), Some("^18.18 || ^20.9")));
        assert!(matches!(&issues[..], [(id, Some(FixAction::SetNode { version }))] if id == "node-engines" && version == "20.18.0"), "{issues:?}");
        assert!(node_issues(node(&["22.11.0"], None, Some("22.11.0"), Some(">=18.0.0"))).is_empty());
        // A range the matcher can't read ("18 - 20") is no claim either way.
        assert!(node_issues(node(&["22.11.0"], None, Some("22.11.0"), Some("18 - 20"))).is_empty());
    }

    #[test]
    fn other_runtimes() {
        let mut out = Vec::new();
        let rt = RuntimeInput { label: "Python", installed: true, request: Some(("3.12".into(), ".python-version")), matched: false, version: Some("3.13.1".into()) };
        runtime_checks(&rt, &mut |id: &str, _, _, detail: String, _| out.push((id.to_string(), detail)));
        assert_eq!(out[0].0, "runtime-version");
        assert!(out[0].1.contains("runs on Python 3.13.1"), "{}", out[0].1);
    }

    #[test]
    fn laravel_clone() {
        let dir = std::env::temp_dir().join(format!("lamprabit-doctor-laravel-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("storage/app/public")).unwrap();
        std::fs::create_dir_all(dir.join("public")).unwrap();
        std::fs::write(dir.join(".env"), "APP_NAME=Shop\nAPP_KEY=\nAPP_URL=https://shop.test\n").unwrap();
        std::fs::write(dir.join("vite.config.js"), "export default {}").unwrap();
        let ids = |dir: &Path| {
            let mut out: Vec<(String, Option<String>)> = Vec::new();
            laravel_checks(dir, "pnpm", true, &mut |id: &str, _, _, _, fix: Option<Fix>| out.push((id.to_string(), fix.map(|f| f.label))));
            out
        };
        let want: Vec<(String, Option<String>)> = vec![
            ("app-key".into(), Some("Generate a key".into())),
            ("vite-manifest".into(), Some("Run pnpm run build".into())),
            ("storage-link".into(), Some("Create the link".into())),
        ];
        assert_eq!(ids(&dir), want);
        // A key, Vite's dev server running (public/hot), and the link: nothing to say.
        std::fs::write(dir.join(".env"), "APP_KEY=base64:abc=\n").unwrap();
        std::fs::write(dir.join("public/hot"), "http://localhost:5173").unwrap();
        std::fs::create_dir_all(dir.join("public/storage")).unwrap();
        assert!(ids(&dir).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
