use crate::db::Database;
use crate::models::{jj_slug, Task, TaskStatus};
use crate::services::{TaskService, TodoService, WorktreeService};
use crate::utils::Result;

/// Per-task summary for legacy worktree migration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyWorktreeTaskReport {
    pub task_id: i64,
    pub task_name: String,
    pub jj_slug: String,
    pub flagged_todos: usize,
    pub track_worktrees: usize,
}

/// Result of scanning or applying legacy worktree migration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrateLegacyWorktreesOutcome {
    pub dry_run: bool,
    pub tasks: Vec<LegacyWorktreeTaskReport>,
    pub todos_cleared: usize,
}

/// Clears legacy `worktree_requested` flags so tasks use jj-task workspaces.
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

        for task in tasks {
            let todos = todo_service.list_todos(task.id)?;
            let flagged = todos.iter().filter(|todo| todo.worktree_requested).count();
            if flagged == 0 {
                continue;
            }

            let worktrees = worktree_service.list_worktrees(task.id)?;
            let slug = jj_slug(&task);
            reports.push(LegacyWorktreeTaskReport {
                task_id: task.id,
                task_name: task.name.clone(),
                jj_slug: slug,
                flagged_todos: flagged,
                track_worktrees: worktrees.len(),
            });

            if !dry_run {
                todos_cleared += todo_service.clear_legacy_worktree_flags(Some(task.id))?;
            }
        }

        if dry_run {
            todos_cleared = reports.iter().map(|report| report.flagged_todos).sum();
        }

        Ok(MigrateLegacyWorktreesOutcome {
            dry_run,
            tasks: reports,
            todos_cleared,
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
            .execute(Some(task.id), false)
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
            .execute(Some(task.id), true)
            .unwrap();

        assert_eq!(outcome.todos_cleared, 1);
        let todos = todo_service.list_todos(task.id).unwrap();
        assert!(todos[0].worktree_requested);
    }
}
