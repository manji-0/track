use crate::db::Database;
use crate::models::TodoStatus;
use crate::services::{TodoService, WorktreeService};
use crate::utils::{Result, TrackError};

/// Result of completing a TODO, including optional workspace bookmark name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompleteTodoOutcome {
    pub task_index: i64,
    pub merged_bookmark: Option<String>,
}

/// Completes a TODO: merges/removes JJ workspaces, then marks the TODO done in SQLite.
///
/// JJ operations cannot participate in the database transaction. The workflow therefore
/// completes external side effects first, then persists the terminal state. If the DB
/// update fails after a successful merge, a typed error is returned so the caller can
/// recover manually.
pub struct CompleteTodoUseCase<'a> {
    db: &'a Database,
}

impl<'a> CompleteTodoUseCase<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Complete the TODO identified by task-scoped `task_index` on `task_id`.
    pub fn execute(&self, task_id: i64, task_index: i64) -> Result<CompleteTodoOutcome> {
        let todo_service = TodoService::new(self.db);
        let worktree_service = WorktreeService::new(self.db);

        let todo = todo_service.get_todo_by_index(task_id, task_index)?;

        if todo.status != TodoStatus::Pending {
            return Err(TrackError::InvalidStatusTransition {
                from: todo.status.as_str().to_string(),
                to: TodoStatus::Done.as_str().to_string(),
            });
        }

        let merged_bookmark = worktree_service.complete_worktree_for_todo(todo.id)?;

        if let Err(err) = todo_service.mark_done(todo.id) {
            if let Some(bookmark) = merged_bookmark.clone() {
                return Err(TrackError::TodoCompletionDbFailed {
                    todo_index: task_index,
                    bookmark,
                    detail: err.to_string(),
                });
            }
            return Err(err);
        }

        Ok(CompleteTodoOutcome {
            task_index,
            merged_bookmark,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::services::TaskService;

    fn setup_db() -> Database {
        Database::new_in_memory().unwrap()
    }

    #[test]
    fn complete_todo_rejects_already_done() {
        let db = setup_db();
        let task_id = TaskService::new(&db)
            .create_task("Task", None, None, None)
            .unwrap()
            .id;
        let todo_service = TodoService::new(&db);
        let todo = todo_service.add_todo(task_id, "Item", false).unwrap();
        todo_service.mark_done(todo.id).unwrap();

        let result = CompleteTodoUseCase::new(&db).execute(task_id, todo.task_index);
        assert!(matches!(
            result,
            Err(TrackError::InvalidStatusTransition { .. })
        ));
    }
}
