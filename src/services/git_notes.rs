//! Persist scraps as git notes on the task revision (`refs/notes/track`).

use crate::models::Scrap;
use crate::services::git_worktree;
use crate::utils::{Result, TrackError};
use std::process::Command;

pub const NOTES_REF: &str = "refs/notes/track";

pub fn format_notes(slug: &str, scraps: &[Scrap]) -> String {
    let mut body = format!("track-scraps v1\ntask: {slug}\n");
    for scrap in scraps {
        body.push('\n');
        body.push_str(&format!(
            "[{}] #{}\n{}\n",
            scrap.created_at.to_rfc3339(),
            scrap.scrap_id,
            scrap.content.trim()
        ));
    }
    body
}

fn git_notes_command(repo_or_workspace: &str) -> Option<std::process::Command> {
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
pub fn write_scraps(
    repo_or_workspace: &str,
    git_commit: &str,
    slug: &str,
    scraps: &[Scrap],
) -> Result<()> {
    let Some(mut cmd) = git_notes_command(repo_or_workspace) else {
        return Ok(());
    };

    let body = format_notes(slug, scraps);
    let output = cmd
        .args([
            "-c",
            "commit.gpgsign=false",
            "notes",
            "--ref",
            NOTES_REF,
            "add",
            "-f",
            "-m",
            &body,
            git_commit,
        ])
        .output()?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Scrap, ScrapId, ScrapIndex, TaskId};
    use chrono::Utc;

    #[test]
    fn notes_include_slug_and_content() {
        let scrap = Scrap {
            id: ScrapId::from_i64(1),
            task_id: TaskId::from_i64(1),
            scrap_id: ScrapIndex::from_i64(1),
            content: "decided on oauth".to_string(),
            created_at: Utc::now(),
            active_todo_id: None,
        };
        let text = format_notes("fix-auth", &[scrap]);
        assert!(text.contains("task: fix-auth"));
        assert!(text.contains("decided on oauth"));
    }
}
