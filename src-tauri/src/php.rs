//! PHP runs as FastCGI worker pools, one pool per PHP version in use: several `php-cgi`
//! processes on Windows (each serves one request at a time), one `php-fpm` elsewhere.
//! Web servers balance requests across a pool, so each site can use its own PHP version.

use std::{
    collections::{BTreeSet, HashMap},
    path::PathBuf,
    process::Stdio,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

use serde::Serialize;
use tauri::AppHandle;
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, BufReader},
    process::Command,
    sync::{Mutex, watch},
};

use tauri::async_runtime::JoinHandle;

use crate::{
    config::Settings,
    configs,
    error::{Error, Result},
    layout::{Layout, exe},
    logs::{self, Level},
    procgroup::{self, ProcGroup},
    sites::Site,
};

/// Ports reserved per PHP version (workers use the first `php_workers` of them).
const PORTS_PER_VERSION: u16 = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolPlan {
    pub version: String,
    /// First worker port; worker i listens on `port + i`.
    pub port: u16,
    pub workers: u16,
}

impl PoolPlan {
    pub fn ports(&self) -> impl Iterator<Item = u16> + '_ {
        (0..self.workers).map(|i| self.port + i)
    }

    /// A hostname-safe name for the pool, used as Apache balancer / Nginx upstream name.
    pub fn name(&self) -> String {
        format!("php{}", self.version.replace('.', "x"))
    }
}

/// Which PHP versions the given sites need, each with its port block. The active version
/// is always included (localhost and the tools use it).
pub fn plan(settings: &Settings, layout: &Layout, sites: &[Site]) -> Vec<PoolPlan> {
    let installed = crate::runtimes::installed(layout, crate::runtimes::Runtime::Php);
    let mut versions: BTreeSet<String> = sites.iter().filter_map(|s| s.php.clone()).collect();
    versions.extend(settings.versions.php.clone());
    versions.retain(|v| installed.contains(v));
    let workers = if cfg!(windows) { u16::from(settings.php_workers.clamp(1, PORTS_PER_VERSION as u8)) } else { 1 };
    versions
        .into_iter()
        .enumerate()
        .map(|(i, version)| PoolPlan { port: settings.ports.php_fpm + i as u16 * PORTS_PER_VERSION, version, workers })
        .collect()
}

/// The pool serving `version`, falling back to the active version's pool.
pub fn pool_for<'a>(plan: &'a [PoolPlan], version: Option<&str>, settings: &Settings) -> Option<&'a PoolPlan> {
    version
        .and_then(|v| plan.iter().find(|p| p.version == v))
        .or_else(|| settings.versions.php.as_deref().and_then(|v| plan.iter().find(|p| p.version == v)))
        .or_else(|| plan.first())
}

struct Pool {
    plan: PoolPlan,
    stop: watch::Sender<bool>,
    tasks: Vec<JoinHandle<()>>,
    alive: Arc<AtomicUsize>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PoolStatus {
    pub version: String,
    pub port: u16,
    pub workers: u16,
    /// Workers currently running.
    pub alive: usize,
}

#[derive(Default)]
pub struct PhpPools {
    pools: Mutex<HashMap<String, Pool>>,
}

impl PhpPools {
    /// Makes the running pools match `plan`: stops pools no longer needed or whose ports
    /// changed, and starts missing ones.
    pub async fn ensure(&self, app: &AppHandle, settings: &Settings, layout: &Layout, plan: &[PoolPlan]) -> Result<()> {
        let mut pools = self.pools.lock().await;
        let stale: Vec<String> =
            pools.iter().filter(|(_, p)| !plan.contains(&p.plan)).map(|(v, _)| v.clone()).collect();
        for version in stale {
            if let Some(pool) = pools.remove(&version) {
                stop_pool(pool).await;
                logs::system(app, Level::Info, format!("Stopped PHP {version} workers"));
            }
        }
        for p in plan {
            if pools.contains_key(&p.version) {
                continue;
            }
            for port in p.ports() {
                if !crate::services::port_free(port) {
                    return Err(Error::msg(format!(
                        "PHP {} can't use port {port}: it is already in use. Change \"PHP FastCGI ports\" in Settings.",
                        p.version
                    )));
                }
            }
            let pool = start_pool(app, settings, layout, p)?;
            logs::system(
                app,
                Level::Info,
                format!("Started PHP {} with {} worker(s) on port {}", p.version, p.workers, p.port),
            );
            pools.insert(p.version.clone(), pool);
        }
        Ok(())
    }

    pub async fn stop_all(&self, app: &AppHandle) {
        let pools: Vec<Pool> = self.pools.lock().await.drain().map(|(_, p)| p).collect();
        if pools.is_empty() {
            return;
        }
        for pool in pools {
            stop_pool(pool).await;
        }
        logs::system(app, Level::Info, "Stopped PHP workers");
    }

    pub async fn status(&self) -> Vec<PoolStatus> {
        let mut out: Vec<PoolStatus> = self
            .pools
            .lock()
            .await
            .values()
            .map(|p| PoolStatus {
                version: p.plan.version.clone(),
                port: p.plan.port,
                workers: p.plan.workers,
                alive: p.alive.load(Ordering::Relaxed),
            })
            .collect();
        out.sort_by_key(|a| a.port);
        out
    }
}

async fn stop_pool(pool: Pool) {
    let _ = pool.stop.send(true);
    for task in pool.tasks {
        let _ = task.await;
    }
}

fn start_pool(app: &AppHandle, settings: &Settings, layout: &Layout, plan: &PoolPlan) -> Result<Pool> {
    let home = layout.bin("php", &plan.version);
    let ini = configs::write_php(settings, layout, &plan.version, plan.port)?;
    let (stop, stop_rx) = watch::channel(false);
    let alive = Arc::new(AtomicUsize::new(0));
    let path = |p: PathBuf| p.to_string_lossy().into_owned();

    let mut tasks = Vec::new();
    for port in plan.ports() {
        let worker = if cfg!(windows) {
            Worker {
                program: home.join(exe("php-cgi")),
                args: vec!["-b".into(), format!("127.0.0.1:{port}"), "-c".into(), path(ini.clone())],
                // php-cgi exits after 500 requests by default; the pool restarts it, but
                // there's no reason to churn.
                env: vec![("PHP_FCGI_MAX_REQUESTS", "0".into())],
            }
        } else {
            Worker {
                program: home.join("php-fpm"),
                args: vec![
                    "--nodaemonize".into(),
                    "--fpm-config".into(),
                    path(layout.etc("php").join(&plan.version).join("php-fpm.conf")),
                    "-c".into(),
                    path(ini.clone()),
                ],
                env: vec![],
            }
        };
        if !worker.program.exists() {
            return Err(Error::msg(format!("{} not found", worker.program.display())));
        }
        let label = format!("PHP {} worker on port {port}", plan.version);
        tasks.push(tauri::async_runtime::spawn(run_worker(
            app.clone(),
            worker,
            home.clone(),
            label,
            stop_rx.clone(),
            alive.clone(),
        )));
    }
    Ok(Pool { plan: plan.clone(), stop, tasks, alive })
}

struct Worker {
    program: PathBuf,
    args: Vec<String>,
    env: Vec<(&'static str, String)>,
}

/// Keeps one worker process running until told to stop, restarting it if it exits.
/// Gives up when it keeps crashing (e.g. a missing DLL) instead of looping forever.
async fn run_worker(
    app: AppHandle,
    worker: Worker,
    home: PathBuf,
    label: String,
    mut stop: watch::Receiver<bool>,
    alive: Arc<AtomicUsize>,
) {
    let mut recent_exits: Vec<Instant> = Vec::new();
    loop {
        if *stop.borrow() {
            return;
        }
        let mut cmd = Command::new(&worker.program);
        cmd.args(&worker.args)
            .current_dir(&home)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        for (k, v) in &worker.env {
            cmd.env(k, v);
        }
        // PHP's extensions load DLLs from its own folder.
        let existing = std::env::var_os("PATH").unwrap_or_default();
        let paths = std::iter::once(home.clone()).chain(std::env::split_paths(&existing));
        if let Ok(joined) = std::env::join_paths(paths) {
            cmd.env("PATH", joined);
        }
        procgroup::prepare(&mut cmd);

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                logs::system(&app, Level::Error, format!("{label} could not start: {e}"));
                return;
            }
        };
        let group = ProcGroup::attach(&child).ok();
        pipe(app.clone(), child.stdout.take());
        pipe(app.clone(), child.stderr.take());
        alive.fetch_add(1, Ordering::Relaxed);

        let stopped = tokio::select! {
            _ = child.wait() => false,
            _ = stop.changed() => true,
        };
        if let Some(g) = &group {
            g.kill();
        } else {
            let _ = child.start_kill();
        }
        let _ = child.wait().await;
        alive.fetch_sub(1, Ordering::Relaxed);
        if stopped {
            return;
        }

        let now = Instant::now();
        recent_exits.retain(|t| now.duration_since(*t) < Duration::from_secs(30));
        recent_exits.push(now);
        if recent_exits.len() >= 5 {
            logs::system(
                &app,
                Level::Error,
                format!("{label} keeps exiting; stopped restarting it. Check the PHP error log and the Visual C++ runtime."),
            );
            return;
        }
        logs::system(&app, Level::Warn, format!("{label} exited; restarting it"));
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

fn pipe<R: AsyncRead + Unpin + Send + 'static>(app: AppHandle, reader: Option<R>) {
    let Some(reader) = reader else { return };
    tauri::async_runtime::spawn(async move {
        let mut lines = BufReader::new(reader).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            logs::push(&app, "php", "output", line);
        }
    });
}
