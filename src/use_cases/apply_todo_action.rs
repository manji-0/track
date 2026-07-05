use crate::db::Database;
use crate::models::{TodoAction, TodoStatus};
use crate::services::TodoService;
use crate::use_cases::CompleteTodoOutcome;
use crate::use_cases::CompleteTodoUseCase;
use crate::utils::Result;

/// Applies intent-based TODO operations through the correct service/use-case path.
pub struct ApplyTodoActionUseCase<'a> {
    db: &'a Database,
}

impl<'a> ApplyTodoActionUseCase<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn execute(
        &self,
        task_id: i64,
        todo_index: i64,
        action: TodoAction,
    ) -> Result<Option<CompleteTodoOutcome>> {
        let todo_service = TodoService::new(self.db);

        match action {
            TodoAction::Complete => {
                let outcome = CompleteTodoUseCase::new(self.db).execute(task_id, todo_index)?;
                Ok(Some(outcome))
            }
            TodoAction::Cancel => {
                let todo = todo_service.get_todo_by_index(task_id, todo_index)?;
                todo_service.update_status(todo.id, TodoStatus::Cancelled.as_str())?;
                Ok(None)
            }
            TodoAction::MakeNext => {
                todo_service.move_to_next(task_id, todo_index)?;
                Ok(None)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::services::TaskService;

    #[test]
    fn cancel_action_updates_status() {
        let db = Database::new_in_memory().unwrap();
        let task = TaskService::new(&db)
            .create_task("Task", None, None, None)
            .unwrap();
        let todo_service = TodoService::new(&db);
        let todo = todo_service.add_todo(task.id, "Cancel me", false).unwrap();

        ApplyTodoActionUseCase::new(&db)
            .execute(task.id, todo.task_index, TodoAction::Cancel)
            .unwrap();

        let updated = todo_service.get_todo(todo.id).unwrap();
        assert_eq!(updated.status, TodoStatus::Cancelled);
    }

    #[test]
    fn complete_via_update_status_string_is_rejected_at_parse() {
        use crate::utils::TrackError;
        let err = TodoAction::from_cli_update_status("done").unwrap_err();
        assert!(matches!(err, TrackError::TodoCompleteRequiresDoneCommand));
    }
}
