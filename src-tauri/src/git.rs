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
    Remove { name: String },
}

/// Runs git in `dir`; Ok(stdout) on success, else git's own message (stderr) as the error.
async fn git(dir: &Path, args: &[&str], timeout: Duration) -> Result<String> {
    let mut cmd = tokio::process::Command::new("git");
    // Paths as they are (not "\303\251" escapes), and no editor or pager ever.
    cmd.args(["-c", "core.quotepath=false", "-c", "color.ui=false", "-c", "protocol.ext.allow=never"])
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
        let (Some(name), Some(url)) = (parts.next(), parts.next()) else { continue };
        if !list.iter().any(|r| r.name == name) {
            list.push(Remote { name: name.to_string(), url: crate::facts::without_credentials(url) });
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
        // --no-index exits with 1 when the files differ, which is always here.
        let empty = if cfg!(windows) { "NUL" } else { "/dev/null" };
        let out = git(dir, &["diff", "--no-index", "--no-color", "--", empty, path], QUICK).await;
        return Ok(cut(match out {
            Ok(text) => text,
            Err(e) => e.to_string(),
        }));
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

pub async fn commit(dir: &Path, message: &str, amend: bool) -> Result<String> {
    let message = message.trim();
    if message.is_empty() && !amend {
        return Err(Error::msg("Write a commit message first."));
    }
    let who = identity(dir).await;
    if who.name.is_none() || who.email.is_none() {
        return Err(Error::msg("Git doesn't know who you are yet: set your name and email under Identity, then commit again."));
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

/// A commit's message and changes.
pub async fn show(dir: &Path, hash: &str) -> Result<String> {
    if hash.is_empty() || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(Error::msg("That isn't a commit."));
    }
    Ok(cut(git(dir, &["show", "--no-color", "--stat", "--patch", "--format=commit %H%nAuthor: %an <%ae>%nDate:   %ad%n%n%B", hash], QUICK).await?))
}

pub async fn sync(dir: &Path, action: Sync) -> Result<String> {
    let status = status(dir).await?;
    if status.remotes.is_empty() {
        return Err(Error::msg("This repository has no remote yet: add one under Remotes."));
    }
    let out = match action {
        Sync::Fetch => git(dir, &["fetch", "--all", "--prune"], NETWORK).await?,
        Sync::Pull => {
            if status.upstream.is_none() {
                return Err(Error::msg("This branch doesn't track a remote branch yet: push it first, or switch to one that does."));
            }
            git(dir, &["pull", "--no-edit", "--ff"], NETWORK).await?
        }
        Sync::Push => {
            let branch = status.branch.ok_or_else(|| Error::msg("HEAD is detached: switch to a branch before pushing."))?;
            if status.upstream.is_some() {
                git(dir, &["push"], NETWORK).await?
            } else {
                // The first push of a branch sets it up to track the same name on the first remote.
                let remote = status.remotes.iter().find(|r| r.name == "origin").unwrap_or(&status.remotes[0]).name.clone();
                git(dir, &["push", "--set-upstream", &remote, &branch], NETWORK).await?
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
pub async fn clone(www: &Path, url: &str, folder: &str) -> Result<PathBuf> {
    check_url(url)?;
    let target = www.join(folder);
    if target.exists() {
        return Err(Error::msg(format!("www/{folder} exists already: pick another folder name.")));
    }
    git(www, &["clone", "--", url.trim(), folder], Duration::from_secs(900)).await?;
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
        commit(&dir, "First", false).await.unwrap();

        std::fs::write(dir.join("a.txt"), "two\n").unwrap();
        assert!(diff(&dir, "a.txt", false, false).await.unwrap().contains("+two"));
        discard(&dir, &["a.txt".into()]).await.unwrap();
        assert_eq!(std::fs::read_to_string(dir.join("a.txt")).unwrap().trim(), "one");

        branch(&dir, BranchAction::Create { branch: "feature".into(), from: None, switch: true }).await.unwrap();
        assert!(branch(&dir, BranchAction::Create { branch: "-x".into(), from: None, switch: false }).await.is_err());
        assert!(branch(&dir, BranchAction::Create { branch: "a..b".into(), from: None, switch: false }).await.is_err());
        std::fs::write(dir.join("b.txt"), "b\n").unwrap();
        stage(&dir, &["b.txt".into()], true).await.unwrap();
        commit(&dir, "Second", false).await.unwrap();
        let list = branches(&dir).await.unwrap();
        assert!(list.iter().any(|b| b.name == "feature" && b.current));
        branch(&dir, BranchAction::Switch { branch: "main".into() }).await.unwrap();
        branch(&dir, BranchAction::Merge { branch: "feature".into() }).await.unwrap();
        branch(&dir, BranchAction::Delete { branch: "feature".into(), force: false }).await.unwrap();

        let history = log(&dir, 0, 10, None).await.unwrap();
        assert_eq!(history.iter().map(|c| c.subject.as_str()).collect::<Vec<_>>(), ["Second", "First"]);
        assert!(show(&dir, &history[0].hash).await.unwrap().contains("b.txt"));
        assert!(sync(&dir, Sync::Push).await.is_err());
        remote(&dir, RemoteAction::Add { name: "origin".into(), url: "https://user:secret@example.com/x.git".into() }).await.unwrap();
        assert_eq!(status(&dir).await.unwrap().remotes[0].url, "https://example.com/x.git");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn secrets_are_cut_from_messages() {
        assert_eq!(without_secrets("fatal: unable to access 'https://me:tok@github.com/a/b.git/': 403"), "fatal: unable to access 'https://github.com/a/b.git/': 403");
    }
}
