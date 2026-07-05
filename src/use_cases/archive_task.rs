use crate::db::Database;
use crate::models::Task;
use crate::services::{TaskService, WorktreeService};
use crate::utils::{Result, TrackError};

/// A workspace with uncommitted JJ changes blocking archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirtyWorkspace {
    pub id: i64,
    pub path: String,
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

    pub fn find_dirty_workspaces(&self, task_id: i64) -> Result<Vec<DirtyWorkspace>> {
        let worktree_service = WorktreeService::new(self.db);
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
    /// When `allow_dirty` is false and dirty workspaces exist, returns
    /// [`TrackError::UncommittedWorkspaces`] so the caller can prompt the user.
    pub fn execute(&self, task_id: i64, allow_dirty: bool) -> Result<ArchiveTaskOutcome> {
        let task_service = TaskService::new(self.db);
        let worktree_service = WorktreeService::new(self.db);

        let task = task_service.get_task(task_id)?;
        let dirty = self.find_dirty_workspaces(task_id)?;

        if !dirty.is_empty() && !allow_dirty {
            return Err(TrackError::UncommittedWorkspaces(
                dirty
                    .into_iter()
                    .map(|ws| format!("#{} {}", ws.id, ws.path))
                    .collect(),
            ));
        }

        let worktrees = worktree_service.list_worktrees(task_id)?;
        let mut removed_workspaces = Vec::new();
        let mut workspace_errors = Vec::new();

        for worktree in worktrees {
            match worktree_service.remove_worktree(worktree.id, false) {
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
}
