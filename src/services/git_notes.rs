//! Persist track notes (`refs/notes/track`): marker identity and per-TODO scraps.

use crate::services::git_worktree;
use crate::services::task_notes::{self, TaskNotesDto};
use crate::utils::{Result, TrackError};
use std::io::Write;
use std::process::{Command, Stdio};

pub const NOTES_REF: &str = "refs/notes/track";

fn git_notes_command(repo_or_workspace: &str) -> Option<Command> {
    if crate::services::git_worktree::is_git_repository(repo_or_workspace) {
        let mut cmd = Command::new("git");
        cmd.args(["-C", repo_or_workspace]);
        return Some(cmd);
    }
    let git_dir = git_worktree::git_dir(repo_or_workspace)?;
    let mut cmd = Command::new("git");
    cmd.arg("--git-dir").arg(git_dir);
    Some(cmd)
}

/// Rewrite git notes for the task revision. No-op when git is unavailable.
pub fn write_snapshot(
    repo_or_workspace: &str,
    git_commit: &str,
    snapshot: &TaskNotesDto,
) -> Result<()> {
    let body = task_notes::format_snapshot(snapshot)?;
    write_note_body(repo_or_workspace, git_commit, &body)
}

pub fn write_todo_notes(
    repo_or_workspace: &str,
    git_commit: &str,
    notes: &task_notes::TodoCommitNotesDto,
) -> Result<()> {
    let body = task_notes::format_todo_notes(notes)?;
    write_note_body(repo_or_workspace, git_commit, &body)
}

fn write_note_body(repo_or_workspace: &str, git_commit: &str, body: &str) -> Result<()> {
    let Some(mut cmd) = git_notes_command(repo_or_workspace) else {
        return Ok(());
    };

    let mut child = cmd
        .args([
            "-c",
            "commit.gpgsign=false",
            "notes",
            "--ref",
            NOTES_REF,
            "add",
            "-f",
            "-F",
            "-",
            git_commit,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(body.as_bytes())?;
    }

    let output = child.wait_with_output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(TrackError::Git(format!("git notes add failed: {stderr}")));
    }
    Ok(())
}

pub fn read_notes(repo_or_workspace: &str, git_commit: &str) -> Result<Option<String>> {
    let Some(mut cmd) = git_notes_command(repo_or_workspace) else {
        return Ok(None);
    };
    let output = cmd
        .args(["notes", "--ref", NOTES_REF, "show", git_commit])
        .output()?;
    if !output.status.success() {
        return Ok(None);
    }
    Ok(Some(
        String::from_utf8_lossy(&output.stdout).trim().to_string(),
    ))
}

pub fn read_snapshot(repo_or_workspace: &str, git_commit: &str) -> Result<Option<TaskNotesDto>> {
    match read_notes(repo_or_workspace, git_commit)? {
        Some(body) if !body.is_empty() => {
            if task_notes::parse_todo_notes(&body).is_ok() {
                return Ok(None);
            }
            Ok(Some(task_notes::parse_notes(&body)?))
        }
        _ => Ok(None),
    }
}

pub fn read_todo_notes(
    repo_or_workspace: &str,
    git_commit: &str,
) -> Result<Option<task_notes::TodoCommitNotesDto>> {
    match read_notes(repo_or_workspace, git_commit)? {
        Some(body) if !body.is_empty() => match task_notes::parse_todo_notes(&body) {
            Ok(dto) => Ok(Some(dto)),
            Err(_) => Ok(None),
        },
        _ => Ok(None),
    }
}

/// Commits that have a `refs/notes/track` note.
pub fn list_noted_commits(repo_or_workspace: &str) -> Result<Vec<String>> {
    let Some(mut cmd) = git_notes_command(repo_or_workspace) else {
        return Ok(Vec::new());
    };
    let output = cmd.args(["notes", "--ref", NOTES_REF, "list"]).output()?;
    if !output.status.success() {
        return Ok(Vec::new());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let _note = parts.next()?;
            parts.next().map(ToOwned::to_owned)
        })
        .collect())
}

pub fn is_ancestor(repo_or_workspace: &str, commit: &str, descendant: &str) -> bool {
    let Some(mut cmd) = git_notes_command(repo_or_workspace) else {
        return false;
    };
    cmd.args(["merge-base", "--is-ancestor", commit, descendant])
        .output()
        .is_ok_and(|o| o.status.success())
}

/// Snapshot attached to a noted commit that is an ancestor of `HEAD` (the marker).
pub fn find_snapshot_on_head(repo_or_workspace: &str) -> Result<Option<(String, TaskNotesDto)>> {
    let mut found = Vec::new();
    for commit in list_noted_commits(repo_or_workspace)? {
        if !is_ancestor(repo_or_workspace, &commit, "HEAD") {
            continue;
        }
        if let Some(snapshot) = read_snapshot(repo_or_workspace, &commit)? {
            found.push((commit, snapshot));
        }
    }
    if found.len() <= 1 {
        return Ok(found.into_iter().next());
    }
    // Prefer the oldest ancestor (the immutable marker).
    found.sort_by(|a, b| {
        let a_first = is_ancestor(repo_or_workspace, &a.0, &b.0);
        let b_first = is_ancestor(repo_or_workspace, &b.0, &a.0);
        b_first.cmp(&a_first)
    });
    Ok(found.into_iter().next())
}

pub fn fetch_notes(repo_or_workspace: &str, remote: &str) -> Result<()> {
    let Some(mut cmd) = git_notes_command(repo_or_workspace) else {
        return Err(TrackError::NotGitRepository(repo_or_workspace.to_string()));
    };
    let spec = format!("{NOTES_REF}:{NOTES_REF}");
    let output = cmd.args(["fetch", remote, &spec]).output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(TrackError::Git(format!(
            "git fetch {NOTES_REF} failed: {stderr}"
        )));
    }
    Ok(())
}

pub fn push_notes(repo_or_workspace: &str, remote: &str) -> Result<()> {
    let Some(mut cmd) = git_notes_command(repo_or_workspace) else {
        return Err(TrackError::NotGitRepository(repo_or_workspace.to_string()));
    };
    let output = cmd.args(["push", remote, NOTES_REF]).output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(TrackError::Git(format!(
            "git push {NOTES_REF} failed: {stderr}"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Scrap, ScrapId, ScrapIndex, ScrapVisibility, Task, TaskId, TaskStatus};
    use crate::services::task_notes::build_snapshot;
    use chrono::Utc;

    #[test]
    fn snapshot_roundtrip_text_contains_shared_scrap() {
        let task = Task {
            id: TaskId::from_i64(1),
            name: "Auth".to_string(),
            description: None,
            status: TaskStatus::Active,
            ticket_id: None,
            ticket_url: None,
            alias: None,
            is_today_task: false,
            created_at: Utc::now(),
        };
        let scrap = Scrap {
            id: ScrapId::from_i64(1),
            task_id: TaskId::from_i64(1),
            scrap_id: ScrapIndex::from_i64(1),
            content: "decided on oauth".to_string(),
            created_at: Utc::now(),
            active_todo_id: None,
            visibility: ScrapVisibility::Shared,
        };
        let dto = build_snapshot("fix-auth", &task, &[], &[], &[scrap]);
        let text = task_notes::format_snapshot(&dto).unwrap();
        assert!(text.contains("fix-auth"));
        assert!(text.contains("decided on oauth"));
        let parsed = task_notes::parse_notes(&text).unwrap();
        assert_eq!(parsed.name, "Auth");
    }
}
