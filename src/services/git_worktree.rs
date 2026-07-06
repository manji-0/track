use crate::utils::{Result, TrackError};
use std::path::Path;
use std::process::Command;

/// Git branch name for a task slug.
pub fn git_branch_name(slug: &str) -> String {
    format!("track/{slug}")
}

/// Expected git worktree path (`.worktrees/<slug>/`).
pub fn git_worktree_path(repo_path: &str, slug: &str) -> String {
    Path::new(repo_path)
        .join(".worktrees")
        .join(slug)
        .to_string_lossy()
        .into_owned()
}

pub fn is_git_repository(repo_path: &str) -> bool {
    Command::new("git")
        .args(["-C", repo_path, "rev-parse", "--git-dir"])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

pub fn git_worktree_exists(path: &str) -> bool {
    Path::new(path).is_dir()
}

pub fn branch_exists(repo_path: &str, branch: &str) -> Result<bool> {
    let output = Command::new("git")
        .args(["-C", repo_path, "show-ref", "--verify", "--quiet", branch])
        .output()?;
    Ok(output.status.success())
}

/// Create a git worktree and branch for a task slug.
pub fn create_git_worktree(repo_path: &str, slug: &str, base_ref: &str) -> Result<String> {
    if !is_git_repository(repo_path) {
        return Err(TrackError::Other(format!(
            "Not a git repository: {repo_path}"
        )));
    }

    let worktree_path = git_worktree_path(repo_path, slug);
    if git_worktree_exists(&worktree_path) {
        return Ok(worktree_path);
    }

    if let Some(parent) = Path::new(&worktree_path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    let branch = git_branch_name(slug);

    let fetch = Command::new("git")
        .args(["-C", repo_path, "fetch", "--all", "--prune"])
        .output()?;
    if !fetch.status.success() {
        let stderr = String::from_utf8_lossy(&fetch.stderr);
        return Err(TrackError::Other(format!("git fetch failed: {stderr}")));
    }

    let create = if branch_exists(repo_path, &format!("refs/heads/{branch}"))? {
        Command::new("git")
            .args(["-C", repo_path, "worktree", "add", &worktree_path, &branch])
            .output()?
    } else {
        Command::new("git")
            .args([
                "-C",
                repo_path,
                "worktree",
                "add",
                "-b",
                &branch,
                &worktree_path,
                base_ref,
            ])
            .output()?
    };

    if !create.status.success() {
        let stderr = String::from_utf8_lossy(&create.stderr);
        return Err(TrackError::Other(format!(
            "git worktree add failed: {stderr}"
        )));
    }

    Ok(worktree_path)
}

pub fn repo_has_uncommitted_changes(repo_path: &str) -> Result<bool> {
    let output = Command::new("git")
        .args(["-C", repo_path, "status", "--porcelain"])
        .output()?;
    if !output.status.success() {
        return Err(TrackError::FailedRepoStatusCheck(repo_path.to_string()));
    }
    Ok(!output.stdout.is_empty())
}

/// Returns true when the base repo has changes outside `.worktrees/<slug>/`.
pub fn base_repo_has_changes(repo_path: &str, slug: &str) -> Result<bool> {
    let output = Command::new("git")
        .args(["-C", repo_path, "status", "--porcelain"])
        .output()?;
    if !output.status.success() {
        return Err(TrackError::FailedRepoStatusCheck(repo_path.to_string()));
    }

    for line in String::from_utf8_lossy(&output.stdout).lines() {
        if line.len() < 4 {
            continue;
        }
        let path = line[3..].trim();
        if path.is_empty() {
            continue;
        }
        if !is_task_worktree_path(path, slug) {
            return Ok(true);
        }
    }

    Ok(false)
}

fn is_task_worktree_path(path: &str, slug: &str) -> bool {
    path == format!(".worktrees/{slug}") || path.starts_with(&format!(".worktrees/{slug}/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn git_branch_and_path() {
        assert_eq!(git_branch_name("proj-123"), "track/proj-123");
        assert_eq!(
            git_worktree_path("/repo/app", "proj-123"),
            "/repo/app/.worktrees/proj-123"
        );
    }
}
