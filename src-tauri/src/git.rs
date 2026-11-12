//! Git for a project, through the `git` command so the user's own credential helpers, SSH
//! keys and config apply: status, staging, commits, branches, history with diffs, fetch,
//! pull and push, conflicts, remotes, the repository's own identity, clone and init.
//!
//! Git never asks on a terminal here (GIT_TERMINAL_PROMPT=0, stdin closed): a remote that
//! needs a password the credential helper doesn't have fails with git's message instead of
//! hanging. User input goes in as separate arguments, after `--` where git allows it, and
//! names that would read as options are refused. Remote URLs lose their credentials before
//! they reach the UI.

use std::{
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

use serde::{Deserialize, Serialize};

use crate::{
    error::{Error, Result},
    procgroup,
};

/// Diffs bigger than this are cut, so a minified bundle or a lock file can't stall the UI.
const MAX_DIFF: usize = 400 * 1024;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    /// None when the folder isn't a repository (yet).
    pub repo: bool,
    /// The branch, or None when HEAD is detached (then `head` is the commit).
    pub branch: Option<String>,
    pub head: Option<String>,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub files: Vec<FileChange>,
    /// "merge", "rebase", "cherry-pick" or "revert" while one is in progress.
    pub operation: Option<&'static str>,
    pub identity: Identity,
    pub remotes: Vec<Remote>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FileChange {
    pub path: String,
    /// The old path of a rename or copy.
    pub from: Option<String>,
    /// The index (staged) side: ' ' unchanged, M, A, D, R, C, T, or '?' for untracked.
    pub staged: char,
    /// The working-tree side, same letters.
    pub unstaged: char,
    pub conflicted: bool,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Identity {
    pub name: Option<String>,
    pub email: Option<String>,
    /// Whether this repository sets its own (git config --local), rather than the global one.
    pub local: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Remote {
    pub name: String,
    pub url: String,
    /// Where pushes go when it differs from `url` (remote.<name>.pushurl).
    pub push_url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Branch {
    pub name: String,
    pub remote: bool,
    pub current: bool,
    pub upstream: Option<String>,
    /// "[ahead 1, behind 2]", "[gone]" or empty.
    pub track: String,
    pub time: i64,
    pub subject: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Commit {
    pub hash: String,
    pub short: String,
    pub author: String,
    pub email: String,
    pub time: i64,
    /// Branches and tags pointing at it ("HEAD -> main, origin/main, tag: v1").
    pub refs: String,
    pub subject: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum BranchAction {
    Create { branch: String, from: Option<String>, switch: bool },
    Switch { branch: String },
    Rename { from: String, to: String },
    Delete { branch: String, force: bool },
    Merge { branch: String },
    /// The remote branch the current one tracks (for pull, push and ahead/behind).
    SetUpstream { upstream: String },
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum Sync {
    Fetch,
    Pull,
    Push,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum RemoteAction {
    Add { name: String, url: String },
    SetUrl { name: String, url: String },
    /// A separate URL for pushes; None removes it, so pushes use the fetch URL again.
    SetPushUrl { name: String, url: Option<String> },
    Rename { from: String, to: String },
    Remove { name: String },
}

/// The repository's own settings for syncing (git config --local), as the Remotes view edits them.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoSettings {
    /// How pull combines: "merge", "rebase" or "ff-only"; None follows the global config.
    pub pull: Option<String>,
    /// Remove remote branches that are gone on fetch (fetch.prune).
    pub prune: bool,
    /// A new branch's first push sets its upstream (push.autoSetupRemote).
    pub auto_setup_remote: bool,
}

/// One commit in full, for the History view.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitDetails {
    pub hash: String,
    pub parents: Vec<String>,
    pub author: String,
    pub author_email: String,
    pub author_time: i64,
    pub committer: String,
    pub committer_email: String,
    pub committer_time: i64,
    pub refs: String,
    pub subject: String,
    pub body: String,
    pub files: Vec<CommitFile>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommitFile {
    pub path: String,
    pub from: Option<String>,
    /// A added, M modified, D deleted, R renamed, C copied, T type changed.
    pub status: char,
    pub additions: u32,
    pub deletions: u32,
    pub binary: bool,
}

/// Runs git in `dir` and returns its output whatever the exit code.
async fn run(dir: &Path, args: &[&str], timeout: Duration) -> Result<std::process::Output> {
    let mut cmd = tokio::process::Command::new("git");
    // Paths as they are (not "\303\251" escapes), no colour, no line-ending warnings mixed
    // into output ("LF will be replaced by CRLF"), and no editor or pager ever.
    cmd.args(["-c", "core.quotepath=false", "-c", "color.ui=false", "-c", "protocol.ext.allow=never", "-c", "core.safecrlf=false"])
        .args(args)
        .current_dir(dir)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_EDITOR", "true")
        .env("GIT_PAGER", "cat")
        .env("GCM_INTERACTIVE", "auto")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    procgroup::prepare(&mut cmd);
    let out = match tokio::time::timeout(timeout, cmd.output()).await {
        Err(_) => return Err(Error::msg(format!("git {} took longer than {} seconds and was stopped.", args.first().unwrap_or(&""), timeout.as_secs()))),
        Ok(Err(e)) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(Error::msg("Git isn't installed (or not on PATH). Install it from git-scm.com, then try again."));
        }
        Ok(r) => r?,
    };
    Ok(out)
}

/// Network operations in flight, by the id the UI gave them, so they can be cancelled.
static RUNNING: std::sync::LazyLock<std::sync::Mutex<std::collections::HashMap<String, tokio::sync::oneshot::Sender<()>>>> = std::sync::LazyLock::new(Default::default);

/// Progress of a clone, fetch, pull or push: git's own progress lines ("Receiving objects:
/// 45% (450/1000)"), sent as `git-progress` while it runs.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub op: String,
    pub line: String,
    pub done: bool,
}

/// Stops an operation started with `streamed`; false when it already ended.
pub fn cancel(op: &str) -> bool {
    RUNNING.lock().ok().and_then(|mut m| m.remove(op)).is_some_and(|tx| tx.send(()).is_ok())
}

/// Runs a long git command (with --progress), sending its progress lines as events and
/// stopping it when `cancel(op)` is called. Ok(stdout and the last messages) on success.
async fn streamed(app: &tauri::AppHandle, op: &str, dir: &Path, args: &[&str], timeout: Duration) -> Result<String> {
    use tauri::Emitter;
    use tokio::io::AsyncReadExt;
    let mut cmd = tokio::process::Command::new("git");
    cmd.args(["-c", "core.quotepath=false", "-c", "color.ui=false", "-c", "protocol.ext.allow=never", "-c", "core.safecrlf=false"])
        .args(args)
        .current_dir(dir)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_EDITOR", "true")
        .env("GCM_INTERACTIVE", "auto")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    procgroup::prepare(&mut cmd);
    let mut child = cmd.spawn().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            Error::msg("Git isn't installed (or not on PATH). Install it from Services → Essentials, then try again.")
        } else {
            e.into()
        }
    })?;
    let (tx, mut rx) = tokio::sync::oneshot::channel();
    if let Ok(mut m) = RUNNING.lock() {
        m.insert(op.to_string(), tx);
    }
    let mut stdout = child.stdout.take().expect("piped");
    let mut stderr = child.stderr.take().expect("piped");
    let out_task = tokio::spawn(async move {
        let mut buf = Vec::new();
        let _ = stdout.read_to_end(&mut buf).await;
        buf
    });
    // Progress lines end in \r (redrawn in place) or \n; each becomes one event.
    let (app2, op2) = (app.clone(), op.to_string());
    let err_task = tokio::spawn(async move {
        let mut lines: Vec<String> = Vec::new();
        let mut current = Vec::new();
        let mut chunk = [0u8; 4096];
        while let Ok(n) = stderr.read(&mut chunk).await {
            if n == 0 {
                break;
            }
            for &b in &chunk[..n] {
                if b == b'\r' || b == b'\n' {
                    let line = without_secrets(String::from_utf8_lossy(&current).trim());
                    current.clear();
                    if line.is_empty() {
                        continue;
                    }
                    let _ = app2.emit("git-progress", Progress { op: op2.clone(), line: line.clone(), done: false });
                    // Keep messages, not every redraw of a percentage.
                    if b == b'\n' {
                        lines.push(line);
                    }
                } else {
                    current.push(b);
                }
            }
        }
        if !current.is_empty() {
            lines.push(without_secrets(String::from_utf8_lossy(&current).trim()));
        }
        lines
    });
    let outcome = tokio::select! {
        status = child.wait() => Some(status),
        _ = &mut rx => None,
        _ = tokio::time::sleep(timeout) => {
            let _ = child.start_kill();
            let _ = app.emit("git-progress", Progress { op: op.to_string(), line: String::new(), done: true });
            if let Ok(mut m) = RUNNING.lock() { m.remove(op); }
            return Err(Error::msg(format!("git {} took longer than {} minutes and was stopped.", args.first().unwrap_or(&""), timeout.as_secs() / 60)));
        }
    };
    if let Ok(mut m) = RUNNING.lock() {
        m.remove(op);
    }
    let _ = app.emit("git-progress", Progress { op: op.to_string(), line: String::new(), done: true });
    let Some(status) = outcome else {
        let _ = child.start_kill();
        let _ = child.wait().await;
        return Err(Error::msg("Cancelled."));
    };
    let status = status?;
    let stdout = String::from_utf8_lossy(&out_task.await.unwrap_or_default()).into_owned();
    let messages: Vec<String> = err_task.await.unwrap_or_default().into_iter().filter(|l| !l.starts_with("hint:") && !is_progress(l)).collect();
    if status.success() {
        return Ok(without_secrets(&format!("{}{}", stdout, messages.join("\n"))));
    }
    let message = messages.join("\n");
    Err(Error::msg(if message.is_empty() { format!("git {} failed.", args.first().unwrap_or(&"")) } else { message }))
}

/// "Receiving objects:  45% (450/1000)": a redraw, not a message worth keeping.
fn is_progress(line: &str) -> bool {
    line.contains('%') && (line.contains("objects") || line.contains("deltas") || line.contains("files"))
}

/// Runs git in `dir`; Ok(stdout) on success, else git's own message (stderr) as the error.
async fn git(dir: &Path, args: &[&str], timeout: Duration) -> Result<String> {
    let out = run(dir, args, timeout).await?;
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    if out.status.success() {
        return Ok(stdout);
    }
    let stderr = String::from_utf8_lossy(&out.stderr);
    let message = stderr.lines().chain(stdout.lines()).map(str::trim).filter(|l| !l.is_empty() && !l.starts_with("hint:")).collect::<Vec<_>>().join("\n");
    Err(Error::msg(if message.is_empty() { format!("git {} failed.", args.first().unwrap_or(&"")) } else { without_secrets(&message) }))
}

const QUICK: Duration = Duration::from_secs(20);
const NETWORK: Duration = Duration::from_secs(300);

/// Whether git is installed, and its version.
pub async fn version() -> Option<String> {
    let dir = std::env::temp_dir();
    git(&dir, &["--version"], QUICK).await.ok().map(|v| v.trim().trim_start_matches("git version ").to_string())
}

/// A name for a branch or remote that git accepts and that can't be read as an option.
async fn check_name(dir: &Path, name: &str, what: &str) -> Result<()> {
    if name.is_empty() || name.starts_with('-') || name.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(Error::msg(format!("\"{name}\" can't be a {what} name: no spaces, and it can't start with -.")));
    }
    if what == "branch" && git(dir, &["check-ref-format", "--branch", name], QUICK).await.is_err() {
        return Err(Error::msg(format!("\"{name}\" isn't a valid branch name (no .., ~, ^, :, ?, *, [ or a trailing / or .lock).")));
    }
    Ok(())
}

fn check_url(url: &str) -> Result<()> {
    let url = url.trim();
    if url.is_empty() || url.starts_with('-') || url.contains(char::is_whitespace) || url.to_ascii_lowercase().starts_with("ext::") {
        return Err(Error::msg("That isn't a repository URL: use https://…, git@host:owner/repo.git or ssh://…"));
    }
    Ok(())
}

/// Paths inside the repository only (relative, no ..), so a crafted path can't reach out.
fn check_paths(paths: &[String]) -> Result<()> {
    for p in paths {
        let path = Path::new(p);
        if p.is_empty() || path.is_absolute() || path.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
            return Err(Error::msg(format!("\"{p}\" isn't a path inside the project.")));
        }
    }
    Ok(())
}

pub async fn status(dir: &Path) -> Result<Status> {
    let identity = identity(dir).await;
    if !is_repo(dir).await {
        return Ok(Status { repo: false, branch: None, head: None, upstream: None, ahead: 0, behind: 0, files: Vec::new(), operation: None, identity, remotes: Vec::new() });
    }
    let out = git(dir, &["--no-optional-locks", "status", "--porcelain=v2", "--branch", "-z", "--untracked-files=all"], QUICK).await?;
    let mut status = parse_status(&out);
    status.identity = identity;
    status.operation = operation(dir).await;
    status.remotes = remotes(dir).await;
    Ok(status)
}

async fn is_repo(dir: &Path) -> bool {
    git(dir, &["rev-parse", "--is-inside-work-tree"], QUICK).await.is_ok_and(|o| o.trim() == "true")
}

fn parse_status(out: &str) -> Status {
    let mut status = Status { repo: true, branch: None, head: None, upstream: None, ahead: 0, behind: 0, files: Vec::new(), operation: None, identity: Identity::default(), remotes: Vec::new() };
    let mut fields = out.split('\0').filter(|f| !f.is_empty());
    while let Some(entry) = fields.next() {
        if let Some(header) = entry.strip_prefix("# ") {
            let (key, value) = header.split_once(' ').unwrap_or((header, ""));
            match key {
                "branch.oid" if value != "(initial)" => status.head = Some(value.chars().take(12).collect()),
                "branch.head" if value != "(detached)" => status.branch = Some(value.to_string()),
                "branch.upstream" => status.upstream = Some(value.to_string()),
                "branch.ab" => {
                    for part in value.split_whitespace() {
                        if let Some(n) = part.strip_prefix('+') {
                            status.ahead = n.parse().unwrap_or(0);
                        } else if let Some(n) = part.strip_prefix('-') {
                            status.behind = n.parse().unwrap_or(0);
                        }
                    }
                }
                _ => {}
            }
            continue;
        }
        let kind = entry.as_bytes()[0];
        let xy = |e: &str| -> (char, char) {
            let mut c = e.split(' ').nth(1).unwrap_or("..").chars();
            let dot = |c: char| if c == '.' { ' ' } else { c };
            (dot(c.next().unwrap_or('.')), dot(c.next().unwrap_or('.')))
        };
        match kind {
            b'1' => {
                let (staged, unstaged) = xy(entry);
                let path = entry.splitn(9, ' ').nth(8).unwrap_or_default().to_string();
                status.files.push(FileChange { path, from: None, staged, unstaged, conflicted: false });
            }
            b'2' => {
                let (staged, unstaged) = xy(entry);
                let path = entry.splitn(10, ' ').nth(9).unwrap_or_default().to_string();
                let from = fields.next().map(str::to_string);
                status.files.push(FileChange { path, from, staged, unstaged, conflicted: false });
            }
            b'u' => {
                let (staged, unstaged) = xy(entry);
                let path = entry.splitn(11, ' ').nth(10).unwrap_or_default().to_string();
                status.files.push(FileChange { path, from: None, staged, unstaged, conflicted: true });
            }
            b'?' => status.files.push(FileChange { path: entry[2..].to_string(), from: None, staged: '?', unstaged: '?', conflicted: false }),
            _ => {}
        }
    }
    status
}

/// A merge, rebase, cherry-pick or revert left half done, from the files git keeps for it.
async fn operation(dir: &Path) -> Option<&'static str> {
    let git_dir = git(dir, &["rev-parse", "--absolute-git-dir"], QUICK).await.ok()?;
    let git_dir = PathBuf::from(git_dir.trim());
    [("MERGE_HEAD", "merge"), ("rebase-merge", "rebase"), ("rebase-apply", "rebase"), ("CHERRY_PICK_HEAD", "cherry-pick"), ("REVERT_HEAD", "revert")]
        .into_iter()
        .find(|(file, _)| git_dir.join(file).exists())
        .map(|(_, op)| op)
}

async fn identity(dir: &Path) -> Identity {
    let get = |args: &'static [&'static str]| async move { git(dir, args, QUICK).await.ok().map(|v| v.trim().to_string()).filter(|v| !v.is_empty()) };
    let local_name = get(&["config", "--local", "user.name"]).await;
    let local_email = get(&["config", "--local", "user.email"]).await;
    let local = local_name.is_some() || local_email.is_some();
    Identity {
        name: local_name.or(get(&["config", "user.name"]).await),
        email: local_email.or(get(&["config", "user.email"]).await),
        local,
    }
}

async fn remotes(dir: &Path) -> Vec<Remote> {
    let out = git(dir, &["remote", "-v"], QUICK).await.unwrap_or_default();
    let mut list: Vec<Remote> = Vec::new();
    for line in out.lines() {
        let mut parts = line.split_whitespace();
        let (Some(name), Some(url), kind) = (parts.next(), parts.next(), parts.next()) else { continue };
        let url = crate::facts::without_credentials(url);
        match list.iter_mut().find(|r| r.name == name) {
            Some(r) if kind == Some("(push)") && r.url != url => r.push_url = Some(url),
            Some(_) => {}
            None => list.push(Remote { name: name.to_string(), url, push_url: None }),
        }
    }
    list
}

/// A credential that slipped into a message (a remote URL with a token) is cut out.
fn without_secrets(text: &str) -> String {
    text.split_inclusive(char::is_whitespace)
        .map(|word| {
            let trimmed = word.trim_end().trim_matches(|c| c == '\'' || c == '"');
            if trimmed.contains("://") && trimmed.contains('@') {
                word.replace(trimmed, &crate::facts::without_credentials(trimmed))
            } else {
                word.to_string()
            }
        })
        .collect()
}

fn cut(mut text: String) -> String {
    if text.len() > MAX_DIFF {
        let mut end = MAX_DIFF;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        text.truncate(end);
        text.push_str("\n… (diff cut here: it's over 400 KB)\n");
    }
    text
}

/// One file's changes: staged (index vs HEAD) or not (working tree vs index); an untracked
/// file shows as all new.
pub async fn diff(dir: &Path, path: &str, staged: bool, untracked: bool) -> Result<String> {
    check_paths(&[path.to_string()])?;
    if untracked {
        // --no-index exits with 1 when the files differ, which is always here: only its
        // stdout is the diff (stderr may hold warnings that aren't part of the file).
        let empty = if cfg!(windows) { "NUL" } else { "/dev/null" };
        let out = run(dir, &["diff", "--no-index", "--no-color", "--", empty, path], QUICK).await?;
        if out.status.code().is_some_and(|c| c > 1) {
            return Err(Error::msg(String::from_utf8_lossy(&out.stderr).trim().to_string()));
        }
        return Ok(cut(String::from_utf8_lossy(&out.stdout).into_owned()));
    }
    let mut args = vec!["diff", "--no-color", "--no-ext-diff"];
    if staged {
        args.push("--cached");
    }
    args.extend(["--", path]);
    Ok(cut(git(dir, &args, QUICK).await?))
}

pub async fn stage(dir: &Path, paths: &[String], stage: bool) -> Result<()> {
    check_paths(paths)?;
    let refs: Vec<&str> = paths.iter().map(String::as_str).collect();
    if stage {
        let mut args = vec!["add", "--all", "--"];
        args.extend(refs);
        git(dir, &args, QUICK).await?;
    } else if git(dir, &["rev-parse", "--verify", "HEAD"], QUICK).await.is_ok() {
        let mut args = vec!["restore", "--staged", "--"];
        args.extend(refs);
        git(dir, &args, QUICK).await?;
    } else {
        // No commit yet, so nothing to restore from: just take them out of the index.
        let mut args = vec!["rm", "--cached", "-r", "--quiet", "--"];
        args.extend(refs);
        git(dir, &args, QUICK).await?;
    }
    Ok(())
}

/// Throws away working-tree changes; untracked files are deleted. The UI asks first.
pub async fn discard(dir: &Path, paths: &[String]) -> Result<()> {
    check_paths(paths)?;
    let status = status(dir).await?;
    let (untracked, tracked): (Vec<&String>, Vec<&String>) = paths.iter().partition(|p| status.files.iter().any(|f| &f.path == *p && f.staged == '?'));
    if !tracked.is_empty() {
        let mut args = vec!["restore", "--worktree", "--"];
        args.extend(tracked.iter().map(|s| s.as_str()));
        git(dir, &args, QUICK).await?;
    }
    if !untracked.is_empty() {
        let mut args = vec!["clean", "-f", "-q", "--"];
        args.extend(untracked.iter().map(|s| s.as_str()));
        git(dir, &args, QUICK).await?;
    }
    Ok(())
}

/// Commits what's staged; with `all`, everything changed is staged first (new files too),
/// for a commit straight from a message without staging by hand.
pub async fn commit(dir: &Path, message: &str, amend: bool, all: bool) -> Result<String> {
    let message = message.trim();
    if message.is_empty() && !amend {
        return Err(Error::msg("Write a commit message first."));
    }
    let who = identity(dir).await;
    if who.name.is_none() || who.email.is_none() {
        return Err(Error::msg("Git doesn't know who you are yet: set your name and email under Identity, then commit again."));
    }
    if all {
        git(dir, &["add", "--all"], QUICK).await?;
    }
    // The user's hooks and signing settings apply as they would on the command line.
    let mut args = vec!["commit"];
    if amend {
        args.push("--amend");
        if message.is_empty() {
            args.push("--no-edit");
        }
    }
    if !message.is_empty() {
        args.extend(["-m", message]);
    }
    git(dir, &args, Duration::from_secs(120)).await
}

pub async fn branches(dir: &Path) -> Result<Vec<Branch>> {
    let out = git(
        dir,
        &["for-each-ref", "--sort=-committerdate", "--format=%(refname)%1f%(refname:short)%1f%(upstream:short)%1f%(upstream:track)%1f%(HEAD)%1f%(committerdate:unix)%1f%(subject)", "refs/heads", "refs/remotes"],
        QUICK,
    )
    .await?;
    Ok(out
        .lines()
        .filter_map(|line| {
            let f: Vec<&str> = line.split('\u{1f}').collect();
            if f.len() < 7 || f[0].ends_with("/HEAD") {
                return None;
            }
            Some(Branch {
                name: f[1].to_string(),
                remote: f[0].starts_with("refs/remotes/"),
                current: f[4] == "*",
                upstream: Some(f[2].to_string()).filter(|u| !u.is_empty()),
                track: f[3].to_string(),
                time: f[5].parse().unwrap_or(0),
                subject: f[6].to_string(),
            })
        })
        .collect())
}

pub async fn branch(dir: &Path, action: BranchAction) -> Result<String> {
    match action {
        BranchAction::Create { branch, from, switch } => {
            check_name(dir, &branch, "branch").await?;
            let mut args = if switch { vec!["switch", "-c", branch.as_str()] } else { vec!["branch", branch.as_str()] };
            if let Some(from) = from.as_deref().filter(|f| !f.is_empty()) {
                if from.starts_with('-') {
                    return Err(Error::msg("That isn't a branch to start from."));
                }
                args.push(from);
            }
            git(dir, &args, QUICK).await
        }
        BranchAction::Switch { branch } => {
            if branch.starts_with('-') {
                return Err(Error::msg("That isn't a branch."));
            }
            // A remote branch (origin/feature) switches to a local one tracking it.
            match branch.split_once('/') {
                Some((remote, local)) if remotes(dir).await.iter().any(|r| r.name == remote) => {
                    if branches(dir).await?.iter().any(|b| !b.remote && b.name == local) {
                        git(dir, &["switch", local], QUICK).await
                    } else {
                        git(dir, &["switch", "--track", &branch], QUICK).await
                    }
                }
                _ => git(dir, &["switch", &branch], QUICK).await,
            }
        }
        BranchAction::Rename { from, to } => {
            check_name(dir, &to, "branch").await?;
            if from.starts_with('-') {
                return Err(Error::msg("That isn't a branch."));
            }
            git(dir, &["branch", "-m", &from, &to], QUICK).await
        }
        BranchAction::Delete { branch, force } => {
            if branch.starts_with('-') {
                return Err(Error::msg("That isn't a branch."));
            }
            git(dir, &["branch", if force { "-D" } else { "-d" }, &branch], QUICK).await.map_err(|e| {
                let text = e.to_string();
                if text.contains("not fully merged") {
                    Error::msg(format!("{branch} has commits that aren't merged anywhere. Delete it anyway to lose them."))
                } else {
                    e
                }
            })
        }
        BranchAction::SetUpstream { upstream } => {
            if upstream.starts_with('-') || upstream.is_empty() {
                return Err(Error::msg("That isn't a remote branch."));
            }
            git(dir, &["branch", &format!("--set-upstream-to={upstream}")], QUICK).await
        }
        BranchAction::Merge { branch } => {
            if branch.starts_with('-') {
                return Err(Error::msg("That isn't a branch."));
            }
            git(dir, &["merge", "--no-edit", &branch], Duration::from_secs(120)).await
        }
    }
}

/// Stops a merge, rebase, cherry-pick or revert in progress, back to where it started.
pub async fn abort(dir: &Path) -> Result<()> {
    let Some(op) = operation(dir).await else { return Err(Error::msg("Nothing is in progress to abort.")) };
    git(dir, &[op, "--abort"], QUICK).await?;
    Ok(())
}

/// Carries on a rebase, cherry-pick or merge once conflicts are resolved and staged.
pub async fn continue_operation(dir: &Path) -> Result<String> {
    let Some(op) = operation(dir).await else { return Err(Error::msg("Nothing is in progress.")) };
    if op == "merge" {
        return git(dir, &["commit", "--no-edit"], Duration::from_secs(120)).await;
    }
    git(dir, &[op, "--continue"], Duration::from_secs(120)).await
}

pub async fn log(dir: &Path, skip: u32, limit: u32, path: Option<&str>) -> Result<Vec<Commit>> {
    if git(dir, &["rev-parse", "--verify", "HEAD"], QUICK).await.is_err() {
        return Ok(Vec::new());
    }
    let skip = format!("--skip={skip}");
    let limit = format!("--max-count={}", limit.clamp(1, 500));
    let mut args = vec!["log", "--format=%H%x1f%h%x1f%an%x1f%ae%x1f%ct%x1f%D%x1f%s%x1e", skip.as_str(), limit.as_str()];
    if let Some(path) = path {
        check_paths(&[path.to_string()])?;
        args.extend(["--", path]);
    }
    let out = git(dir, &args, QUICK).await?;
    Ok(out
        .split('\u{1e}')
        .filter_map(|record| {
            let f: Vec<&str> = record.trim_start_matches('\n').split('\u{1f}').collect();
            (f.len() >= 7).then(|| Commit {
                hash: f[0].to_string(),
                short: f[1].to_string(),
                author: f[2].to_string(),
                email: f[3].to_string(),
                time: f[4].parse().unwrap_or(0),
                refs: f[5].to_string(),
                subject: f[6].to_string(),
            })
        })
        .collect())
}

fn check_hash(hash: &str) -> Result<()> {
    if hash.is_empty() || hash.len() > 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(Error::msg("That isn't a commit."));
    }
    Ok(())
}

/// A commit's metadata, message and the files it changed with their line counts. A merge
/// is compared with its first parent, which is what it brought into the branch.
pub async fn details(dir: &Path, hash: &str) -> Result<CommitDetails> {
    check_hash(hash)?;
    let meta = git(dir, &["show", "-s", "--format=%H%x1f%P%x1f%an%x1f%ae%x1f%at%x1f%cn%x1f%ce%x1f%ct%x1f%D%x1f%s%x1f%b", hash], QUICK).await?;
    let f: Vec<&str> = meta.splitn(11, '\u{1f}').collect();
    if f.len() < 11 {
        return Err(Error::msg("Git didn't describe that commit."));
    }
    let numstat = git(dir, &["show", "--format=", "--numstat", "-z", "-M", "--diff-merges=first-parent", hash], QUICK).await?;
    let names = git(dir, &["show", "--format=", "--name-status", "-z", "-M", "--diff-merges=first-parent", hash], QUICK).await?;
    Ok(CommitDetails {
        hash: f[0].trim().to_string(),
        parents: f[1].split_whitespace().map(str::to_string).collect(),
        author: f[2].to_string(),
        author_email: f[3].to_string(),
        author_time: f[4].parse().unwrap_or(0),
        committer: f[5].to_string(),
        committer_email: f[6].to_string(),
        committer_time: f[7].parse().unwrap_or(0),
        refs: f[8].to_string(),
        subject: f[9].to_string(),
        body: f[10].trim().to_string(),
        files: parse_commit_files(&numstat, &names),
    })
}

/// `--numstat -z` ("12\t3\tpath\0", renames "12\t3\t\0from\0to\0") joined with
/// `--name-status -z` ("M\0path\0", "R100\0from\0to\0").
fn parse_commit_files(numstat: &str, names: &str) -> Vec<CommitFile> {
    let mut files = Vec::new();
    let mut fields = numstat.split('\0').map(|f| f.trim_start_matches('\n'));
    while let Some(entry) = fields.next() {
        if entry.is_empty() {
            continue;
        }
        let mut parts = entry.splitn(3, '\t');
        let (Some(add), Some(del), Some(path)) = (parts.next(), parts.next(), parts.next()) else { continue };
        let (path, from) = if path.is_empty() {
            let from = fields.next().unwrap_or_default().to_string();
            (fields.next().unwrap_or_default().to_string(), Some(from))
        } else {
            (path.to_string(), None)
        };
        let binary = add == "-";
        files.push(CommitFile { path, from, status: 'M', additions: add.parse().unwrap_or(0), deletions: del.parse().unwrap_or(0), binary });
    }
    let mut fields = names.split('\0').map(|f| f.trim_start_matches('\n'));
    while let Some(code) = fields.next() {
        let Some(status) = code.chars().next() else { continue };
        let path = if matches!(status, 'R' | 'C') {
            fields.next();
            fields.next()
        } else {
            fields.next()
        };
        if let Some(file) = path.and_then(|p| files.iter_mut().find(|f| f.path == p)) {
            file.status = status;
        }
    }
    files
}

/// One file's change in a commit (against its first parent).
pub async fn commit_file_diff(dir: &Path, hash: &str, path: &str) -> Result<String> {
    check_hash(hash)?;
    check_paths(&[path.to_string()])?;
    Ok(cut(git(dir, &["show", "--format=", "--no-color", "-M", "--diff-merges=first-parent", hash, "--", path], QUICK).await?))
}

/// A stash entry: work set aside with `git stash`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Stash {
    /// stash@{0}, newest first.
    pub index: u32,
    pub message: String,
    pub time: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum StashAction {
    /// Sets the working tree's changes aside (new files too when `untracked`).
    Save { message: String, untracked: bool },
    /// Brings a stash back and keeps it in the list.
    Apply { index: u32 },
    /// Brings a stash back and removes it.
    Pop { index: u32 },
    Drop { index: u32 },
}

pub async fn stashes(dir: &Path) -> Result<Vec<Stash>> {
    let out = git(dir, &["stash", "list", "--format=%gd%x1f%ct%x1f%gs"], QUICK).await?;
    Ok(out
        .lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.splitn(3, '\u{1f}').collect();
            let index = f.first()?.trim_start_matches("stash@{").trim_end_matches('}').parse().ok()?;
            Some(Stash { index, time: f.get(1)?.parse().unwrap_or(0), message: f.get(2)?.to_string() })
        })
        .collect())
}

pub async fn stash(dir: &Path, action: StashAction) -> Result<String> {
    match action {
        StashAction::Save { message, untracked } => {
            let mut args = vec!["stash", "push"];
            if untracked {
                args.push("--include-untracked");
            }
            let message = message.trim().to_string();
            if !message.is_empty() {
                args.extend(["-m", message.as_str()]);
            }
            git(dir, &args, QUICK).await
        }
        StashAction::Apply { index } => git(dir, &["stash", "apply", &format!("stash@{{{index}}}")], QUICK).await,
        StashAction::Pop { index } => git(dir, &["stash", "pop", &format!("stash@{{{index}}}")], QUICK).await,
        StashAction::Drop { index } => git(dir, &["stash", "drop", &format!("stash@{{{index}}}")], QUICK).await,
    }
}

/// Stages (or unstages) one hunk of a file: the hunk is cut from the file's current diff and
/// applied to the index alone, like `git add -p` choosing just that part.
pub async fn stage_hunk(dir: &Path, path: &str, hunk: usize, unstage: bool) -> Result<()> {
    check_paths(&[path.to_string()])?;
    let mut args = vec!["diff", "--no-color", "--no-ext-diff", "-U3"];
    if unstage {
        args.push("--cached");
    }
    args.extend(["--", path]);
    let diff = git(dir, &args, QUICK).await?;
    let patch = hunk_patch(&diff, hunk).ok_or_else(|| Error::msg("That part of the file changed meanwhile; look at the diff again."))?;
    let file = std::env::temp_dir().join(format!("lamprabit-hunk-{}-{hunk}.patch", std::process::id()));
    std::fs::write(&file, patch)?;
    let file_arg = file.to_string_lossy().to_string();
    let mut apply = vec!["apply", "--cached", "--recount", "--whitespace=nowarn"];
    if unstage {
        apply.push("--reverse");
    }
    apply.push(&file_arg);
    let result = git(dir, &apply, QUICK).await;
    let _ = std::fs::remove_file(&file);
    result.map(drop)
}

/// The file header plus one hunk of a single-file diff, as a patch `git apply` takes.
fn hunk_patch(diff: &str, index: usize) -> Option<String> {
    let first = diff.find("\n@@")? + 1;
    let header = &diff[..first];
    let hunks: Vec<&str> = {
        let body = &diff[first..];
        let mut starts: Vec<usize> = body.match_indices("\n@@").map(|(i, _)| i + 1).collect();
        starts.insert(0, 0);
        starts.iter().enumerate().map(|(i, &s)| &body[s..*starts.get(i + 1).unwrap_or(&body.len())]).collect()
    };
    let hunk = hunks.get(index)?;
    let mut patch = format!("{header}{hunk}");
    if !patch.ends_with('\n') {
        patch.push('\n');
    }
    Some(patch)
}

/// Settles a conflicted file with one side's version (`ours` is the branch you're on,
/// `theirs` the one coming in), then marks it resolved.
pub async fn resolve(dir: &Path, path: &str, theirs: bool) -> Result<()> {
    check_paths(&[path.to_string()])?;
    git(dir, &["checkout", if theirs { "--theirs" } else { "--ours" }, "--", path], QUICK).await?;
    git(dir, &["add", "--", path], QUICK).await?;
    Ok(())
}

/// A tag: a name for a commit, usually a release.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    pub name: String,
    pub commit: String,
    pub subject: String,
    pub time: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum TagAction {
    /// An annotated tag when there's a message, a lightweight one otherwise; on HEAD unless `commit`.
    Create { name: String, message: String, commit: Option<String> },
    Delete { name: String },
    /// Sends one tag to a remote (pushes don't carry tags by default).
    Push { name: String, remote: String },
}

pub async fn tags(dir: &Path) -> Result<Vec<Tag>> {
    let out = git(dir, &["for-each-ref", "--sort=-creatordate", "--format=%(refname:short)%1f%(objectname:short)%1f%(contents:subject)%1f%(creatordate:unix)", "refs/tags"], QUICK).await?;
    Ok(out
        .lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split('\u{1f}').collect();
            (f.len() >= 4).then(|| Tag { name: f[0].to_string(), commit: f[1].to_string(), subject: f[2].to_string(), time: f[3].parse().unwrap_or(0) })
        })
        .collect())
}

pub async fn tag(dir: &Path, action: TagAction) -> Result<String> {
    match action {
        TagAction::Create { name, message, commit } => {
            if name.starts_with('-') || git(dir, &["check-ref-format", &format!("refs/tags/{name}")], QUICK).await.is_err() {
                return Err(Error::msg(format!("\"{name}\" isn't a valid tag name (like v1.2.0).")));
            }
            let mut args = vec!["tag"];
            let message = message.trim().to_string();
            if !message.is_empty() {
                args.extend(["-a", "-m", message.as_str()]);
            }
            args.push(&name);
            if let Some(c) = commit.as_deref() {
                check_hash(c)?;
                args.push(c);
            }
            git(dir, &args, QUICK).await
        }
        TagAction::Delete { name } => {
            if name.starts_with('-') {
                return Err(Error::msg("That isn't a tag."));
            }
            git(dir, &["tag", "-d", &name], QUICK).await
        }
        TagAction::Push { name, remote } => {
            if name.starts_with('-') || remote.starts_with('-') {
                return Err(Error::msg("That isn't a tag."));
            }
            git(dir, &["push", &remote, &format!("refs/tags/{name}")], NETWORK).await
        }
    }
}

/// Undoes a commit with a new commit that reverses it (history stays as it is).
pub async fn revert(dir: &Path, hash: &str) -> Result<String> {
    check_hash(hash)?;
    git(dir, &["revert", "--no-edit", hash], Duration::from_secs(120)).await
}

/// Puts a file back the way it was in a commit; the change shows under Changes, staged.
pub async fn restore_file(dir: &Path, hash: &str, path: &str) -> Result<()> {
    check_hash(hash)?;
    check_paths(&[path.to_string()])?;
    git(dir, &["checkout", hash, "--", path], QUICK).await.map(drop)
}

/// The repository's sync settings, from its own config.
pub async fn repo_settings(dir: &Path) -> RepoSettings {
    let get = |key: &'static str| async move { git(dir, &["config", "--local", "--get", key], QUICK).await.ok().map(|v| v.trim().to_lowercase()) };
    let pull = match (get("pull.rebase").await.as_deref(), get("pull.ff").await.as_deref()) {
        (Some("true" | "merges" | "interactive"), _) => Some("rebase".to_string()),
        (_, Some("only")) => Some("ff-only".to_string()),
        (Some("false"), _) => Some("merge".to_string()),
        _ => None,
    };
    RepoSettings { pull, prune: get("fetch.prune").await.as_deref() == Some("true"), auto_setup_remote: get("push.autosetupremote").await.as_deref() == Some("true") }
}

pub async fn set_repo_settings(dir: &Path, settings: &RepoSettings) -> Result<()> {
    let set = |key: &'static str, value: Option<&'static str>| async move {
        match value {
            Some(v) => git(dir, &["config", "--local", key, v], QUICK).await.map(drop),
            // Unsetting something that isn't set (exit 5) is fine.
            None => {
                let _ = git(dir, &["config", "--local", "--unset", key], QUICK).await;
                Ok(())
            }
        }
    };
    let (rebase, ff) = match settings.pull.as_deref() {
        Some("rebase") => (Some("true"), None),
        Some("ff-only") => (Some("false"), Some("only")),
        Some("merge") => (Some("false"), None),
        None => (None, None),
        Some(other) => return Err(Error::msg(format!("\"{other}\" isn't a pull mode: use merge, rebase or ff-only."))),
    };
    set("pull.rebase", rebase).await?;
    set("pull.ff", ff).await?;
    set("fetch.prune", settings.prune.then_some("true")).await?;
    set("push.autoSetupRemote", settings.auto_setup_remote.then_some("true")).await?;
    Ok(())
}

pub async fn sync(app: &tauri::AppHandle, op: &str, dir: &Path, action: Sync) -> Result<String> {
    let status = status(dir).await?;
    if status.remotes.is_empty() {
        return Err(Error::msg("This repository has no remote yet: add one under Remotes."));
    }
    let out = match action {
        Sync::Fetch => streamed(app, op, dir, &["fetch", "--all", "--progress"], NETWORK).await?,
        Sync::Pull => {
            if status.upstream.is_none() {
                return Err(Error::msg("This branch doesn't track a remote branch yet: push it first, or switch to one that does."));
            }
            streamed(app, op, dir, &["pull", "--no-edit", "--progress"], NETWORK).await?
        }
        Sync::Push => {
            let branch = status.branch.ok_or_else(|| Error::msg("HEAD is detached: switch to a branch before pushing."))?;
            if status.upstream.is_some() {
                streamed(app, op, dir, &["push", "--progress"], NETWORK).await?
            } else {
                // The first push of a branch sets it up to track the same name on the first remote.
                let remote = status.remotes.iter().find(|r| r.name == "origin").unwrap_or(&status.remotes[0]).name.clone();
                streamed(app, op, dir, &["push", "--progress", "--set-upstream", &remote, &branch], NETWORK).await?
            }
        }
    };
    Ok(without_secrets(&out))
}

pub async fn remote(dir: &Path, action: RemoteAction) -> Result<()> {
    match action {
        RemoteAction::Add { name, url } => {
            check_name(dir, &name, "remote").await?;
            check_url(&url)?;
            git(dir, &["remote", "add", "--", &name, url.trim()], QUICK).await?;
        }
        RemoteAction::SetUrl { name, url } => {
            check_name(dir, &name, "remote").await?;
            check_url(&url)?;
            git(dir, &["remote", "set-url", "--", &name, url.trim()], QUICK).await?;
        }
        RemoteAction::SetPushUrl { name, url } => {
            check_name(dir, &name, "remote").await?;
            match url.as_deref().map(str::trim).filter(|u| !u.is_empty()) {
                Some(url) => {
                    check_url(url)?;
                    git(dir, &["remote", "set-url", "--push", "--", &name, url], QUICK).await?;
                }
                None => {
                    let _ = git(dir, &["config", "--local", "--unset-all", &format!("remote.{name}.pushurl")], QUICK).await;
                }
            }
        }
        RemoteAction::Rename { from, to } => {
            check_name(dir, &from, "remote").await?;
            check_name(dir, &to, "remote").await?;
            git(dir, &["remote", "rename", "--", &from, &to], QUICK).await?;
        }
        RemoteAction::Remove { name } => {
            check_name(dir, &name, "remote").await?;
            git(dir, &["remote", "remove", "--", &name], QUICK).await?;
        }
    }
    Ok(())
}

/// The repository's own name and email (git config --local); empty values remove them, so
/// the global identity applies again. The global config is never changed here.
pub async fn set_identity(dir: &Path, name: &str, email: &str) -> Result<()> {
    for (key, value) in [("user.name", name.trim()), ("user.email", email.trim())] {
        if value.contains(['\n', '\r']) {
            return Err(Error::msg("Name and email have to fit on one line."));
        }
        if value.is_empty() {
            // Exit code 5 means it wasn't set, which is fine.
            let _ = git(dir, &["config", "--local", "--unset", key], QUICK).await;
        } else {
            git(dir, &["config", "--local", key, value], QUICK).await?;
        }
    }
    Ok(())
}

pub async fn init(dir: &Path) -> Result<()> {
    if is_repo(dir).await {
        return Err(Error::msg("This project is already a Git repository."));
    }
    git(dir, &["init"], QUICK).await?;
    Ok(())
}

/// Clones into www/<folder>; the folder must not exist yet.
pub async fn clone(app: &tauri::AppHandle, op: &str, www: &Path, url: &str, folder: &str) -> Result<PathBuf> {
    check_url(url)?;
    let target = www.join(folder);
    if target.exists() {
        return Err(Error::msg(format!("www/{folder} exists already: pick another folder name.")));
    }
    if let Err(e) = streamed(app, op, www, &["clone", "--progress", "--", url.trim(), folder], Duration::from_secs(1800)).await {
        // A cancelled or failed clone leaves a partial folder; it isn't a project.
        let _ = std::fs::remove_dir_all(&target);
        return Err(e);
    }
    Ok(target)
}

/// The folder name a clone gets by default: the repository's name.
pub fn folder_for(url: &str) -> String {
    let name = url.trim().trim_end_matches('/').rsplit(['/', ':']).next().unwrap_or_default().trim_end_matches(".git");
    name.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c.to_ascii_lowercase() } else { '-' }).collect::<String>().trim_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_porcelain_v2() {
        let out = "# branch.oid 1234567890abcdef1234\0# branch.head main\0# branch.upstream origin/main\0# branch.ab +2 -1\0\
1 .M N... 100644 100644 100644 aaa bbb src/app.php\0\
1 A. N... 000000 100644 100644 000 ccc new file.txt\0\
2 R. N... 100644 100644 100644 ddd eee R100 lib/new.rs\0lib/old.rs\0\
u UU N... 100644 100644 100644 100644 f1 f2 f3 conflict.txt\0\
? notes/todo.md\0";
        let s = parse_status(out);
        assert_eq!(s.branch.as_deref(), Some("main"));
        assert_eq!(s.upstream.as_deref(), Some("origin/main"));
        assert_eq!((s.ahead, s.behind), (2, 1));
        assert_eq!(s.head.as_deref(), Some("1234567890ab"));
        assert_eq!(s.files.len(), 5);
        assert_eq!(s.files[0], FileChange { path: "src/app.php".into(), from: None, staged: ' ', unstaged: 'M', conflicted: false });
        assert_eq!(s.files[1].path, "new file.txt");
        assert_eq!(s.files[1].staged, 'A');
        assert_eq!(s.files[2].path, "lib/new.rs");
        assert_eq!(s.files[2].from.as_deref(), Some("lib/old.rs"));
        assert!(s.files[3].conflicted);
        assert_eq!(s.files[4].path, "notes/todo.md");
        assert_eq!(s.files[4].staged, '?');
    }

    #[test]
    fn detached_and_new_repositories() {
        let s = parse_status("# branch.oid (initial)\0# branch.head main\0");
        assert_eq!(s.head, None);
        assert_eq!(s.branch.as_deref(), Some("main"));
        let s = parse_status("# branch.oid abcdef\0# branch.head (detached)\0");
        assert_eq!(s.branch, None);
    }

    #[test]
    fn refuses_paths_and_urls_that_reach_out() {
        assert!(check_paths(&["../secret".into()]).is_err());
        assert!(check_paths(&["src/../../x".into()]).is_err());
        assert!(check_paths(&["src/app.php".into()]).is_ok());
        assert!(check_url("--upload-pack=evil").is_err());
        assert!(check_url("ext::sh -c touch% /tmp/x").is_err());
        assert!(check_url("https://github.com/a/b.git").is_ok());
        assert!(check_url("git@github.com:a/b.git").is_ok());
    }

    #[test]
    fn cuts_one_hunk_out_of_a_diff() {
        let diff = "diff --git a/x b/x\nindex 1..2 100644\n--- a/x\n+++ b/x\n@@ -1,2 +1,2 @@\n-a\n+A\n b\n@@ -10,2 +10,2 @@\n c\n-d\n+D\n";
        assert_eq!(hunk_patch(diff, 1).unwrap(), "diff --git a/x b/x\nindex 1..2 100644\n--- a/x\n+++ b/x\n@@ -10,2 +10,2 @@\n c\n-d\n+D\n");
        assert!(hunk_patch(diff, 0).unwrap().contains("+A\n b\n") && !hunk_patch(diff, 0).unwrap().contains("+D"));
        assert!(hunk_patch(diff, 2).is_none());
    }

    #[test]
    fn reads_commit_files() {
        let numstat = "3\t1\tsrc/a.rs\0-\t-\tlogo.png\0\n5\t0\t\0old/b.rs\0new/b.rs\0";
        let names = "M\0src/a.rs\0A\0logo.png\0R090\0old/b.rs\0new/b.rs\0";
        let files = parse_commit_files(numstat, names);
        assert_eq!(files.len(), 3);
        assert_eq!((files[0].status, files[0].additions, files[0].deletions), ('M', 3, 1));
        assert!(files[1].binary && files[1].status == 'A');
        assert_eq!((files[2].path.as_str(), files[2].from.as_deref(), files[2].status), ("new/b.rs", Some("old/b.rs"), 'R'));
    }

    #[test]
    fn clone_folder_names() {
        assert_eq!(folder_for("https://github.com/laravel/Laravel.git"), "laravel");
        assert_eq!(folder_for("git@github.com:acme/shop-front.git/"), "shop-front");
        assert_eq!(folder_for("https://host/x/my repo"), "my-repo");
    }

    /// The whole flow on a real repository, when git is installed (CI has it).
    #[tokio::test]
    async fn works_on_a_real_repository() {
        if version().await.is_none() {
            return;
        }
        let dir = std::env::temp_dir().join(format!("lamprabit-git-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert!(!status(&dir).await.unwrap().repo);
        init(&dir).await.unwrap();
        git(&dir, &["checkout", "-q", "-b", "main"], QUICK).await.unwrap();
        set_identity(&dir, "Test", "test@example.com").await.unwrap();
        std::fs::write(dir.join("a.txt"), "one\n").unwrap();

        let s = status(&dir).await.unwrap();
        assert!(s.repo && s.identity.local);
        assert_eq!(s.files.len(), 1);
        assert_eq!(s.files[0].staged, '?');
        assert!(diff(&dir, "a.txt", false, true).await.unwrap().contains("+one"));

        stage(&dir, &["a.txt".into()], true).await.unwrap();
        assert_eq!(status(&dir).await.unwrap().files[0].staged, 'A');
        stage(&dir, &["a.txt".into()], false).await.unwrap();
        assert_eq!(status(&dir).await.unwrap().files[0].staged, '?');
        stage(&dir, &["a.txt".into()], true).await.unwrap();
        commit(&dir, "First", false, false).await.unwrap();

        std::fs::write(dir.join("a.txt"), "two\n").unwrap();
        assert!(diff(&dir, "a.txt", false, false).await.unwrap().contains("+two"));
        discard(&dir, &["a.txt".into()]).await.unwrap();
        assert_eq!(std::fs::read_to_string(dir.join("a.txt")).unwrap().trim(), "one");

        branch(&dir, BranchAction::Create { branch: "feature".into(), from: None, switch: true }).await.unwrap();
        assert!(branch(&dir, BranchAction::Create { branch: "-x".into(), from: None, switch: false }).await.is_err());
        assert!(branch(&dir, BranchAction::Create { branch: "a..b".into(), from: None, switch: false }).await.is_err());
        std::fs::write(dir.join("b.txt"), "b\n").unwrap();
        stage(&dir, &["b.txt".into()], true).await.unwrap();
        commit(&dir, "Second", false, false).await.unwrap();
        let list = branches(&dir).await.unwrap();
        assert!(list.iter().any(|b| b.name == "feature" && b.current));
        branch(&dir, BranchAction::Switch { branch: "main".into() }).await.unwrap();
        branch(&dir, BranchAction::Merge { branch: "feature".into() }).await.unwrap();
        branch(&dir, BranchAction::Delete { branch: "feature".into(), force: false }).await.unwrap();

        // One hunk of two staged on its own, then unstaged again.
        let lines: String = (1..=30).map(|i| format!("line {i}\n")).collect();
        std::fs::write(dir.join("long.txt"), &lines).unwrap();
        stage(&dir, &["long.txt".into()], true).await.unwrap();
        commit(&dir, "Long", false, false).await.unwrap();
        std::fs::write(dir.join("long.txt"), lines.replace("line 2\n", "line two\n").replace("line 29\n", "line twenty-nine\n")).unwrap();
        stage_hunk(&dir, "long.txt", 1, false).await.unwrap();
        let staged_diff = diff(&dir, "long.txt", true, false).await.unwrap();
        assert!(staged_diff.contains("+line twenty-nine") && !staged_diff.contains("+line two"));
        stage_hunk(&dir, "long.txt", 0, true).await.unwrap();
        assert!(diff(&dir, "long.txt", true, false).await.unwrap().is_empty());
        // Stash it away and back.
        stash(&dir, StashAction::Save { message: "wip".into(), untracked: true }).await.unwrap();
        assert!(status(&dir).await.unwrap().files.is_empty());
        assert_eq!(stashes(&dir).await.unwrap()[0].index, 0);
        stash(&dir, StashAction::Pop { index: 0 }).await.unwrap();
        assert!(stashes(&dir).await.unwrap().is_empty());
        discard(&dir, &["long.txt".into()]).await.unwrap();
        // Tags.
        tag(&dir, TagAction::Create { name: "v1.0".into(), message: "First".into(), commit: None }).await.unwrap();
        assert!(tag(&dir, TagAction::Create { name: "bad..name".into(), message: String::new(), commit: None }).await.is_err());
        assert_eq!(tags(&dir).await.unwrap()[0].name, "v1.0");
        tag(&dir, TagAction::Delete { name: "v1.0".into() }).await.unwrap();
        // Revert the last commit, and restore a file from an older one.
        let before = log(&dir, 0, 1, None).await.unwrap()[0].hash.clone();
        revert(&dir, &before).await.unwrap();
        assert!(!dir.join("long.txt").exists());
        restore_file(&dir, &before, "long.txt").await.unwrap();
        assert!(dir.join("long.txt").exists());
        commit(&dir, "Back", false, false).await.unwrap();

        let history = log(&dir, 0, 10, None).await.unwrap();
        assert_eq!(history.iter().map(|c| c.subject.as_str()).take(1).collect::<Vec<_>>(), ["Back"]);
        let history: Vec<_> = history.into_iter().filter(|c| c.subject == "Second" || c.subject == "First").collect();
        assert_eq!(history.iter().map(|c| c.subject.as_str()).collect::<Vec<_>>(), ["Second", "First"]);
        let second = details(&dir, &history[0].hash).await.unwrap();
        assert_eq!(second.subject, "Second");
        assert_eq!(second.files, vec![CommitFile { path: "b.txt".into(), from: None, status: 'A', additions: 1, deletions: 0, binary: false }]);
        assert!(commit_file_diff(&dir, &history[0].hash, "b.txt").await.unwrap().contains("+b"));
        std::fs::write(dir.join("c.txt"), "c\n").unwrap();
        commit(&dir, "Third, staged for me", false, true).await.unwrap();
        assert!(status(&dir).await.unwrap().files.is_empty());
        set_repo_settings(&dir, &RepoSettings { pull: Some("rebase".into()), prune: true, auto_setup_remote: true }).await.unwrap();
        let rs = repo_settings(&dir).await;
        assert_eq!((rs.pull.as_deref(), rs.prune, rs.auto_setup_remote), (Some("rebase"), true, true));
        assert!(status(&dir).await.unwrap().remotes.is_empty());
        remote(&dir, RemoteAction::Add { name: "origin".into(), url: "https://user:secret@example.com/x.git".into() }).await.unwrap();
        assert_eq!(status(&dir).await.unwrap().remotes[0].url, "https://example.com/x.git");
        remote(&dir, RemoteAction::SetPushUrl { name: "origin".into(), url: Some("git@example.com:x.git".into()) }).await.unwrap();
        remote(&dir, RemoteAction::Rename { from: "origin".into(), to: "upstream".into() }).await.unwrap();
        let r = &status(&dir).await.unwrap().remotes[0];
        assert_eq!((r.name.as_str(), r.push_url.as_deref()), ("upstream", Some("git@example.com:x.git")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn secrets_are_cut_from_messages() {
        assert_eq!(without_secrets("fatal: unable to access 'https://me:tok@github.com/a/b.git/': 403"), "fatal: unable to access 'https://github.com/a/b.git/': 403");
    }
}
