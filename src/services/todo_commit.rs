//! Fold unpublished WIP into one git/jj commit per completed TODO.

use crate::services::git_notes;
use crate::services::git_worktree;
use crate::services::task_notes::{parse_task_todo_index, todo_commit_message};
use crate::services::worktree_service::jj as jj_ws;
use crate::utils::{Result, TrackError};
use std::path::Path;
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
    if let Some(mut cmd) = git_worktree::git_store_command(workspace) {
        let output = cmd
            .args(["rev-parse", "--verify", "--quiet", &spec])
            .output()
            .ok();
        if let Some(output) = output
            && output.status.success()
        {
            let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !sha.is_empty() {
                return Some(sha);
            }
        }
    }
    // `jj git push` updates `track/<slug>@origin` even when git remote-tracking
    // refs are missing from a workspace that has no local `.git`.
    // Do not use `{branch}@git`: that is the local colocated export, not origin.
    if std::path::Path::new(workspace).join(".jj").exists() {
        let remote = format!("{branch}@origin");
        if let Ok(sha) = jj_ws::commit_id(workspace, &remote)
            && !sha.is_empty()
        {
            return Some(sha);
        }
    }
    None
}

pub fn is_published(workspace: &str, commit: &str, branch: &str) -> bool {
    published_tip(workspace, branch)
        .is_some_and(|tip| git_notes::is_ancestor(workspace, commit, &tip))
}

pub fn list_todo_commits(workspace: &str, marker: &str) -> Result<Vec<TodoCommit>> {
    if !git_worktree::is_git_repository(workspace) && git_worktree::git_dir(workspace).is_none() {
        return Ok(Vec::new());
    }
    let tip = git_worktree::history_tip(workspace)?;
    let range = format!("{marker}..{tip}");
    let Some(mut cmd) = git_worktree::git_store_command(workspace) else {
        return Ok(Vec::new());
    };
    let output = cmd
        .args(["log", "--reverse", "--format=%H%x1f%B%x1e", &range])
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
    if let Some(trunk) = git_notes::trunk_commit(workspace)
        && tip != trunk
        && !git_notes::is_ancestor(workspace, &tip, &trunk)
    {
        commits.retain(|c| c.sha != trunk && !git_notes::is_ancestor(workspace, &c.sha, &trunk));
    }
    Ok(commits)
}

fn last_todo_commit(workspace: &str, marker: &str) -> Result<Option<TodoCommit>> {
    Ok(list_todo_commits(workspace, marker)?.pop())
}

fn fold_base<'a>(marker: &'a str, last_todo: Option<&'a TodoCommit>) -> &'a str {
    last_todo.map(|c| c.sha.as_str()).unwrap_or(marker)
}

/// Do not fold through a merge from trunk: `base..tip` then contains trunk commits
/// that jj will not abandon and that git should keep as merge parents.
fn fold_target(workspace: &str, last_base: &str, tip: &str) -> Result<String> {
    if tip == last_base {
        return Ok(last_base.to_string());
    }
    if range_includes_trunk(workspace, last_base, tip)?
        || jj_range_includes_trunk(workspace, last_base, tip)
    {
        return Ok(tip.to_string());
    }
    Ok(last_base.to_string())
}

fn range_includes_trunk(workspace: &str, base: &str, tip: &str) -> Result<bool> {
    let Some(trunk) = git_notes::trunk_commit(workspace) else {
        return Ok(false);
    };
    for sha in rev_list_store(workspace, &format!("{base}..{tip}"))? {
        if sha == trunk || git_notes::is_ancestor(workspace, &sha, &trunk) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn jj_range_includes_trunk(workspace: &str, base: &str, tip: &str) -> bool {
    if !Path::new(workspace).join(".jj").exists() {
        return false;
    }
    let revset = format!("trunk() & ::{tip} ~ ::{base}");
    let output = Command::new("jj")
        .args([
            "-R",
            workspace,
            "log",
            "--no-graph",
            "-r",
            &revset,
            "-T",
            "commit_id",
        ])
        .output();
    output
        .is_ok_and(|o| o.status.success() && !String::from_utf8_lossy(&o.stdout).trim().is_empty())
}

fn refuse_published_range(workspace: &str, branch: &str, base: &str, tip: &str) -> Result<()> {
    if let Some(published) = published_tip(workspace, branch)
        && !git_notes::is_ancestor(workspace, &published, tip)
        && published != tip
    {
        return Err(TrackError::HistoryDiverged {
            branch: branch.to_string(),
        });
    }
    if tip == base {
        return Ok(());
    }
    for sha in rev_list_store(workspace, &format!("{base}..{tip}"))? {
        if is_published(workspace, &sha, branch) {
            return Err(TrackError::CannotRewritePublishedHistory {
                branch: branch.to_string(),
            });
        }
    }
    Ok(())
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
    let last = last_todo_commit(workspace, marker)?;
    let last_base = fold_base(marker, last.as_ref()).to_string();
    let base = fold_target(workspace, &last_base, &head)?;
    refuse_published_range(workspace, branch, &base, &head)?;

    if head != base {
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

fn rev_list_store(workspace: &str, range: &str) -> Result<Vec<String>> {
    let Some(mut cmd) = git_worktree::git_store_command(workspace) else {
        return Ok(Vec::new());
    };
    let output = cmd.args(["rev-list", range]).output()?;
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

/// Fold unpublished changes after `base` into one described TODO, then `jj new`.
pub fn commit_todo_jj(
    workspace: &str,
    branch: &str,
    marker: &str,
    todo_index: i64,
    content: &str,
    slug: &str,
) -> Result<String> {
    let tip = git_worktree::history_tip(workspace)?;
    let last = last_todo_commit(workspace, marker)?;
    let last_base = fold_base(marker, last.as_ref()).to_string();
    let base = fold_target(workspace, &last_base, &tip)?;
    refuse_published_range(workspace, branch, &base, &tip)?;

    let message = todo_commit_message(todo_index, content, slug);
    if tip != base {
        jj_ws::new_change_on(workspace, &base, &message)?;
        jj_ws::restore_from(workspace, &tip)?;
        let range = format!("{base}..{tip}");
        jj_ws::abandon(workspace, &range)?;
    } else {
        jj_ws::new_change_on(workspace, &base, &message)?;
    }
    let sha = jj_ws::current_commit_id(workspace)?;
    jj_ws::set_bookmark(workspace, branch, "@")?;
    jj_ws::new_change_with_message(workspace, &wip_message(slug))?;
    jj_ws::set_bookmark(workspace, branch, "@-")?;
    Ok(sha)
}

fn wip_message(slug: &str) -> String {
    format!("[track:{slug}] wip")
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
    refuse_conflicted(workspace)?;
    if vcs_git {
        commit_todo_git(workspace, branch, marker, todo_index, content, slug)
    } else {
        commit_todo_jj(workspace, branch, marker, todo_index, content, slug)
    }
}

fn refuse_conflicted(workspace: &str) -> Result<()> {
    if workspace_conflicted(workspace) {
        return Err(TrackError::WorkspaceHasConflict {
            path: workspace.to_string(),
        });
    }
    Ok(())
}

fn workspace_conflicted(workspace: &str) -> bool {
    if Path::new(workspace).join(".jj").exists() {
        return jj_ws::working_copy_conflicted(workspace);
    }
    Command::new("git")
        .args(["-C", workspace, "rev-parse", "-q", "--verify", "MERGE_HEAD"])
        .output()
        .is_ok_and(|o| o.status.success())
}
