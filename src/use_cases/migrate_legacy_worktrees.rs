use crate::db::Database;
use crate::models::{jj_slug, Task, TaskStatus};
use crate::services::{LegacyWorktreeCleanupOutcome, TaskService, TodoService, WorktreeService};
use crate::utils::Result;

/// Per-task summary for legacy worktree migration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyWorktreeTaskReport {
    pub task_id: i64,
    pub task_name: String,
    pub jj_slug: String,
    pub flagged_todos: usize,
    pub legacy_worktrees: usize,
    pub worktrees_removed: usize,
    pub worktrees_skipped_dirty: Vec<String>,
    pub legacy_worktree_paths: Vec<String>,
}

/// Result of scanning or applying legacy worktree migration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrateLegacyWorktreesOutcome {
    pub dry_run: bool,
    pub tasks: Vec<LegacyWorktreeTaskReport>,
    pub todos_cleared: usize,
    pub worktrees_removed: usize,
}

/// Clears legacy `worktree_requested` flags and removes track-managed worktree records.
pub struct MigrateLegacyWorktreesUseCase<'a> {
    db: &'a Database,
}

impl<'a> MigrateLegacyWorktreesUseCase<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn execute(
        &self,
        task_id: Option<i64>,
        dry_run: bool,
        force: bool,
    ) -> Result<MigrateLegacyWorktreesOutcome> {
        let task_service = TaskService::new(self.db);
        let todo_service = TodoService::new(self.db);
        let worktree_service = WorktreeService::new(self.db);

        let tasks: Vec<Task> = match task_id {
            Some(id) => vec![task_service.get_task(id)?],
            None => task_service
                .list_tasks(true)?
                .into_iter()
                .filter(|task| task.status == TaskStatus::Active)
                .collect(),
        };

        let mut reports = Vec::new();
        let mut todos_cleared = 0;
        let mut worktrees_removed = 0;

        for task in tasks {
            let todos = todo_service.list_todos(task.id)?;
            let flagged = todos.iter().filter(|todo| todo.worktree_requested).count();
            let legacy_worktrees = worktree_service.list_legacy_worktrees(task.id)?;
            if flagged == 0 && legacy_worktrees.is_empty() {
                continue;
            }

            let slug = jj_slug(&task);
            let legacy_paths: Vec<String> = legacy_worktrees
                .iter()
                .map(|worktree| worktree.path.clone())
                .collect();

            let cleanup = if dry_run {
                LegacyWorktreeCleanupOutcome::default()
            } else {
                if flagged > 0 {
                    todos_cleared += todo_service.clear_legacy_worktree_flags(Some(task.id))?;
                }
                worktree_service.cleanup_legacy_worktrees(task.id, force)?
            };

            worktrees_removed += cleanup.removed.len();

            reports.push(LegacyWorktreeTaskReport {
                task_id: task.id,
                task_name: task.name.clone(),
                jj_slug: slug,
                flagged_todos: flagged,
                legacy_worktrees: legacy_paths.len(),
                worktrees_removed: if dry_run { 0 } else { cleanup.removed.len() },
                worktrees_skipped_dirty: cleanup.skipped_dirty,
                legacy_worktree_paths: legacy_paths,
            });
        }

        if dry_run {
            todos_cleared = reports.iter().map(|report| report.flagged_todos).sum();
            worktrees_removed = reports.iter().map(|report| report.legacy_worktrees).sum();
        }

        Ok(MigrateLegacyWorktreesOutcome {
            dry_run,
            tasks: reports,
            todos_cleared,
            worktrees_removed,
        })
    }

    pub fn resolve_task_id(&self, task_ref: Option<&str>) -> Result<Option<i64>> {
        let task_service = TaskService::new(self.db);
        match task_ref {
            Some(reference) => Ok(Some(task_service.resolve_task_id(reference)?)),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::TodoAddOptions;
    use crate::services::WorktreeService;
    use chrono::Utc;

    #[test]
    fn migrate_clears_legacy_flags() {
        let db = Database::new_in_memory().unwrap();
        let task_service = TaskService::new(&db);
        let todo_service = TodoService::new(&db);
        let task = task_service
            .create_task("Legacy", None, None, None)
            .unwrap();
        todo_service
            .add_todo(
                task.id,
                "Old style",
                TodoAddOptions::from_flags(true, false),
            )
            .unwrap();

        let outcome = MigrateLegacyWorktreesUseCase::new(&db)
            .execute(Some(task.id), false, false)
            .unwrap();

        assert_eq!(outcome.todos_cleared, 1);
        let todos = todo_service.list_todos(task.id).unwrap();
        assert!(!todos[0].worktree_requested);
        assert!(todos[0].requires_workspace);
    }

    #[test]
    fn migrate_dry_run_does_not_clear() {
        let db = Database::new_in_memory().unwrap();
        let task_service = TaskService::new(&db);
        let todo_service = TodoService::new(&db);
        let task = task_service
            .create_task("Legacy", None, None, None)
            .unwrap();
        todo_service
            .add_todo(
                task.id,
                "Old style",
                TodoAddOptions::from_flags(true, false),
            )
            .unwrap();

        let outcome = MigrateLegacyWorktreesUseCase::new(&db)
            .execute(Some(task.id), true, false)
            .unwrap();

        assert_eq!(outcome.todos_cleared, 1);
        let todos = todo_service.list_todos(task.id).unwrap();
        assert!(todos[0].worktree_requested);
    }

    #[test]
    fn migrate_removes_orphan_worktree_records() {
        let db = Database::new_in_memory().unwrap();
        let task_service = TaskService::new(&db);
        let todo_service = TodoService::new(&db);
        let worktree_service = WorktreeService::new(&db);

        let task = task_service
            .create_task("Orphan WT", None, None, None)
            .unwrap();
        let todo = todo_service
            .add_todo(task.id, "Done legacy", TodoAddOptions::default())
            .unwrap();
        todo_service.mark_done(todo.id).unwrap();

        let now = Utc::now().to_rfc3339();
        db.get_connection()
            .execute(
                "INSERT INTO worktrees (task_id, path, branch, base_repo, status, created_at, todo_id, is_base) VALUES (?1, ?2, ?3, ?4, 'active', ?5, ?6, 0)",
                rusqlite::params![task.id, "/gone/workspace", "task-1-todo-1", "/repo", now, todo.id],
            )
            .unwrap();

        let outcome = MigrateLegacyWorktreesUseCase::new(&db)
            .execute(Some(task.id), false, false)
            .unwrap();

        assert_eq!(outcome.worktrees_removed, 1);
        assert!(worktree_service.list_worktrees(task.id).unwrap().is_empty());
    }

    #[test]
    fn migrate_dry_run_reports_orphan_worktrees() {
        let db = Database::new_in_memory().unwrap();
        let task_service = TaskService::new(&db);
        let task = task_service
            .create_task("Orphan WT", None, None, None)
            .unwrap();

        let now = Utc::now().to_rfc3339();
        db.get_connection()
            .execute(
                "INSERT INTO worktrees (task_id, path, branch, base_repo, status, created_at, todo_id, is_base) VALUES (?1, '/gone/base', 'task/task-1', '/repo', 'active', ?2, NULL, 1)",
                rusqlite::params![task.id, now],
            )
            .unwrap();

        let outcome = MigrateLegacyWorktreesUseCase::new(&db)
            .execute(Some(task.id), true, false)
            .unwrap();

        assert_eq!(outcome.tasks.len(), 1);
        assert_eq!(outcome.worktrees_removed, 1);
        assert_eq!(outcome.tasks[0].legacy_worktree_paths, vec!["/gone/base"]);
    }
}
