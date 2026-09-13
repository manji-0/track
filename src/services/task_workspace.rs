//! Track-owned task workspaces: `.worktrees/<slug>` on `track/<slug>` (git branch or jj bookmark).

use crate::models::{AggressiveMode, Task, VcsMode, jj_slug};
use crate::services::git_worktree;
use crate::services::worktree_service::jj as jj_ws;
use crate::utils::{Result, TrackError};

/// GitHub-compatible branch/bookmark for a task slug.
pub fn branch_name(slug: &str) -> String {
    git_worktree::git_branch_name(slug)
}

/// Workspace path shared by git worktrees and jj workspaces.
pub fn workspace_path(repo_path: &str, slug: &str) -> String {
    git_worktree::git_worktree_path(repo_path, slug)
}

pub fn workspace_exists(path: &str) -> bool {
    git_worktree::git_worktree_exists(path)
}

pub struct EnsureWorkspaceOutcome {
    pub path: String,
    pub branch: String,
    pub created: bool,
    pub git_commit: Option<String>,
    pub jj_change_id: Option<String>,
}

/// Create or reuse the task workspace in one registered repo.
pub fn ensure_workspace(
    vcs_mode: VcsMode,
    repo_path: &str,
    slug: &str,
    base_ref: &str,
    aggressive: AggressiveMode,
    task: &Task,
) -> Result<EnsureWorkspaceOutcome> {
    match vcs_mode {
        VcsMode::Git => ensure_git(repo_path, slug, base_ref, aggressive, task),
        VcsMode::Jj => ensure_jj(repo_path, slug, base_ref, aggressive, task),
    }
}

fn ensure_git(
    repo_path: &str,
    slug: &str,
    base_ref: &str,
    aggressive: AggressiveMode,
    task: &Task,
) -> Result<EnsureWorkspaceOutcome> {
    if !git_worktree::is_git_repository(repo_path) {
        return Err(TrackError::NotGitRepository(repo_path.to_string()));
    }

    let path = workspace_path(repo_path, slug);
    let branch = branch_name(slug);
    let existed = workspace_exists(&path);
    git_worktree::create_git_worktree(repo_path, slug, base_ref)?;

    let mut git_commit = None;
    if aggressive.is_on() && !existed {
        git_commit = Some(create_git_marker(&path, slug, &task.name)?);
    }

    Ok(EnsureWorkspaceOutcome {
        path,
        branch,
        created: !existed,
        git_commit,
        jj_change_id: None,
    })
}

fn ensure_jj(
    repo_path: &str,
    slug: &str,
    base_ref: &str,
    aggressive: AggressiveMode,
    task: &Task,
) -> Result<EnsureWorkspaceOutcome> {
    jj_ws::ensure_colocated(repo_path)?;
    if !jj_ws::is_jj_repository(repo_path) {
        return Err(TrackError::NotJjRepository(repo_path.to_string()));
    }

    let path = workspace_path(repo_path, slug);
    let branch = branch_name(slug);
    let existed = workspace_exists(&path);

    if !existed {
        git_worktree::ensure_worktrees_ignored(repo_path)?;
        if let Some(parent) = std::path::Path::new(&path).parent() {
            std::fs::create_dir_all(parent)?;
        }
        jj_ws::create_workspace(repo_path, &path, &branch, base_ref)?;
    }

    let mut git_commit = None;
    let mut jj_change_id = None;

    if aggressive.is_on() && !existed {
        let marker = create_jj_birth_marker(&path, &branch, slug, &task.name)?;
        git_commit = Some(marker.0);
        jj_change_id = Some(marker.1);
    }

    Ok(EnsureWorkspaceOutcome {
        path,
        branch,
        created: !existed,
        git_commit,
        jj_change_id,
    })
}

/// Observed VCS backend of an existing workspace directory.
pub fn detect_backend(path: &str) -> Option<VcsMode> {
    if jj_ws::is_jj_repository(path) {
        Some(VcsMode::Jj)
    } else if git_worktree::is_git_repository(path) {
        Some(VcsMode::Git)
    } else {
        None
    }
}

pub fn assert_backend(path: &str, expected: VcsMode) -> Result<()> {
    match detect_backend(path) {
        Some(actual) if actual != expected => Err(TrackError::WorkspaceVcsMismatch {
            path: path.to_string(),
            expected: expected.to_string(),
            actual: actual.to_string(),
        }),
        _ => Ok(()),
    }
}

/// Create a marker revision in an existing workspace (aggressive backfill).
pub fn create_marker(
    vcs_mode: VcsMode,
    workspace_path: &str,
    slug: &str,
    task: &Task,
) -> Result<(String, Option<String>)> {
    match vcs_mode {
        VcsMode::Git => {
            let sha = create_git_marker(workspace_path, slug, &task.name)?;
            Ok((sha, None))
        }
        VcsMode::Jj => {
            let branch = branch_name(slug);
            let (sha, change) =
                create_jj_backfill_marker(workspace_path, &branch, slug, &task.name)?;
            Ok((sha, Some(change)))
        }
    }
}

fn marker_message(slug: &str, task_name: &str) -> String {
    format!("[track:{slug}] {task_name}")
}

fn create_git_marker(worktree_path: &str, slug: &str, task_name: &str) -> Result<String> {
    git_worktree::create_empty_task_commit(worktree_path, slug, task_name)
}

/// Fresh jj workspace: the empty working-copy change becomes the marker, then
/// `@` moves to a child so work does not rewrite the notes target. Bookmark
/// `track/<slug>` follows `@` (the GitHub PR head), not the marker.
fn create_jj_birth_marker(
    workspace_path: &str,
    branch: &str,
    slug: &str,
    task_name: &str,
) -> Result<(String, String)> {
    jj_ws::describe_current(workspace_path, &marker_message(slug, task_name))?;
    let git_commit = jj_ws::current_commit_id(workspace_path)?;
    let jj_change_id = jj_ws::current_change_id(workspace_path)?;
    jj_ws::new_empty_change(workspace_path)?;
    jj_ws::set_bookmark(workspace_path, branch, "@")?;
    Ok((git_commit, jj_change_id))
}

/// Existing jj workspace: insert an empty marker child without rewriting the
/// user's current description, then sit on a new working-copy change.
fn create_jj_backfill_marker(
    workspace_path: &str,
    branch: &str,
    slug: &str,
    task_name: &str,
) -> Result<(String, String)> {
    jj_ws::new_change_with_message(workspace_path, &marker_message(slug, task_name))?;
    let git_commit = jj_ws::current_commit_id(workspace_path)?;
    let jj_change_id = jj_ws::current_change_id(workspace_path)?;
    jj_ws::new_empty_change(workspace_path)?;
    jj_ws::set_bookmark(workspace_path, branch, "@")?;
    Ok((git_commit, jj_change_id))
}

pub fn remove_workspace(
    vcs_mode: VcsMode,
    repo_path: &str,
    worktree_path: &str,
    force: bool,
) -> Result<()> {
    match vcs_mode {
        VcsMode::Git => git_worktree::remove_git_worktree(repo_path, worktree_path, force),
        VcsMode::Jj => {
            if std::path::Path::new(worktree_path).exists() {
                jj_ws::remove_workspace(repo_path, worktree_path)
            } else {
                Ok(())
            }
        }
    }
}

pub fn has_uncommitted_changes(vcs_mode: VcsMode, path: &str) -> Result<bool> {
    if !std::path::Path::new(path).exists() {
        return Ok(false);
    }
    match vcs_mode {
        VcsMode::Git => {
            if git_worktree::is_git_repository(path) {
                git_worktree::repo_has_uncommitted_changes(path)
            } else {
                Ok(false)
            }
        }
        VcsMode::Jj => {
            if jj_ws::is_jj_repository(path) {
                jj_ws::has_uncommitted_changes(path)
            } else if git_worktree::is_git_repository(path) {
                git_worktree::repo_has_uncommitted_changes(path)
            } else {
                Ok(false)
            }
        }
    }
}

pub fn default_base_ref(vcs_mode: VcsMode) -> &'static str {
    match vcs_mode {
        VcsMode::Git => "HEAD",
        VcsMode::Jj => "@",
    }
}

pub fn resolve_base(
    vcs_mode: VcsMode,
    repo_path: &str,
    explicit: Option<&str>,
) -> Result<(Option<String>, Option<String>)> {
    match vcs_mode {
        VcsMode::Git => {
            if let Some(name) = explicit {
                let hash = git_worktree::rev_parse(repo_path, name)?;
                Ok((Some(name.to_string()), Some(hash)))
            } else {
                let branch = git_worktree::current_branch(repo_path)?;
                let hash = git_worktree::current_commit(repo_path).ok();
                Ok((branch, hash))
            }
        }
        VcsMode::Jj => {
            jj_ws::ensure_colocated(repo_path)?;
            if let Some(name) = explicit {
                let hash = jj_ws::bookmark_change_id(repo_path, name)?;
                Ok((Some(name.to_string()), Some(hash)))
            } else {
                let bookmark = jj_ws::current_bookmark(repo_path)?;
                let hash = if let Some(ref name) = bookmark {
                    jj_ws::bookmark_change_id(repo_path, name).ok()
                } else {
                    jj_ws::current_commit_id(repo_path).ok()
                };
                Ok((bookmark, hash))
            }
        }
    }
}

pub fn slug_for(task: &Task) -> String {
    jj_slug(task).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_layout() {
        assert_eq!(branch_name("proj-1"), "track/proj-1");
        assert_eq!(workspace_path("/repo", "proj-1"), "/repo/.worktrees/proj-1");
    }
}
