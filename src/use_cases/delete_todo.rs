use crate::db::Database;
use crate::services::TodoService;
use crate::utils::Result;

/// Result of deleting a TODO.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteTodoOutcome {
    pub task_index: i64,
}

/// CLI-facing line after a successful delete.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteTodoCompletionView {
    pub summary: String,
}

impl DeleteTodoOutcome {
    pub fn completion_view(&self) -> DeleteTodoCompletionView {
        DeleteTodoCompletionView {
            summary: format!("Deleted TODO #{}", self.task_index),
        }
    }
}

/// Interactive delete flow step.
#[derive(Debug, Clone)]
pub enum DeleteTodoStep {
    Completed(DeleteTodoOutcome),
    NeedsConfirmation(DeleteTodoPrompt),
}

/// Confirmation required before deleting a TODO.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteTodoPrompt {
    pub task_index: i64,
    pub content: String,
}

/// CLI-facing prompt text for delete confirmation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteTodoPromptView {
    pub prompt: String,
}

impl DeleteTodoPrompt {
    pub fn view(&self) -> DeleteTodoPromptView {
        DeleteTodoPromptView {
            prompt: format!(
                "Delete TODO #{}: \"{}\"? [y/N]: ",
                self.task_index, self.content
            ),
        }
    }
}

/// Deletes a TODO, optionally requiring interactive confirmation.
pub struct DeleteTodoUseCase<'a> {
    db: &'a Database,
}

impl<'a> DeleteTodoUseCase<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Runs delete, returning either completion or a confirmation prompt.
    ///
    /// When `force` is true, the TODO is deleted immediately.
    pub fn run(&self, task_id: i64, todo_index: i64, force: bool) -> Result<DeleteTodoStep> {
        if force {
            return self
                .execute(task_id, todo_index)
                .map(DeleteTodoStep::Completed);
        }

        let todo = TodoService::new(self.db).get_todo_by_index(task_id, todo_index)?;
        Ok(DeleteTodoStep::NeedsConfirmation(DeleteTodoPrompt {
            task_index: todo.task_index,
            content: todo.content,
        }))
    }

    /// Deletes after the user confirmed a [`DeleteTodoPrompt`].
    pub fn confirm_and_run(&self, task_id: i64, todo_index: i64) -> Result<DeleteTodoOutcome> {
        self.execute(task_id, todo_index)
    }

    /// Deletes the TODO without prompting.
    pub fn execute(&self, task_id: i64, todo_index: i64) -> Result<DeleteTodoOutcome> {
        let todo_service = TodoService::new(self.db);
        let todo = todo_service.get_todo_by_index(task_id, todo_index)?;
        todo_service.delete_todo(todo.id)?;

        Ok(DeleteTodoOutcome {
            task_index: todo.task_index,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::TaskService;

    #[test]
    fn delete_prompt_view_formats_confirmation() {
        let prompt = DeleteTodoPrompt {
            task_index: 2,
            content: "Ship it".to_string(),
        };
        let view = prompt.view();
        assert_eq!(view.prompt, "Delete TODO #2: \"Ship it\"? [y/N]: ");
    }

    #[test]
    fn run_with_force_deletes_immediately() {
        let db = Database::new_in_memory().unwrap();
        let task = TaskService::new(&db)
            .create_task("Task", None, None, None)
            .unwrap();
        TodoService::new(&db)
            .add_todo(task.id, "Remove me", false)
            .unwrap();

        let step = DeleteTodoUseCase::new(&db).run(task.id, 1, true).unwrap();
        match step {
            DeleteTodoStep::Completed(outcome) => assert_eq!(outcome.task_index, 1),
            DeleteTodoStep::NeedsConfirmation(_) => panic!("expected immediate delete"),
        }

        assert!(TodoService::new(&db)
            .list_todos(task.id)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn run_without_force_returns_prompt() {
        let db = Database::new_in_memory().unwrap();
        let task = TaskService::new(&db)
            .create_task("Task", None, None, None)
            .unwrap();
        TodoService::new(&db)
            .add_todo(task.id, "Keep for now", false)
            .unwrap();

        let step = DeleteTodoUseCase::new(&db).run(task.id, 1, false).unwrap();
        match step {
            DeleteTodoStep::NeedsConfirmation(prompt) => {
                assert_eq!(prompt.task_index, 1);
                assert_eq!(prompt.content, "Keep for now");
            }
            DeleteTodoStep::Completed(_) => panic!("expected confirmation prompt"),
        }
    }

    #[test]
    fn confirm_and_run_deletes_todo() {
        let db = Database::new_in_memory().unwrap();
        let task = TaskService::new(&db)
            .create_task("Task", None, None, None)
            .unwrap();
        TodoService::new(&db)
            .add_todo(task.id, "Gone", false)
            .unwrap();

        let outcome = DeleteTodoUseCase::new(&db)
            .confirm_and_run(task.id, 1)
            .unwrap();
        assert_eq!(outcome.completion_view().summary, "Deleted TODO #1");
        assert!(TodoService::new(&db)
            .list_todos(task.id)
            .unwrap()
            .is_empty());
    }
}
