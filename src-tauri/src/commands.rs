//! Commands a project can run from its Sites panel: discovered from the project (package.json
//! and composer.json scripts, and the framework CLIs' own command lists), listed by the
//! template it came from, and added by the user. apps.rs runs them.

use std::{ffi::OsString, path::Path, process::Stdio, time::Duration};

use serde::{Deserialize, Serialize};

use crate::{
    apps,
    config::PackageManager,
    procgroup,
    projects::{shell, strip_ansi},
};

/// Where commands come from. Templates name them in `commands: discover:`; projects without
/// one get every source their files show.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Source {
    #[serde(rename = "package.json")]
    PackageJson,
    #[serde(rename = "composer.json")]
    ComposerJson,
    /// Laravel's `php artisan`.
    #[serde(rename = "artisan")]
    Artisan,
    /// Symfony's `php bin/console`.
    #[serde(rename = "console")]
    Console,
    /// CakePHP's `bin/cake`.
    #[serde(rename = "cake")]
    Cake,
    /// CodeIgniter's `php spark`.
    #[serde(rename = "spark")]
    Spark,
    /// `php bin/magento`.
    #[serde(rename = "magento")]
    Magento,
    /// Statamic's `php please`.
    #[serde(rename = "please")]
    Please,
    /// Drupal's `vendor/bin/drush`.
    #[serde(rename = "drush")]
    Drush,
    /// deno.json's tasks.
    #[serde(rename = "deno.json")]
    DenoJson,
    /// pyproject.toml's scripts (uv, Poetry, PDM or the project's own environment).
    #[serde(rename = "pyproject.toml")]
    Pyproject,
    /// requirements.txt: installing it into the project's virtual environment.
    #[serde(rename = "requirements.txt")]
    Requirements,
    /// Maven's lifecycle phases (through the project's mvnw when it has one).
    #[serde(rename = "pom.xml")]
    Maven,
    /// Gradle's common tasks (through the project's gradlew when it has one).
    #[serde(rename = "build.gradle")]
    Gradle,
    /// Cargo's commands and the package's binaries.
    #[serde(rename = "Cargo.toml")]
    Cargo,
    /// Go's build, test and run.
    #[serde(rename = "go.mod")]
    GoMod,
}

impl Source {
    pub const ALL: [Source; 16] = [
        Source::PackageJson,
        Source::ComposerJson,
        Source::Artisan,
        Source::Console,
        Source::Cake,
        Source::Spark,
        Source::Magento,
        Source::Please,
        Source::Drush,
        Source::DenoJson,
        Source::Pyproject,
        Source::Requirements,
        Source::Maven,
        Source::Gradle,
        Source::Cargo,
        Source::GoMod,
    ];

    /// Whether the project's files show this source.
    fn present(self, dir: &Path) -> bool {
        match self {
            Source::DenoJson => dir.join("deno.json").exists() || dir.join("deno.jsonc").exists(),
            Source::Gradle => ["build.gradle", "build.gradle.kts"].iter().any(|f| dir.join(f).exists()),
            _ => dir.join(self.marker()).exists(),
        }
    }

    /// The file that shows a project has this source.
    fn marker(self) -> &'static str {
        match self {
            Source::PackageJson => "package.json",
            Source::ComposerJson => "composer.json",
            Source::Artisan => "artisan",
            Source::Console => "bin/console",
            Source::Cake => "bin/cake.php",
            Source::Spark => "spark",
            Source::Magento => "bin/magento",
            Source::Please => "please",
            Source::Drush => "vendor/bin/drush",
            Source::DenoJson => "deno.json",
            Source::Pyproject => "pyproject.toml",
            Source::Requirements => "requirements.txt",
            Source::Maven => "pom.xml",
            Source::Gradle => "build.gradle",
            Source::Cargo => "Cargo.toml",
            Source::GoMod => "go.mod",
        }
    }

    fn group(self) -> &'static str {
        match self {
            Source::PackageJson => "package.json scripts",
            Source::ComposerJson => "Composer scripts",
            Source::Artisan => "Artisan",
            Source::Console => "Symfony console",
            Source::Cake => "CakePHP",
            Source::Spark => "Spark",
            Source::Magento => "Magento",
            Source::Please => "Statamic",
            Source::Drush => "Drush",
            Source::DenoJson => "Deno tasks",
            Source::Pyproject => "pyproject.toml scripts",
            Source::Requirements => "pip",
            Source::Maven => "Maven",
            Source::Gradle => "Gradle",
            Source::Cargo => "Cargo",
            Source::GoMod => "Go",
        }
    }

    /// How to run one of the CLI's commands, and how to list them (Symfony-console CLIs
    /// all answer `list --format=json`).
    fn cli(self) -> Option<(&'static str, &'static str)> {
        match self {
            Source::Artisan => Some(("php artisan", "php artisan list --format=json")),
            Source::Console => Some(("php bin/console", "php bin/console list --format=json")),
            Source::Magento => Some(("php bin/magento", "php bin/magento list --format=json")),
            Source::Please => Some(("php please", "php please list --format=json")),
            Source::Drush => Some(("php vendor/bin/drush", "php vendor/bin/drush list --format=json")),
            Source::Cake => Some(("php bin/cake.php", "php bin/cake.php")),
            Source::Spark => Some(("php spark", "php spark list --simple")),
            _ => None,
        }
    }
}

/// The sources a project's files show.
pub fn detect(dir: &Path) -> Vec<Source> {
    Source::ALL.into_iter().filter(|s| s.present(dir)).collect()
}

/// A command the user added to a project (saved in its settings).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct UserCommand {
    pub name: String,
    pub run: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Command {
    /// Heading it's listed under ("Artisan", "package.json scripts", "Yours"…).
    pub group: String,
    pub name: String,
    /// What runs, in the project folder.
    pub command: String,
    pub description: Option<String>,
    /// "yours", "template" or "discovered".
    pub origin: &'static str,
}

/// Runs a listing command with a time limit; its stdout, or why it failed.
async fn output(dir: &Path, path: &OsString, command: &str) -> Result<String, String> {
    let (program, args) = shell(command);
    let mut cmd = tokio::process::Command::new(program);
    cmd.args(args)
        .current_dir(dir)
        .env("PATH", path)
        .env("NO_COLOR", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    procgroup::prepare(&mut cmd);
    let child = cmd.spawn().map_err(|e| e.to_string())?;
    match tokio::time::timeout(Duration::from_secs(45), child.wait_with_output()).await {
        Ok(Ok(out)) if out.status.success() => Ok(String::from_utf8_lossy(&out.stdout).into_owned()),
        Ok(Ok(out)) => {
            let last = crate::error::last_line(&out.stderr).unwrap_or_else(|| "it failed".into());
            Err(strip_ansi(&last))
        }
        Ok(Err(e)) => Err(e.to_string()),
        Err(_) => Err("it took longer than 45 seconds".into()),
    }
}

/// `list --format=json` from a Symfony-console CLI: its visible commands.
fn parse_console_json(text: &str) -> Vec<(String, Option<String>)> {
    // Some CLIs print a warning line before the JSON.
    let json = text.find('{').map_or(text, |i| &text[i..]);
    let Ok(value) = serde_json::from_str::<serde_json::Value>(json) else { return Vec::new() };
    value["commands"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|c| c["hidden"] != true)
        .filter_map(|c| {
            let name = c["name"].as_str()?.to_string();
            // `list`, `help` and `_complete` are about the CLI itself.
            (!["list", "help", "completion", "_complete"].contains(&name.as_str())).then(|| (name, c["description"].as_str().filter(|d| !d.is_empty()).map(str::to_string)))
        })
        .collect()
}

/// CakePHP's command list: lines like ` - cache clear`.
fn parse_cake(text: &str) -> Vec<(String, Option<String>)> {
    text.lines().filter_map(|l| l.trim().strip_prefix("- ")).map(|n| (n.trim().to_string(), None)).filter(|(n, _)| !n.is_empty() && n != "help" && n != "version").collect()
}

/// `php spark list --simple`: one command per line.
fn parse_spark(text: &str) -> Vec<(String, Option<String>)> {
    text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.contains(' ') && *l != "help" && *l != "list").map(|n| (n.to_string(), None)).collect()
}

/// Every command a project offers from the given sources. Problems (a CLI that didn't
/// answer) are returned instead of failing the whole list.
pub async fn discover(dir: &Path, sources: &[Source], path: &OsString, pm: PackageManager) -> (Vec<Command>, Vec<String>) {
    let mut commands = Vec::new();
    let mut problems = Vec::new();
    for &source in sources {
        if !source.present(dir) {
            continue;
        }
        let group = source.group().to_string();
        let discovered = |name: String, command: String, description: Option<String>| Command { group: group.clone(), name, command, description, origin: "discovered" };
        match source {
            Source::PackageJson => {
                let run = apps::pm_commands(pm, path).map(|p| p.run).unwrap_or_else(|_| format!("{} run", pm.id()));
                for (name, script) in apps::scripts(dir) {
                    commands.push(discovered(name.clone(), format!("{run} {name}"), Some(script)));
                }
            }
            Source::ComposerJson => {
                let json: serde_json::Value = std::fs::read_to_string(dir.join("composer.json")).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
                for (name, script) in json["scripts"].as_object().into_iter().flatten() {
                    // Composer runs pre-/post- hooks on its own events.
                    if name.starts_with("pre-") || name.starts_with("post-") {
                        continue;
                    }
                    let description = json["scripts-descriptions"][name].as_str().map(str::to_string).or_else(|| match script {
                        serde_json::Value::String(s) => Some(s.clone()),
                        serde_json::Value::Array(a) => Some(a.iter().filter_map(|x| x.as_str()).collect::<Vec<_>>().join(" && ")),
                        _ => None,
                    });
                    commands.push(discovered(name.clone(), format!("composer run-script {name}"), description));
                }
            }
            Source::DenoJson | Source::Pyproject | Source::Requirements | Source::Maven | Source::Gradle | Source::Cargo | Source::GoMod => {
                for (name, command, description) in build_commands(dir, source) {
                    commands.push(discovered(name, command, description));
                }
            }
            cli => {
                let Some((prefix, list)) = cli.cli() else { continue };
                match output(dir, path, list).await {
                    Ok(text) => {
                        let found = match cli {
                            Source::Cake => parse_cake(&text),
                            Source::Spark => parse_spark(&text),
                            _ => parse_console_json(&text),
                        };
                        if found.is_empty() {
                            problems.push(format!("{} listed no commands.", source.group()));
                        }
                        for (name, description) in found {
                            commands.push(discovered(name.clone(), format!("{prefix} {name}"), description));
                        }
                    }
                    Err(why) => problems.push(format!("Couldn't list {} commands: {why}", source.group())),
                }
            }
        }
    }
    (commands, problems)
}

/// How a Python project runs its scripts: uv, Poetry or PDM when their lock file is there,
/// else the project's virtual environment's Python.
pub fn python_runner(dir: &Path) -> Option<&'static str> {
    if dir.join("uv.lock").exists() {
        Some("uv run")
    } else if dir.join("poetry.lock").exists() {
        Some("poetry run")
    } else if dir.join("pdm.lock").exists() {
        Some("pdm run")
    } else {
        None
    }
}

/// The commands of the build tools and languages whose files list them (or whose commands
/// are always the same), read from the files without running anything.
fn build_commands(dir: &Path, source: Source) -> Vec<(String, String, Option<String>)> {
    let fixed = |prefix: &str, list: &[(&str, &str)]| list.iter().map(|(n, d)| (n.to_string(), format!("{prefix} {n}"), Some(d.to_string()))).collect::<Vec<_>>();
    let read = |file: &str| std::fs::read_to_string(dir.join(file)).unwrap_or_default();
    match source {
        Source::DenoJson => {
            let text = if dir.join("deno.json").exists() { read("deno.json") } else { read("deno.jsonc") };
            let json: serde_json::Value = serde_json::from_str(&strip_json_comments(&text)).unwrap_or_default();
            json["tasks"]
                .as_object()
                .into_iter()
                .flatten()
                .map(|(name, task)| {
                    // A task is a command line, or { command, description }.
                    let description = task.as_str().or_else(|| task["description"].as_str()).or_else(|| task["command"].as_str()).map(str::to_string);
                    (name.clone(), format!("deno task {name}"), description)
                })
                .collect()
        }
        Source::Pyproject => {
            let toml: toml::Value = read("pyproject.toml").parse().unwrap_or(toml::Value::Table(Default::default()));
            let runner = python_runner(dir);
            let mut out = Vec::new();
            let tables = [(toml.get("project").and_then(|p| p.get("scripts")), runner.unwrap_or("")), (toml.get("tool").and_then(|t| t.get("poetry")).and_then(|p| p.get("scripts")), "poetry run")];
            for (table, prefix) in tables {
                for (name, target) in table.and_then(|t| t.as_table()).into_iter().flatten() {
                    out.push((name.clone(), format!("{prefix} {name}").trim().to_string(), target.as_str().map(str::to_string)));
                }
            }
            for (name, script) in toml.get("tool").and_then(|t| t.get("pdm")).and_then(|p| p.get("scripts")).and_then(|s| s.as_table()).into_iter().flatten() {
                if name != "_" {
                    let description = script.as_str().map(str::to_string).or_else(|| script.get("help").and_then(|h| h.as_str()).map(str::to_string));
                    out.push((name.clone(), format!("pdm run {name}"), description));
                }
            }
            if runner == Some("uv run") {
                out.push(("sync".into(), "uv sync".into(), Some("Install the locked dependencies into .venv".into())));
            }
            out
        }
        Source::Requirements => {
            vec![("install".into(), crate::deps::pip_install_command(dir), Some("Install requirements.txt into the project's .venv".into()))]
        }
        Source::Maven => {
            let mvn = crate::deps::wrapper(dir, "mvnw", "mvn");
            let mut out = fixed(&format!("{mvn} -B"), &[("compile", "Compile the sources"), ("test", "Run the tests"), ("package", "Build the jar or war"), ("verify", "Run the checks"), ("clean", "Delete target/")]);
            if read("pom.xml").contains("spring-boot-maven-plugin") {
                out.push(("spring-boot:run".into(), format!("{mvn} spring-boot:run"), Some("Run the Spring Boot app".into())));
            }
            out
        }
        Source::Gradle => {
            let gradle = crate::deps::wrapper(dir, "gradlew", "gradle");
            let build = read("build.gradle") + &read("build.gradle.kts");
            let mut out = fixed(&gradle, &[("build", "Compile, test and assemble"), ("test", "Run the tests"), ("clean", "Delete build/"), ("tasks", "List every task")]);
            if build.contains("org.springframework.boot") {
                out.push(("bootRun".into(), format!("{gradle} bootRun"), Some("Run the Spring Boot app".into())));
            } else if build.contains("application") {
                out.push(("run".into(), format!("{gradle} run"), Some("Run the application".into())));
            }
            out
        }
        Source::Cargo => {
            let toml: toml::Value = read("Cargo.toml").parse().unwrap_or(toml::Value::Table(Default::default()));
            let mut out = fixed("cargo", &[("build", "Compile the package"), ("run", "Build and run the main binary"), ("test", "Run the tests"), ("check", "Check for errors without building"), ("clippy", "Lint the code"), ("fmt", "Format the code")]);
            let bins: Vec<String> = toml.get("bin").and_then(|b| b.as_array()).into_iter().flatten().filter_map(|b| b.get("name")?.as_str().map(str::to_string)).collect();
            if bins.len() > 1 {
                for bin in bins {
                    out.push((format!("run --bin {bin}"), format!("cargo run --bin {bin}"), Some(format!("Build and run {bin}"))));
                }
            }
            out
        }
        Source::GoMod => fixed("go", &[("build ./...", "Compile every package"), ("test ./...", "Run the tests"), ("run .", "Build and run the main package"), ("vet ./...", "Report likely mistakes"), ("mod tidy", "Add missing and remove unused modules")]),
        _ => Vec::new(),
    }
}

/// deno.jsonc allows // and /* */ comments; serde_json doesn't.
pub fn strip_json_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let mut in_string = false;
    while let Some(c) = chars.next() {
        if in_string {
            out.push(c);
            if c == '\\' {
                if let Some(n) = chars.next() {
                    out.push(n);
                }
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }
        match (c, chars.peek()) {
            ('"', _) => {
                in_string = true;
                out.push(c);
            }
            ('/', Some('/')) => {
                for n in chars.by_ref() {
                    if n == '\n' {
                        out.push('\n');
                        break;
                    }
                }
            }
            ('/', Some('*')) => {
                chars.next();
                let mut last = ' ';
                for n in chars.by_ref() {
                    if last == '*' && n == '/' {
                        break;
                    }
                    last = n;
                }
            }
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_build_tool_commands() {
        let dir = std::env::temp_dir().join(format!("lamprabit-cmd-build-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("deno.jsonc"), "{\n // tasks\n \"tasks\": { \"dev\": \"deno run -A main.ts\", \"url\": \"echo http://x\" } }").unwrap();
        std::fs::write(dir.join("pyproject.toml"), "[project]\nname = \"x\"\n[project.scripts]\nserve = \"x.app:main\"\n").unwrap();
        std::fs::write(dir.join("uv.lock"), "").unwrap();
        std::fs::write(dir.join("Cargo.toml"), "[package]\nname = \"x\"\n[[bin]]\nname = \"a\"\n[[bin]]\nname = \"b\"\n").unwrap();
        let deno = build_commands(&dir, Source::DenoJson);
        assert_eq!(deno.iter().map(|c| c.1.as_str()).collect::<Vec<_>>(), vec!["deno task dev", "deno task url"], "comments and // inside strings");
        let py = build_commands(&dir, Source::Pyproject);
        assert!(py.iter().any(|c| c.1 == "uv run serve") && py.iter().any(|c| c.1 == "uv sync"));
        assert!(build_commands(&dir, Source::Cargo).iter().any(|c| c.1 == "cargo run --bin b"));
        assert!(Source::DenoJson.present(&dir) && !Source::Gradle.present(&dir));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn reads_symfony_console_lists() {
        let text = r#"Deprecated: something
{"application":{"name":"Laravel"},"commands":[
  {"name":"list","description":"List commands","hidden":false},
  {"name":"migrate","description":"Run the database migrations","hidden":false},
  {"name":"secret:thing","description":"","hidden":true},
  {"name":"about","description":"","hidden":false}]}"#;
        let found = parse_console_json(text);
        assert_eq!(found, vec![("migrate".into(), Some("Run the database migrations".into())), ("about".into(), None)]);
    }

    #[test]
    fn reads_cake_and_spark_lists() {
        let cake = "Current Paths:\n\nAvailable Commands:\n\nCakePHP:\n - cache clear\n - cache clear_all\n - help\n";
        assert_eq!(parse_cake(cake).into_iter().map(|c| c.0).collect::<Vec<_>>(), vec!["cache clear", "cache clear_all"]);
        let spark = "cache:clear\nmigrate\n\nhelp\nlist\n";
        assert_eq!(parse_spark(spark).into_iter().map(|c| c.0).collect::<Vec<_>>(), vec!["cache:clear", "migrate"]);
    }

    #[test]
    fn sources_from_files() {
        let dir = std::env::temp_dir().join(format!("lamprabit-cmd-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("bin")).unwrap();
        for f in ["package.json", "composer.json", "artisan", "bin/console"] {
            std::fs::write(dir.join(f), "{}").unwrap();
        }
        assert_eq!(detect(&dir), vec![Source::PackageJson, Source::ComposerJson, Source::Artisan, Source::Console]);
        let _ = std::fs::remove_dir_all(dir);
    }
}
