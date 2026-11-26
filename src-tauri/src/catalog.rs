//! What the Services page offers, from the backend: each runtime's group, label, what it's
//! for and where it comes from, where it can't run and why, and what it needs (with the
//! versions that work) or goes well with. The frontend builds its lists from this, so a new
//! runtime shows up without frontend tables; only its icon lives there.

use serde::Serialize;
use tauri::AppHandle;

use crate::{
    config::Settings,
    error::Result,
    layout::Layout,
    manifest,
    runtimes::{self, Release, Runtime},
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub id: &'static str,
    pub title: &'static str,
    pub blurb: &'static str,
    /// Runtime ids, plus "node" (NVM's versions) and "tunnels" (the tunnel list).
    pub items: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeInfo {
    pub id: &'static str,
    pub label: &'static str,
    pub blurb: &'static str,
    /// Where the download comes from, on this OS.
    pub source: &'static str,
    /// Why it can't be installed on this OS or machine, when it can't.
    pub unavailable: Option<&'static str>,
    /// Ids of what it needs; the versions depend on its own version (`requirements`).
    pub needs: Vec<&'static str>,
    /// Ids of what goes well with it: a UI for its data, its shell.
    pub suggests: Vec<&'static str>,
    /// Its ports from its manifest, edited on its Settings card and in Settings → Ports.
    pub ports: Vec<PortField>,
    /// Added by the user from a file (`<root>/services/<id>/`), so it can be removed.
    pub local: bool,
    /// Its web interface's port name (served at its .test domain), and a page of its own.
    pub ui_port: Option<String>,
    pub open: Option<manifest::OpenLink>,
    /// Other words it's found by, and its documentation.
    pub keywords: Vec<&'static str>,
    pub docs: Option<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortField {
    /// Its key in `Ports.extra`: `<service id>.<port name>`.
    pub key: String,
    /// What it's for ("HTTP", "Console"); none for a service's only port.
    pub label: Option<String>,
    pub hint: Option<String>,
    pub default: u16,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Catalog {
    pub groups: Vec<Group>,
    pub runtimes: Vec<RuntimeInfo>,
}

/// Services by what they do, in the order the Services page shows them.
fn groups() -> Vec<Group> {
    let g = |id, title, blurb, items: &[&'static str]| Group { id, title, blurb, items: items.to_vec() };
    vec![
        g("web", "Web and app servers", "Apache, Nginx, Caddy or FrankenPHP serve your sites; Tomcat runs Java web apps behind them. Several can run side by side.", &[]),
        g("languages", "Languages", "Several versions side by side; projects pick theirs.", &["php", "java", "python", "go", "rust", "node"]),
        g("packages", "Package managers", "Run them from LampRabit's terminal, or anywhere once LampRabit's commands are on PATH. npm comes with Node.js (pnpm and Yarn through corepack), pip with Python, Cargo with Rust; each project's Dependencies tab uses them.", &["composer", "bun", "deno", "uv", "conda", "maven", "gradle"]),
        // (Bun, Deno, uv, Adminer and mongosh have manifests; listing them here keeps their order.)
        g("databases", "Databases", "They can all run side by side, each on its own port.", &["sqlite"]),
        g("search", "Search", "Full-text search engines for your apps.", &[]),
        g("storage", "Storage", "S3-compatible object storage. Their storage folder can be anywhere, even outside the LampRabit folder.", &[]),
        g("queues", "Queues", "Message brokers for background jobs and events.", &["erlang"]),
        g("ai", "AI and automation", "Run language models locally and automate workflows.", &[]),
        g("cache", "Cache", "In-memory stores for caches, sessions and queues.", &[]),
        g("tools", "Developer tools", "Mail catching and web interfaces for your databases and caches.", &["git", "phpmyadmin", "adminer", "phppgadmin", "phpredisadmin", "compass", "mongosh", "mongotools"]),
        g("sharing", "Sharing", "Put a project or local port on a public URL.", &["cloudflared", "ngrok", "tunnels"]),
    ]
}

pub fn catalog() -> Catalog {
    let mut groups = groups();
    // One look at the added services, so one added meanwhile can't be listed but ungrouped.
    let extras = manifest::extras();
    let all: Vec<Runtime> = Runtime::BUILT_IN.iter().copied().chain(extras.iter().filter_map(|m| Runtime::from_id(&m.id))).collect();
    // Services go to the group their manifest names, before its other runtimes: LampRabit's
    // own bundled ones (KarotX) first, then the built-in ones in manifest order, then any
    // added from a file.
    let services = extras.iter().filter(|m| manifest::is_bundled(&m.id)).copied().chain(manifest::built_ins()).chain(extras.iter().filter(|m| !manifest::is_bundled(&m.id)).copied());
    let mut placed: Vec<(usize, &'static str)> = Vec::new();
    for m in services {
        if let Some(g) = groups.iter().position(|g| g.id == m.category) {
            placed.push((g, &m.id));
        }
    }
    for (g, group) in groups.iter_mut().enumerate() {
        // A runtime listed by hand above keeps its place there, manifest or not (Composer
        // stays ahead of Bun); the other manifests' services go first.
        let mut items: Vec<&'static str> = placed.iter().filter(|(i, id)| *i == g && !group.items.contains(id)).map(|(_, id)| *id).collect();
        items.append(&mut group.items);
        group.items = items;
    }
    // A runtime nobody put in a group still shows, under Other.
    let grouped: Vec<&str> = groups.iter().flat_map(|g| g.items.clone()).collect();
    let other: Vec<&'static str> = all.iter().map(|r| r.id()).filter(|id| !grouped.contains(id)).collect();
    if !other.is_empty() {
        groups.push(Group { id: "other", title: "Other", blurb: "More services.", items: other });
    }
    Catalog { groups, runtimes: all.into_iter().map(info).collect() }
}

fn info(runtime: Runtime) -> RuntimeInfo {
    // Services describe themselves in their manifest (src-tauri/services/<id>.yaml).
    if let Some(m) = manifest::get(runtime.id()) {
        return RuntimeInfo {
            id: runtime.id(),
            label: &m.name,
            blurb: &m.blurb,
            source: m.origin.as_ref().and_then(|o| o.pick()).unwrap_or_default(),
            unavailable: m.unavailable_here(),
            needs: needs(runtime, None).into_iter().map(|n| n.runtime).collect(),
            suggests: m.suggests.iter().map(String::as_str).collect(),
            ports: m
                .ports
                .iter()
                .map(|(name, d)| PortField { key: format!("{}.{name}", m.id), label: d.label.clone(), hint: d.hint.clone(), default: d.default })
                .collect(),
            local: manifest::extras().iter().any(|e| e.id == m.id) && !manifest::is_bundled(&m.id),
            ui_port: m.ui.as_ref().map(|u| u.port.clone().or_else(|| m.main_port().map(str::to_string)).unwrap_or_default()),
            open: m.open.clone(),
            keywords: m.keywords.iter().map(String::as_str).collect(),
            docs: m.docs.as_deref(),
        };
    }
    let win = cfg!(windows);
    let unix = |windows: &'static str, other: &'static str| if win { windows } else { other };
    let (label, blurb, source, unavailable): (&str, &str, &str, Option<&str>) = match runtime {
        Runtime::Php => ("PHP", "Runs your code", unix("Thread-safe builds from windows.php.net", "LampRabit's static builds with common extensions built in, Redis and Memcached clients included"), None),
        Runtime::Sqlite => ("SQLite", "File databases", "The sqlite3 shell from sqlite.org (SHA3 checked). PHP has SQLite built in, so projects don't need this to use it; backups use it to copy safely", None),
        Runtime::Rust => (
            "Rust",
            "Runs Rust code",
            unix(
                "Rust stable through rustup, kept inside LampRabit's folder (not your ~/.cargo), with the GNU toolchain, which needs no Visual Studio",
                "Rust stable through rustup, kept inside LampRabit's folder (not your ~/.cargo)",
            ),
            None,
        ),
        Runtime::Ngrok => ("Ngrok", "Public tunnels", "Share sites on a public URL (free account needed)", None),
        Runtime::Conda => ("conda", "Python environments", "Miniforge, conda-forge's installer (checksum checked), installed into LampRabit's folder without changing PATH or your shell; conda and mamba work in LampRabit's terminal", None),
        // Services: from their manifest, above.
        _ => (runtime.id(), "", "", None),
    };
    let suggests: &[&str] = match runtime {
        Runtime::Php => &["composer"],
        _ => &[],
    };
    RuntimeInfo {
        id: runtime.id(),
        label,
        blurb,
        source,
        unavailable,
        needs: needs(runtime, None).into_iter().map(|n| n.runtime).collect(),
        suggests: suggests.to_vec(),
        ports: vec![],
        local: false,
        ui_port: None,
        open: None,
        keywords: vec![],
        docs: None,
    }
}

/// One thing a runtime needs, and which of its versions work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Need {
    /// A runtime id, or "node" for NVM's Node.js.
    pub runtime: &'static str,
    /// Oldest version that works; the comparison stops at its last part ("17" takes 17.x).
    pub min: Option<String>,
    /// Newest version that works, by prefix ("3.14" takes 3.14.x).
    pub max: Option<String>,
    /// The version an install picks when several fit (pgAdmin's wheels are built for 3.13).
    pub prefer: Option<&'static str>,
    pub why: &'static str,
}

fn need(runtime: &'static str, min: Option<&str>, max: Option<&str>, why: &'static str) -> Need {
    Need { runtime, min: min.map(str::to_string), max: max.map(str::to_string), prefer: None, why }
}

/// What a runtime needs at `version` (or at any version, for listing): its manifest's
/// `needs`, which the start-up checks read too. Runtimes without a manifest need nothing.
pub fn needs(runtime: Runtime, version: Option<&str>) -> Vec<Need> {
    let Some(m) = manifest::get(runtime.id()) else { return Vec::new() };
    m.needs
        .iter()
        .filter(|n| n.applies(version))
        .map(|n| Need { runtime: &n.runtime, min: n.min.clone(), max: n.max.clone(), prefer: n.prefer.as_deref(), why: &n.why })
        .collect()
}

use crate::version::release as parts;

/// Compares only as many parts as `bound` has, missing parts counting as 0.
fn cmp(version: &[u64], bound: &[u64]) -> std::cmp::Ordering {
    (0..bound.len()).map(|i| version.get(i).unwrap_or(&0).cmp(&bound[i])).find(|o| o.is_ne()).unwrap_or(std::cmp::Ordering::Equal)
}

impl Need {
    pub fn fits(&self, version: &str) -> bool {
        let v = parts(version);
        self.min.as_deref().is_none_or(|m| cmp(&v, &parts(m)).is_ge()) && self.max.as_deref().is_none_or(|m| cmp(&v, &parts(m)).is_le())
    }

    /// "17 or newer", "3.10 to 3.14", "26.2 to 27.x", or "" for any version.
    fn range(&self) -> String {
        match (&self.min, &self.max) {
            (Some(min), Some(max)) if min == max => format!("{max}.x"),
            (Some(min), Some(max)) => format!("{min} to {max}.x"),
            (Some(min), None) => format!("{min} or newer"),
            (None, Some(max)) => format!("{max}.x or older"),
            (None, None) => String::new(),
        }
    }
}

/// What happens to one need before the runtime is installed.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Action {
    /// The active version fits already.
    Ready { version: String },
    /// An installed version fits but isn't the active one: it becomes active.
    Activate { version: String },
    /// Downloaded, then made active.
    Install { release: Release },
    /// A Node.js version installed through NVM, then made the default.
    InstallNode { version: String },
    /// NVM's Node.js that fits becomes the default.
    UseNode { version: String },
    /// Can't be done here; the reason says what to do instead.
    Unavailable { reason: String },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Requirement {
    pub runtime: &'static str,
    pub label: String,
    pub why: &'static str,
    /// Empty when any version works.
    pub range: String,
    pub action: Action,
}

/// What `runtime` at `version` (its active version, else its newest download, when None)
/// needs, and how each need gets met: the active version, another installed one, or the
/// download that fits.
pub async fn requirements(app: &AppHandle, client: &reqwest::Client, settings: &Settings, layout: &Layout, runtime: Runtime, version: Option<String>) -> Result<Vec<Requirement>> {
    let version = match version.or_else(|| settings.versions.get(runtime.id()).cloned()) {
        Some(v) => Some(v),
        None if needs(runtime, None).iter().any(|n| n.min.is_some() || n.max.is_some()) => {
            runtimes::available(client, runtime, false).await.ok().and_then(|c| c.first().map(|r| r.version.clone()))
        }
        None => None,
    };
    let mut out = Vec::new();
    for need in needs(runtime, version.as_deref()) {
        let label = Runtime::from_id(need.runtime).map_or("Node.js", |r| info(r).label).to_string();
        let action = if need.runtime == "node" { node_action(app, client, &need).await } else { runtime_action(client, settings, layout, &need, &label).await };
        out.push(Requirement { runtime: need.runtime, label, why: need.why, range: need.range(), action });
    }
    Ok(out)
}

/// How each of a new project's needs gets met: (runtime id or "node", oldest version that
/// works). New Project installs what's missing instead of sending the user to Services.
pub async fn project_requirements(app: &AppHandle, client: &reqwest::Client, settings: &Settings, layout: &Layout, list: &[(String, Option<String>)]) -> Vec<Requirement> {
    let mut out = Vec::new();
    for (id, min) in list {
        let runtime: &'static str = if id == "node" {
            "node"
        } else {
            match Runtime::from_id(id) {
                Some(r) => r.id(),
                None => continue,
            }
        };
        let need = need(runtime, min.as_deref(), None, "The new project runs on it");
        let label = Runtime::from_id(runtime).map_or("Node.js", |r| info(r).label).to_string();
        let action = if runtime == "node" { node_action(app, client, &need).await } else { runtime_action(client, settings, layout, &need, &label).await };
        out.push(Requirement { runtime, label, why: need.why, range: need.range(), action });
    }
    out
}

async fn runtime_action(client: &reqwest::Client, settings: &Settings, layout: &Layout, need: &Need, label: &str) -> Action {
    let Some(runtime) = Runtime::from_id(need.runtime) else {
        return Action::Unavailable { reason: format!("{label} can't be installed here") };
    };
    if let Some(active) = settings.versions.get(need.runtime).filter(|v| need.fits(v)) {
        return Action::Ready { version: active.clone() };
    }
    let installed = runtimes::installed(layout, runtime);
    if let Some(version) = newest(installed.iter().map(String::as_str), need) {
        return Action::Activate { version: version.to_string() };
    }
    if let Some(reason) = info(runtime).unavailable {
        return Action::Unavailable { reason: reason.to_string() };
    }
    match runtimes::available(client, runtime, false).await {
        Ok(catalog) => {
            let fitting = || catalog.iter().filter(|r| need.fits(&r.version));
            // Catalogs are newest first; an LTS line is the safer pick when there is one.
            let pick = need
                .prefer
                .and_then(|p| fitting().find(|r| r.version.starts_with(&format!("{p}."))))
                .or_else(|| fitting().find(|r| r.label.as_deref() == Some("LTS")))
                .or_else(|| fitting().next());
            match pick {
                Some(release) => Action::Install { release: release.clone() },
                None => Action::Unavailable { reason: format!("No {label} {} download for this platform", need.range()).split_whitespace().collect::<Vec<_>>().join(" ") },
            }
        }
        Err(e) => Action::Unavailable { reason: e.to_string() },
    }
}

async fn node_action(app: &AppHandle, client: &reqwest::Client, need: &Need) -> Action {
    let status = crate::node::status(app).await;
    if status.manager.is_none() {
        return Action::Unavailable { reason: "Node.js comes through NVM: install NVM on the Services page (Languages → Node.js) first.".into() };
    }
    if let Some(current) = status.current.as_deref().filter(|v| need.fits(v)) {
        return Action::Ready { version: current.to_string() };
    }
    if let Some(version) = newest(status.installed.iter().map(String::as_str), need) {
        return Action::UseNode { version: version.to_string() };
    }
    match crate::node::available(client).await {
        Ok(list) => {
            let fits = |r: &&crate::node::NodeRelease| need.fits(&r.version);
            match list.iter().filter(fits).find(|r| r.lts.is_some()).or_else(|| list.iter().find(fits)) {
                Some(r) => Action::InstallNode { version: r.version.trim_start_matches('v').to_string() },
                None => Action::Unavailable { reason: format!("No Node.js {} release found", need.range()) },
            }
        }
        Err(e) => Action::Unavailable { reason: e.to_string() },
    }
}

/// One component of a starter stack on the first run: how it gets met (like a need, with no
/// version bounds) and the download's size when it's a download.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StarterItem {
    pub runtime: &'static str,
    pub label: &'static str,
    pub blurb: &'static str,
    pub action: Action,
    /// Bytes, from the server's Content-Length; None when it doesn't say.
    pub size: Option<u64>,
}

/// The plan for a starter stack: each runtime's installed version, or the download that
/// would be used (LTS lines first, as for needs). Node.js isn't here: it comes through NVM,
/// which the wizard sets up itself.
pub async fn starter(client: &reqwest::Client, settings: &Settings, layout: &Layout, ids: &[String]) -> Vec<StarterItem> {
    let plans = ids.iter().filter_map(|id| Runtime::from_id(id)).map(|runtime| async move {
        let meta = info(runtime);
        let need = Need { runtime: runtime.id(), min: None, max: None, prefer: None, why: "" };
        let system_git = if runtime == Runtime::Git && settings.versions.get("git").is_none() { crate::git::system_version().await } else { None };
        let action = match system_git {
            Some(version) => Action::Ready { version: format!("{version} (already on this computer)") },
            None => runtime_action(client, settings, layout, &need, meta.label).await,
        };
        let size = match &action {
            Action::Install { release } => download_size(client, &release.url).await,
            _ => None,
        };
        StarterItem { runtime: runtime.id(), label: meta.label, blurb: meta.blurb, action, size }
    });
    futures_util::future::join_all(plans).await
}

/// A download's size from a HEAD request (redirects followed); a few seconds at most.
async fn download_size(client: &reqwest::Client, url: &str) -> Option<u64> {
    let response = tokio::time::timeout(std::time::Duration::from_secs(6), client.head(url).send()).await.ok()?.ok()?;
    response.headers().get(reqwest::header::CONTENT_LENGTH)?.to_str().ok()?.parse().ok().filter(|n| *n > 0)
}

fn newest<'a>(versions: impl Iterator<Item = &'a str>, need: &Need) -> Option<&'a str> {
    versions.filter(|v| need.fits(v)).max_by(|a, b| cmp(&parts(a), &parts(b)).then(parts(a).len().cmp(&parts(b).len())))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Writes the default settings and the catalog as JSON for the UI screenshot harness
    /// (scripts/screenshots): `cargo test --lib write_ui_fixtures -- --ignored`.
    #[test]
    #[ignore]
    fn write_ui_fixtures() {
        let out = serde_json::json!({ "settings": Settings::default(), "catalog": catalog() });
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../scripts/screenshots/fixtures.json");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, serde_json::to_string_pretty(&out).unwrap()).unwrap();
    }

    #[test]
    fn ranges_compare_by_the_bounds_parts() {
        let java17 = need("java", Some("17"), None, "");
        assert!(java17.fits("17.0.13+11") && java17.fits("21.0.5") && !java17.fits("11.0.25"));
        let python = need("python", Some("3.10"), Some("3.14"), "");
        assert!(python.fits("3.14.2") && python.fits("3.10.0") && !python.fits("3.9.21") && !python.fits("3.15.0"));
        assert!(need("node", Some("24"), None, "").fits("v24.11.1"));
        assert_eq!(python.range(), "3.10 to 3.14.x");
        assert_eq!(java17.range(), "17 or newer");
    }

    #[test]
    fn tomcat_lines_need_their_java() {
        let java = |v| needs(Runtime::Tomcat, Some(v))[0].min.clone().unwrap();
        assert_eq!(java("9.0.112"), "8");
        assert_eq!(java("10.1.48"), "11");
        assert_eq!(java("11.0.14"), "17");
    }

    #[test]
    fn rabbitmq_follows_its_erlang_table() {
        // Its manifest's rows (they're Windows-only, so read them directly here).
        let rows = &manifest::get("rabbitmq").unwrap().needs;
        let erlang = |v: &str| rows.iter().find(|n| n.runtime == "erlang" && (n.since.as_deref().is_none_or(|s| crate::version::numbers(v) >= crate::version::numbers(s))) && n.until.as_deref().is_none_or(|u| crate::version::numbers(v) < crate::version::numbers(u))).map(|n| (n.min.clone().unwrap(), n.max.clone().unwrap()));
        let pair = |a: &str, b: &str| Some((a.to_string(), b.to_string()));
        assert_eq!(erlang("4.0.2"), pair("26.2", "26.2"));
        assert_eq!(erlang("4.1.4"), pair("26.2", "27"));
        assert_eq!(erlang("4.2.8"), pair("26.2", "27"));
        assert_eq!(erlang("4.2.10"), pair("27.0", "27"));
        assert_eq!(erlang("4.3.1"), pair("26.2", "27"));
        assert_eq!(erlang("4.3.6"), pair("27.0", "28"));
        assert_eq!(erlang("4.4.0"), pair("27.0", "29"));
        let otp = need("erlang", Some("26.2"), Some("27"), "");
        assert!(otp.fits("27.3.4") && otp.fits("26.2.5") && !otp.fits("28.1") && !otp.fits("26.1"));
    }

    #[test]
    fn build_tools_need_a_java_they_run_on() {
        let java = |r, v| needs(r, Some(v)).remove(0);
        assert_eq!(java(Runtime::Maven, "3.9.11").min.as_deref(), Some("8"));
        assert_eq!(java(Runtime::Maven, "4.0.1").min.as_deref(), Some("17"));
        let gradle9 = java(Runtime::Gradle, "9.2.1");
        assert!(gradle9.fits("21.0.9") && gradle9.fits("25.0.1") && !gradle9.fits("11.0.29"));
        let gradle8 = java(Runtime::Gradle, "8.14.3");
        assert!(gradle8.fits("8.0.472") && gradle8.fits("21.0.9") && !gradle8.fits("25.0.1"), "Gradle 8 can't run on Java 25");
        assert!(java(Runtime::Gradle, "9.8.0").fits("27.0.1") && !java(Runtime::Gradle, "9.4.1").fits("27.0.1"));
    }

    #[test]
    fn every_runtime_is_grouped_and_named() {
        let catalog = catalog();
        assert!(catalog.groups.iter().all(|g| g.id != "other"), "add new runtimes to a group: {:?}", catalog.groups.iter().find(|g| g.id == "other").map(|g| &g.items));
        // Services other tests add from a file (the shared list) aren't LampRabit's own.
        for r in catalog.runtimes.iter().filter(|r| !r.local) {
            assert!(!r.label.is_empty() && !r.source.is_empty(), "{}", r.id);
            for id in r.needs.iter().chain(&r.suggests) {
                assert!(*id == "node" || Runtime::from_id(id).is_some(), "{} names {id}", r.id);
            }
        }
    }
}
