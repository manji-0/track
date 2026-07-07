use crate::db::Database;
use crate::models::{jj_slug, Task, VcsMode};
use crate::services::{jj_task, RepoService, TaskService, WorktreeService};
use crate::utils::{Result, TrackError};

/// A workspace with uncommitted JJ changes blocking archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirtyWorkspace {
    pub id: i64,
    pub path: String,
}

/// Blockers discovered before archive.
#[derive(Debug, Clone, Default)]
pub struct ArchiveBlockers {
    pub dirty_workspaces: Vec<DirtyWorkspace>,
    pub jj_task_slug: Option<String>,
    pub jj_task_workspaces: Vec<String>,
}

/// Result of archiving a task and cleaning up workspaces.
#[derive(Debug, Clone)]
pub struct ArchiveTaskOutcome {
    pub task: Task,
    pub removed_workspaces: Vec<(i64, String)>,
    pub workspace_errors: Vec<String>,
}

/// Archives a task after optionally removing JJ workspaces.
pub struct ArchiveTaskUseCase<'a> {
    db: &'a Database,
}

impl<'a> ArchiveTaskUseCase<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn resolve_task_id(&self, task_ref: Option<&str>) -> Result<i64> {
        let task_service = TaskService::new(self.db);
        match task_ref {
            Some(r) => task_service.resolve_task_id(r),
            None => self
                .db
                .get_current_task_id()?
                .ok_or(TrackError::NoActiveTask),
        }
    }

    pub fn find_archive_blockers(&self, task_id: i64) -> Result<ArchiveBlockers> {
        let worktree_service = WorktreeService::new(self.db);
        let mut blockers = ArchiveBlockers {
            dirty_workspaces: self.find_dirty_track_workspaces(task_id, &worktree_service)?,
            ..Default::default()
        };

        if self.db.get_vcs_mode()? != VcsMode::Jj {
            return Ok(blockers);
        }

        let task_service = TaskService::new(self.db);
        let task = task_service.get_task(task_id)?;
        let repo_service = RepoService::new(self.db);
        let repos = repo_service.list_repos(task_id)?;
        if repos.is_empty() {
            return Ok(blockers);
        }

        let slug = jj_slug(&task);
        let repo_paths: Vec<String> = repos.iter().map(|repo| repo.repo_path.clone()).collect();
        let active = jj_task::active_registrations(&slug, &repo_paths);
        if !active.is_empty() {
            blockers.jj_task_slug = Some(slug.clone());
            blockers.jj_task_workspaces = active
                .iter()
                .filter_map(|status| status.workspace_path.clone())
                .collect();
        }

        for path in jj_task::active_workspace_paths(&slug, &repo_paths) {
            if std::path::Path::new(&path).exists()
                && worktree_service.has_uncommitted_changes(&path)?
                && !blockers.dirty_workspaces.iter().any(|ws| ws.path == path)
            {
                blockers
                    .dirty_workspaces
                    .push(DirtyWorkspace { id: 0, path });
            }
        }

        Ok(blockers)
    }

    fn find_dirty_track_workspaces(
        &self,
        task_id: i64,
        worktree_service: &WorktreeService<'_>,
    ) -> Result<Vec<DirtyWorkspace>> {
        let worktrees = worktree_service.list_worktrees(task_id)?;

        Ok(worktrees
            .into_iter()
            .filter(|worktree| {
                std::path::Path::new(&worktree.path).exists()
                    && worktree_service
                        .has_uncommitted_changes(&worktree.path)
                        .unwrap_or(false)
            })
            .map(|worktree| DirtyWorkspace {
                id: worktree.id,
                path: worktree.path,
            })
            .collect())
    }

    /// Removes workspaces and archives the task.
    ///
    /// When `force` is false, returns [`TrackError::UncommittedWorkspaces`] or
    /// [`TrackError::JjTaskNotCompleted`] so the caller can prompt the user.
    pub fn execute(&self, task_id: i64, force: bool) -> Result<ArchiveTaskOutcome> {
        let task_service = TaskService::new(self.db);
        let worktree_service = WorktreeService::new(self.db);

        let task = task_service.get_task(task_id)?;
        let blockers = self.find_archive_blockers(task_id)?;

        if !force {
            if let Some(slug) = &blockers.jj_task_slug {
                return Err(TrackError::JjTaskNotCompleted {
                    slug: slug.clone(),
                    workspaces: blockers.jj_task_workspaces.clone(),
                });
            }

            if !blockers.dirty_workspaces.is_empty() {
                return Err(TrackError::UncommittedWorkspaces(
                    blockers
                        .dirty_workspaces
                        .iter()
                        .map(|ws| {
                            if ws.id > 0 {
                                format!("#{} {}", ws.id, ws.path)
                            } else {
                                format!("jj-task {}", ws.path)
                            }
                        })
                        .collect(),
                ));
            }
        }

        let worktrees = worktree_service.list_worktrees(task_id)?;
        let mut removed_workspaces = Vec::new();
        let mut workspace_errors = Vec::new();

        for worktree in worktrees {
            match worktree_service.remove_worktree(worktree.id, force) {
                Ok(()) => removed_workspaces.push((worktree.id, worktree.path)),
                Err(err) => workspace_errors.push(format!("#{}: {}", worktree.id, err)),
            }
        }

        task_service.archive_task(task_id)?;

        Ok(ArchiveTaskOutcome {
            task,
            removed_workspaces,
            workspace_errors,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::TaskStatus;
    use crate::services::jj_task;
    use std::fs;

    fn insert_repo(db: &Database, task_id: i64, repo_path: &str) {
        db.get_connection()
            .execute(
                "INSERT INTO task_repos (task_id, task_index, repo_path, created_at) VALUES (?1, 1, ?2, datetime('now'))",
                rusqlite::params![task_id, repo_path],
            )
            .unwrap();
    }

    #[test]
    fn archive_task_marks_task_archived() {
        let db = Database::new_in_memory().unwrap();
        let task_service = TaskService::new(&db);
        let task = task_service
            .create_task("Archive me", None, None, None)
            .unwrap();

        let outcome = ArchiveTaskUseCase::new(&db).execute(task.id, true).unwrap();

        assert_eq!(outcome.task.id, task.id);
        let archived = task_service.get_task(task.id).unwrap();
        assert_eq!(archived.status, TaskStatus::Archived);
    }

    #[test]
    fn find_archive_blockers_detects_active_jj_task() {
        let temp = tempfile::tempdir().unwrap();
        let map_path = temp.path().join("task-workspaces.json");
        let repo_path = temp.path().join("repo");
        fs::create_dir_all(&repo_path).unwrap();
        let repo_key = jj_task::repo_key(repo_path.to_str().unwrap());
        fs::write(
            &map_path,
            format!(
                r#"{{
              "repos": {{
                {repo_key:?}: {{
                  "tasks": {{
                    "task-1": {{
                      "workspace": "/repo/.worktrees/task-1",
                      "phase": "active"
                    }}
                  }}
                }}
              }}
            }}"#
            ),
        )
        .unwrap();

        let prev = std::env::var("JJ_TASK_MAP").ok();
        unsafe { std::env::set_var("JJ_TASK_MAP", &map_path) };

        let db = Database::new_in_memory().unwrap();
        let task_service = TaskService::new(&db);
        let task = task_service.create_task("Task", None, None, None).unwrap();
        insert_repo(&db, task.id, repo_path.to_str().unwrap());

        let blockers = ArchiveTaskUseCase::new(&db)
            .find_archive_blockers(task.id)
            .unwrap();

        match prev {
            Some(value) => unsafe { std::env::set_var("JJ_TASK_MAP", value) },
            None => unsafe { std::env::remove_var("JJ_TASK_MAP") },
        }

        assert_eq!(blockers.jj_task_slug.as_deref(), Some("task-1"));
        assert!(!blockers.jj_task_workspaces.is_empty());
    }

    #[test]
    fn archive_allows_done_jj_task_phase() {
        let temp = tempfile::tempdir().unwrap();
        let map_path = temp.path().join("task-workspaces.json");
        let repo_path = temp.path().join("repo");
        fs::create_dir_all(&repo_path).unwrap();
        let repo_key = jj_task::repo_key(repo_path.to_str().unwrap());
        fs::write(
            &map_path,
            format!(
                r#"{{
              "repos": {{
                {repo_key:?}: {{
                  "tasks": {{
                    "task-1": {{
                      "workspace": "/repo/.worktrees/task-1",
                      "phase": "done"
                    }}
                  }}
                }}
              }}
            }}"#
            ),
        )
        .unwrap();

        let prev = std::env::var("JJ_TASK_MAP").ok();
        unsafe { std::env::set_var("JJ_TASK_MAP", &map_path) };

        let db = Database::new_in_memory().unwrap();
        let task_service = TaskService::new(&db);
        let task = task_service.create_task("Task", None, None, None).unwrap();
        insert_repo(&db, task.id, repo_path.to_str().unwrap());

        let result = ArchiveTaskUseCase::new(&db).execute(task.id, false);

        match prev {
            Some(value) => unsafe { std::env::set_var("JJ_TASK_MAP", value) },
            None => unsafe { std::env::remove_var("JJ_TASK_MAP") },
        }

        assert!(result.is_ok());
    }
}
