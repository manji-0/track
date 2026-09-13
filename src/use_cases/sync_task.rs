use crate::db::Database;
use crate::models::{AggressiveMode, Task, TodoStatus, VcsMode, jj_slug};
use crate::services::{
    RepoService, ScrapService, TaskRevisionService, TaskService, TodoService, WorktreeService,
    git_notes, git_worktree, task_workspace,
};
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
    pub todo_index: crate::models::TodoIndex,
    pub todo_content: String,
    pub repo_path: String,
    pub workspace_path: String,
    pub branch: String,
}

/// A workspace creation failure during sync (non-fatal).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceCreateError {
    pub todo_index: crate::models::TodoIndex,
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

    pub fn execute(&self, task_id: crate::models::TaskId, legacy: bool) -> Result<SyncTaskOutcome> {
        let vcs_mode = self.db.get_vcs_mode()?;
        let aggressive = self.db.get_aggressive_mode()?;
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

        let task_bookmark = task_workspace::branch_name(&slug);

        let mut repo_outcomes = Vec::new();

        for repo in &repos {
            let outcome = if vcs_mode == VcsMode::Jj && legacy {
                self.sync_repo_jj(&worktree_service, repo, &task_bookmark, &existing_worktrees)?
            } else {
                self.sync_repo_workspace(
                    &worktree_service,
                    &task,
                    repo,
                    &slug,
                    vcs_mode,
                    aggressive,
                )?
            };
            repo_outcomes.push((repo.repo_path.clone(), outcome));
        }

        let mut workspaces_created = Vec::new();
        let mut workspace_errors = Vec::new();

        if vcs_mode == VcsMode::Jj && legacy {
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
                        let result = worktree_service.add_worktree(
                            task_id,
                            &repo.repo_path,
                            None,
                            task.ticket_id.as_deref(),
                            Some(todo.id),
                            false,
                        );
                        match result {
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

    fn sync_repo_workspace(
        &self,
        worktree_service: &WorktreeService<'_>,
        task: &Task,
        repo: &crate::models::TaskRepo,
        slug: &str,
        vcs_mode: VcsMode,
        aggressive: AggressiveMode,
    ) -> Result<RepoSyncOutcome> {
        if !Path::new(&repo.repo_path).exists() {
            return Ok(RepoSyncOutcome::Missing);
        }

        let worktree_path = task_workspace::workspace_path(&repo.repo_path, slug);
        if task_workspace::workspace_exists(&worktree_path) {
            task_workspace::assert_backend(&worktree_path, vcs_mode)?;
            worktree_service.register_task_workspace(
                task.id,
                &repo.repo_path,
                &worktree_path,
                &task_workspace::branch_name(slug),
            )?;
            self.ensure_aggressive_marker(task, repo, slug, vcs_mode, aggressive, &worktree_path)?;
            return Ok(RepoSyncOutcome::WorktreeExists {
                workspace_path: worktree_path,
            });
        }

        match vcs_mode {
            VcsMode::Git => {
                if git_worktree::base_repo_has_changes(&repo.repo_path, slug)? {
                    return Err(TrackError::RepoHasPendingChanges(repo.repo_path.clone()));
                }
            }
            VcsMode::Jj => {
                let status_output = Command::new("jj")
                    .current_dir(&repo.repo_path)
                    .args(["-R", &repo.repo_path, "diff", "--summary"])
                    .output()?;
                if !status_output.status.success() {
                    return Err(TrackError::FailedRepoStatusCheck(repo.repo_path.clone()));
                }
                if Self::base_workspace_has_changes(&repo.repo_path, &status_output.stdout, &[])? {
                    return Err(TrackError::RepoHasPendingChanges(repo.repo_path.clone()));
                }
            }
        }

        let base_ref = repo
            .base_branch
            .clone()
            .or_else(|| repo.base_commit_hash.clone())
            .unwrap_or_else(|| task_workspace::default_base_ref(vcs_mode).to_string());

        match task_workspace::ensure_workspace(
            vcs_mode,
            &repo.repo_path,
            slug,
            &base_ref,
            aggressive,
            task,
        ) {
            Ok(ensured) => {
                worktree_service.register_task_workspace(
                    task.id,
                    &repo.repo_path,
                    &ensured.path,
                    &ensured.branch,
                )?;
                self.persist_aggressive_revision(task, repo, slug, &ensured)?;
                Ok(RepoSyncOutcome::WorktreeCreated {
                    base_ref,
                    workspace_path: ensured.path,
                })
            }
            Err(err) => Ok(RepoSyncOutcome::WorktreeCreateFailed {
                base_ref,
                detail: err.to_string(),
            }),
        }
    }

    fn persist_aggressive_revision(
        &self,
        task: &Task,
        repo: &crate::models::TaskRepo,
        slug: &str,
        ensured: &task_workspace::EnsureWorkspaceOutcome,
    ) -> Result<()> {
        let Some(git_commit) = ensured.git_commit.as_deref() else {
            return Ok(());
        };

        TaskRevisionService::new(self.db).insert_if_absent(
            task.id,
            &repo.repo_path,
            git_commit,
            ensured.jj_change_id.as_deref(),
        )?;
        self.write_notes_or_warn(task.id, slug, git_commit, &ensured.path, &repo.repo_path);
        Ok(())
    }

    fn ensure_aggressive_marker(
        &self,
        task: &Task,
        repo: &crate::models::TaskRepo,
        slug: &str,
        vcs_mode: VcsMode,
        aggressive: AggressiveMode,
        workspace_path: &str,
    ) -> Result<()> {
        if !aggressive.is_on() {
            return Ok(());
        }

        let revisions = TaskRevisionService::new(self.db);
        if revisions.get(task.id, &repo.repo_path)?.is_some() {
            return Ok(());
        }

        let (git_commit, jj_change_id) =
            task_workspace::create_marker(vcs_mode, workspace_path, slug, task)?;
        revisions.insert_if_absent(
            task.id,
            &repo.repo_path,
            &git_commit,
            jj_change_id.as_deref(),
        )?;
        self.write_notes_or_warn(task.id, slug, &git_commit, workspace_path, &repo.repo_path);
        Ok(())
    }

    fn write_notes_or_warn(
        &self,
        task_id: crate::models::TaskId,
        slug: &str,
        git_commit: &str,
        workspace_path: &str,
        repo_path: &str,
    ) {
        let Ok(scraps) = ScrapService::new(self.db).list_scraps(task_id) else {
            return;
        };
        let notes_root = if task_workspace::workspace_exists(workspace_path) {
            workspace_path
        } else {
            repo_path
        };
        if let Err(err) = git_notes::write_scraps(notes_root, git_commit, slug, &scraps) {
            eprintln!("warning: git notes not updated: {err}");
        }
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
    fn sync_skips_missing_repo_path() {
        let db = Database::new_in_memory().unwrap();
        db.set_vcs_mode(VcsMode::Git).unwrap();
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

        let outcome = SyncTaskUseCase::new(&db).execute(task.id, false).unwrap();
        assert!(matches!(outcome.repos[0].1, RepoSyncOutcome::Missing));
    }
}
