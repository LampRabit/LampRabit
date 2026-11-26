//! The Git tab (git.rs does the work).

use crate::*;

/// Git, for a project's folder (git.rs).
#[tauri::command]
pub(crate) async fn git_version(state: State<'_, AppState>) -> Result<Option<String>> {
    let (settings, layout) = state.snapshot().await;
    git::configure(&settings, &layout);
    Ok(git::version().await)
}

#[tauri::command]
pub(crate) async fn git_status(state: State<'_, AppState>, name: String) -> Result<git::Status> {
    let (settings, layout) = state.snapshot().await;
    git::configure(&settings, &layout);
    git::status(&project_dir(&layout, &name)?).await
}

#[tauri::command]
pub(crate) async fn git_diff(state: State<'_, AppState>, name: String, path: String, staged: bool, untracked: bool) -> Result<String> {
    let (settings, layout) = state.snapshot().await;
    git::configure(&settings, &layout);
    git::diff(&project_dir(&layout, &name)?, &path, staged, untracked).await
}

#[tauri::command]
pub(crate) async fn git_stage(state: State<'_, AppState>, name: String, paths: Vec<String>, stage: bool) -> Result<()> {
    let (settings, layout) = state.snapshot().await;
    git::configure(&settings, &layout);
    git::stage(&project_dir(&layout, &name)?, &paths, stage).await
}

#[tauri::command]
pub(crate) async fn git_discard(app: AppHandle, state: State<'_, AppState>, name: String, paths: Vec<String>) -> Result<()> {
    let (settings, layout) = state.snapshot().await;
    git::configure(&settings, &layout);
    git::discard(&project_dir(&layout, &name)?, &paths).await?;
    logs::system(&app, logs::Level::Info, format!("Discarded changes to {} in {name}", paths.join(", ")));
    Ok(())
}

#[tauri::command]
pub(crate) async fn git_commit(app: AppHandle, state: State<'_, AppState>, name: String, message: String, amend: bool, all: Option<bool>) -> Result<String> {
    let (settings, layout) = state.snapshot().await;
    git::configure(&settings, &layout);
    let out = git::commit(&project_dir(&layout, &name)?, &message, amend, all.unwrap_or(false)).await?;
    logs::system(&app, logs::Level::Info, format!("{} in {name}: {}", if amend { "Amended the last commit" } else { "Committed" }, message.lines().next().unwrap_or_default()));
    Ok(out)
}

#[tauri::command]
pub(crate) async fn git_branches(state: State<'_, AppState>, name: String) -> Result<Vec<git::Branch>> {
    let (settings, layout) = state.snapshot().await;
    git::configure(&settings, &layout);
    git::branches(&project_dir(&layout, &name)?).await
}

/// Branch actions; a switch or merge changes the files, so the site's facts refresh.
#[tauri::command]
pub(crate) async fn git_branch(app: AppHandle, state: State<'_, AppState>, name: String, action: git::BranchAction) -> Result<String> {
    let (settings, layout) = state.snapshot().await;
    git::configure(&settings, &layout);
    let what = format!("{action:?}");
    let out = git::branch(&project_dir(&layout, &name)?, action).await?;
    logs::system(&app, logs::Level::Info, format!("Git in {name}: {what}"));
    Ok(out)
}

#[tauri::command]
pub(crate) async fn git_abort(state: State<'_, AppState>, name: String) -> Result<()> {
    let (settings, layout) = state.snapshot().await;
    git::configure(&settings, &layout);
    git::abort(&project_dir(&layout, &name)?).await
}

#[tauri::command]
pub(crate) async fn git_continue(state: State<'_, AppState>, name: String) -> Result<String> {
    let (settings, layout) = state.snapshot().await;
    git::configure(&settings, &layout);
    git::continue_operation(&project_dir(&layout, &name)?).await
}

#[tauri::command]
pub(crate) async fn git_log(state: State<'_, AppState>, name: String, skip: u32, limit: u32, path: Option<String>) -> Result<Vec<git::Commit>> {
    let (settings, layout) = state.snapshot().await;
    git::configure(&settings, &layout);
    git::log(&project_dir(&layout, &name)?, skip, limit, path.as_deref()).await
}

#[tauri::command]
pub(crate) async fn git_commit_details(state: State<'_, AppState>, name: String, hash: String) -> Result<git::CommitDetails> {
    let (settings, layout) = state.snapshot().await;
    git::configure(&settings, &layout);
    git::details(&project_dir(&layout, &name)?, &hash).await
}

#[tauri::command]
pub(crate) async fn git_commit_file_diff(state: State<'_, AppState>, name: String, hash: String, path: String) -> Result<String> {
    let (settings, layout) = state.snapshot().await;
    git::configure(&settings, &layout);
    git::commit_file_diff(&project_dir(&layout, &name)?, &hash, &path).await
}

#[tauri::command]
pub(crate) async fn git_repo_settings(state: State<'_, AppState>, name: String) -> Result<git::RepoSettings> {
    let (settings, layout) = state.snapshot().await;
    git::configure(&settings, &layout);
    Ok(git::repo_settings(&project_dir(&layout, &name)?).await)
}

#[tauri::command]
pub(crate) async fn git_set_repo_settings(state: State<'_, AppState>, name: String, settings: git::RepoSettings) -> Result<()> {
    let (app_settings, layout) = state.snapshot().await;
    git::configure(&app_settings, &layout);
    git::set_repo_settings(&project_dir(&layout, &name)?, &settings).await
}

#[tauri::command]
pub(crate) async fn git_sync(app: AppHandle, state: State<'_, AppState>, name: String, action: git::Sync, op: Option<String>) -> Result<String> {
    let (settings, layout) = state.snapshot().await;
    git::configure(&settings, &layout);
    let op = op.unwrap_or_else(|| format!("{name}:{action:?}"));
    let out = git::sync(&app, &op, &project_dir(&layout, &name)?, action).await?;
    logs::system(&app, logs::Level::Info, format!("Git {action:?} in {name} finished"));
    Ok(out)
}

/// Pull from, or push to, a branch the current one doesn't track.
#[tauri::command]
pub(crate) async fn git_sync_branch(app: AppHandle, state: State<'_, AppState>, name: String, pull: bool, remote: String, branch: String, op: Option<String>) -> Result<String> {
    let (settings, layout) = state.snapshot().await;
    git::configure(&settings, &layout);
    let op = op.unwrap_or_else(|| format!("{name}:{}:{branch}", if pull { "pull" } else { "push" }));
    let out = git::sync_branch(&app, &op, &project_dir(&layout, &name)?, pull, &remote, &branch).await?;
    logs::system(&app, logs::Level::Info, format!("Git {} {remote}/{branch} in {name} finished", if pull { "pull from" } else { "push to" }));
    Ok(out)
}

#[tauri::command]
pub(crate) async fn git_remote_tags(state: State<'_, AppState>, name: String, remote: String) -> Result<Vec<String>> {
    let (settings, layout) = state.snapshot().await;
    git::configure(&settings, &layout);
    git::remote_tags(&project_dir(&layout, &name)?, &remote).await
}

#[tauri::command]
pub(crate) async fn git_remote(state: State<'_, AppState>, name: String, action: git::RemoteAction) -> Result<()> {
    let (settings, layout) = state.snapshot().await;
    git::configure(&settings, &layout);
    git::remote(&project_dir(&layout, &name)?, action).await
}

#[tauri::command]
pub(crate) async fn git_set_identity(state: State<'_, AppState>, name: String, user: String, email: String) -> Result<()> {
    let (settings, layout) = state.snapshot().await;
    git::configure(&settings, &layout);
    git::set_identity(&project_dir(&layout, &name)?, &user, &email).await
}

#[tauri::command]
pub(crate) async fn git_init(state: State<'_, AppState>, name: String) -> Result<()> {
    let (settings, layout) = state.snapshot().await;
    git::configure(&settings, &layout);
    git::init(&project_dir(&layout, &name)?).await
}

/// Clones a repository into www/; the folder becomes a project like any other.
#[tauri::command]
pub(crate) async fn git_clone(app: AppHandle, state: State<'_, AppState>, url: String, folder: Option<String>, op: Option<String>) -> Result<String> {
    let (settings, layout) = state.snapshot().await;
    git::configure(&settings, &layout);
    let folder = folder.map(|f| f.trim().to_string()).filter(|f| !f.is_empty()).unwrap_or_else(|| git::folder_for(&url));
    manage::check_project_name(&layout, &folder)?;
    logs::system(&app, logs::Level::Info, format!("Cloning into www/{folder}"));
    git::clone(&app, op.as_deref().unwrap_or("clone"), &layout.www(), &url, &folder).await?;
    logs::system(&app, logs::Level::Info, format!("Cloned www/{folder}"));
    Ok(folder)
}

#[tauri::command]
pub(crate) async fn git_stashes(state: State<'_, AppState>, name: String) -> Result<Vec<git::Stash>> {
    let (settings, layout) = state.snapshot().await;
    git::configure(&settings, &layout);
    git::stashes(&project_dir(&layout, &name)?).await
}

#[tauri::command]
pub(crate) async fn git_stash_files(state: State<'_, AppState>, name: String, index: u32) -> Result<Vec<git::StashFile>> {
    let (settings, layout) = state.snapshot().await;
    git::configure(&settings, &layout);
    git::stash_files(&project_dir(&layout, &name)?, index).await
}

#[tauri::command]
pub(crate) async fn git_stash_file_diff(state: State<'_, AppState>, name: String, index: u32, path: String, untracked: bool) -> Result<String> {
    let (settings, layout) = state.snapshot().await;
    git::configure(&settings, &layout);
    git::stash_file_diff(&project_dir(&layout, &name)?, index, &path, untracked).await
}

#[tauri::command]
pub(crate) async fn git_stash(state: State<'_, AppState>, name: String, action: git::StashAction) -> Result<String> {
    let (settings, layout) = state.snapshot().await;
    git::configure(&settings, &layout);
    git::stash(&project_dir(&layout, &name)?, action).await
}

#[tauri::command]
pub(crate) async fn git_stage_hunk(state: State<'_, AppState>, name: String, path: String, hunk: usize, unstage: bool) -> Result<()> {
    let (settings, layout) = state.snapshot().await;
    git::configure(&settings, &layout);
    git::stage_hunk(&project_dir(&layout, &name)?, &path, hunk, unstage).await
}

#[tauri::command]
pub(crate) async fn git_resolve(state: State<'_, AppState>, name: String, path: String, theirs: bool) -> Result<()> {
    let (settings, layout) = state.snapshot().await;
    git::configure(&settings, &layout);
    git::resolve(&project_dir(&layout, &name)?, &path, theirs).await
}

#[tauri::command]
pub(crate) async fn git_tags(state: State<'_, AppState>, name: String) -> Result<Vec<git::Tag>> {
    let (settings, layout) = state.snapshot().await;
    git::configure(&settings, &layout);
    git::tags(&project_dir(&layout, &name)?).await
}

#[tauri::command]
pub(crate) async fn git_tag(state: State<'_, AppState>, name: String, action: git::TagAction) -> Result<String> {
    let (settings, layout) = state.snapshot().await;
    git::configure(&settings, &layout);
    git::tag(&project_dir(&layout, &name)?, action).await
}

#[tauri::command]
pub(crate) async fn git_revert(app: AppHandle, state: State<'_, AppState>, name: String, hash: String) -> Result<String> {
    let (settings, layout) = state.snapshot().await;
    git::configure(&settings, &layout);
    let out = git::revert(&project_dir(&layout, &name)?, &hash).await?;
    logs::system(&app, logs::Level::Info, format!("Reverted {} in {name}", &hash[..hash.len().min(8)]));
    Ok(out)
}

#[tauri::command]
pub(crate) async fn git_restore_file(state: State<'_, AppState>, name: String, hash: String, path: String) -> Result<()> {
    let (settings, layout) = state.snapshot().await;
    git::configure(&settings, &layout);
    git::restore_file(&project_dir(&layout, &name)?, &hash, &path).await
}

/// Stops a clone, fetch, pull or push in progress.
#[tauri::command]
pub(crate) fn git_cancel(op: String) -> bool {
    git::cancel(&op)
}
