//! Fold unpublished WIP into one git commit per completed TODO.

use crate::services::git_notes;
use crate::services::git_worktree;
use crate::services::task_notes::{parse_task_todo_index, todo_commit_message};
use crate::services::worktree_service::jj as jj_ws;
use crate::utils::{Result, TrackError};
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TodoCommit {
    pub sha: String,
    pub todo_index: i64,
}

pub fn remote_tracking_ref(branch: &str) -> String {
    format!("refs/remotes/origin/{branch}")
}

pub fn published_tip(workspace: &str, branch: &str) -> Option<String> {
    let spec = remote_tracking_ref(branch);
    let output = Command::new("git")
        .args(["-C", workspace, "rev-parse", "--verify", "--quiet", &spec])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if sha.is_empty() { None } else { Some(sha) }
}

pub fn is_published(workspace: &str, commit: &str, branch: &str) -> bool {
    published_tip(workspace, branch)
        .is_some_and(|tip| git_notes::is_ancestor(workspace, commit, &tip))
}

pub fn list_todo_commits(workspace: &str, marker: &str) -> Result<Vec<TodoCommit>> {
    if !git_worktree::is_git_repository(workspace) && git_worktree::git_dir(workspace).is_none() {
        return Ok(Vec::new());
    }
    let range = format!("{marker}..HEAD");
    let output = Command::new("git")
        .args([
            "-C",
            workspace,
            "log",
            "--reverse",
            "--format=%H%x1f%B%x1e",
            &range,
        ])
        .output()?;
    if !output.status.success() {
        return Ok(Vec::new());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut commits = Vec::new();
    for record in stdout.split('\u{1e}') {
        let record = record.trim();
        if record.is_empty() {
            continue;
        }
        let mut parts = record.splitn(2, '\u{1f}');
        let Some(sha) = parts.next() else {
            continue;
        };
        let body = parts.next().unwrap_or("");
        if let Some(todo_index) = parse_task_todo_index(body) {
            commits.push(TodoCommit {
                sha: sha.trim().to_string(),
                todo_index,
            });
        }
    }
    Ok(commits)
}

fn last_todo_commit(workspace: &str, marker: &str) -> Result<Option<TodoCommit>> {
    Ok(list_todo_commits(workspace, marker)?.pop())
}

fn fold_base<'a>(marker: &'a str, last_todo: Option<&'a TodoCommit>) -> &'a str {
    last_todo.map(|c| c.sha.as_str()).unwrap_or(marker)
}

/// Create (or fold unpublished WIP into) one commit for `todo_index`.
pub fn commit_todo_git(
    workspace: &str,
    branch: &str,
    marker: &str,
    todo_index: i64,
    content: &str,
    slug: &str,
) -> Result<String> {
    let head = git_worktree::current_commit(workspace)?;
    if let Some(published) = published_tip(workspace, branch)
        && !git_notes::is_ancestor(workspace, &published, &head)
        && published != head
    {
        return Err(TrackError::HistoryDiverged {
            branch: branch.to_string(),
        });
    }

    let last = last_todo_commit(workspace, marker)?;
    let base = fold_base(marker, last.as_ref()).to_string();

    if head != base {
        for sha in rev_list(workspace, &format!("{base}..HEAD"))? {
            if is_published(workspace, &sha, branch) {
                return Err(TrackError::CannotRewritePublishedHistory {
                    branch: branch.to_string(),
                });
            }
        }
        git_run(workspace, &["reset", "--soft", &base], "git reset --soft")?;
    }

    git_run(workspace, &["add", "-A"], "git add")?;
    let message = todo_commit_message(todo_index, content, slug);
    git_run(
        workspace,
        &["commit", "--allow-empty", "-m", &message],
        "git commit",
    )?;
    git_worktree::current_commit(workspace)
}

fn rev_list(workspace: &str, range: &str) -> Result<Vec<String>> {
    let output = Command::new("git")
        .args(["-C", workspace, "rev-list", range])
        .output()?;
    if !output.status.success() {
        return Ok(Vec::new());
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
        .collect())
}

fn git_run(workspace: &str, args: &[&str], label: &str) -> Result<()> {
    let output = Command::new("git")
        .args(["-C", workspace, "-c", "commit.gpgsign=false"])
        .args(args)
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(TrackError::Git(format!("{label} failed: {stderr}")));
    }
    Ok(())
}

/// JJ: describe the working copy as this TODO, bookmark the result, then `jj new`.
pub fn commit_todo_jj(
    workspace: &str,
    branch: &str,
    todo_index: i64,
    content: &str,
    slug: &str,
) -> Result<String> {
    let message = todo_commit_message(todo_index, content, slug);
    jj_ws::describe_current(workspace, &message)?;
    let sha = jj_ws::current_commit_id(workspace)?;
    jj_ws::set_bookmark(workspace, branch, "@")?;
    jj_ws::new_empty_change(workspace)?;
    jj_ws::set_bookmark(workspace, branch, "@-")?;
    Ok(sha)
}

pub fn commit_todo(
    vcs_git: bool,
    workspace: &str,
    branch: &str,
    marker: &str,
    todo_index: i64,
    content: &str,
    slug: &str,
) -> Result<String> {
    if vcs_git {
        commit_todo_git(workspace, branch, marker, todo_index, content, slug)
    } else {
        commit_todo_jj(workspace, branch, todo_index, content, slug)
    }
}
