//! jj-task workspace map integration (reads `~/.config/jj/task-workspaces.json`).

mod map;

use map::load_map;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;

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

/// Per-repository jj-task workspace registration for a slug.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RepoWorkspaceStatus {
    pub repo_path: String,
    pub registered: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
}

/// Returns workspace registration status for each repo path.
pub fn repos_workspace_status(slug: &str, repo_paths: &[String]) -> Vec<RepoWorkspaceStatus> {
    let map = load_map();
    repo_paths
        .iter()
        .map(|repo_path| {
            let key = repo_key(repo_path);
            let entry = map
                .as_ref()
                .and_then(|m| m.repos.get(&key))
                .and_then(|repo| repo.tasks.get(slug));
            RepoWorkspaceStatus {
                repo_path: repo_path.clone(),
                registered: entry.is_some(),
                workspace_path: entry.map(|task| task.workspace.clone()),
                phase: entry
                    .map(|task| task.phase.clone())
                    .filter(|phase| !phase.is_empty()),
            }
        })
        .collect()
}

/// True when jj-task has registered the slug in every given repo.
pub fn all_repos_registered(slug: &str, repo_paths: &[String]) -> bool {
    if repo_paths.is_empty() {
        return false;
    }
    repos_workspace_status(slug, repo_paths)
        .iter()
        .all(|status| status.registered)
}

/// Repo paths that still need `jj-task start` for this slug.
pub fn unregistered_repo_paths(slug: &str, repo_paths: &[String]) -> Vec<String> {
    repos_workspace_status(slug, repo_paths)
        .into_iter()
        .filter(|status| !status.registered)
        .map(|status| status.repo_path)
        .collect()
}

/// True when `jj-task repo init` has been run for this repo (repo key exists in map).
pub fn repo_initialized(repo_path: &str) -> bool {
    let Some(map) = load_map() else {
        return false;
    };
    map.repos.contains_key(&repo_key(repo_path))
}

/// True when the jj-task map phase means the workspace is finished.
///
/// agent-skill-jj uses `merged` after `jj-task done`. `done` is accepted for
/// older maps / tests that used that label.
pub fn is_completed_phase(phase: Option<&str>) -> bool {
    matches!(phase, Some("merged") | Some("done"))
}

/// Returns registrations that are not marked completed in the jj-task map.
pub fn active_registrations(slug: &str, repo_paths: &[String]) -> Vec<RepoWorkspaceStatus> {
    repos_workspace_status(slug, repo_paths)
        .into_iter()
        .filter(|status| status.registered && !is_completed_phase(status.phase.as_deref()))
        .collect()
}

/// Workspace paths for active (not done) jj-task registrations.
pub fn active_workspace_paths(slug: &str, repo_paths: &[String]) -> Vec<String> {
    active_registrations(slug, repo_paths)
        .into_iter()
        .filter_map(|status| status.workspace_path)
        .collect()
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
    fn is_completed_phase_accepts_merged_and_done() {
        assert!(is_completed_phase(Some("merged")));
        assert!(is_completed_phase(Some("done")));
        assert!(!is_completed_phase(Some("draft")));
        assert!(!is_completed_phase(Some("in_review")));
        assert!(!is_completed_phase(None));
    }

    #[test]
    fn active_registrations_excludes_completed_phases() {
        let statuses = vec![
            RepoWorkspaceStatus {
                repo_path: "/a".into(),
                registered: true,
                workspace_path: Some("/a/.worktrees/slug".into()),
                phase: Some("merged".into()),
            },
            RepoWorkspaceStatus {
                repo_path: "/c".into(),
                registered: true,
                workspace_path: Some("/c/.worktrees/slug".into()),
                phase: Some("done".into()),
            },
            RepoWorkspaceStatus {
                repo_path: "/b".into(),
                registered: true,
                workspace_path: Some("/b/.worktrees/slug".into()),
                phase: Some("draft".into()),
            },
        ];

        let active: Vec<_> = statuses
            .into_iter()
            .filter(|s| s.registered && !is_completed_phase(s.phase.as_deref()))
            .collect();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].repo_path, "/b");
    }

    #[test]
    fn expected_workspace_path_uses_worktrees_dir() {
        assert_eq!(
            expected_workspace_path("/repo/app", "fix-auth"),
            "/repo/app/.worktrees/fix-auth"
        );
    }
}
