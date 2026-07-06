use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Deserialize)]
struct JjTaskMap {
    #[serde(default)]
    repos: HashMap<String, JjRepoEntry>,
}

#[derive(Debug, Deserialize)]
struct JjRepoEntry {
    #[serde(default)]
    tasks: HashMap<String, JjTaskEntry>,
}

#[derive(Debug, Deserialize)]
struct JjTaskEntry {
    workspace: String,
    #[serde(default)]
    phase: String,
}

fn map_path() -> PathBuf {
    std::env::var("JJ_TASK_MAP")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            directories::BaseDirs::new()
                .map(|dirs| dirs.home_dir().join(".config/jj/task-workspaces.json"))
                .unwrap_or_else(|| PathBuf::from(".config/jj/task-workspaces.json"))
        })
}

fn load_map() -> Option<JjTaskMap> {
    let path = map_path();
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Normalizes a git remote URL to owner/repo (same logic as jj-task.sh).
pub fn repo_key(repo_path: &str) -> String {
    let output = Command::new("git")
        .args(["-C", repo_path, "remote", "get-url", "origin"])
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if let Some(key) = normalize_remote_url(&url) {
                return key;
            }
        }
    }

    std::fs::canonicalize(repo_path)
        .unwrap_or_else(|_| PathBuf::from(repo_path))
        .to_string_lossy()
        .into_owned()
}

fn normalize_remote_url(url: &str) -> Option<String> {
    let stripped = url
        .trim()
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("git@"))
        .or_else(|| url.strip_prefix("ssh://"))
        .unwrap_or(url);

    let stripped = stripped
        .strip_prefix("github.com:")
        .or_else(|| stripped.strip_prefix("github.com/"))
        .unwrap_or(stripped);

    let stripped = stripped.strip_suffix(".git").unwrap_or(stripped);
    let key = stripped.to_ascii_lowercase();
    if key.contains('/') && key != url {
        Some(key)
    } else {
        None
    }
}

/// Returns the jj-task workspace path for a slug in a repo, if registered.
pub fn workspace_path(repo_path: &str, slug: &str) -> Option<String> {
    let map = load_map()?;
    let key = repo_key(repo_path);
    map.repos
        .get(&key)
        .and_then(|repo| repo.tasks.get(slug))
        .map(|task| task.workspace.clone())
}

/// Returns true when jj-task has registered the slug for any of the given repo paths.
pub fn slug_registered(slug: &str, repo_paths: &[String]) -> bool {
    let Some(map) = load_map() else {
        return false;
    };

    repo_paths.iter().any(|repo_path| {
        let key = repo_key(repo_path);
        map.repos
            .get(&key)
            .and_then(|repo| repo.tasks.get(slug))
            .is_some()
    })
}

/// Returns jj-task phase for a slug in the first matching repo, if any.
pub fn task_phase(slug: &str, repo_paths: &[String]) -> Option<String> {
    let map = load_map()?;
    for repo_path in repo_paths {
        let key = repo_key(repo_path);
        if let Some(entry) = map.repos.get(&key).and_then(|repo| repo.tasks.get(slug)) {
            return Some(entry.phase.clone());
        }
    }
    None
}

/// Default workspace path following agent-skill-jj layout (before jj-task start).
pub fn expected_workspace_path(repo_path: &str, slug: &str) -> String {
    Path::new(repo_path)
        .join(".worktrees")
        .join(slug)
        .to_string_lossy()
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_github_https_url() {
        assert_eq!(
            normalize_remote_url("https://github.com/manji-0/track.git"),
            Some("manji-0/track".to_string())
        );
    }

    #[test]
    fn normalize_github_ssh_url() {
        assert_eq!(
            normalize_remote_url("git@github.com:manji-0/track.git"),
            Some("manji-0/track".to_string())
        );
    }

    #[test]
    fn expected_workspace_path_uses_worktrees_dir() {
        assert_eq!(
            expected_workspace_path("/repo/app", "fix-auth"),
            "/repo/app/.worktrees/fix-auth"
        );
    }
}
