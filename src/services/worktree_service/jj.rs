use crate::utils::{Result, TrackError};
use std::path::Path;
use std::process::Command;

pub fn is_jj_repository(path: &str) -> bool {
    Path::new(path).join(".jj").exists()
}

/// True when the working copy is a colocated git+jj repo (`.git` + `.jj`).
pub fn is_colocated(path: &str) -> bool {
    is_jj_repository(path) && crate::services::git_worktree::is_git_repository(path)
}

/// Ensure a git working copy has a colocated jj repo (`jj git init --colocate`).
pub fn ensure_colocated(repo_path: &str) -> Result<()> {
    if is_colocated(repo_path) {
        return Ok(());
    }

    if crate::services::git_worktree::is_git_repository(repo_path) && !is_jj_repository(repo_path) {
        let output = Command::new("jj")
            .current_dir(repo_path)
            .args(["git", "init", "--colocate", repo_path])
            .output()?;
        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(TrackError::Jj(format!(
                "failed to colocate jj in {repo_path}: {error}"
            )));
        }
        return Ok(());
    }

    if is_jj_repository(repo_path) {
        // Non-colocated jj still works for workspaces; git notes/PR use the git store.
        return Ok(());
    }

    Err(TrackError::NotVcsRepository(repo_path.to_string()))
}

pub fn current_change_id(repo_path: &str) -> Result<String> {
    jj_template(repo_path, "@", "change_id.short()")
}

pub fn current_commit_id(repo_path: &str) -> Result<String> {
    jj_template(repo_path, "@", "commit_id")
}

pub fn describe_current(repo_path: &str, message: &str) -> Result<()> {
    let output = Command::new("jj")
        .current_dir(repo_path)
        .args(["-R", repo_path, "describe", "-m", message])
        .output()?;
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(TrackError::Jj(error.to_string()));
    }
    Ok(())
}

pub fn new_empty_change(repo_path: &str) -> Result<()> {
    let output = Command::new("jj")
        .current_dir(repo_path)
        .args(["-R", repo_path, "new"])
        .output()?;
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(TrackError::Jj(error.to_string()));
    }
    Ok(())
}

pub fn new_change_with_message(repo_path: &str, message: &str) -> Result<()> {
    let output = Command::new("jj")
        .current_dir(repo_path)
        .args(["-R", repo_path, "new", "-m", message])
        .output()?;
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(TrackError::Jj(error.to_string()));
    }
    Ok(())
}

/// Point `bookmark` at `rev`. Retries with `--allow-backwards` when jj requires it.
pub fn set_bookmark(repo_path: &str, bookmark: &str, rev: &str) -> Result<()> {
    let output = Command::new("jj")
        .current_dir(repo_path)
        .args(["-R", repo_path, "bookmark", "set", bookmark, "-r", rev])
        .output()?;
    if output.status.success() {
        return Ok(());
    }

    let retry = Command::new("jj")
        .current_dir(repo_path)
        .args([
            "-R",
            repo_path,
            "bookmark",
            "set",
            bookmark,
            "-r",
            rev,
            "--allow-backwards",
        ])
        .output()?;
    if retry.status.success() {
        return Ok(());
    }

    let error = String::from_utf8_lossy(&retry.stderr);
    Err(TrackError::Jj(format!(
        "failed to set bookmark {bookmark} to {rev}: {error}"
    )))
}

fn jj_template(repo_path: &str, rev: &str, template: &str) -> Result<String> {
    let output = Command::new("jj")
        .current_dir(repo_path)
        .args([
            "-R",
            repo_path,
            "log",
            "-r",
            rev,
            "--no-graph",
            "-T",
            template,
        ])
        .output()?;
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(TrackError::Jj(error.to_string()));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn current_bookmark(repo_path: &str) -> Result<Option<String>> {
    let output = Command::new("jj")
        .current_dir(repo_path)
        .args([
            "-R",
            repo_path,
            "bookmark",
            "list",
            "-r",
            "@",
            "-T",
            "name ++ \"\\n\"",
        ])
        .output()?;
    if !output.status.success() {
        return Ok(None);
    }
    let name = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(ToString::to_string);
    Ok(name)
}

pub fn bookmark_change_id(repo_path: &str, bookmark: &str) -> Result<String> {
    jj_template(repo_path, bookmark, "commit_id")
}

pub fn bookmark_exists(repo_path: &str, bookmark: &str) -> Result<bool> {
    if !is_jj_repository(repo_path) {
        return Err(TrackError::NotJjRepository(repo_path.to_string()));
    }

    let output = Command::new("jj")
        .current_dir(repo_path)
        .args(["-R", repo_path, "bookmark", "list", bookmark])
        .output()?;

    if !output.status.success() {
        return Ok(false);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout
        .lines()
        .any(|line| line.trim_start().starts_with(&format!("{}:", bookmark))))
}

pub fn create_workspace(
    repo_path: &str,
    worktree_path: &str,
    bookmark: &str,
    base_revset: &str,
) -> Result<()> {
    let output = Command::new("jj")
        .current_dir(repo_path)
        .args([
            "-R",
            repo_path,
            "workspace",
            "add",
            worktree_path,
            "-r",
            base_revset,
        ])
        .output()?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(TrackError::Jj(error.to_string()));
    }

    let bookmark_output = Command::new("jj")
        .current_dir(worktree_path)
        .args([
            "-R",
            worktree_path,
            "bookmark",
            "create",
            bookmark,
            "-r",
            "@",
        ])
        .output()?;

    if !bookmark_output.status.success() {
        let error = String::from_utf8_lossy(&bookmark_output.stderr);
        return Err(TrackError::Jj(error.to_string()));
    }

    Ok(())
}

pub fn create_workspace_for_existing_bookmark(
    repo_path: &str,
    worktree_path: &str,
    bookmark: &str,
) -> Result<()> {
    let output = Command::new("jj")
        .current_dir(repo_path)
        .args(["-R", repo_path, "workspace", "add", worktree_path])
        .output()?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(TrackError::Jj(error.to_string()));
    }

    let edit_output = Command::new("jj")
        .current_dir(worktree_path)
        .args(["-R", worktree_path, "edit", bookmark])
        .output()?;

    if !edit_output.status.success() {
        let error = String::from_utf8_lossy(&edit_output.stderr);
        return Err(TrackError::Jj(error.to_string()));
    }

    Ok(())
}

pub fn remove_workspace(repo_path: &str, worktree_path: &str) -> Result<()> {
    let workspace_name = Path::new(worktree_path)
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| TrackError::PathResolutionFailed(worktree_path.to_string()))?;

    let output = Command::new("jj")
        .current_dir(repo_path)
        .args(["-R", repo_path, "workspace", "forget", workspace_name])
        .output()?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(TrackError::Jj(error.to_string()));
    }

    if Path::new(worktree_path).exists() {
        std::fs::remove_dir_all(worktree_path)?;
    }

    Ok(())
}

pub fn has_uncommitted_changes(path: &str) -> Result<bool> {
    let output = Command::new("jj")
        .current_dir(path)
        .args(["-R", path, "diff", "--summary"])
        .output()?;

    Ok(!output.stdout.is_empty())
}

fn update_stale_workspace(path: &str) -> Result<()> {
    let output = Command::new("jj")
        .current_dir(path)
        .args(["-R", path, "workspace", "update-stale"])
        .output()?;

    if output.status.success() {
        return Ok(());
    }

    let error = String::from_utf8_lossy(&output.stderr);
    Err(TrackError::Jj(error.to_string()))
}

pub fn integrate_todo_bookmark(
    target_path: &str,
    todo_bookmark: &str,
    task_bookmark: &str,
) -> Result<()> {
    update_stale_workspace(target_path)?;

    let rebase_output = Command::new("jj")
        .current_dir(target_path)
        .args([
            "-R",
            target_path,
            "rebase",
            "-r",
            todo_bookmark,
            "-d",
            task_bookmark,
        ])
        .output()?;

    if !rebase_output.status.success() {
        let error = String::from_utf8_lossy(&rebase_output.stderr);
        return Err(TrackError::Jj(format!("Rebase failed: {}", error)));
    }

    let move_output = Command::new("jj")
        .current_dir(target_path)
        .args([
            "-R",
            target_path,
            "bookmark",
            "move",
            task_bookmark,
            "-t",
            todo_bookmark,
        ])
        .output()?;

    if !move_output.status.success() {
        let error = String::from_utf8_lossy(&move_output.stderr);
        return Err(TrackError::Jj(format!("Bookmark move failed: {}", error)));
    }

    let edit_output = Command::new("jj")
        .current_dir(target_path)
        .args(["-R", target_path, "edit", task_bookmark])
        .output()?;

    if !edit_output.status.success() {
        let error = String::from_utf8_lossy(&edit_output.stderr);
        return Err(TrackError::Jj(format!(
            "Workspace update failed: {}",
            error
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::process::Command;

    fn jj_available() -> bool {
        Command::new("jj")
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    fn init_jj_repo(path: &str) {
        let output = Command::new("jj")
            .args(["git", "init", path])
            .output()
            .expect("failed to run jj git init");
        assert!(
            output.status.success(),
            "jj git init failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn describe_change(path: &str, message: &str) {
        let output = Command::new("jj")
            .args(["-R", path, "describe", "-m", message])
            .output()
            .expect("failed to run jj describe");
        assert!(
            output.status.success(),
            "jj describe failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn new_change(path: &str) {
        let output = Command::new("jj")
            .args(["-R", path, "new"])
            .output()
            .expect("failed to run jj new");
        assert!(
            output.status.success(),
            "jj new failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn has_uncommitted_changes_detects_dirty_workspace() {
        if !jj_available() {
            eprintln!("Skipping test: jj binary not available");
            return;
        }

        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().to_str().unwrap();
        init_jj_repo(path);

        assert!(!has_uncommitted_changes(path).unwrap());

        File::create(temp_dir.path().join("test.txt")).unwrap();
        assert!(has_uncommitted_changes(path).unwrap());

        describe_change(path, "init");
        new_change(path);
        assert!(!has_uncommitted_changes(path).unwrap());

        std::fs::write(temp_dir.path().join("test.txt"), "mod").unwrap();
        assert!(has_uncommitted_changes(path).unwrap());
    }
}
