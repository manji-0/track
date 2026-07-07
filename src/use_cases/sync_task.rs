use crate::db::Database;
use crate::models::{jj_slug, Task, TodoStatus, VcsMode};
use crate::services::{git_worktree, RepoService, TaskService, TodoService, WorktreeService};
use crate::utils::{Result, TrackError};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Per-repository result from a sync run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepoSyncOutcome {
    Missing,
    BookmarkCreated {
        base_ref: String,
        edit_ok: bool,
    },
    BookmarkExists {
        edit_ok: bool,
    },
    BookmarkCreateFailed {
        base_ref: String,
        detail: String,
    },
    WorktreeCreated {
        base_ref: String,
        workspace_path: String,
    },
    WorktreeExists {
        workspace_path: String,
    },
    WorktreeCreateFailed {
        base_ref: String,
        detail: String,
    },
}

/// A TODO workspace created during sync.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceCreated {
    pub todo_index: i64,
    pub todo_content: String,
    pub repo_path: String,
    pub workspace_path: String,
    pub branch: String,
}

/// A workspace creation failure during sync (non-fatal).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceCreateError {
    pub todo_index: i64,
    pub repo_path: String,
    pub detail: String,
}

/// Result of syncing the current task's VCS workspaces.
#[derive(Debug, Clone)]
pub struct SyncTaskOutcome {
    pub vcs_mode: VcsMode,
    pub task: Task,
    pub task_bookmark: String,
    pub repos: Vec<(String, RepoSyncOutcome)>,
    pub workspaces_created: Vec<WorkspaceCreated>,
    pub workspace_errors: Vec<WorkspaceCreateError>,
}

/// Syncs task bookmarks/worktrees across registered repos and creates pending TODO workspaces.
pub struct SyncTaskUseCase<'a> {
    db: &'a Database,
}

impl<'a> SyncTaskUseCase<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn execute(&self, task_id: i64, legacy: bool) -> Result<SyncTaskOutcome> {
        let vcs_mode = self.db.get_vcs_mode()?;
        let task_service = TaskService::new(self.db);
        let task = task_service.get_task(task_id)?;
        let repo_service = RepoService::new(self.db);
        let repos = repo_service.list_repos(task_id)?;

        if repos.is_empty() {
            return Err(TrackError::NoRepositoriesRegistered);
        }

        let slug = jj_slug(&task);
        let worktree_service = WorktreeService::new(self.db);
        let existing_worktrees = worktree_service.list_worktrees(task_id)?;

        if vcs_mode == VcsMode::Jj && !legacy {
            let todo_service = TodoService::new(self.db);
            let todos = todo_service.list_todos(task_id)?;
            if !crate::models::legacy_worktree_pending(&todos) {
                return Err(TrackError::SyncUseJjTask { slug });
            }
        }

        let task_bookmark = match vcs_mode {
            VcsMode::Jj => {
                let worktree_service = WorktreeService::new(self.db);
                worktree_service.task_bookmark_name(task.id, task.ticket_id.as_deref())
            }
            VcsMode::Git => git_worktree::git_branch_name(&slug),
        };

        let worktree_service = WorktreeService::new(self.db);

        let mut repo_outcomes = Vec::new();

        for repo in &repos {
            let outcome = match vcs_mode {
                VcsMode::Jj => {
                    self.sync_repo_jj(&worktree_service, repo, &task_bookmark, &existing_worktrees)?
                }
                VcsMode::Git => self.sync_repo_git(repo, &slug)?,
            };
            repo_outcomes.push((repo.repo_path.clone(), outcome));
        }

        let mut workspaces_created = Vec::new();
        let mut workspace_errors = Vec::new();

        if vcs_mode == VcsMode::Jj {
            let todo_service = TodoService::new(self.db);
            let todos = todo_service.list_todos(task_id)?;

            for todo in todos {
                if todo.worktree_requested && todo.status != TodoStatus::Done {
                    let worktrees = worktree_service.list_worktrees(task_id)?;
                    let exists = worktrees.iter().any(|wt| wt.todo_id == Some(todo.id));
                    if exists {
                        continue;
                    }

                    for repo in &repos {
                        match worktree_service.add_worktree(
                            task_id,
                            &repo.repo_path,
                            None,
                            task.ticket_id.as_deref(),
                            Some(todo.id),
                            false,
                        ) {
                            Ok(wt) => workspaces_created.push(WorkspaceCreated {
                                todo_index: todo.task_index,
                                todo_content: todo.content.clone(),
                                repo_path: repo.repo_path.clone(),
                                workspace_path: wt.path,
                                branch: wt.branch,
                            }),
                            Err(err) => workspace_errors.push(WorkspaceCreateError {
                                todo_index: todo.task_index,
                                repo_path: repo.repo_path.clone(),
                                detail: err.to_string(),
                            }),
                        }
                    }
                }
            }
        }

        Ok(SyncTaskOutcome {
            vcs_mode,
            task,
            task_bookmark,
            repos: repo_outcomes,
            workspaces_created,
            workspace_errors,
        })
    }

    fn sync_repo_jj(
        &self,
        worktree_service: &WorktreeService<'_>,
        repo: &crate::models::TaskRepo,
        task_bookmark: &str,
        existing_worktrees: &[crate::models::Worktree],
    ) -> Result<RepoSyncOutcome> {
        if !Path::new(&repo.repo_path).exists() {
            return Ok(RepoSyncOutcome::Missing);
        }

        let status_output = Command::new("jj")
            .current_dir(&repo.repo_path)
            .args(["-R", &repo.repo_path, "diff", "--summary"])
            .output()?;

        if !status_output.status.success() {
            return Err(TrackError::FailedRepoStatusCheck(repo.repo_path.clone()));
        }

        if Self::base_workspace_has_changes(
            &repo.repo_path,
            &status_output.stdout,
            existing_worktrees,
        )? {
            return Err(TrackError::RepoHasPendingChanges(repo.repo_path.clone()));
        }

        let bookmark_exists =
            worktree_service.bookmark_exists_in_repo(&repo.repo_path, task_bookmark)?;

        if !bookmark_exists {
            let base_ref = repo
                .base_branch
                .clone()
                .or_else(|| repo.base_commit_hash.clone())
                .unwrap_or_else(|| "@".to_string());

            let create_result = Command::new("jj")
                .args([
                    "-R",
                    &repo.repo_path,
                    "bookmark",
                    "create",
                    task_bookmark,
                    "-r",
                    &base_ref,
                ])
                .output()?;

            if create_result.status.success() {
                let edit_ok = try_edit_workspace(&repo.repo_path, task_bookmark);
                return Ok(RepoSyncOutcome::BookmarkCreated { base_ref, edit_ok });
            }

            let detail = String::from_utf8_lossy(&create_result.stderr)
                .trim()
                .to_string();
            return Ok(RepoSyncOutcome::BookmarkCreateFailed { base_ref, detail });
        }

        let edit_ok = try_edit_workspace(&repo.repo_path, task_bookmark);
        Ok(RepoSyncOutcome::BookmarkExists { edit_ok })
    }

    fn sync_repo_git(&self, repo: &crate::models::TaskRepo, slug: &str) -> Result<RepoSyncOutcome> {
        if !Path::new(&repo.repo_path).exists() {
            return Ok(RepoSyncOutcome::Missing);
        }

        if !git_worktree::is_git_repository(&repo.repo_path) {
            return Err(TrackError::NotGitRepository(repo.repo_path.clone()));
        }

        let worktree_path = git_worktree::git_worktree_path(&repo.repo_path, slug);
        if git_worktree::git_worktree_exists(&worktree_path) {
            return Ok(RepoSyncOutcome::WorktreeExists {
                workspace_path: worktree_path,
            });
        }

        if git_worktree::base_repo_has_changes(&repo.repo_path, slug)? {
            return Err(TrackError::RepoHasPendingChanges(repo.repo_path.clone()));
        }

        let base_ref = repo
            .base_branch
            .clone()
            .or_else(|| repo.base_commit_hash.clone())
            .unwrap_or_else(|| "HEAD".to_string());

        match git_worktree::create_git_worktree(&repo.repo_path, slug, &base_ref) {
            Ok(path) => Ok(RepoSyncOutcome::WorktreeCreated {
                base_ref,
                workspace_path: path,
            }),
            Err(err) => Ok(RepoSyncOutcome::WorktreeCreateFailed {
                base_ref,
                detail: err.to_string(),
            }),
        }
    }

    fn base_workspace_has_changes(
        repo_path: &str,
        status_stdout: &[u8],
        existing_worktrees: &[crate::models::Worktree],
    ) -> Result<bool> {
        let repo_root = Path::new(repo_path)
            .canonicalize()
            .map_err(|e| TrackError::PathResolutionFailed(e.to_string()))?;
        let repo_worktrees: Vec<PathBuf> = existing_worktrees
            .iter()
            .filter(|wt| wt.base_repo.as_deref() == Some(repo_path))
            .filter_map(|wt| Path::new(&wt.path).canonicalize().ok())
            .filter_map(|wt_path| wt_path.strip_prefix(&repo_root).ok().map(PathBuf::from))
            .collect();

        let status_stdout = String::from_utf8_lossy(status_stdout);
        for line in status_stdout.lines() {
            let path = line.split_whitespace().last().unwrap_or("").trim();
            if path.is_empty() {
                continue;
            }

            let path = Path::new(path);
            let resolved = Self::resolve_diff_path(&repo_root, path);
            let is_worktree = resolved
                .as_ref()
                .and_then(|resolved| resolved.strip_prefix(&repo_root).ok())
                .is_some_and(|relative| {
                    repo_worktrees
                        .iter()
                        .any(|worktree_path| relative.starts_with(worktree_path))
                });

            if !is_worktree {
                return Ok(true);
            }
        }

        Ok(false)
    }

    fn resolve_diff_path(repo_root: &Path, path: &Path) -> Option<PathBuf> {
        if path.is_absolute() {
            return path.canonicalize().ok();
        }

        repo_root
            .join(path)
            .canonicalize()
            .ok()
            .or_else(|| std::env::current_dir().ok()?.join(path).canonicalize().ok())
    }
}

fn try_edit_workspace(repo_path: &str, task_bookmark: &str) -> bool {
    Command::new("jj")
        .args(["-R", repo_path, "edit", task_bookmark])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::services::TodoService;

    #[test]
    fn sync_requires_registered_repos() {
        let db = Database::new_in_memory().unwrap();
        let task_service = TaskService::new(&db);
        let task = task_service
            .create_task("Sync task", None, None, None)
            .unwrap();

        let result = SyncTaskUseCase::new(&db).execute(task.id, false);
        assert!(matches!(result, Err(TrackError::NoRepositoriesRegistered)));
    }

    #[test]
    fn sync_rejects_jj_mode_without_legacy_or_worktree_todos() {
        let db = Database::new_in_memory().unwrap();
        let task_service = TaskService::new(&db);
        let task = task_service
            .create_task("Modern", None, Some("MOD-1"), None)
            .unwrap();
        let todo_service = TodoService::new(&db);
        todo_service.add_todo(task.id, "Implement", false).unwrap();

        db.get_connection()
            .execute(
                "INSERT INTO task_repos (task_id, task_index, repo_path, created_at) VALUES (?1, 1, '/repo', datetime('now'))",
                rusqlite::params![task.id],
            )
            .unwrap();

        let result = SyncTaskUseCase::new(&db).execute(task.id, false);
        assert!(matches!(result, Err(TrackError::SyncUseJjTask { .. })));
    }
}
