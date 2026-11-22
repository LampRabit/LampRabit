//! Database backups: one file per dump in `<root>/backups/<engine>/`, made and restored with
//! each server's own tools (mariadb-dump/mysqldump, pg_dump/pg_restore). SQLite has no
//! server: its databases are files in the projects, copied with sqlite3's `.backup` when
//! the shell is installed. Backups are the user's data, so uninstall keeps the folder like
//! www/ and data/.

use std::path::{Path, PathBuf};
use std::process::Stdio;

use serde::Serialize;

use crate::{
    config::Settings,
    error::{Error, Result},
    layout::{Layout, exe},
    templates::DbEngine,
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Backup {
    pub engine: DbEngine,
    pub database: String,
    pub path: PathBuf,
    pub size: u64,
    /// Unix time in milliseconds.
    pub created: u64,
}

pub fn folder(layout: &Layout) -> PathBuf {
    layout.root().join("backups")
}

/// File types taken for SQLite databases in projects (Laravel's database.sqlite,
/// Symfony's var/data.db…).
const SQLITE_EXTENSIONS: &[&str] = &["sqlite", "sqlite3", "db"];
/// Folders that hold dependencies or caches, not the project's databases.
const SQLITE_SKIP: &[&str] = &["node_modules", "vendor", "storage/framework", "var/cache"];

/// SQLite databases in the projects, as "<project>/<path>" with forward slashes.
fn sqlite_databases(layout: &Layout) -> Vec<String> {
    fn walk(dir: &Path, rel: &str, depth: u8, out: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let rel = if rel.is_empty() { name.clone() } else { format!("{rel}/{name}") };
            let path = entry.path();
            if path.is_dir() {
                // Relative to the project, to match "storage/framework".
                let inner = rel.split_once('/').map_or("", |(_, r)| r);
                if depth > 0 && !name.starts_with('.') && !SQLITE_SKIP.contains(&inner) && !SQLITE_SKIP.contains(&name.as_str()) {
                    walk(&path, &rel, depth - 1, out);
                }
            } else if is_sqlite(&path) {
                out.push(rel);
            }
        }
    }
    let mut out = Vec::new();
    // www/<project>/database/database.sqlite is three levels down; a little more for monorepos.
    walk(&layout.www(), "", 4, &mut out);
    out.sort();
    out
}

/// An SQLite file by its header. Laravel creates database.sqlite empty, which SQLite
/// treats as an empty database; an empty .db could be anything.
fn is_sqlite(path: &Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or_default().to_ascii_lowercase();
    if !SQLITE_EXTENSIONS.contains(&ext.as_str()) {
        return false;
    }
    let mut head = [0u8; 16];
    match std::fs::File::open(path).and_then(|mut f| std::io::Read::read(&mut f, &mut head)) {
        Ok(0) => ext != "db",
        Ok(16) => &head == b"SQLite format 3\0",
        _ => false,
    }
}

/// The file behind "<project>/<path>", which has to stay inside www/.
fn sqlite_file(layout: &Layout, database: &str) -> Result<PathBuf> {
    let rel = Path::new(database);
    let plain = rel.components().all(|c| matches!(c, std::path::Component::Normal(_)));
    let ext = rel.extension().and_then(|e| e.to_str()).unwrap_or_default().to_ascii_lowercase();
    // "~" stands for "/" in backup file names.
    if database.is_empty() || !plain || !SQLITE_EXTENSIONS.contains(&ext.as_str()) || database.contains('~') {
        return Err(Error::msg("Pick an SQLite file inside a project (www/<project>/…, ending in .sqlite, .sqlite3 or .db)."));
    }
    Ok(layout.www().join(rel))
}

/// The sqlite3 shell, when installed: its `.backup` copies a database safely while an
/// app writes to it, including what's still in the WAL file.
fn sqlite_shell(settings: &Settings, layout: &Layout) -> Option<PathBuf> {
    let version = settings.versions.get("sqlite")?;
    Some(layout.bin("sqlite", version).join(exe("sqlite3"))).filter(|p| p.exists())
}

/// `<file>-wal` and the like, next to a database.
fn sqlite_side(file: &Path, suffix: &str) -> PathBuf {
    let mut side = file.as_os_str().to_owned();
    side.push(suffix);
    PathBuf::from(side)
}

/// Removes a database's WAL and shared-memory files, which would otherwise be replayed
/// onto the restored file and corrupt it.
fn remove_sqlite_journal(file: &Path) -> Result<()> {
    for suffix in ["-wal", "-shm", "-journal"] {
        match std::fs::remove_file(sqlite_side(file, suffix)) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                return Err(Error::msg(format!("Couldn't replace {}: {e}. Stop the app that uses it, then try again.", file.display())));
            }
            _ => {}
        }
    }
    Ok(())
}

async fn output(program: &Path, args: &[String], stdin: Option<&Path>) -> Result<String> {
    let mut cmd = tokio::process::Command::new(program);
    cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());
    if let Some(file) = stdin {
        cmd.stdin(std::fs::File::open(file)?);
    } else {
        cmd.stdin(Stdio::null());
    }
    crate::procgroup::prepare(&mut cmd);
    let out = cmd.output().await.map_err(|e| Error::msg(format!("Couldn't run {}: {e}", program.display())))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(Error::msg(if err.is_empty() { format!("{} failed", program.display()) } else { err }));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// The databases on a server, without its system databases. The server must be running.
pub async fn databases(settings: &Settings, layout: &Layout, engine: DbEngine) -> Result<Vec<String>> {
    if engine == DbEngine::Sqlite {
        return Ok(sqlite_databases(layout));
    }
    let (m, hooks) = hooks(engine)?;
    let text = run_step(m, settings, layout, &hooks.list, &[]).await?;
    Ok(text.lines().map(str::trim).filter(|d| !d.is_empty() && !hooks.skip.iter().any(|s| s.eq_ignore_ascii_case(d))).map(str::to_string).collect())
}

/// A database service's `backup` hooks from its manifest.
fn hooks(engine: DbEngine) -> Result<(&'static crate::manifest::Manifest, &'static crate::manifest::BackupHooks)> {
    let m = crate::manifest::get(engine.id()).ok_or_else(|| Error::msg(format!("{} has no manifest", engine.id())))?;
    let hooks = m.backup.as_ref().ok_or_else(|| Error::msg(format!("{} can't be backed up", m.name)))?;
    Ok((m, hooks))
}

/// Runs one backup step with `{{database}}`, `{{file}}`, `{{original}}` and
/// `{{runtime.<id>.home}}` (another installed runtime, MongoDB's tools) filled in.
async fn run_step(m: &crate::manifest::Manifest, settings: &Settings, layout: &Layout, step: &crate::manifest::BackupStep, values: &[(&str, String)]) -> Result<String> {
    let mut ctx = crate::manifest::Ctx::new(m, settings, layout)?;
    for (k, v) in values {
        ctx.set(*k, v.clone());
    }
    for text in std::iter::once(step.program.as_str()).chain(step.args.iter().flat_map(crate::manifest::Arg::all)) {
        let mut rest = text;
        while let Some(at) = rest.find("{{runtime.") {
            let id = rest[at + 10..].split('.').next().unwrap_or_default().to_string();
            let version = settings.versions.get(&id).ok_or_else(|| {
                let label = crate::catalog::catalog().runtimes.into_iter().find(|r| r.id == id).map_or(id.clone(), |r| r.label.to_string());
                Error::msg(format!("{}'s backups use {label}: install it on the Services page.", m.name))
            })?;
            ctx.set(format!("runtime.{id}.home"), layout.bin(&id, version).to_string_lossy());
            rest = &rest[at + 10..];
        }
    }
    let program = PathBuf::from(ctx.render(&step.program)?);
    let args = ctx.args(&step.args)?;
    let stdin = step.stdin.as_ref().map(|s| ctx.render(s).map(PathBuf::from)).transpose()?;
    output(&program, &args, stdin.as_deref()).await
}

/// A database name that is safe in a file name and in the quoted SQL below.
fn check_name(database: &str) -> Result<()> {
    let ok = !database.is_empty() && database.len() <= 64 && database.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    if ok { Ok(()) } else { Err(Error::msg("Database names for backups can use letters, numbers, dashes and underscores.")) }
}

/// Dumps one database into backups/<engine>/<database>-<date>.<sql|dump>.
pub async fn create(settings: &Settings, layout: &Layout, engine: DbEngine, database: &str) -> Result<Backup> {
    if engine != DbEngine::Sqlite {
        check_name(database)?;
    }
    let dir = folder(layout).join(engine.id());
    std::fs::create_dir_all(&dir)?;
    let stamp = chrono_stamp();
    let result = match engine {
        // "shop/database/database.sqlite" → shop~database~database.sqlite-<stamp>.sqlite
        DbEngine::Sqlite => {
            let source = sqlite_file(layout, database)?;
            let path = dir.join(format!("{}-{stamp}.sqlite", database.replace('/', "~")));
            match sqlite_shell(settings, layout) {
                Some(shell) => {
                    let args = [source.display().to_string(), format!(".backup '{}'", path.display())];
                    output(&shell, &args, None).await.map(|_| path)
                }
                None if sqlite_side(&source, "-wal").exists() => Err(Error::msg(
                    "This database has changes in its WAL file that a plain copy would miss: install SQLite on the Services page (Databases → SQLite), then back it up again.",
                )),
                None => std::fs::copy(&source, &path).map(|_| path).map_err(Error::from),
            }
        }
        _ => {
            let (m, hooks) = hooks(engine)?;
            let path = dir.join(format!("{database}-{stamp}.{}", hooks.extension));
            let values = [("database", database.to_string()), ("file", path.display().to_string())];
            async {
                for step in &hooks.dump {
                    run_step(m, settings, layout, step, &values).await?;
                }
                Ok::<_, Error>(path)
            }
            .await
        }
    };
    let path = result.map_err(|e| Error::msg(format!("Backing up {database} failed: {e}")))?;
    describe(engine, &path).ok_or_else(|| Error::msg("The backup file wasn't written"))
}

/// Restores a backup into `database` (created when it doesn't exist), replacing the
/// tables the backup contains.
pub async fn restore(settings: &Settings, layout: &Layout, engine: DbEngine, file: &Path, database: &str) -> Result<()> {
    if engine != DbEngine::Sqlite {
        check_name(database)?;
    }
    if !file.starts_with(folder(layout)) || !file.is_file() {
        return Err(Error::msg("Pick a backup from LampRabit's backups folder."));
    }
    match engine {
        // The file is replaced as a whole, as the other engines replace the tables.
        DbEngine::Sqlite => {
            let target = sqlite_file(layout, database)?;
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            remove_sqlite_journal(&target)?;
            std::fs::copy(file, &target)
                .map_err(|e| Error::msg(format!("Couldn't write {}: {e}. Stop the app that uses it, then try again.", target.display())))?;
        }
        _ => {
            let (m, hooks) = hooks(engine)?;
            let original = describe(engine, file).map(|b| b.database).unwrap_or_else(|| database.to_string());
            let values = [("database", database.to_string()), ("file", file.display().to_string()), ("original", original)];
            let listed = databases(settings, layout, engine).await?.iter().any(|d| d == database);
            for step in hooks.restore.iter().filter(|s| !(s.if_missing && listed)) {
                run_step(m, settings, layout, step, &values).await?;
            }
        }
    }
    Ok(())
}

pub fn delete(layout: &Layout, file: &Path) -> Result<()> {
    if !file.starts_with(folder(layout)) {
        return Err(Error::msg("Only backups in LampRabit's backups folder can be deleted here."));
    }
    std::fs::remove_file(file)?;
    Ok(())
}

/// Every backup, newest first.
pub fn list(layout: &Layout) -> Vec<Backup> {
    let mut out = Vec::new();
    for engine in [DbEngine::Mariadb, DbEngine::Mysql, DbEngine::Postgres, DbEngine::Mongodb, DbEngine::Sqlite] {
        let Ok(entries) = std::fs::read_dir(folder(layout).join(engine.id())) else { continue };
        out.extend(entries.flatten().filter_map(|e| describe(engine, &e.path())));
    }
    out.sort_by_key(|b| std::cmp::Reverse(b.created));
    out
}

fn describe(engine: DbEngine, path: &Path) -> Option<Backup> {
    let meta = std::fs::metadata(path).ok().filter(|m| m.is_file())?;
    // shop-20261101-120000.sql, .dump or .archive.gz
    let name = path.file_name()?.to_str()?;
    let stem = name.strip_suffix(".archive.gz").or_else(|| path.file_stem()?.to_str())?;
    // "<database>-YYYYMMDD-HHMMSS"
    let database = stem.rsplitn(3, '-').nth(2)?.to_string();
    let database = if engine == DbEngine::Sqlite { database.replace('~', "/") } else { database };
    let created = meta.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_millis() as u64;
    Some(Backup { engine, database, path: path.to_path_buf(), size: meta.len(), created })
}

/// YYYYMMDD-HHMMSS in UTC, without a date library.
fn chrono_stamp() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let (days, rest) = (secs / 86_400, secs % 86_400);
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}{month:02}{day:02}-{:02}{:02}{:02}", rest / 3600, rest % 3600 / 60, rest % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backup_names_keep_the_database_name() {
        let dir = std::env::temp_dir().join(format!("lr-backups-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("my_shop-20261101-120000.sql");
        std::fs::write(&file, "--").unwrap();
        assert_eq!(describe(DbEngine::Mariadb, &file).unwrap().database, "my_shop");
        let file = dir.join("blog-2-20261101-120000.dump");
        std::fs::write(&file, "--").unwrap();
        assert_eq!(describe(DbEngine::Postgres, &file).unwrap().database, "blog-2", "dashes in names survive");
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(chrono_stamp().len(), 15);
        assert!(check_name("a`b").is_err() && check_name("shop_1").is_ok());
    }

    #[test]
    fn sqlite_backups_find_and_name_project_files() {
        let root = std::env::temp_dir().join(format!("lr-sqlite-{}", std::process::id()));
        let layout = Layout::new(&root);
        let db = layout.www().join("shop").join("database");
        std::fs::create_dir_all(&db).unwrap();
        std::fs::write(db.join("database.sqlite"), "").unwrap();
        // Not SQLite, and inside vendor/: neither is listed.
        std::fs::write(db.join("notes.db"), "plain text, not a database").unwrap();
        let vendor = layout.www().join("shop").join("vendor");
        std::fs::create_dir_all(&vendor).unwrap();
        std::fs::write(vendor.join("fixture.sqlite"), "").unwrap();
        assert_eq!(sqlite_databases(&layout), ["shop/database/database.sqlite"]);

        let backup = root.join("shop~database~database.sqlite-20261101-120000.sqlite");
        std::fs::write(&backup, "").unwrap();
        assert_eq!(describe(DbEngine::Sqlite, &backup).unwrap().database, "shop/database/database.sqlite");

        assert!(sqlite_file(&layout, "../outside.sqlite").is_err());
        assert!(sqlite_file(&layout, "shop/.env").is_err());
        assert!(sqlite_file(&layout, "shop/database/database.sqlite").is_ok());
        let _ = std::fs::remove_dir_all(&root);
    }
}
