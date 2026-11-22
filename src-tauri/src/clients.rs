//! Database clients (`connects` in a manifest, KarotX for one). LampRabit keeps a services
//! file listing every local database (the installed database services and the projects'
//! SQLite files), and the client watches it: databases appear, change and disappear in it
//! on their own. The file's format is KarotX's (version 1). Format reference: docs/services.md.

use std::{collections::HashSet, path::Path};

use serde::Deserialize;
use serde_json::{json, Map, Value};

use crate::{
    config::Settings,
    layout::Layout,
    manifest::{self, Ordered},
    services::ServiceKind,
};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Connects {
    /// The services file, inside the client's data folder; its `run` passes the path on.
    pub file: String,
    /// The client's name for LampRabit's group of databases.
    #[serde(default = "source")]
    pub source: String,
    /// Which services are databases to the client: service id → its type and, when it isn't
    /// the service's main port, the port drivers connect to.
    pub databases: Ordered<Listed>,
    /// The client's type for the projects' SQLite files; none leaves them out.
    #[serde(default)]
    pub sqlite: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Listed {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub port: Option<String>,
    /// When the service's connection URL doesn't say (ClickHouse's `default`).
    #[serde(default)]
    pub user: Option<String>,
    #[serde(default)]
    pub database: Option<String>,
}

fn source() -> String {
    "LampRabit".into()
}

/// The file stays in the client's data folder, and something is listed.
pub fn check(c: &Connects) -> std::result::Result<(), String> {
    if !c.file.starts_with("{{data}}/") || c.file.contains("..") {
        return Err("connects.file must be inside its data folder ({{data}}/…)".into());
    }
    if c.databases.0.is_empty() {
        return Err("connects.databases lists no services".into());
    }
    Ok(())
}

/// The services file's content: every installed database service (running or stopped) and
/// every SQLite file in the projects.
pub fn services_file(c: &Connects, settings: &Settings, layout: &Layout, running: &HashSet<ServiceKind>, sqlite: &[String]) -> Value {
    let mut databases = Vec::new();
    for kind in ServiceKind::all() {
        let m = kind.manifest();
        let (Some(listed), Some(version)) = (c.databases.get(&m.id), settings.versions.get(&m.id)) else { continue };
        // Passwords and default databases come from the address apps get (`connection.url`).
        let url = kind.connection(settings, layout).and_then(|i| reqwest::Url::parse(&i.url).ok());
        let port = listed.port.as_deref().and_then(|p| m.port(settings, p)).unwrap_or_else(|| kind.port_of(settings));
        let mut db = Map::new();
        db.insert("id".into(), json!(m.id));
        db.insert("name".into(), json!(format!("{} {}", m.name, version.split('.').next().unwrap_or(version))));
        db.insert("type".into(), json!(listed.kind));
        db.insert("host".into(), json!("127.0.0.1"));
        db.insert("port".into(), json!(port));
        let user = listed.user.clone().or_else(|| url.as_ref().map(|u| u.username().to_string()));
        let password = url.as_ref().and_then(|u| u.password()).map(percent_decode);
        let database = listed.database.clone().or_else(|| url.as_ref().map(|u| u.path().trim_start_matches('/').to_string()));
        for (k, v) in [("user", user), ("password", password), ("database", database)] {
            if let Some(v) = v.filter(|v| !v.is_empty()) {
                db.insert(k.into(), json!(v));
            }
        }
        db.insert("status".into(), json!(if running.contains(&kind) { "running" } else { "stopped" }));
        databases.push(Value::Object(db));
    }
    if let Some(kind) = &c.sqlite {
        let mut ids = HashSet::new();
        for rel in sqlite {
            let id = sqlite_id(rel, &mut ids);
            let file = layout.www().join(rel);
            databases.push(json!({ "id": id, "name": rel, "type": kind, "file": file.to_string_lossy(), "status": "running" }));
        }
    }
    json!({ "version": 1, "source": c.source, "databases": databases })
}

/// A stable id for an SQLite file: letters, digits and `_ . -`, at most 64 characters,
/// unique in the file (a suffix when two paths clean up the same way).
fn sqlite_id(rel: &str, taken: &mut HashSet<String>) -> String {
    let clean: String = rel.chars().map(|ch| if ch.is_ascii_alphanumeric() || "_.-".contains(ch) { ch } else { '-' }).collect();
    let base: String = format!("sqlite-{clean}").chars().take(60).collect();
    let mut id = base.clone();
    let mut n = 2;
    while !taken.insert(id.clone()) {
        id = format!("{base}-{n}");
        n += 1;
    }
    id
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%'
            && i + 2 < b.len()
            && let Some(v) = std::str::from_utf8(&b[i + 1..i + 3]).ok().and_then(|h| u8::from_str_radix(h, 16).ok())
        {
            out.push(v);
            i += 3;
            continue;
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Rewrites every installed client's services file when its content changed. Called every
/// second, so starts, stops, installs and removals reach the client quickly.
pub fn refresh(settings: &Settings, layout: &Layout, running: &HashSet<ServiceKind>, sqlite: &[String]) {
    for kind in ServiceKind::all() {
        let m = kind.manifest();
        let Some(c) = &m.connects else { continue };
        if settings.versions.get(&m.id).is_none() {
            continue;
        }
        let Ok(path) = manifest::Ctx::any(m, settings, layout).render(&c.file) else { continue };
        let text = serde_json::to_string_pretty(&services_file(c, settings, layout, running, sqlite)).unwrap_or_default();
        if let Err(e) = write_private(Path::new(&path), &text) {
            eprintln!("couldn't write {path}: {e}");
        }
    }
}

/// Writes a file only its owner can read (it holds passwords), through a temporary file
/// renamed over it so the client never reads half of it. Unchanged content isn't rewritten.
fn write_private(path: &Path, text: &str) -> std::io::Result<()> {
    if std::fs::read_to_string(path).is_ok_and(|old| old == text) {
        return Ok(());
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    #[cfg(unix)]
    {
        use std::{io::Write, os::unix::fs::OpenOptionsExt};
        let _ = std::fs::remove_file(&tmp);
        std::fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(&tmp)?.write_all(text.as_bytes())?;
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        std::fs::write(&tmp, text)?;
        // Only the current user: no inherited entries, full control for them alone. The ACL
        // moves with the file when it's renamed.
        let user = std::env::var("USERNAME").unwrap_or_default();
        let ok = std::process::Command::new("icacls")
            .arg(&tmp)
            .args(["/inheritance:r", "/grant:r", &format!("{user}:F")])
            .creation_flags(crate::procgroup::CREATE_NO_WINDOW)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|s| s.success());
        if !ok {
            let _ = std::fs::remove_file(&tmp);
            return Err(std::io::Error::other("couldn't make it readable only by this user"));
        }
    }
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// KarotX ships as a bundled manifest and lists every installed database, running or
    /// stopped, with the address its drivers use, plus the projects' SQLite files.
    #[test]
    fn karotx_lists_every_local_database() {
        let kind = ServiceKind::from_id("karotx").expect("KarotX is bundled");
        let m = kind.manifest();
        assert!(manifest::is_bundled("karotx") && m.engine.is_none());
        let c = m.connects.as_ref().unwrap();
        let mut settings = Settings::default();
        for (id, v) in [("mariadb", "12.3.4"), ("postgres", "18.1.0"), ("redis", "8.4.0"), ("clickhouse", "25.1.0"), ("memcached", "1.6.39"), ("neo4j", "2025.1.0"), ("mailpit", "1.34.0"), ("karotx", "0.1.0")] {
            settings.versions.set(id, Some(v.into()));
        }
        let layout = Layout::new(std::env::temp_dir().join("lr-clients-test"));
        let running: HashSet<_> = [ServiceKind::from_id("mariadb").unwrap()].into();
        let file = services_file(c, &settings, &layout, &running, &["blog/database/database.sqlite".into()]);
        assert_eq!((file["version"].as_i64(), file["source"].as_str()), (Some(1), Some("LampRabit")));
        let dbs = file["databases"].as_array().unwrap();
        let by = |id: &str| dbs.iter().find(|d| d["id"] == id).unwrap_or_else(|| panic!("{id} listed"));
        let mut ids: Vec<&str> = dbs.iter().map(|d| d["id"].as_str().unwrap()).collect();
        ids.sort();
        assert_eq!(ids, ["clickhouse", "mariadb", "memcached", "neo4j", "postgres", "redis", "sqlite-blog-database-database.sqlite"], "databases only, installed only, not itself");

        let maria = by("mariadb");
        assert_eq!((maria["type"].as_str(), maria["name"].as_str(), maria["user"].as_str(), maria["status"].as_str()), (Some("mariadb"), Some("MariaDB 12"), Some("root"), Some("running")));
        assert_eq!(maria["port"], json!(3306), "a number");
        assert!(maria.get("password").is_none(), "no empty fields");
        assert_eq!((by("postgres")["database"].as_str(), by("postgres")["status"].as_str()), (Some("postgres"), Some("stopped")));
        assert_eq!((by("clickhouse")["port"].as_u64(), by("clickhouse")["user"].as_str()), (Some(19000), Some("default")), "the native port");
        assert_eq!(by("neo4j")["port"], json!(7687), "Bolt");
        assert_eq!(by("redis")["database"].as_str(), Some("0"));
        let lite = by("sqlite-blog-database-database.sqlite");
        assert_eq!(lite["type"].as_str(), Some("sqlite"));
        assert!(lite["file"].as_str().unwrap().ends_with("database.sqlite") && lite.get("host").is_none());
    }

    #[test]
    fn sqlite_ids_are_clean_and_unique() {
        let mut taken = HashSet::new();
        let a = sqlite_id("my app/db.sqlite", &mut taken);
        let b = sqlite_id("my-app/db.sqlite", &mut taken);
        assert_eq!(a, "sqlite-my-app-db.sqlite");
        assert_eq!(b, "sqlite-my-app-db.sqlite-2");
        let long = sqlite_id(&"x".repeat(200), &mut taken);
        assert!(long.len() <= 64);
        assert_eq!(percent_decode("p%40ss%"), "p@ss%");
    }

    #[test]
    fn the_file_stays_in_the_data_folder() {
        let c: Connects = serde_yaml_ng::from_str("file: /etc/x.json\ndatabases: { mysql: { type: mysql } }\n").unwrap();
        assert!(check(&c).unwrap_err().contains("data folder"));
    }

    #[test]
    fn the_file_is_written_whole_and_only_on_change() {
        let dir = std::env::temp_dir().join(format!("lr-services-file-{}", std::process::id()));
        let path = dir.join("karotx-services.json");
        write_private(&path, "{}").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "{}");
        assert!(!path.with_extension("json.tmp").exists());
        write_private(&path, "{\"a\":1}").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "{\"a\":1}");
        let _ = std::fs::remove_dir_all(dir);
    }
}
