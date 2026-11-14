//! A project's dependencies, per package manager: what its manifest declares, what is
//! installed (read from the files, without running anything), which have newer releases,
//! and the commands that install, add, update or remove them. Commands run through the
//! project's command runner (apps.rs), so their output shows in the Commands tab, and
//! nothing changes until the user confirms the command the panel shows.

use std::{collections::BTreeMap, ffi::OsString, path::Path, process::Stdio, time::Duration};

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    apps,
    config::PackageManager,
    error::{Error, Result},
    procgroup,
    projects::shell,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Manager {
    Composer,
    Npm,
    Pnpm,
    Yarn,
    Bun,
    Deno,
    Uv,
    Poetry,
    Pip,
    Cargo,
    Go,
    Maven,
    Gradle,
}

impl Manager {
    fn label(self) -> &'static str {
        match self {
            Manager::Composer => "Composer",
            Manager::Npm => "npm",
            Manager::Pnpm => "pnpm",
            Manager::Yarn => "Yarn",
            Manager::Bun => "Bun",
            Manager::Deno => "Deno",
            Manager::Uv => "uv",
            Manager::Poetry => "Poetry",
            Manager::Pip => "pip",
            Manager::Cargo => "Cargo",
            Manager::Go => "Go modules",
            Manager::Maven => "Maven",
            Manager::Gradle => "Gradle",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Package {
    pub name: String,
    /// What the manifest asks for ("^11.0", ">=2.1", "1.4.2"); None when it names no version.
    pub wanted: Option<String>,
    /// The version in the project's installed packages or lock file.
    pub installed: Option<String>,
    /// A development-only dependency (require-dev, devDependencies, test scope…).
    pub dev: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Ecosystem {
    pub manager: Manager,
    pub label: &'static str,
    /// The file that lists the dependencies.
    pub manifest: &'static str,
    /// The lock file, when there is one.
    pub lock: Option<&'static str>,
    /// Whether the dependencies are installed (vendor/, node_modules/, .venv…), where that
    /// can be told from the files.
    pub installed: Option<bool>,
    pub packages: Vec<Package>,
    /// Packages can be added and removed with a command (Maven and Gradle need the build file edited).
    pub can_edit: bool,
    /// The command that installs what the manifest and lock file say.
    pub install: String,
    /// Updates everything within the manifest's constraints, when the tool can.
    pub update_all: Option<String>,
    /// Why its commands can't run yet (the tool isn't installed), and where to get it.
    pub missing: Option<String>,
    pub note: Option<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Outdated {
    pub name: String,
    pub current: Option<String>,
    pub latest: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Install,
    UpdateAll,
    Add,
    Remove,
    /// To the newest version the manifest's constraint allows.
    Update,
    /// To a given (newest) version, changing the manifest's constraint.
    Upgrade,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub manager: Manager,
    pub action: Action,
    #[serde(default)]
    pub package: Option<String>,
    #[serde(default)]
    pub dev: bool,
    #[serde(default)]
    pub version: Option<String>,
}

/// What a request runs, shown before it does.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub label: String,
    pub command: String,
    /// A change LampRabit makes to a file first (pip's requirements.txt).
    pub edit: Option<String>,
}

fn read(dir: &Path, file: &str) -> String {
    std::fs::read_to_string(dir.join(file)).unwrap_or_default()
}

fn json(dir: &Path, file: &str) -> Value {
    serde_json::from_str(&read(dir, file)).unwrap_or_default()
}

fn toml_file(dir: &Path, file: &str) -> toml::Value {
    read(dir, file).parse().unwrap_or(toml::Value::Table(Default::default()))
}

fn exists(dir: &Path, file: &str) -> bool {
    dir.join(file).exists()
}

/// The project's virtual environment's Python, as a command in its folder.
pub fn venv_python_command(_dir: &Path) -> &'static str {
    if cfg!(windows) { r".venv\Scripts\python.exe" } else { ".venv/bin/python" }
}

/// Installs requirements.txt into .venv, creating it with LampRabit's Python first.
pub fn pip_install_command(dir: &Path) -> String {
    let python = venv_python_command(dir);
    let create = if dir.join(".venv").exists() { String::new() } else { "python -m venv .venv && ".to_string() };
    format!("{create}{python} -m pip install -r requirements.txt")
}

/// The package managers a project uses, from its files.
pub fn managers(dir: &Path) -> Vec<Manager> {
    let mut out = Vec::new();
    if exists(dir, "composer.json") {
        out.push(Manager::Composer);
    }
    if exists(dir, "package.json") {
        out.push(match apps::detect_package_manager(dir) {
            PackageManager::Npm => Manager::Npm,
            PackageManager::Pnpm => Manager::Pnpm,
            PackageManager::Yarn => Manager::Yarn,
            PackageManager::Bun => Manager::Bun,
        });
    }
    if exists(dir, "deno.json") || exists(dir, "deno.jsonc") {
        out.push(Manager::Deno);
    }
    if exists(dir, "pyproject.toml") {
        let py = read(dir, "pyproject.toml");
        if exists(dir, "poetry.lock") || (py.contains("[tool.poetry") && !exists(dir, "uv.lock")) {
            out.push(Manager::Poetry);
        } else if exists(dir, "uv.lock") || py.contains("[project]") {
            out.push(Manager::Uv);
        }
    }
    if exists(dir, "requirements.txt") && !out.iter().any(|m| matches!(m, Manager::Uv | Manager::Poetry)) {
        out.push(Manager::Pip);
    }
    if exists(dir, "Cargo.toml") {
        out.push(Manager::Cargo);
    }
    if exists(dir, "go.mod") {
        out.push(Manager::Go);
    }
    if exists(dir, "pom.xml") {
        out.push(Manager::Maven);
    } else if exists(dir, "build.gradle") || exists(dir, "build.gradle.kts") {
        out.push(Manager::Gradle);
    }
    out
}

/// Every package manager's dependencies in a project. `path` is the PATH its commands get.
pub fn list(dir: &Path, path: &OsString) -> Vec<Ecosystem> {
    managers(dir).into_iter().map(|m| ecosystem(dir, m, path)).collect()
}

fn ecosystem(dir: &Path, manager: Manager, path: &OsString) -> Ecosystem {
    let has = |tool: &str| apps::on_path(tool, path);
    let lock = |files: &[&'static str]| files.iter().copied().find(|f| exists(dir, f));
    let base = |manifest, lock, installed, packages, install: String, update_all: Option<String>| Ecosystem {
        manager,
        label: manager.label(),
        manifest,
        lock,
        installed,
        packages,
        can_edit: true,
        install,
        update_all,
        missing: None,
        note: None,
    };
    let mut e = match manager {
        Manager::Composer => {
            let mut e = base("composer.json", lock(&["composer.lock"]), Some(exists(dir, "vendor/autoload.php")), composer_packages(dir), "composer install".into(), Some("composer update".into()));
            e.missing = (!has("composer")).then(|| "Composer isn't installed. Install it on the Services page (Package managers → Composer).".into());
            e
        }
        Manager::Npm | Manager::Pnpm | Manager::Yarn | Manager::Bun => {
            let pm = node_pm(manager);
            let commands = apps::pm_commands(pm, path);
            let lock_files: &[&'static str] = match manager {
                Manager::Npm => &["package-lock.json"],
                Manager::Pnpm => &["pnpm-lock.yaml"],
                Manager::Yarn => &["yarn.lock"],
                _ => &["bun.lock", "bun.lockb"],
            };
            let install = commands.as_ref().map(|c| c.install.clone()).unwrap_or_else(|_| format!("{} install", pm.id()));
            let update_all = match manager {
                Manager::Yarn if berry(dir) => None,
                Manager::Yarn => Some(format!("{} upgrade", node_prefix(manager, path))),
                _ => Some(format!("{} update", node_prefix(manager, path))),
            };
            let mut e = base("package.json", lock(lock_files), Some(exists(dir, "node_modules")), node_packages(dir), install, update_all);
            e.missing = match &commands {
                Err(why) => Some(why.to_string()),
                Ok(_) if !has("npm") && manager != Manager::Bun => Some("Node.js isn't installed. Install it on the Services page (Languages → Node.js).".into()),
                Ok(_) => None,
            };
            e
        }
        Manager::Deno => {
            let manifest = if exists(dir, "deno.json") { "deno.json" } else { "deno.jsonc" };
            let mut e = base(manifest, lock(&["deno.lock"]), None, deno_packages(dir, manifest), "deno install".into(), Some("deno outdated --update".into()));
            e.missing = (!has("deno")).then(|| "Deno isn't installed. Install it on the Services page (Package managers → Deno).".into());
            e.note = Some("Packages are added as npm: or jsr: specifiers (lodash becomes npm:lodash).");
            e
        }
        Manager::Uv => {
            let mut e = base("pyproject.toml", lock(&["uv.lock"]), Some(exists(dir, ".venv")), pyproject_packages(dir, false), "uv sync".into(), Some("uv lock --upgrade && uv sync".into()));
            fill_python_installed(dir, &mut e.packages);
            e.missing = (!has("uv")).then(|| "uv isn't installed. Install it on the Services page (Package managers → uv).".into());
            e
        }
        Manager::Poetry => {
            let mut e = base("pyproject.toml", lock(&["poetry.lock"]), None, pyproject_packages(dir, true), "poetry install".into(), Some("poetry update".into()));
            fill_python_installed(dir, &mut e.packages);
            e.missing = (!has("poetry")).then(|| "Poetry isn't installed. Install it with uv (uv tool install poetry) or pipx, then reopen this tab.".into());
            e
        }
        Manager::Pip => {
            let python = venv_python_command(dir);
            let mut e = base("requirements.txt", None, Some(exists(dir, ".venv")), requirements_packages(dir), pip_install_command(dir), Some(format!("{python} -m pip install --upgrade -r requirements.txt")));
            fill_python_installed(dir, &mut e.packages);
            e.missing = (!has("python") && !exists(dir, ".venv")).then(|| "Python isn't installed. Install it on the Services page (Languages → Python); the packages go into the project's .venv.".into());
            e.note = Some("Adding or removing a package also edits requirements.txt; installs go into the project's .venv.");
            e
        }
        Manager::Cargo => {
            let mut e = base("Cargo.toml", lock(&["Cargo.lock"]), None, cargo_packages(dir), "cargo fetch".into(), Some("cargo update".into()));
            e.missing = (!has("cargo")).then(|| "Rust isn't installed. Install it on the Services page (Languages → Rust).".into());
            e
        }
        Manager::Go => {
            let mut e = base("go.mod", lock(&["go.sum"]), None, go_packages(dir), "go mod download".into(), Some("go get -u ./... && go mod tidy".into()));
            e.missing = (!has("go")).then(|| "Go isn't installed. Install it on the Services page (Languages → Go).".into());
            e
        }
        Manager::Maven => {
            let mvn = wrapper(dir, "mvnw", "mvn");
            let mut e = base("pom.xml", None, None, maven_packages(dir), format!("{mvn} -B dependency:resolve"), None);
            e.can_edit = false;
            e.missing = (mvn == "mvn" && !has("mvn")).then(|| "Maven isn't installed. Install it on the Services page (Package managers → Maven), or add the Maven wrapper (mvnw) to the project.".into());
            e.note = Some("Add or remove dependencies in pom.xml; upgrading one sets its version there.");
            e
        }
        Manager::Gradle => {
            let gradle = wrapper(dir, "gradlew", "gradle");
            let manifest = if exists(dir, "build.gradle.kts") { "build.gradle.kts" } else { "build.gradle" };
            let mut e = base(manifest, lock(&["gradle.lockfile"]), None, gradle_packages(dir, manifest), format!("{gradle} --refresh-dependencies dependencies"), None);
            e.can_edit = false;
            e.missing = (gradle == "gradle" && !has("gradle")).then(|| "Gradle isn't installed. Install it on the Services page (Package managers → Gradle), or add the Gradle wrapper (gradlew) to the project.".into());
            e.note = Some("Gradle's build file is code: add, remove and change versions there.");
            e
        }
    };
    e.packages.sort_by(|a, b| a.dev.cmp(&b.dev).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    e
}

fn node_pm(manager: Manager) -> PackageManager {
    match manager {
        Manager::Pnpm => PackageManager::Pnpm,
        Manager::Yarn => PackageManager::Yarn,
        Manager::Bun => PackageManager::Bun,
        _ => PackageManager::Npm,
    }
}

/// How pnpm and Yarn are run: themselves when installed (corepack), else through npx.
fn node_prefix(manager: Manager, path: &OsString) -> String {
    let tool = node_pm(manager).id();
    if manager == Manager::Npm || manager == Manager::Bun || apps::on_path(tool, path) { tool.to_string() } else { format!("npx -y {tool}") }
}

/// Yarn 2 and later ("berry") keep their settings in .yarnrc.yml and spell some commands differently.
fn berry(dir: &Path) -> bool {
    exists(dir, ".yarnrc.yml")
}

/// A project's own wrapper script (mvnw, gradlew) when it has one, else the tool's shim.
pub fn wrapper(dir: &Path, script: &str, tool: &str) -> String {
    match (dir.join(script).exists(), cfg!(windows)) {
        (true, true) if dir.join(format!("{script}.cmd")).exists() => format!("{script}.cmd"),
        (true, true) if dir.join(format!("{script}.bat")).exists() => format!("{script}.bat"),
        (true, false) => format!("./{script}"),
        _ => tool.to_string(),
    }
}

fn composer_packages(dir: &Path) -> Vec<Package> {
    let manifest = json(dir, "composer.json");
    let installed_file = json(dir, "vendor/composer/installed.json");
    // Composer 2 wraps the list in {"packages": […]}; Composer 1 wrote the list itself.
    let installed: BTreeMap<String, String> = installed_file["packages"]
        .as_array()
        .or(installed_file.as_array())
        .into_iter()
        .flatten()
        .filter_map(|p| Some((p["name"].as_str()?.to_lowercase(), p["version"].as_str()?.to_string())))
        .collect();
    let mut out = Vec::new();
    for (key, dev) in [("require", false), ("require-dev", true)] {
        for (name, wanted) in manifest[key].as_object().into_iter().flatten() {
            // The platform (php, ext-*, lib-*) isn't a package Composer installs.
            if name == "php" || name.starts_with("ext-") || name.starts_with("lib-") || name.starts_with("composer-") || !name.contains('/') {
                continue;
            }
            out.push(Package { name: name.clone(), wanted: wanted.as_str().map(str::to_string), installed: installed.get(&name.to_lowercase()).cloned(), dev });
        }
    }
    out
}

fn node_packages(dir: &Path) -> Vec<Package> {
    let manifest = json(dir, "package.json");
    let mut out = Vec::new();
    for (key, dev) in [("dependencies", false), ("devDependencies", true)] {
        for (name, wanted) in manifest[key].as_object().into_iter().flatten() {
            let installed = json(dir, &format!("node_modules/{name}/package.json"))["version"].as_str().map(str::to_string);
            out.push(Package { name: name.clone(), wanted: wanted.as_str().map(str::to_string), installed, dev });
        }
    }
    out
}

/// deno.json's imports ("@std/assert": "jsr:@std/assert@^1.0.0") with the versions deno.lock resolved.
fn deno_packages(dir: &Path, manifest: &str) -> Vec<Package> {
    let config: Value = serde_json::from_str(&crate::commands::strip_json_comments(&read(dir, manifest))).unwrap_or_default();
    let lock = json(dir, "deno.lock");
    let mut out = Vec::new();
    for (name, spec) in config["imports"].as_object().into_iter().flatten() {
        let Some(spec) = spec.as_str() else { continue };
        let Some((registry, rest)) = spec.split_once(':').filter(|(r, _)| *r == "jsr" || *r == "npm") else { continue };
        // "@std/assert@^1.0.0" → the package and its range (scoped names start with @).
        let (package, wanted) = match rest.trim_start_matches('/').rsplit_once('@') {
            Some((p, v)) if !p.is_empty() => (p.to_string(), Some(v.to_string())),
            _ => (rest.to_string(), None),
        };
        let installed = lock["specifiers"][spec].as_str().map(|v| v.split('_').next().unwrap_or(v).to_string());
        out.push(Package { name: format!("{registry}:{package}"), wanted, installed, dev: false });
        let _ = name;
    }
    out
}

/// "requests>=2.31; python_version > '3.8'" → ("requests", Some(">=2.31")).
fn pep508(spec: &str) -> Option<(String, Option<String>)> {
    let spec = spec.split(';').next()?.trim();
    let end = spec.find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')).unwrap_or(spec.len());
    let name = spec[..end].to_string();
    if name.is_empty() {
        return None;
    }
    let mut rest = spec[end..].trim();
    // Extras: "uvicorn[standard]>=0.30".
    if rest.starts_with('[') {
        rest = rest.split_once(']').map_or("", |(_, r)| r).trim();
    }
    Some((name, (!rest.is_empty()).then(|| rest.to_string())))
}

fn pyproject_packages(dir: &Path, poetry: bool) -> Vec<Package> {
    let toml = toml_file(dir, "pyproject.toml");
    let mut out = Vec::new();
    let strings = |v: Option<&toml::Value>| v.and_then(|v| v.as_array()).into_iter().flatten().filter_map(|s| s.as_str().and_then(pep508)).collect::<Vec<_>>();
    if poetry {
        let tables = [
            (toml.get("tool").and_then(|t| t.get("poetry")).and_then(|p| p.get("dependencies")), false),
            (toml.get("tool").and_then(|t| t.get("poetry")).and_then(|p| p.get("dev-dependencies")), true),
            (toml.get("tool").and_then(|t| t.get("poetry")).and_then(|p| p.get("group")).and_then(|g| g.get("dev")).and_then(|d| d.get("dependencies")), true),
        ];
        for (table, dev) in tables {
            for (name, value) in table.and_then(|t| t.as_table()).into_iter().flatten() {
                if name == "python" {
                    continue;
                }
                let wanted = value.as_str().map(str::to_string).or_else(|| value.get("version").and_then(|v| v.as_str()).map(str::to_string));
                out.push(Package { name: name.clone(), wanted, installed: None, dev });
            }
        }
    }
    let project = toml.get("project");
    for (name, wanted) in strings(project.and_then(|p| p.get("dependencies"))) {
        out.push(Package { name, wanted, installed: None, dev: false });
    }
    let dev = [toml.get("dependency-groups").and_then(|g| g.get("dev")), toml.get("tool").and_then(|t| t.get("uv")).and_then(|u| u.get("dev-dependencies"))];
    for list in dev {
        for (name, wanted) in strings(list) {
            out.push(Package { name, wanted, installed: None, dev: true });
        }
    }
    // Poetry 2 also reads [project]; a package listed both ways shows once.
    let mut seen = std::collections::HashSet::new();
    out.retain(|p| seen.insert((normalize(&p.name), p.dev)));
    out
}

fn requirements_packages(dir: &Path) -> Vec<Package> {
    read(dir, "requirements.txt")
        .lines()
        .map(|l| l.split(" #").next().unwrap_or(l).trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#') && !l.starts_with('-'))
        .filter_map(pep508)
        .map(|(name, wanted)| Package { name, wanted, installed: None, dev: false })
        .collect()
}

/// PyPI names compare case-insensitively, with -, _ and . the same.
fn normalize(name: &str) -> String {
    name.to_lowercase().replace(['_', '.'], "-")
}

/// Installed versions from the .venv's *.dist-info folders.
fn fill_python_installed(dir: &Path, packages: &mut [Package]) {
    let venv = dir.join(".venv");
    let site = if cfg!(windows) {
        Some(venv.join("Lib").join("site-packages"))
    } else {
        std::fs::read_dir(venv.join("lib")).ok().and_then(|mut d| d.find_map(|e| e.ok().filter(|e| e.file_name().to_string_lossy().starts_with("python")))).map(|e| e.path().join("site-packages"))
    };
    let Some(site) = site else { return };
    let installed: BTreeMap<String, String> = std::fs::read_dir(site)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let stem = name.strip_suffix(".dist-info")?;
            let (package, version) = stem.rsplit_once('-')?;
            Some((normalize(package), version.to_string()))
        })
        .collect();
    for p in packages {
        p.installed = installed.get(&normalize(&p.name)).cloned();
    }
}

fn cargo_packages(dir: &Path) -> Vec<Package> {
    let manifest = toml_file(dir, "Cargo.toml");
    let lock = toml_file(dir, "Cargo.lock");
    let locked: BTreeMap<String, String> = lock
        .get("package")
        .and_then(|p| p.as_array())
        .into_iter()
        .flatten()
        .filter_map(|p| Some((p.get("name")?.as_str()?.to_string(), p.get("version")?.as_str()?.to_string())))
        .collect();
    let mut out = Vec::new();
    for (key, dev) in [("dependencies", false), ("dev-dependencies", true), ("build-dependencies", true)] {
        for (name, value) in manifest.get(key).and_then(|t| t.as_table()).into_iter().flatten() {
            let wanted = value.as_str().map(str::to_string).or_else(|| value.get("version").and_then(|v| v.as_str()).map(str::to_string));
            // `package = "real-name"` renames a dependency; the lock file has the real one.
            let real = value.get("package").and_then(|p| p.as_str()).unwrap_or(name);
            out.push(Package { name: name.clone(), wanted, installed: locked.get(real).cloned(), dev });
        }
    }
    out
}

fn go_packages(dir: &Path) -> Vec<Package> {
    let text = read(dir, "go.mod");
    let mut out = Vec::new();
    let mut block = false;
    for line in text.lines().map(str::trim) {
        let entry = if block {
            if line == ")" {
                block = false;
                continue;
            }
            line
        } else if line == "require (" {
            block = true;
            continue;
        } else if let Some(one) = line.strip_prefix("require ") {
            one
        } else {
            continue;
        };
        // Indirect modules are other modules' dependencies, managed by go mod tidy.
        if entry.contains("// indirect") || entry.is_empty() || entry.starts_with("//") {
            continue;
        }
        let mut parts = entry.split_whitespace();
        if let (Some(module), Some(version)) = (parts.next(), parts.next()) {
            out.push(Package { name: module.to_string(), wanted: Some(version.to_string()), installed: Some(version.to_string()), dev: false });
        }
    }
    out
}

/// The text inside the first <tag>…</tag> of `xml`.
fn xml_tag<'a>(xml: &'a str, tag: &str) -> Option<&'a str> {
    let start = xml.find(&format!("<{tag}>"))? + tag.len() + 2;
    let end = xml[start..].find(&format!("</{tag}>"))? + start;
    Some(xml[start..end].trim())
}

/// `xml` without every <tag>…</tag> section.
fn without(xml: &str, tag: &str) -> String {
    let mut out = xml.to_string();
    while let (Some(start), Some(end)) = (out.find(&format!("<{tag}>")), out.find(&format!("</{tag}>"))) {
        if end < start {
            break;
        }
        out.replace_range(start..end + tag.len() + 3, "");
    }
    out
}

fn maven_packages(dir: &Path) -> Vec<Package> {
    let pom = read(dir, "pom.xml");
    let properties = xml_tag(&pom, "properties").unwrap_or_default().to_string();
    // Plugins' dependencies, managed versions and profiles aren't the project's own.
    let mut body = pom.clone();
    for tag in ["build", "dependencyManagement", "profiles", "parent"] {
        body = without(&body, tag);
    }
    let resolve = |v: &str| match v.strip_prefix("${").and_then(|v| v.strip_suffix('}')) {
        Some(prop) => xml_tag(&properties, prop).map(str::to_string).unwrap_or_else(|| v.to_string()),
        None => v.to_string(),
    };
    body.split("<dependency>")
        .skip(1)
        .filter_map(|d| {
            let d = d.split("</dependency>").next()?;
            let name = format!("{}:{}", xml_tag(d, "groupId")?, xml_tag(d, "artifactId")?);
            let version = xml_tag(d, "version").map(resolve);
            let dev = xml_tag(d, "scope") == Some("test");
            Some(Package { name, wanted: version.clone(), installed: version, dev })
        })
        .collect()
}

const GRADLE_CONFIGURATIONS: [&str; 10] = ["implementation", "api", "compileOnly", "runtimeOnly", "annotationProcessor", "developmentOnly", "kapt", "testImplementation", "testRuntimeOnly", "testCompileOnly"];

fn gradle_packages(dir: &Path, manifest: &str) -> Vec<Package> {
    read(dir, manifest)
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            let config = GRADLE_CONFIGURATIONS.iter().find(|c| line.starts_with(**c) && line[c.len()..].starts_with(['(', ' ', '\'', '"']))?;
            // The first quoted "group:name:version" on the line (platform(...) BOMs included).
            let quote = line.find(['"', '\''])?;
            let q = &line[quote..quote + 1];
            let rest = &line[quote + 1..];
            let coordinate = &rest[..rest.find(q)?];
            let mut parts = coordinate.split(':');
            let (group, artifact) = (parts.next()?, parts.next()?);
            let version = parts.next().map(str::to_string);
            Some(Package { name: format!("{group}:{artifact}"), wanted: version.clone(), installed: version, dev: config.starts_with("test") })
        })
        .collect()
}

/// Package names and versions go into a command line, so they may only hold the characters
/// package names and version ranges use: no spaces, quotes or shell operators.
fn checked(value: &str, what: &str) -> Result<String> {
    let value = value.trim();
    let ok = !value.is_empty()
        && !value.starts_with('-')
        && value.len() <= 214
        && value.chars().all(|c| c.is_ascii_alphanumeric() || "@/._:+~^=!*-[],".contains(c));
    if ok {
        Ok(value.to_string())
    } else {
        Err(Error::msg(format!(
            "\"{value}\" isn't a valid {what}. Use the package's name, optionally with a version (lodash@4, laravel/framework:^11, requests==2.32); spaces, quotes and < > aren't allowed."
        )))
    }
}

/// The command a request runs, and the file change made before it.
pub fn plan(dir: &Path, path: &OsString, req: &Request) -> Result<Plan> {
    let m = req.manager;
    if !managers(dir).contains(&m) {
        return Err(Error::msg(format!("This project doesn't use {}.", m.label())));
    }
    let e = ecosystem(dir, m, path);
    let package = req.package.as_deref().map(|p| checked(p, "package name")).transpose()?;
    let version = req.version.as_deref().map(|v| checked(v.trim_start_matches('v'), "version")).transpose()?;
    let need_package = || package.clone().ok_or_else(|| Error::msg("Choose a package first."));
    let need_version = || version.clone().ok_or_else(|| Error::msg("Check for updates first, so LampRabit knows the newest version."));
    let dev = req.dev;
    let unsupported = || Error::msg(format!("{} can't do that from LampRabit: {}", m.label(), e.note.unwrap_or("use the project's terminal.")));
    let node = node_prefix(m, path);
    let mut edit = None;
    let command = match req.action {
        Action::Install => e.install.clone(),
        Action::UpdateAll => e.update_all.clone().ok_or_else(unsupported)?,
        Action::Add => {
            if !e.can_edit {
                return Err(unsupported());
            }
            let p = need_package()?;
            match m {
                Manager::Composer => format!("composer require{} {p}", if dev { " --dev" } else { "" }),
                Manager::Npm => format!("npm install{} {p}", if dev { " -D" } else { "" }),
                Manager::Pnpm | Manager::Yarn => format!("{node} add{} {p}", if dev { " -D" } else { "" }),
                Manager::Bun => format!("bun add{} {p}", if dev { " -d" } else { "" }),
                Manager::Deno => {
                    let p = if p.starts_with("npm:") || p.starts_with("jsr:") { p } else { format!("npm:{p}") };
                    format!("deno add{} {p}", if dev { " --dev" } else { "" })
                }
                Manager::Uv => format!("uv add{} {p}", if dev { " --dev" } else { "" }),
                Manager::Poetry => format!("poetry add{} {p}", if dev { " --group dev" } else { "" }),
                Manager::Pip => {
                    edit = Some(format!("Adds {p} to requirements.txt"));
                    pip_install_command(dir)
                }
                Manager::Cargo => format!("cargo add{} {p}", if dev { " --dev" } else { "" }),
                Manager::Go => format!("go get {p}"),
                Manager::Maven | Manager::Gradle => return Err(unsupported()),
            }
        }
        Action::Remove => {
            if !e.can_edit {
                return Err(unsupported());
            }
            let p = need_package()?;
            match m {
                Manager::Composer => format!("composer remove{} {p}", if dev { " --dev" } else { "" }),
                Manager::Npm => format!("npm uninstall {p}"),
                Manager::Pnpm | Manager::Yarn => format!("{node} remove {p}"),
                Manager::Bun => format!("bun remove {p}"),
                Manager::Deno => format!("deno remove {p}"),
                Manager::Uv => format!("uv remove{} {p}", if dev { " --dev" } else { "" }),
                Manager::Poetry => format!("poetry remove{} {p}", if dev { " --group dev" } else { "" }),
                Manager::Pip => {
                    edit = Some(format!("Removes {p} from requirements.txt"));
                    format!("{} -m pip uninstall -y {p}", venv_python_command(dir))
                }
                Manager::Cargo => format!("cargo remove{} {p}", if dev { " --dev" } else { "" }),
                Manager::Go => format!("go get {p}@none && go mod tidy"),
                Manager::Maven | Manager::Gradle => return Err(unsupported()),
            }
        }
        Action::Update => {
            let p = need_package()?;
            match m {
                Manager::Composer => format!("composer update {p} --with-dependencies"),
                Manager::Npm => format!("npm update {p}"),
                Manager::Pnpm => format!("{node} update {p}"),
                Manager::Yarn if berry(dir) => format!("{node} up {p}"),
                Manager::Yarn => format!("{node} upgrade {p}"),
                Manager::Bun => format!("bun update {p}"),
                Manager::Deno => format!("deno outdated --update {p}"),
                Manager::Uv => format!("uv lock --upgrade-package {p} && uv sync"),
                Manager::Poetry => format!("poetry update {p}"),
                Manager::Pip => format!("{} -m pip install --upgrade {p}", venv_python_command(dir)),
                Manager::Cargo => format!("cargo update -p {p}"),
                Manager::Go => format!("go get -u=patch {p}"),
                Manager::Maven | Manager::Gradle => return Err(unsupported()),
            }
        }
        Action::Upgrade => {
            let p = need_package()?;
            let v = need_version()?;
            match m {
                Manager::Composer => format!("composer require{} {p}:^{v} --with-dependencies", if dev { " --dev" } else { "" }),
                Manager::Npm => format!("npm install{} {p}@{v}", if dev { " -D" } else { "" }),
                Manager::Pnpm | Manager::Yarn => format!("{node} add{} {p}@{v}", if dev { " -D" } else { "" }),
                Manager::Bun => format!("bun add{} {p}@{v}", if dev { " -d" } else { "" }),
                Manager::Deno => format!("deno add {p}@^{v}"),
                Manager::Uv => format!("uv add{} {p}=={v}", if dev { " --dev" } else { "" }),
                Manager::Poetry => format!("poetry add{} {p}@^{v}", if dev { " --group dev" } else { "" }),
                Manager::Pip => {
                    edit = Some(format!("Sets {p}=={v} in requirements.txt"));
                    format!("{} -m pip install {p}=={v}", venv_python_command(dir))
                }
                Manager::Cargo => format!("cargo add{} {p}@{v}", if dev { " --dev" } else { "" }),
                Manager::Go => format!("go get {p}@v{v}"),
                Manager::Maven => {
                    let mvn = wrapper(dir, "mvnw", "mvn");
                    format!("{mvn} -B versions:use-dep-version -Dincludes={p} -DdepVersion={v} -DforceVersion=true -DgenerateBackupPoms=false")
                }
                Manager::Gradle => return Err(unsupported()),
            }
        }
    };
    let what = package.as_deref().unwrap_or("dependencies");
    let label = match req.action {
        Action::Install => format!("Installing dependencies with {}", m.label()),
        Action::UpdateAll => format!("Updating dependencies with {}", m.label()),
        Action::Add => format!("Adding {what}"),
        Action::Remove => format!("Removing {what}"),
        Action::Update => format!("Updating {what}"),
        Action::Upgrade => format!("Upgrading {what} to {}", version.as_deref().unwrap_or("the newest version")),
    };
    Ok(Plan { label, command, edit })
}

/// The change to requirements.txt that pip's add, remove and upgrade make before they run.
pub fn apply_edit(dir: &Path, req: &Request) -> Result<()> {
    if req.manager != Manager::Pip || !matches!(req.action, Action::Add | Action::Remove | Action::Upgrade) {
        return Ok(());
    }
    let package = checked(req.package.as_deref().unwrap_or_default(), "package name")?;
    let file = dir.join("requirements.txt");
    let text = std::fs::read_to_string(&file).unwrap_or_default();
    let target = normalize(pep508(&package).map(|(n, _)| n).unwrap_or_default().as_str());
    let is_target = |line: &str| pep508(line.trim()).is_some_and(|(n, _)| normalize(&n) == target) && !line.trim_start().starts_with(['#', '-']);
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    match req.action {
        Action::Add => {
            if lines.iter().any(|l| is_target(l)) {
                return Err(Error::msg(format!("{package} is already in requirements.txt. Use Update to change its version.")));
            }
            lines.push(package.clone());
        }
        Action::Remove => lines.retain(|l| !is_target(l)),
        _ => {
            let version = checked(req.version.as_deref().unwrap_or_default().trim_start_matches('v'), "version")?;
            // Keeps the line's own spelling of the name (Django, not django).
            for l in lines.iter_mut().filter(|l| is_target(l)) {
                let name = pep508(l.trim()).map(|(n, _)| n).unwrap_or_else(|| package.clone());
                *l = format!("{name}=={version}");
            }
        }
    }
    let mut out = lines.join(newline);
    out.push_str(newline);
    std::fs::write(file, out)?;
    Ok(())
}

/// Runs a listing command; its stdout even when it exits non-zero (`npm outdated` exits 1
/// when something is outdated), or why it produced none.
async fn capture(dir: &Path, path: &OsString, env: &[(String, String)], command: &str) -> Result<String> {
    let (program, args) = shell(command);
    let mut cmd = tokio::process::Command::new(program);
    cmd.args(args)
        .current_dir(dir)
        .env("PATH", path)
        .envs(env.iter().map(|(k, v)| (k, v)))
        .env("NO_COLOR", "1")
        .env("COMPOSER_NO_INTERACTION", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    procgroup::prepare(&mut cmd);
    let child = cmd.spawn().map_err(|e| Error::msg(format!("Couldn't run {command}: {e}")))?;
    let out = tokio::time::timeout(Duration::from_secs(120), child.wait_with_output())
        .await
        .map_err(|_| Error::msg(format!("{command} took longer than two minutes. Check your connection and try again.")))??;
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    if stdout.trim().is_empty() && !out.status.success() {
        let last = crate::error::last_line(&out.stderr).unwrap_or_else(|| "it failed".into());
        return Err(Error::msg(format!("{command} failed: {}", crate::projects::strip_ansi(&last))));
    }
    Ok(stdout)
}

/// The newest release of each listed package, from the tool itself where it reports it as
/// JSON (Composer, npm, pnpm), else from the package registry. Only packages with a newer
/// release than the installed one are returned.
pub async fn outdated(client: &reqwest::Client, dir: &Path, path: &OsString, env: &[(String, String)], manager: Manager) -> Result<Vec<Outdated>> {
    let e = ecosystem(dir, manager, path);
    if let Some(why) = &e.missing {
        return Err(Error::msg(why.clone()));
    }
    let current: BTreeMap<String, Option<String>> = e.packages.iter().map(|p| (p.name.clone(), p.installed.clone().or(p.wanted.clone()))).collect();
    let latest: Vec<(String, String)> = match manager {
        Manager::Composer => {
            let text = capture(dir, path, env, "composer outdated --direct --format=json").await?;
            let value: Value = serde_json::from_str(text.find('{').map_or(&text[..], |i| &text[i..])).unwrap_or_default();
            value["installed"].as_array().into_iter().flatten().filter_map(|p| Some((p["name"].as_str()?.to_string(), p["latest"].as_str()?.to_string()))).collect()
        }
        Manager::Npm | Manager::Pnpm => {
            let command = if manager == Manager::Npm { "npm outdated --json".to_string() } else { format!("{} outdated --format json", node_prefix(manager, path)) };
            let text = capture(dir, path, env, &command).await.unwrap_or_default();
            let value: Value = serde_json::from_str(text.find('{').map_or("{}", |i| &text[i..])).unwrap_or_default();
            value
                .as_object()
                .into_iter()
                .flatten()
                .filter_map(|(name, info)| {
                    // With workspaces npm lists each package as an array of entries.
                    let info = info.as_array().and_then(|a| a.first()).unwrap_or(info);
                    Some((name.clone(), info["latest"].as_str()?.to_string()))
                })
                .collect()
        }
        _ => registry_latest(client, manager, e.packages.iter().map(|p| p.name.clone()).collect()).await,
    };
    let newer = |installed: &Option<String>, latest: &str| match installed {
        Some(v) => version_key(v) < version_key(latest),
        None => true,
    };
    Ok(latest
        .into_iter()
        .filter_map(|(name, latest)| {
            let installed = current.get(&name).cloned().flatten();
            // A plain constraint like "^1.2" isn't an installed version: only compare real ones.
            newer(&installed, &latest).then_some(Outdated { name, current: installed, latest: latest.trim_start_matches('v').to_string() })
        })
        .collect())
}

/// Maven-style pre-release versions: 4.2.0-RC2, 6.0.0-M3, 2.0.0.beta1, 1.0-SNAPSHOT (33.3.1-jre is a release).
fn prerelease(v: &str) -> bool {
    v.to_lowercase().split(['-', '.']).any(|part| {
        let letters: String = part.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
        let rest = &part[letters.len()..];
        matches!(letters.as_str(), "alpha" | "beta" | "snapshot" | "preview" | "ea") || (matches!(letters.as_str(), "rc" | "m" | "cr" | "a" | "b") && rest.chars().all(|c| c.is_ascii_digit()) && !rest.is_empty())
            || letters == "rc"
    })
}

use crate::version::release as version_key;

/// Each package's newest stable release from its registry, eight requests at a time.
async fn registry_latest(client: &reqwest::Client, manager: Manager, names: Vec<String>) -> Vec<(String, String)> {
    futures_util::stream::iter(names)
        .map(|name| async move {
            let latest = lookup(client, manager, &name).await;
            latest.map(|l| (name, l))
        })
        .buffer_unordered(8)
        .filter_map(|r| async move { r })
        .collect()
        .await
}

async fn get_json(client: &reqwest::Client, url: &str, accept: Option<&str>) -> Option<Value> {
    let mut request = client.get(url).timeout(Duration::from_secs(20));
    if let Some(a) = accept {
        request = request.header("Accept", a);
    }
    request.send().await.ok()?.error_for_status().ok()?.json().await.ok()
}

async fn get_text(client: &reqwest::Client, url: &str) -> Option<String> {
    client.get(url).timeout(Duration::from_secs(20)).send().await.ok()?.error_for_status().ok()?.text().await.ok()
}

/// Go's module proxy wants capital letters escaped as "!" and the lower-case letter.
fn go_escape(module: &str) -> String {
    module.chars().map(|c| if c.is_ascii_uppercase() { format!("!{}", c.to_ascii_lowercase()) } else { c.to_string() }).collect()
}

async fn npm_latest(client: &reqwest::Client, name: &str) -> Option<String> {
    let url = format!("https://registry.npmjs.org/{}", name.replace('/', "%2F"));
    let value = get_json(client, &url, Some("application/vnd.npm.install-v1+json")).await?;
    value["dist-tags"]["latest"].as_str().map(str::to_string)
}

async fn lookup(client: &reqwest::Client, manager: Manager, name: &str) -> Option<String> {
    match manager {
        Manager::Yarn | Manager::Bun | Manager::Npm | Manager::Pnpm => npm_latest(client, name).await,
        Manager::Deno => match name.split_once(':')? {
            ("npm", package) => npm_latest(client, package).await,
            (_, package) => get_json(client, &format!("https://jsr.io/{package}/meta.json"), None).await?["latest"].as_str().map(str::to_string),
        },
        Manager::Uv | Manager::Poetry | Manager::Pip => get_json(client, &format!("https://pypi.org/pypi/{name}/json"), None).await?["info"]["version"].as_str().map(str::to_string),
        Manager::Cargo => get_json(client, &format!("https://crates.io/api/v1/crates/{name}"), None).await?["crate"]["max_stable_version"].as_str().map(str::to_string),
        Manager::Go => get_json(client, &format!("https://proxy.golang.org/{}/@latest", go_escape(name)), None).await?["Version"].as_str().map(|v| v.trim_start_matches('v').to_string()),
        Manager::Maven | Manager::Gradle => {
            let (group, artifact) = name.split_once(':')?;
            let xml = get_text(client, &format!("https://repo1.maven.org/maven2/{}/{artifact}/maven-metadata.xml", group.replace('.', "/"))).await?;
            // <release> can be a release candidate; the newest stable one in <versions> is wanted.
            xml.split("<version>").skip(1).filter_map(|v| v.split("</version>").next()).map(str::trim).filter(|v| !prerelease(v)).max_by_key(|v| version_key(v)).map(str::to_string)
        }
        Manager::Composer => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(files: &[(&str, &str)]) -> std::path::PathBuf {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("lamprabit-deps-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for (file, text) in files {
            let path = dir.join(file);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        dir
    }

    #[test]
    fn reads_composer_and_node_manifests_with_installed_versions() {
        let dir = project(&[
            ("composer.json", r#"{"require":{"php":"^8.2","ext-pdo":"*","laravel/framework":"^11.0"},"require-dev":{"phpunit/phpunit":"^11"}}"#),
            ("vendor/composer/installed.json", r#"{"packages":[{"name":"laravel/framework","version":"v11.30.0"}]}"#),
            ("package.json", r#"{"dependencies":{"vue":"^3.5"},"devDependencies":{"vite":"^6"}}"#),
            ("node_modules/vue/package.json", r#"{"version":"3.5.12"}"#),
        ]);
        let composer = composer_packages(&dir);
        assert_eq!(composer.len(), 2, "php and extensions aren't packages");
        assert_eq!(composer[0].installed.as_deref(), Some("v11.30.0"));
        assert!(composer[1].dev && composer[1].installed.is_none());
        let node = node_packages(&dir);
        assert_eq!(node.iter().find(|p| p.name == "vue").unwrap().installed.as_deref(), Some("3.5.12"));
        assert_eq!(managers(&dir), vec![Manager::Composer, Manager::Npm]);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn reads_python_rust_go_and_java_manifests() {
        let dir = project(&[
            ("pyproject.toml", "[project]\nname = \"x\"\ndependencies = [\"fastapi>=0.115\", \"uvicorn[standard]>=0.30; python_version>'3.8'\"]\n[dependency-groups]\ndev = [\"pytest\"]\n"),
            ("Cargo.toml", "[package]\nname = \"x\"\n[dependencies]\nserde = { version = \"1\", features = [\"derive\"] }\naxum = \"0.8\"\n"),
            ("Cargo.lock", "[[package]]\nname = \"serde\"\nversion = \"1.0.215\"\n"),
            ("go.mod", "module x\n\ngo 1.23\n\nrequire (\n\tgithub.com/gin-gonic/gin v1.10.0\n\tgolang.org/x/net v0.30.0 // indirect\n)\nrequire github.com/A/b v0.1.0\n"),
            ("pom.xml", "<project><parent><version>3.4.0</version></parent><properties><jjwt.version>0.12.6</jjwt.version></properties><dependencies><dependency><groupId>io.jsonwebtoken</groupId><artifactId>jjwt-api</artifactId><version>${jjwt.version}</version></dependency><dependency><groupId>org.junit</groupId><artifactId>junit</artifactId><scope>test</scope></dependency></dependencies><build><plugins><plugin><dependencies><dependency><groupId>p</groupId><artifactId>q</artifactId></dependency></dependencies></plugin></plugins></build></project>"),
        ]);
        let py = pyproject_packages(&dir, false);
        assert_eq!(py.iter().map(|p| (p.name.as_str(), p.wanted.as_deref(), p.dev)).collect::<Vec<_>>(), vec![("fastapi", Some(">=0.115"), false), ("uvicorn", Some(">=0.30"), false), ("pytest", None, true)]);
        let cargo = cargo_packages(&dir);
        assert_eq!(cargo.iter().find(|p| p.name == "serde").unwrap().installed.as_deref(), Some("1.0.215"));
        let go = go_packages(&dir);
        assert_eq!(go.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(), vec!["github.com/gin-gonic/gin", "github.com/A/b"]);
        let maven = maven_packages(&dir);
        assert_eq!(maven.len(), 2, "a plugin's dependencies aren't the project's");
        assert_eq!(maven[0].wanted.as_deref(), Some("0.12.6"), "properties are resolved");
        assert!(maven[1].dev);
        assert_eq!(go_escape("github.com/A/b"), "github.com/!a/b");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn reads_gradle_and_deno() {
        let dir = project(&[
            ("build.gradle.kts", "dependencies {\n    implementation(\"org.springframework.boot:spring-boot-starter-web\")\n    implementation 'com.google.guava:guava:33.3.1-jre'\n    testImplementation(\"org.junit.jupiter:junit-jupiter:5.11.3\")\n    // implementation(\"x:y:1\")\n}\n"),
            ("deno.json", r#"{"imports":{"@std/assert":"jsr:@std/assert@^1.0.0","chalk":"npm:chalk@5","local":"./src/mod.ts"}}"#),
            ("deno.lock", r#"{"specifiers":{"jsr:@std/assert@^1.0.0":"1.0.13","npm:chalk@5":"5.3.0"}}"#),
        ]);
        let gradle = gradle_packages(&dir, "build.gradle.kts");
        assert_eq!(gradle.len(), 3);
        assert_eq!(gradle[1].wanted.as_deref(), Some("33.3.1-jre"));
        assert!(gradle[2].dev);
        let deno = deno_packages(&dir, "deno.json");
        assert_eq!(deno.iter().map(|p| (p.name.as_str(), p.installed.as_deref())).collect::<Vec<_>>(), vec![("jsr:@std/assert", Some("1.0.13")), ("npm:chalk", Some("5.3.0"))]);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn plans_commands_and_refuses_shell_syntax() {
        let dir = project(&[("composer.json", r#"{"require":{}}"#), ("requirements.txt", "Django==5.1\r\n# comment\r\nrequests\r\n")]);
        let path = OsString::new();
        let req = |manager, action, package: Option<&str>, version: Option<&str>| Request { manager, action, package: package.map(str::to_string), dev: false, version: version.map(str::to_string) };
        assert_eq!(plan(&dir, &path, &req(Manager::Composer, Action::Upgrade, Some("laravel/framework"), Some("v12.1.0"))).unwrap().command, "composer require laravel/framework:^12.1.0 --with-dependencies");
        for bad in ["x; rm -rf /", "x && y", "-g", "a b", "x|y", "$(id)", "x>1"] {
            assert!(plan(&dir, &path, &req(Manager::Composer, Action::Add, Some(bad), None)).is_err(), "{bad} must be refused");
        }
        assert!(plan(&dir, &path, &req(Manager::Cargo, Action::Add, Some("serde"), None)).is_err(), "the project doesn't use Cargo");
        apply_edit(&dir, &req(Manager::Pip, Action::Upgrade, Some("django"), Some("5.2"))).unwrap();
        apply_edit(&dir, &req(Manager::Pip, Action::Add, Some("httpx"), None)).unwrap();
        apply_edit(&dir, &req(Manager::Pip, Action::Remove, Some("Requests"), None)).unwrap();
        assert_eq!(std::fs::read_to_string(dir.join("requirements.txt")).unwrap(), "Django==5.2\r\n# comment\r\nhttpx\r\n");
        assert!(apply_edit(&dir, &req(Manager::Pip, Action::Add, Some("HTTPX"), None)).is_err(), "already listed");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn compares_versions_numerically() {
        assert!(version_key("v1.10.0") > version_key("1.9.3"));
        assert!(version_key("33.3.1-jre") == vec![33, 3, 1]);
        for pre in ["4.2.0-RC2", "6.0.0-M3", "2.0.0.Beta1", "1.0-SNAPSHOT", "3.0.0-alpha-1", "1.0.0.CR1"] {
            assert!(prerelease(pre), "{pre}");
        }
        for release in ["33.3.1-jre", "3.4.0", "2.17.2", "1.0.0.Final", "9.4.2.jre8"] {
            assert!(!prerelease(release), "{release}");
        }
    }
}

