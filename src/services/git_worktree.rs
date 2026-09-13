use crate::utils::{Result, TrackError};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const GITIGNORE_ENTRY: &str = ".worktrees/";

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
        .is_ok_and(|output| output.status.success())
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
        return Err(TrackError::NotGitRepository(repo_path.to_string()));
    }

    let worktree_path = git_worktree_path(repo_path, slug);
    if git_worktree_exists(&worktree_path) {
        return Ok(worktree_path);
    }

    if let Some(parent) = Path::new(&worktree_path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    let branch = git_branch_name(slug);

    let _ = git(repo_path, &["fetch", "--all", "--prune"]);
    ensure_worktrees_ignored(repo_path)?;

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
        return Err(TrackError::Git(format!(
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

/// Returns true when the base repo has changes outside `.worktrees/`.
pub fn base_repo_has_changes(repo_path: &str, _slug: &str) -> Result<bool> {
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
        if is_worktrees_path(path) {
            continue;
        }
        return Ok(true);
    }

    Ok(false)
}

fn is_worktrees_path(path: &str) -> bool {
    path == ".worktrees" || path.starts_with(".worktrees/")
}

fn git(repo_path: &str, args: &[&str]) -> Result<Output> {
    let output = Command::new("git")
        .args(["-C", repo_path])
        .args(args)
        .output()?;
    Ok(output)
}

fn git_ok(repo_path: &str, args: &[&str]) -> Result<Output> {
    let output = git(repo_path, args)?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(TrackError::Git(format!(
            "git {} failed: {stderr}",
            args.join(" ")
        )));
    }
    Ok(output)
}

/// Remove a git worktree created by track. The branch is kept for open PRs.
pub fn remove_git_worktree(repo_path: &str, worktree_path: &str, force: bool) -> Result<()> {
    if !Path::new(worktree_path).exists() {
        return Ok(());
    }

    if is_git_repository(repo_path) {
        let mut args = vec!["worktree", "remove"];
        if force {
            args.push("--force");
        }
        args.push(worktree_path);
        let output = git(repo_path, &args)?;
        if output.status.success() {
            return Ok(());
        }
        if force {
            let forced = git(repo_path, &["worktree", "remove", "--force", worktree_path])?;
            if forced.status.success() {
                return Ok(());
            }
        }
    }

    if Path::new(worktree_path).exists() {
        std::fs::remove_dir_all(worktree_path)?;
    }
    Ok(())
}

/// Create an empty commit on the task branch and return its SHA.
pub fn create_empty_task_commit(
    worktree_path: &str,
    slug: &str,
    task_name: &str,
) -> Result<String> {
    git_ok(
        worktree_path,
        &[
            "-c",
            "user.email=track@localhost",
            "-c",
            "user.name=track",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--allow-empty",
            "-m",
            &format!("track: {task_name}\n\nTask-Slug: {slug}"),
        ],
    )?;
    current_commit(worktree_path)
}

pub fn current_commit(repo_path: &str) -> Result<String> {
    let output = git_ok(repo_path, &["rev-parse", "HEAD"])?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn current_branch(repo_path: &str) -> Result<Option<String>> {
    let output = git_ok(repo_path, &["branch", "--show-current"])?;
    let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if name.is_empty() {
        Ok(None)
    } else {
        Ok(Some(name))
    }
}

pub fn rev_parse(repo_path: &str, rev: &str) -> Result<String> {
    let output = git_ok(repo_path, &["rev-parse", rev])?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Directory git notes should run against (colocated `.git` or jj's git store).
pub fn git_dir(repo_path: &str) -> Option<PathBuf> {
    if is_git_repository(repo_path) {
        return Some(PathBuf::from(repo_path));
    }
    let jj_store = Path::new(repo_path).join(".jj/repo/store/git");
    if jj_store.exists() {
        Some(jj_store)
    } else {
        None
    }
}

/// Ignore `.worktrees/` locally so track does not dirty the project's `.gitignore`.
pub fn ensure_worktrees_ignored(repo_path: &str) -> Result<()> {
    if is_git_repository(repo_path)
        && let Ok(output) = git_ok(repo_path, &["rev-parse", "--git-path", "info/exclude"])
    {
        let reported = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let exclude = if Path::new(&reported).is_absolute() {
            PathBuf::from(reported)
        } else {
            Path::new(repo_path).join(reported)
        };
        return append_exclude_line(&exclude);
    }

    let jj_exclude = Path::new(repo_path).join(".jj/repo/store/git/info/exclude");
    if jj_exclude.exists() || Path::new(repo_path).join(".jj/repo/store/git").exists() {
        if let Some(parent) = jj_exclude.parent() {
            std::fs::create_dir_all(parent)?;
        }
        return append_exclude_line(&jj_exclude);
    }

    Ok(())
}

fn append_exclude_line(path: &Path) -> Result<()> {
    let contents = if path.exists() {
        std::fs::read_to_string(path)?
    } else {
        String::new()
    };
    if contents
        .lines()
        .any(|line| line.trim() == GITIGNORE_ENTRY || line.trim() == ".worktrees")
    {
        return Ok(());
    }
    let mut updated = contents;
    if !updated.ends_with('\n') && !updated.is_empty() {
        updated.push('\n');
    }
    updated.push_str(GITIGNORE_ENTRY);
    updated.push('\n');
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, updated)?;
    Ok(())
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
