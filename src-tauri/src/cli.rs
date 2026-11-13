//! `lamprabit`, the command line (phase 20). It talks to the running app through its control
//! channel (control.rs), starting LampRabit in the background first when it isn't running,
//! so everything goes through the same code as the buttons. `--json` prints the raw answer
//! for scripts. Exit codes: 0 done, 1 the request failed, 2 usage, 3 LampRabit couldn't be
//! reached, 4 `doctor` found errors.

use serde_json::{json, Value};

use crate::control;

const HELP: &str = "lamprabit: control LampRabit from a terminal or a script

Usage: lamprabit <command> [arguments] [--json] [--no-launch] [--yes] [--dry-run]

  status                         Version, services and how many projects
  services                       Every installed service and its state
  service start|stop|restart <id>
  start [<profile>]              Start everything installed, or a stack profile
  stop                           Stop every service
  profiles                       Stack profiles

  projects                       Projects in www, with their domains
  project start|stop <name>      A project's dev server
  project open <name>            Open its site in the browser
  project create <name> --template <id> [--php <version>] [--database <name>]
  templates                      Project templates

  runtimes [--available <id>]    Installed runtimes, or what <id> can install
  runtime install <id> [version] Download and use it (a line like 8.4 picks its newest)
  runtime use <id> <version>     Switch the version in use
  runtime remove <id> <version>  Delete a version (asks for --yes)

  logs [--source <id>] [--lines N] [--follow]
                                 Recent log lines (source: a service id, system, php,
                                 or project:<name>); --follow keeps printing new ones
  doctor <project> [--fix <id>|--fix-all]
                                 What stands between a project and working; --dry-run
                                 shows what the fixes would do without running them
  git <project>                  Branch, changes and remotes
  config [key.path]              Settings, or one value (ports.http)
  mcp                            Run the MCP server on stdin/stdout, for AI assistants

  --json       Print the answer as JSON
  --no-launch  Fail instead of starting LampRabit when it isn't running
  --yes        Confirm a destructive action (runtime remove, stop)
  --dry-run    Show what would happen, change nothing";

struct Args {
    words: Vec<String>,
    json: bool,
    launch: bool,
    yes: bool,
    dry_run: bool,
    follow: bool,
    options: Vec<(String, String)>,
}

fn parse(raw: Vec<String>) -> Args {
    let mut a = Args { words: Vec::new(), json: false, launch: true, yes: false, dry_run: false, follow: false, options: Vec::new() };
    let mut it = raw.into_iter();
    while let Some(w) = it.next() {
        match w.as_str() {
            "--json" => a.json = true,
            "--no-launch" => a.launch = false,
            "--yes" | "-y" => a.yes = true,
            "--dry-run" => a.dry_run = true,
            "--follow" | "-f" => a.follow = true,
            "--fix-all" => a.options.push(("fix-all".into(), String::new())),
            s if s.starts_with("--") => {
                let key = s.trim_start_matches("--").to_string();
                let (key, value) = match key.split_once('=') {
                    Some((k, v)) => (k.to_string(), v.to_string()),
                    None => (key, it.next().unwrap_or_default()),
                };
                a.options.push((key, value));
            }
            _ => a.words.push(w),
        }
    }
    a
}

impl Args {
    fn opt(&self, key: &str) -> Option<&str> {
        self.options.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }
    fn word(&self, i: usize) -> Option<&str> {
        self.words.get(i).map(String::as_str)
    }
}

enum Failure {
    Usage(String),
    Unreachable(String),
    Failed(String),
    DoctorErrors,
}

/// Runs the command line with the arguments after `--cli`; returns the exit code.
pub fn main(raw: Vec<String>) -> i32 {
    let args = parse(raw);
    if args.words.is_empty() || matches!(args.word(0), Some("help" | "-h" | "--help")) {
        println!("{HELP}");
        return if args.words.is_empty() { 2 } else { 0 };
    }
    if matches!(args.word(0), Some("--version" | "-V")) {
        println!("lamprabit {}", env!("CARGO_PKG_VERSION"));
        return 0;
    }
    let runtime = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("lamprabit: {e}");
            return 1;
        }
    };
    if args.word(0) == Some("mcp") {
        return runtime.block_on(crate::mcp::serve(args.launch));
    }
    match runtime.block_on(run(&args)) {
        Ok(()) => 0,
        Err(Failure::Usage(m)) => {
            eprintln!("lamprabit: {m}\nRun `lamprabit help` for the commands.");
            2
        }
        Err(Failure::Unreachable(m)) => {
            eprintln!("lamprabit: {m}");
            3
        }
        Err(Failure::Failed(m)) => {
            eprintln!("lamprabit: {m}");
            1
        }
        Err(Failure::DoctorErrors) => 4,
    }
}

async fn run(a: &Args) -> Result<(), Failure> {
    let need = |i: usize, what: &str| a.word(i).map(str::to_string).ok_or_else(|| Failure::Usage(format!("Missing {what}.")));
    // What to send, and how to print the answer.
    let (cmd, req, show): (&str, Value, fn(&Value)) = match (a.word(0).unwrap_or(""), a.word(1)) {
        ("status", _) => ("status", json!({}), print_status),
        ("services", _) => ("services", json!({}), print_services),
        ("service", Some(verb @ ("start" | "stop" | "restart"))) => {
            let cmd = match verb {
                "start" => "service.start",
                "stop" => "service.stop",
                _ => "service.restart",
            };
            (cmd, json!({"name": need(2, "the service id (lamprabit services lists them)")?}), print_done)
        }
        ("start", None) => ("start_all", json!({}), print_problems),
        ("start", Some(profile)) => ("profile.start", json!({"name": profile}), print_problems),
        ("stop", _) => ("stop_all", json!({}), print_done),
        ("profiles", _) => ("profiles", json!({}), print_profiles),
        ("projects", _) => ("projects", json!({}), print_projects),
        ("project", Some("start")) => ("project.start", json!({"name": need(2, "the project's name")?}), print_done),
        ("project", Some("stop")) => ("project.stop", json!({"name": need(2, "the project's name")?}), print_done),
        ("project", Some("open")) => ("project.open", json!({"name": need(2, "the project's name")?}), print_value),
        ("project", Some("create")) => {
            let name = need(2, "the new project's name")?;
            let template = a.opt("template").ok_or_else(|| Failure::Usage("Name a template with --template <id> (lamprabit templates lists them).".into()))?;
            let req = json!({
                "name": name, "template": template, "php": a.opt("php"), "server": null, "ssl": true,
                "database": a.opt("database"), "variables": {}, "optional": {},
            });
            ("project.create", req, print_value)
        }
        ("templates", _) => ("templates", json!({}), print_templates),
        ("runtimes", _) => match a.opt("available") {
            Some(id) => ("runtime.available", json!({"name": id}), print_releases),
            None => ("runtimes", json!({}), print_runtimes),
        },
        ("runtime", Some("install")) => ("runtime.install", json!({"name": need(2, "the runtime id")?, "version": a.word(3)}), print_installed),
        ("runtime", Some("use")) => ("runtime.use", json!({"name": need(2, "the runtime id")?, "version": need(3, "the version")?}), print_done),
        ("runtime", Some("remove")) => {
            let (id, version) = (need(2, "the runtime id")?, need(3, "the version")?);
            if !a.yes && !a.dry_run {
                return Err(Failure::Usage(format!("Removing {id} {version} deletes its files; add --yes to confirm.")));
            }
            ("runtime.remove", json!({"name": id, "version": version}), print_done)
        }
        ("logs", _) => ("logs", json!({"source": a.opt("source"), "lines": a.opt("lines").and_then(|n| n.parse::<u64>().ok()).unwrap_or(100)}), print_logs),
        ("doctor", _) => ("doctor", json!({"name": need(1, "the project's name")?}), print_doctor),
        ("git", _) => ("git.status", json!({"name": need(1, "the project's name")?}), print_git),
        ("config", _) => ("config", json!({}), print_value),
        (other, _) => return Err(Failure::Usage(format!("Unknown command \"{other}\"."))),
    };
    // A dry run of a change says what it would send, and stops.
    let changes = !matches!(cmd, "status" | "services" | "profiles" | "projects" | "templates" | "runtimes" | "runtime.available" | "logs" | "doctor" | "git.status" | "config");
    if a.dry_run && changes {
        println!("Would run {cmd} with {req}");
        return Ok(());
    }
    let endpoint = control::reach(a.launch).await.map_err(|e| Failure::Unreachable(e.to_string()))?;
    let answer = control::call(&endpoint, cmd, req.clone()).await.map_err(|e| Failure::Failed(e.to_string()))?;

    // config key.path: one value out of the settings.
    let answer = match (cmd, a.word(1)) {
        ("config", Some(path)) => path.split('.').try_fold(&answer, |v, k| v.get(k)).cloned().ok_or_else(|| Failure::Failed(format!("No setting {path}.")))?,
        _ => answer,
    };
    if a.json {
        println!("{}", serde_json::to_string_pretty(&answer).unwrap_or_default());
    } else {
        show(&answer);
    }

    if cmd == "logs" && a.follow {
        control::follow(&endpoint, |line| {
            if a.json {
                println!("{line}");
            } else {
                print_log_line(&line);
            }
            true
        })
        .await
        .map_err(|e| Failure::Failed(e.to_string()))?;
    }
    if cmd == "doctor" {
        let name = need(1, "the project's name")?;
        let issues = answer.as_array().cloned().unwrap_or_default();
        let fixes: Vec<&Value> = match (a.opt("fix"), a.opt("fix-all")) {
            (Some(id), _) => issues.iter().filter(|i| i["id"] == id).collect(),
            (None, Some(_)) => issues.iter().filter(|i| !i["fix"].is_null()).collect(),
            _ => Vec::new(),
        };
        if a.opt("fix").is_some() && fixes.is_empty() {
            return Err(Failure::Failed("No problem with that id (or it has no fix). `lamprabit doctor <project>` lists the ids.".into()));
        }
        for issue in fixes {
            let preview = issue["fix"]["preview"].as_str().unwrap_or_default();
            if a.dry_run {
                println!("Would fix {}: {preview}", issue["id"].as_str().unwrap_or_default());
                continue;
            }
            println!("Fixing {}: {preview}", issue["id"].as_str().unwrap_or_default());
            match control::call(&endpoint, "doctor.fix", json!({"name": name, "id": issue["id"]})).await {
                Ok(v) => println!("  done: {}", v.as_str().unwrap_or("ok")),
                Err(e) => println!("  failed: {e}"),
            }
        }
        if issues.iter().any(|i| i["severity"] == "error") && a.opt("fix").is_none() && a.opt("fix-all").is_none() {
            return Err(Failure::DoctorErrors);
        }
    }
    Ok(())
}

fn s<'a>(v: &'a Value, k: &str) -> &'a str {
    v[k].as_str().unwrap_or("")
}

fn print_value(v: &Value) {
    match v {
        Value::String(s) => println!("{s}"),
        Value::Null => println!("Done."),
        other => println!("{}", serde_json::to_string_pretty(other).unwrap_or_default()),
    }
}

fn print_done(_: &Value) {
    println!("Done.");
}

fn print_problems(v: &Value) {
    let problems = v.as_array().cloned().unwrap_or_default();
    if problems.is_empty() {
        println!("Started.");
    }
    for p in problems {
        println!("! {}", p.as_str().unwrap_or_default());
    }
}

fn print_status(v: &Value) {
    println!("LampRabit {} · {} · {} projects", s(v, "version"), s(v, "root"), v["projects"]);
    let services = v["services"].as_array().cloned().unwrap_or_default();
    let running = services.iter().filter(|x| x["state"] == "running").count();
    println!("{running} of {} services running", services.len());
    print_services(&v["services"]);
    if v["hostsOk"] == false {
        println!("! The hosts file is out of date (open LampRabit to update it).");
    }
}

fn print_services(v: &Value) {
    for x in v.as_array().into_iter().flatten().filter(|x| x["installed"] != false) {
        let id = x["id"].as_str().or(x["kind"].as_str()).unwrap_or_default();
        println!("  {:<14} {:<9} :{:<6} {}", id, s(x, "state"), x["port"], s(x, "version"));
    }
}

fn print_profiles(v: &Value) {
    for p in v.as_array().into_iter().flatten() {
        let services: Vec<&str> = p["services"].as_array().into_iter().flatten().filter_map(|x| x.as_str()).collect();
        println!("  {:<16} {}", s(p, "name"), services.join(", "));
    }
}

fn print_projects(v: &Value) {
    for p in v.as_array().into_iter().flatten() {
        let kind = p["framework"].as_str().unwrap_or(s(p, "language"));
        let branch = p["gitBranch"].as_str().map(|b| format!(" ({b})")).unwrap_or_default();
        println!("  {:<22} {:<34} {kind}{branch}{}", s(p, "name"), s(p, "url"), if p["running"] == true { " · running" } else { "" });
    }
}

fn print_templates(v: &Value) {
    for t in v["templates"].as_array().into_iter().flatten() {
        println!("  {:<14} {}", s(t, "id"), s(t, "name"));
    }
}

fn print_runtimes(v: &Value) {
    for r in v.as_array().into_iter().flatten().filter(|r| r["versions"].as_array().is_some_and(|a| !a.is_empty())) {
        let versions: Vec<&str> = r["versions"].as_array().into_iter().flatten().filter_map(|x| x.as_str()).collect();
        println!("  {:<14} {:<12} {}", s(r, "runtime"), r["active"].as_str().unwrap_or("-"), versions.join(", "));
    }
}

fn print_releases(v: &Value) {
    for r in v.as_array().into_iter().flatten() {
        println!("  {}{}", s(r, "version"), r["label"].as_str().map(|l| format!("  ({l})")).unwrap_or_default());
    }
}

fn print_installed(v: &Value) {
    println!("Installed and in use: {}", v.as_str().unwrap_or_default());
}

fn print_log_line(l: &Value) {
    let source = match l["project"].as_str() {
        Some(p) => format!("project:{p}"),
        None => s(l, "service").to_string(),
    };
    println!("[{source}] {}", s(l, "line"));
}

fn print_logs(v: &Value) {
    for l in v.as_array().into_iter().flatten() {
        print_log_line(l);
    }
}

fn print_doctor(v: &Value) {
    let issues = v.as_array().cloned().unwrap_or_default();
    if issues.is_empty() {
        println!("Everything checks out.");
    }
    for i in issues {
        let mark = match s(&i, "severity") {
            "error" => "✗",
            "warning" => "!",
            _ => "i",
        };
        println!("{mark} {}  [{}]", s(&i, "title"), s(&i, "id"));
        println!("    {}", s(&i, "detail"));
        if !i["fix"].is_null() {
            println!("    fix: {} (lamprabit doctor <project> --fix {})", i["fix"]["label"].as_str().unwrap_or_default(), s(&i, "id"));
        }
    }
}

fn print_git(v: &Value) {
    if v["repo"] == false {
        println!("Not a Git repository.");
        return;
    }
    let branch = v["branch"].as_str().unwrap_or("(detached)");
    let upstream = v["upstream"].as_str().map(|u| format!(" → {u} (↑{} ↓{})", v["ahead"], v["behind"])).unwrap_or_default();
    println!("{branch}{upstream}");
    let files = v["files"].as_array().cloned().unwrap_or_default();
    println!("{} changed {}", files.len(), if files.len() == 1 { "file" } else { "files" });
    for f in files.iter().take(30) {
        println!("  {}{} {}", s(f, "staged"), s(f, "unstaged"), s(f, "path"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_flags_and_options() {
        let a = parse(["doctor", "shop", "--fix", "env-missing", "--json", "--lines=20"].iter().map(|s| s.to_string()).collect());
        assert_eq!(a.words, ["doctor", "shop"]);
        assert!(a.json && a.launch);
        assert_eq!(a.opt("fix"), Some("env-missing"));
        assert_eq!(a.opt("lines"), Some("20"));
        let b = parse(["logs", "-f", "--no-launch", "--fix-all"].iter().map(|s| s.to_string()).collect());
        assert!(b.follow && !b.launch && b.opt("fix-all").is_some());
    }
}
