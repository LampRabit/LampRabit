//! Bringing projects over from other local environments (Laragon, XAMPP, Herd…).
//!
//! Each tool is found by its usual folders and config files; its projects are listed with
//! what they are and the database they name, and the ones picked are copied into www/.
//! The originals are never changed, so the other tool keeps working. Databases stay in the
//! other tool's server: their names are listed so the user knows what to export.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::{Error, Result};
use crate::layout::Layout;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub id: String,
    pub name: String,
    /// Where its projects were found, to show the user.
    pub folders: Vec<PathBuf>,
    pub projects: Vec<ForeignProject>,
    /// Databases in its MySQL/MariaDB data folders (not LampRabit's).
    pub databases: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForeignProject {
    /// The folder's own name.
    pub name: String,
    pub path: PathBuf,
    /// "Laravel", "WordPress", "Next.js"… None when nothing tells.
    pub framework: Option<&'static str>,
    /// The database its .env or wp-config.php names.
    pub database: Option<String>,
    /// A free www/ name for it: the folder's, made a valid domain, numbered when taken.
    pub target: String,
}

/// The tools looked for, in the order they're shown.
const TOOLS: &[(&str, &str)] = &[
    ("laragon", "Laragon"),
    ("xampp", "XAMPP"),
    ("ampps", "AMPPS"),
    ("mamp", "MAMP"),
    ("herd", "Laravel Herd"),
    ("valet", "Laravel Valet"),
    ("flyenv", "FlyEnv"),
    ("envkit", "EnvKit"),
    ("wamp", "WampServer"),
    ("servbay", "ServBay"),
    ("local", "Local"),
];

/// Folders in a web root that belong to the tool, not to the user.
const TOOL_FOLDERS: &[&str] = &["dashboard", "img", "webalizer", "xampp", "forbidden", "restricted", "ampps", "mamp", "mamp-pro", "cgi-bin", "wamplangues", "wampthemes", "phpmyadmin", "adminer"];

/// The servers' own databases.
const SYSTEM_DATABASES: &[&str] = &["mysql", "information_schema", "performance_schema", "sys", "phpmyadmin", "test"];

/// What a tool's files say: folders whose subfolders are projects, single project folders
/// (a linked site, a FlyEnv host), and MySQL data folders.
#[derive(Default)]
struct Found {
    parents: Vec<PathBuf>,
    projects: Vec<(String, PathBuf)>,
    data: Vec<PathBuf>,
}

/// Every tool found on this computer, with its projects. LampRabit's own folder is left out.
pub fn detect(layout: &Layout) -> Vec<Source> {
    let home = dirs::home_dir().unwrap_or_default();
    let mut taken = Vec::new();
    TOOLS
        .iter()
        .filter_map(|(id, name)| {
            let found = find(id, &home);
            let source = build(id, name, found, layout, &mut taken);
            (!source.projects.is_empty() || !source.databases.is_empty()).then_some(source)
        })
        .collect()
}

/// A folder the user picked (EnvKit or any tool not listed): its subfolders are the projects.
pub fn from_folder(dir: &Path, layout: &Layout) -> Result<Source> {
    if !dir.is_dir() {
        return Err(Error::msg(format!("{} isn't a folder.", dir.display())));
    }
    let found = Found { parents: vec![dir.to_path_buf()], ..Default::default() };
    let source = build("folder", "Chosen folder", found, layout, &mut Vec::new());
    if source.projects.is_empty() {
        return Err(Error::msg(format!("{} has no project folders in it. Pick the folder that holds your projects (often www, htdocs or Sites).", dir.display())));
    }
    Ok(source)
}

fn build(id: &str, name: &str, found: Found, layout: &Layout, taken: &mut Vec<String>) -> Source {
    let root = std::fs::canonicalize(layout.root()).unwrap_or_else(|_| layout.root().to_path_buf());
    let ours = |p: &Path| {
        let p = std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
        p.starts_with(&root) || root.starts_with(&p)
    };
    let mut candidates: Vec<(String, PathBuf)> = Vec::new();
    let mut folders = Vec::new();
    for parent in found.parents.iter().filter(|p| p.is_dir() && !ours(p)) {
        folders.push(parent.clone());
        let Ok(entries) = std::fs::read_dir(parent) else { continue };
        for e in entries.flatten() {
            let Ok(folder) = e.file_name().into_string() else { continue };
            // A link (a Herd/Valet parked folder can hold some) is followed: its target is the project.
            if folder.starts_with('.') || TOOL_FOLDERS.contains(&folder.to_lowercase().as_str()) || !e.path().is_dir() {
                continue;
            }
            candidates.push((folder, e.path()));
        }
    }
    for (folder, path) in found.projects {
        if path.is_dir() && !ours(&path) {
            folders.extend(path.parent().map(Path::to_path_buf).filter(|p| !folders.contains(p)));
            candidates.push((folder, path));
        }
    }
    candidates.sort_by_key(|c| c.0.to_lowercase());
    candidates.dedup_by(|a, b| same(&a.1, &b.1));

    let projects = candidates
        .into_iter()
        .filter(|(_, path)| std::fs::read_dir(path).is_ok_and(|mut d| d.next().is_some()))
        .map(|(folder, path)| {
            let (framework, database) = crate::sites::describe(&path);
            let target = free_name(&folder, layout, taken);
            taken.push(target.clone());
            ForeignProject { name: folder, path, framework, database, target }
        })
        .collect();

    let mut databases: Vec<String> = found
        .data
        .iter()
        .filter_map(|d| std::fs::read_dir(d).ok())
        .flat_map(|d| d.flatten())
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| !n.starts_with(['#', '.']) && !SYSTEM_DATABASES.contains(&n.to_lowercase().as_str()))
        .collect();
    databases.sort();
    databases.dedup();
    Source { id: id.to_string(), name: name.to_string(), folders, projects, databases }
}

fn same(a: &Path, b: &Path) -> bool {
    let c = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    c(a) == c(b)
}

/// The folder's name as a valid project name (it becomes the domain), numbered when
/// www/ or an earlier project in the list has it.
pub fn free_name(folder: &str, layout: &Layout, taken: &[String]) -> String {
    let mut base = String::new();
    for c in folder.chars() {
        let c = if c.is_ascii_alphanumeric() || c == '_' { c } else { '-' };
        if !(c == '-' && base.ends_with('-')) {
            base.push(c);
        }
    }
    let mut base = base.trim_matches(['-', '_']).to_string();
    base.truncate(56);
    if base.is_empty() {
        base = "project".into();
    }
    let free = |n: &str| !taken.iter().any(|t| t.eq_ignore_ascii_case(n)) && crate::projects::check_name(n, layout).is_ok();
    if free(&base) {
        return base;
    }
    (2..).map(|i| format!("{base}-{i}")).find(|n| free(n)).unwrap()
}

/// Where each tool keeps its projects and databases on this OS.
fn find(id: &str, home: &Path) -> Found {
    let mut f = Found::default();
    let drives: Vec<PathBuf> = if cfg!(windows) { ["C:", "D:", "E:"].iter().map(|d| PathBuf::from(format!("{d}\\"))).filter(|d| d.exists()).collect() } else { Vec::new() };
    let subdirs = |dir: PathBuf| -> Vec<PathBuf> { std::fs::read_dir(dir).map(|d| d.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect()).unwrap_or_default() };
    match id {
        "laragon" => {
            for laragon in drives.iter().map(|d| d.join("laragon")) {
                // laragon.ini can move the web root (WWW_Dir=D:\sites).
                let www = std::fs::read_to_string(laragon.join("usr").join("laragon.ini"))
                    .ok()
                    .and_then(|ini| ini_value(&ini, "WWW_Dir"))
                    .map(PathBuf::from)
                    .unwrap_or_else(|| laragon.join("www"));
                f.parents.push(www);
                f.data.extend(subdirs(laragon.join("data")));
            }
        }
        "xampp" => {
            for x in drives.iter().map(|d| d.join("xampp")) {
                f.parents.push(x.join("htdocs"));
                f.data.push(x.join("mysql").join("data"));
            }
            if cfg!(target_os = "macos") {
                f.parents.push("/Applications/XAMPP/xamppfiles/htdocs".into());
                f.data.push("/Applications/XAMPP/xamppfiles/var/mysql".into());
            } else if cfg!(target_os = "linux") {
                f.parents.push("/opt/lampp/htdocs".into());
                f.data.push("/opt/lampp/var/mysql".into());
            }
        }
        "ampps" => {
            let mut roots: Vec<PathBuf> = drives.iter().flat_map(|d| [d.join("Program Files").join("Ampps"), d.join("Program Files (x86)").join("Ampps"), d.join("Ampps")]).collect();
            if cfg!(target_os = "macos") {
                roots.push("/Applications/AMPPS".into());
            } else if cfg!(target_os = "linux") {
                roots.push("/usr/local/ampps".into());
            }
            for r in roots {
                f.parents.push(r.join("www"));
                f.data.push(r.join("mysql").join("data"));
                f.data.push(r.join("var"));
            }
        }
        "mamp" => {
            let mut roots: Vec<PathBuf> = drives.iter().map(|d| d.join("MAMP")).collect();
            if cfg!(target_os = "macos") {
                roots.push("/Applications/MAMP".into());
            }
            for r in roots {
                f.parents.push(r.join("htdocs"));
                // db/mysql57, db/mysql80…
                f.data.extend(subdirs(r.join("db")));
            }
        }
        "wamp" => {
            for w in drives.iter().flat_map(|d| [d.join("wamp64"), d.join("wamp")]) {
                f.parents.push(w.join("www"));
                for engine in ["mysql", "mariadb"] {
                    f.data.extend(subdirs(w.join("bin").join(engine)).into_iter().map(|v| v.join("data")));
                }
            }
        }
        "herd" => {
            f.parents.push(home.join("Herd"));
            let config = if cfg!(target_os = "macos") { home.join("Library/Application Support/Herd/config/valet") } else { home.join(".config").join("herd").join("config").join("valet") };
            valet_config(&config, &mut f);
        }
        "valet" => {
            valet_config(&home.join(".config").join("valet"), &mut f);
            valet_config(&home.join(".valet"), &mut f);
        }
        "flyenv" => {
            // FlyEnv was PhpWebStudy; its sites are in host.json, each with a root folder.
            let mut dirs = vec![home.join("Library/PhpWebStudy"), home.join("Library/Application Support/PhpWebStudy"), home.join(".config").join("PhpWebStudy")];
            if let Some(appdata) = dirs::config_dir() {
                dirs.extend([appdata.join("PhpWebStudy"), appdata.join("FlyEnv")]);
            }
            for dir in dirs {
                if let Some(json) = std::fs::read_to_string(dir.join("host.json")).ok().and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok()) {
                    for root in json_roots(&json) {
                        let path = PathBuf::from(&root);
                        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                            // A Laravel host's root is its public folder: the project is the one above.
                            let project = if name == "public" { path.parent().map(Path::to_path_buf).unwrap_or(path.clone()) } else { path.clone() };
                            let name = project.file_name().and_then(|n| n.to_str()).unwrap_or(name).to_string();
                            f.projects.push((name, project));
                        }
                    }
                }
            }
        }
        "envkit" => {
            // EnvKit doesn't document where it keeps sites; these are its likely folders, and
            // "Choose a folder" covers the rest.
            for base in [home.join("EnvKit"), home.join("envkit"), home.join(".envkit")].into_iter().chain(drives.iter().map(|d| d.join("EnvKit"))) {
                for sub in ["www", "sites", "projects"] {
                    f.parents.push(base.join(sub));
                }
                f.data.extend(subdirs(base.join("data")));
            }
        }
        "servbay" => {
            if cfg!(target_os = "macos") {
                f.parents.push("/Applications/ServBay/www".into());
            }
        }
        "local" => {
            // Local (by Flywheel): each site is "Local Sites/<site>/app/public".
            for site in subdirs(home.join("Local Sites")) {
                if let Some(name) = site.file_name().and_then(|n| n.to_str()) {
                    f.projects.push((name.to_string(), site.join("app").join("public")));
                }
            }
        }
        _ => {}
    }
    f
}

/// Valet and Herd: parked folders in config.json's "paths", linked sites in Sites/.
fn valet_config(dir: &Path, f: &mut Found) {
    if let Some(json) = std::fs::read_to_string(dir.join("config.json")).ok().and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok()) {
        for p in json["paths"].as_array().into_iter().flatten().filter_map(|p| p.as_str()) {
            // Valet parks its own Sites folder too; those links are read below.
            if !Path::new(p).ends_with("Sites") || !Path::new(p).starts_with(dir) {
                f.parents.push(PathBuf::from(p));
            }
        }
    }
    if let Ok(links) = std::fs::read_dir(dir.join("Sites")) {
        for link in links.flatten() {
            if let (Ok(name), Ok(target)) = (link.file_name().into_string(), std::fs::read_link(link.path())) {
                f.projects.push((name, target));
            }
        }
    }
}

/// Every "root" string anywhere in FlyEnv's host.json (its layout has changed over versions).
fn json_roots(v: &serde_json::Value) -> Vec<String> {
    match v {
        serde_json::Value::Array(a) => a.iter().flat_map(json_roots).collect(),
        serde_json::Value::Object(o) => o.iter().flat_map(|(k, v)| if k == "root" { v.as_str().map(str::to_string).into_iter().collect() } else { json_roots(v) }).collect(),
        _ => Vec::new(),
    }
}

fn ini_value(ini: &str, key: &str) -> Option<String> {
    ini.lines().find_map(|l| {
        let (k, v) = l.split_once('=')?;
        (k.trim().eq_ignore_ascii_case(key)).then(|| v.trim().to_string()).filter(|v| !v.is_empty())
    })
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Copied {
    pub name: String,
    pub files: u64,
    /// What wasn't copied: dependency folders (reinstalled from their lock files) and links.
    pub skipped: Vec<String>,
}

/// Copies a project into www/<name>. It's built in tmp/ and moved in at the end, so a
/// half-copied project is never served; the original isn't touched.
pub fn copy_project(source: &Path, name: &str, layout: &Layout, progress: impl Fn(u64)) -> Result<Copied> {
    crate::projects::check_name(name, layout)?;
    if !source.is_dir() {
        return Err(Error::msg(format!("{} isn't there any more.", source.display())));
    }
    std::fs::create_dir_all(layout.tmp())?;
    let root = std::fs::canonicalize(layout.root())?;
    let from = std::fs::canonicalize(source)?;
    if from.starts_with(&root) || root.starts_with(&from) {
        return Err(Error::msg("That folder is inside LampRabit's folder, or holds it. Pick a project folder outside it."));
    }
    let tmp = layout.tmp().join(format!("import-{name}"));
    if tmp.exists() {
        std::fs::remove_dir_all(&tmp)?;
    }
    std::fs::create_dir_all(&tmp)?;
    let mut copied = Copied { name: name.to_string(), files: 0, skipped: Vec::new() };
    let result = copy_tree(&from, &tmp, "", &mut copied, &progress).and_then(|_| {
        std::fs::create_dir_all(layout.www())?;
        std::fs::rename(&tmp, layout.www().join(name)).map_err(Error::from)
    });
    if let Err(e) = result {
        let _ = std::fs::remove_dir_all(&tmp);
        return Err(Error::msg(format!("Couldn't copy {}: {e}", source.display())));
    }
    Ok(copied)
}

fn copy_tree(from: &Path, to: &Path, rel: &str, copied: &mut Copied, progress: &impl Fn(u64)) -> Result<()> {
    for e in std::fs::read_dir(from)? {
        let e = e?;
        let name = e.file_name();
        let name = name.to_string_lossy();
        let path_rel = if rel.is_empty() { name.to_string() } else { format!("{rel}/{name}") };
        let kind = e.file_type()?;
        // Links (Laravel's public/storage, a Herd site's) point into the old place; the
        // project's checks offer to make them again.
        if kind.is_symlink() {
            copied.skipped.push(path_rel);
            continue;
        }
        if kind.is_dir() {
            // Dependencies are reinstalled from the lock file (the doctor offers it), which is
            // quicker than copying thousands of files and right for this computer.
            let deps = rel.is_empty() && ((name == "node_modules" && from.join("package.json").exists()) || (name == "vendor" && from.join("composer.json").exists()));
            if deps {
                copied.skipped.push(path_rel);
                continue;
            }
            std::fs::create_dir(to.join(&*name))?;
            copy_tree(&e.path(), &to.join(&*name), &path_rel, copied, progress)?;
        } else {
            std::fs::copy(e.path(), to.join(&*name))?;
            copied.files += 1;
            if copied.files.is_multiple_of(200) {
                progress(copied.files);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout(dir: &Path) -> Layout {
        Layout::new(dir.join("lr"))
    }

    /// A fresh folder per test, removed when it ends.
    struct Temp(PathBuf);
    impl Temp {
        fn new(tag: &str) -> Self {
            let p = std::env::temp_dir().join(format!("lr-import-{tag}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&p);
            std::fs::create_dir_all(&p).unwrap();
            Self(p)
        }
        fn path(&self) -> &Path {
            &self.0
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn names_become_valid_and_free() {
        let dir = Temp::new("names");
        let l = layout(dir.path());
        std::fs::create_dir_all(l.www().join("shop")).unwrap();
        assert_eq!(free_name("My Site (old)", &l, &[]), "My-Site-old");
        assert_eq!(free_name("shop", &l, &[]), "shop-2");
        assert_eq!(free_name("blog", &l, &["blog".into()]), "blog-2");
        assert_eq!(free_name("...", &l, &[]), "project");
        assert_eq!(free_name("con", &l, &[]), "con-2");
    }

    #[test]
    fn lists_a_web_root_without_the_tools_own_folders() {
        let dir = Temp::new("list");
        let l = layout(dir.path());
        let htdocs = dir.path().join("htdocs");
        std::fs::create_dir_all(htdocs.join("dashboard")).unwrap();
        std::fs::create_dir_all(htdocs.join("empty")).unwrap();
        std::fs::create_dir_all(htdocs.join("blog")).unwrap();
        std::fs::write(htdocs.join("blog").join("wp-login.php"), "").unwrap();
        std::fs::write(htdocs.join("blog").join("wp-config.php"), "define( 'DB_NAME', 'blog_db' );").unwrap();
        let data = dir.path().join("data");
        for d in ["mysql", "blog_db", "#innodb_temp"] {
            std::fs::create_dir_all(data.join(d)).unwrap();
        }
        let found = Found { parents: vec![htdocs], data: vec![data], ..Default::default() };
        let s = build("xampp", "XAMPP", found, &l, &mut Vec::new());
        assert_eq!(s.projects.len(), 1);
        assert_eq!(s.projects[0].framework, Some("WordPress"));
        assert_eq!(s.projects[0].database.as_deref(), Some("blog_db"));
        assert_eq!(s.databases, ["blog_db"]);
    }

    #[test]
    fn copies_without_dependencies_and_refuses_its_own_folder() {
        let dir = Temp::new("copy");
        let l = layout(dir.path());
        let src = dir.path().join("app");
        std::fs::create_dir_all(src.join("vendor/x")).unwrap();
        std::fs::create_dir_all(src.join("node_modules/y")).unwrap();
        std::fs::create_dir_all(src.join("src/vendor")).unwrap();
        std::fs::write(src.join("composer.json"), "{}").unwrap();
        std::fs::write(src.join("src/vendor/keep.php"), "").unwrap();
        std::fs::write(src.join("vendor/x/a.php"), "").unwrap();
        let c = copy_project(&src, "app", &l, |_| {}).unwrap();
        assert_eq!(c.files, 2);
        // node_modules is copied: there's no package.json to reinstall it from.
        assert_eq!(c.skipped, ["vendor"]);
        assert!(l.www().join("app/src/vendor/keep.php").exists());
        assert!(l.www().join("app/node_modules/y").exists());
        assert!(!l.tmp().join("import-app").exists());
        assert!(src.join("vendor/x/a.php").exists());
        // Taken now, and LampRabit's own folders are refused.
        assert!(copy_project(&src, "app", &l, |_| {}).is_err());
        assert!(copy_project(&l.www().join("app"), "again", &l, |_| {}).is_err());
    }

    #[test]
    fn reads_tool_configs() {
        let json: serde_json::Value = serde_json::from_str(r#"[{"id":1,"name":"a.test","root":"/x/a/public"},{"sub":{"root":"/y/b"}}]"#).unwrap();
        assert_eq!(json_roots(&json), ["/x/a/public", "/y/b"]);
        assert_eq!(ini_value("[main]\nWWW_Dir = D:\\sites\n", "www_dir").as_deref(), Some("D:\\sites"));
        assert_eq!(ini_value("WWW_Dir=\n", "WWW_Dir"), None);
    }
}
