//! What a service needs around its start beyond writing files (manifest.rs): setting up a
//! new data folder (`init`), refusing to start an older release line on newer data and
//! upgrading after a move to a newer one (`guard`), links to renamed system libraries
//! (`libraries`), and commands once it answers (`after_start`). Format reference:
//! docs/services.md.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::{
    config::Settings,
    error::{Error, Result},
    layout::Layout,
    manifest::{self, Arg, Ctx, Manifest, Ordered},
};

/// One command; it runs only where its `on` matches.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Command {
    pub program: String,
    #[serde(default)]
    pub args: Vec<Arg>,
    #[serde(default)]
    pub on: Vec<String>,
    /// Folders put in front of PATH (npm needs its Node).
    #[serde(default)]
    pub path: Vec<Arg>,
    /// The working folder; the install folder when not given.
    #[serde(default)]
    pub cwd: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Install {
    /// Removed first, so an interrupted earlier install can't pass for a finished one.
    #[serde(default)]
    pub clean: Vec<String>,
    pub run: Vec<Command>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Once {
    /// Written when the commands worked; while it exists they don't run again.
    pub marker: String,
    #[serde(default, rename = "if")]
    pub when: Option<String>,
    pub run: Vec<Command>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Init {
    /// Paths whose presence means the data folder is set up (any one of them).
    pub done: Vec<String>,
    /// The commands, in order.
    pub run: Vec<Command>,
    /// Leave the data folder for the program to create (mysqld refuses one that exists).
    #[serde(default)]
    pub creates_folder: bool,
    /// Sentences added to a failure whose output contains a key ("administrative": run
    /// LampRabit without administrator rights).
    #[serde(default)]
    pub explain: Ordered<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Guard {
    /// Where the version that last used the data is written.
    pub marker: String,
    /// Present once the data folder is set up; before that there's nothing to guard.
    pub when: String,
    /// How many parts of the version make a release line (2: 10.11).
    #[serde(default = "two")]
    pub line: usize,
    /// Run once the service answers after a move to a newer line; the marker moves on when
    /// it succeeds.
    #[serde(default)]
    pub upgrade: Vec<Arg>,
}

fn two() -> usize {
    2
}

/// Linux: system libraries the program links against that some distributions renamed
/// (libaio.so.1 is libaio.so.1t64 on Ubuntu 24.04+): a link under the old name goes in
/// `dir`, which the service's `run.env` puts on LD_LIBRARY_PATH.
// Read only on Linux, where `libraries` does anything.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Libraries {
    pub check: String,
    pub dir: String,
    #[serde(default)]
    pub renamed: Ordered<String>,
    /// Shown when libraries are still missing; `{missing}` lists them.
    pub hint: String,
}

/// The version the data was last used by, when it's from an older release line and needs
/// `upgrade` after start. Refuses an older line on newer data, which can damage it.
pub fn guard(m: &Manifest, settings: &Settings, layout: &Layout) -> Result<Option<String>> {
    let Some(g) = &m.guard else { return Ok(None) };
    let ctx = Ctx::new(m, settings, layout)?;
    if !Path::new(&ctx.render(&g.when)?).exists() {
        return Ok(None); // a fresh data folder; marked once it's set up
    }
    let marker = PathBuf::from(ctx.render(&g.marker)?);
    let current = ctx.version.clone();
    let Ok(previous) = std::fs::read_to_string(&marker).map(|s| s.trim().to_string()) else {
        // Data from before LampRabit tracked versions: assume it matches.
        let _ = std::fs::write(&marker, &current);
        return Ok(None);
    };
    let line = |v: &str| crate::version::numbers(v).into_iter().take(g.line).collect::<Vec<_>>();
    match line(&previous).cmp(&line(&current)) {
        std::cmp::Ordering::Greater => Err(Error::msg(format!(
            "The database files were last used by {name} {previous}. Starting the older {current} on them isn't supported and can damage data. Switch back to {previous} (Services → {name}), or move {} aside to start with an empty database.",
            ctx.data().display(),
            name = m.name
        ))),
        std::cmp::Ordering::Less if !g.upgrade.is_empty() => Ok(Some(previous)),
        _ => {
            if previous != current {
                let _ = std::fs::write(&marker, &current); // a patch release: nothing to upgrade
            }
            Ok(None)
        }
    }
}

/// Writes the guard's marker for a data folder set up just now (none yet).
pub fn mark_new(m: &Manifest, settings: &Settings, layout: &Layout) -> Result<()> {
    let Some(g) = &m.guard else { return Ok(()) };
    let ctx = Ctx::new(m, settings, layout)?;
    let marker = PathBuf::from(ctx.render(&g.marker)?);
    if !marker.exists() && Path::new(&ctx.render(&g.when)?).exists() {
        std::fs::write(marker, &ctx.version)?;
    }
    Ok(())
}

/// Runs the guard's `upgrade` and moves the marker on; returns what it printed.
pub async fn upgrade(m: &Manifest, settings: &Settings, layout: &Layout) -> Result<String> {
    let Some(g) = &m.guard else { return Ok(String::new()) };
    let ctx = Ctx::new(m, settings, layout)?;
    let args = ctx.args(&g.upgrade)?;
    let (program, rest) = args.split_first().ok_or_else(|| Error::msg(format!("{}: guard.upgrade is empty", m.id)))?;
    let (ok, text) = output(program, rest, &ctx.home(), &env(m, &ctx)?).await?;
    if !ok {
        return Err(Error::msg(format!("{}'s upgrade failed: {}", m.name, text.lines().last().unwrap_or_default())));
    }
    std::fs::write(ctx.render(&g.marker)?, &ctx.version)?;
    Ok(text)
}

/// Sets up a new data folder. A folder that isn't empty but isn't set up is left alone (it
/// may be someone's data); a failed set-up removes the folder so the next start retries.
pub async fn init(m: &Manifest, settings: &Settings, layout: &Layout) -> Result<()> {
    let Some(init) = &m.init else { return Ok(()) };
    let ctx = Ctx::new(m, settings, layout)?;
    for done in &init.done {
        if Path::new(&ctx.render(done)?).exists() {
            return Ok(());
        }
    }
    let data = ctx.data();
    if data.exists() && std::fs::read_dir(&data)?.next().is_some() {
        return Err(Error::msg(format!("{} is not empty but isn't a {} data folder; move it away to set up a new one", data.display(), m.name)));
    }
    if !init.creates_folder {
        std::fs::create_dir_all(&data)?;
    }
    let env = env(m, &ctx)?;
    for c in init.run.iter().filter(|c| manifest::matches(&c.on)) {
        let program = ctx.render(&c.program)?;
        let args = ctx.args(&c.args)?;
        let (ok, text) = output(&program, &args, &ctx.home(), &env).await?;
        if !ok {
            let _ = std::fs::remove_dir_all(&data);
            // Some servers only explain in their own log.
            let log = std::fs::read_to_string(layout.logs().join(m.log_file())).unwrap_or_default();
            let reason = log.lines().rev().find(|l| l.contains("[ERROR]")).unwrap_or_default();
            let hint = init.explain.iter().find(|(key, _)| text.contains(key.as_str())).map(|(_, h)| format!(" {h}")).unwrap_or_default();
            return Err(Error::msg(format!("{} data folder set-up failed: {}{reason}{hint}", m.name, text.trim())));
        }
    }
    Ok(())
}

/// Runs the `after_start` commands; the first failure is returned.
pub async fn after_start(m: &Manifest, settings: &Settings, layout: &Layout) -> Result<()> {
    let ctx = Ctx::new(m, settings, layout)?;
    run_all(&ctx, &m.after_start, &env(m, &ctx)?).await
}

/// The `install` steps for a version just unpacked, with its needs (`{{needs.<id>.bin}}`)
/// filled in. Afterwards the `binary` marker is written, which makes the install count; a
/// failure removes the folder so nothing half-installed stays.
pub async fn install(m: &Manifest, settings: &Settings, layout: &Layout, version: &str, needs: &[(String, String)]) -> Result<()> {
    let Some(steps) = &m.install else { return Ok(()) };
    let mut ctx = Ctx::at(m, settings, layout, version);
    for (k, v) in needs {
        ctx.set(k.clone(), v.clone());
    }
    let home = ctx.home();
    let result = async {
        for path in &steps.clean {
            let p = PathBuf::from(ctx.render(path)?);
            let _ = if p.is_dir() { std::fs::remove_dir_all(&p) } else { std::fs::remove_file(&p) };
        }
        run_all(&ctx, &steps.run, &env(m, &ctx)?).await?;
        if let Some(marker) = ctx.binary()
            && !marker.exists() {
                std::fs::write(marker, format!("{} {version}\n", m.name))?;
            }
        Ok::<_, Error>(())
    }
    .await;
    if let Err(e) = &result {
        let _ = std::fs::remove_dir_all(&home);
        return Err(Error::msg(format!("Setting up {} failed: {e}", m.name)));
    }
    result
}

/// The `once` commands whose marker is missing (and whose `if` holds). They don't stop the
/// start; the failure comes back for a warning.
pub async fn once(m: &Manifest, settings: &Settings, layout: &Layout) -> Result<()> {
    let ctx = Ctx::new(m, settings, layout)?;
    for o in m.once.iter().filter(|o| ctx.holds(o.when.as_deref())) {
        let marker = PathBuf::from(ctx.render(&o.marker)?);
        if marker.exists() {
            continue;
        }
        run_all(&ctx, &o.run, &env(m, &ctx)?).await?;
        if let Some(dir) = marker.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(&marker, &ctx.version)?;
    }
    Ok(())
}

async fn run_all(ctx: &Ctx<'_>, commands: &[Command], env: &[(String, String)]) -> Result<()> {
    for c in commands.iter().filter(|c| manifest::matches(&c.on)) {
        let program = ctx.render(&c.program)?;
        let mut env = env.to_vec();
        let path = ctx.args(&c.path)?;
        if !path.is_empty() {
            let rest = std::env::var_os("PATH").unwrap_or_default();
            let joined = std::env::join_paths(path.iter().map(PathBuf::from).chain(std::env::split_paths(&rest))).map_err(|e| Error::msg(e.to_string()))?;
            env.push(("PATH".into(), joined.to_string_lossy().into_owned()));
        }
        let cwd = match &c.cwd {
            Some(cwd) => PathBuf::from(ctx.render(cwd)?),
            None => ctx.home(),
        };
        let (ok, text) = output(&program, &ctx.args(&c.args)?, &cwd, &env).await?;
        if !ok {
            return Err(Error::msg(text.trim().lines().last().unwrap_or("it failed").to_string()));
        }
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub fn libraries(_m: &Manifest, _settings: &Settings, _layout: &Layout) -> Result<()> {
    Ok(())
}

#[cfg(target_os = "linux")]
pub fn libraries(m: &Manifest, settings: &Settings, layout: &Layout) -> Result<()> {
    let Some(l) = &m.libraries else { return Ok(()) };
    let ctx = Ctx::new(m, settings, layout)?;
    let (check, dir) = (ctx.render(&l.check)?, PathBuf::from(ctx.render(&l.dir)?));
    let missing = || -> Vec<String> {
        let out = std::process::Command::new("ldd").arg(&check).env("LD_LIBRARY_PATH", &dir).output();
        let text = out.map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default();
        text.lines().filter(|l| l.contains("not found")).filter_map(|l| l.split_whitespace().next()).map(str::to_string).collect()
    };
    let mut gone = missing();
    let mut linked = false;
    for (name, renamed) in l.renamed.iter().filter(|(n, _)| gone.contains(n)) {
        let found = ["/usr/lib/x86_64-linux-gnu", "/lib/x86_64-linux-gnu", "/usr/lib/aarch64-linux-gnu", "/lib/aarch64-linux-gnu", "/usr/lib64", "/usr/lib"]
            .iter()
            .map(|d| Path::new(d).join(renamed))
            .find(|p| p.exists());
        if let Some(target) = found {
            std::fs::create_dir_all(&dir)?;
            let link = dir.join(name);
            let _ = std::fs::remove_file(&link);
            std::os::unix::fs::symlink(target, &link)?;
            linked = true;
        }
    }
    if linked {
        gone = missing();
    }
    if gone.is_empty() { Ok(()) } else { Err(Error::msg(l.hint.replace("{missing}", &gone.join(", ")))) }
}

/// The service's `run.env`, for its set-up commands too.
fn env(m: &Manifest, ctx: &Ctx) -> Result<Vec<(String, String)>> {
    let Some(run) = &m.run else { return Ok(vec![]) };
    run.env.iter().filter_map(|(k, v)| ctx.pick(v).map(|v| ctx.render(v).map(|v| (k.clone(), v)))).collect()
}

async fn output(program: &str, args: &[String], cwd: &Path, env: &[(String, String)]) -> Result<(bool, String)> {
    let mut cmd = tokio::process::Command::new(program);
    cmd.args(args).current_dir(cwd).envs(env.iter().map(|(k, v)| (k, v)));
    crate::procgroup::prepare(&mut cmd);
    let out = cmd.output().await.map_err(|e| Error::msg(format!("Couldn't run {program}: {e}")))?;
    Ok((out.status.success(), format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::ServiceKind;

    /// The manifests' my.ini per release line (checked equal to the Rust writers they
    /// replaced, apart from the pid file and socket moving into tmp/<id>/).
    #[test]
    fn database_configs_follow_the_release_line() {
        let layout = Layout::new(std::env::temp_dir().join(format!("lr-setup-{}", std::process::id())));
        let ipv6 = std::net::TcpListener::bind("[::1]:0").is_ok();
        let loopback = if ipv6 { "bind-address=127.0.0.1,::1" } else { "bind-address=127.0.0.1" };
        for (kind, id, version, bind, native) in [
            (ServiceKind::Mariadb, "mariadb", "10.6.22", "bind-address=127.0.0.1", false),
            (ServiceKind::Mariadb, "mariadb", "11.8.5", loopback, false),
            (ServiceKind::Mysql, "mysql", "8.4.7", loopback, true),
            (ServiceKind::Mysql, "mysql", "9.5.0", loopback, false),
        ] {
            let mut settings = Settings::default();
            settings.versions.set(id, Some(version.into()));
            let ctx = Ctx::new(kind.manifest(), &settings, &layout).unwrap();
            ctx.prepare().unwrap();
            let ini = std::fs::read_to_string(layout.etc(id).join("my.ini")).unwrap();
            assert!(ini.lines().any(|l| l == bind), "{id} {version}: {ini}");
            assert_eq!(ini.contains("mysql_native_password=ON"), native, "{id} {version}");
            let slash = |p: std::path::PathBuf| p.to_string_lossy().replace('\\', "/");
            assert!(ini.contains(&format!("datadir=\"{}\"", slash(ctx.data()))));
            assert!(ini.contains(&format!("log-error=\"{}\"", slash(layout.logs().join(format!("{id}-error.log"))))));
            assert_eq!(ini.contains("socket="), cfg!(unix));
        }
        let mut settings = Settings::default();
        settings.versions.set("mysql", Some("8.4.7".into()));
        assert_eq!(crate::configs::mysql_data(&settings, &layout), layout.data("mysql-8.4"));
        let _ = std::fs::remove_dir_all(layout.root());
    }
}
