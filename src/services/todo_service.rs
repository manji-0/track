use crate::db::Database;
use crate::models::{Todo, TodoStatus};
use crate::utils::{Result, TrackError};
use chrono::Utc;
use rusqlite::params;
use std::str::FromStr;

pub struct TodoService<'a> {
    db: &'a Database,
}

impl<'a> TodoService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn add_todo(&self, task_id: i64, content: &str, worktree_requested: bool) -> Result<Todo> {
        let now = Utc::now().to_rfc3339();
        let content = content.to_string();
        let status = TodoStatus::Pending.as_str().to_string();

        // Use transaction to make SELECT MAX + INSERT atomic
        self.db.with_transaction(|| {
            let conn = self.db.get_connection();

            // Get next task_index for this task
            let next_index: i64 = conn.query_row(
                "SELECT COALESCE(MAX(task_index), 0) + 1 FROM todos WHERE task_id = ?1",
                params![task_id],
                |row| row.get(0),
            )?;

            conn.execute(
                "INSERT INTO todos (task_id, task_index, content, status, worktree_requested, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![task_id, next_index, content, status, worktree_requested, now],
            )?;

            let todo_id = conn.last_insert_rowid();
            self.db.increment_rev("todos")?;
            self.get_todo(todo_id)
        })
    }

    pub fn get_todo(&self, todo_id: i64) -> Result<Todo> {
        let conn = self.db.get_connection();
        let mut stmt = conn.prepare(
            "SELECT id, task_id, task_index, content, status, worktree_requested, created_at, completed_at FROM todos WHERE id = ?1"
        )?;

        let todo = stmt
            .query_row(params![todo_id], |row| {
                Ok(Todo {
                    id: row.get(0)?,
                    task_id: row.get(1)?,
                    task_index: row.get(2)?,
                    content: row.get(3)?,
                    status: row.get(4)?,
                    worktree_requested: row.get(5)?,
                    created_at: row.get::<_, String>(6)?.parse().unwrap(),
                    completed_at: row.get::<_, Option<String>>(7)?.map(|s| s.parse().unwrap()),
                })
            })
            .map_err(|_| TrackError::TodoNotFound(todo_id))?;

        Ok(todo)
    }

    pub fn list_todos(&self, task_id: i64) -> Result<Vec<Todo>> {
        let conn = self.db.get_connection();
        let mut stmt = conn.prepare(
            "SELECT id, task_id, task_index, content, status, worktree_requested, created_at, completed_at FROM todos WHERE task_id = ?1 ORDER BY task_index ASC"
        )?;

        let todos = stmt
            .query_map(params![task_id], |row| {
                Ok(Todo {
                    id: row.get(0)?,
                    task_id: row.get(1)?,
                    task_index: row.get(2)?,
                    content: row.get(3)?,
                    status: row.get(4)?,
                    worktree_requested: row.get(5)?,
                    created_at: row.get::<_, String>(6)?.parse().unwrap(),
                    completed_at: row.get::<_, Option<String>>(7)?.map(|s| s.parse().unwrap()),
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(todos)
    }

    pub fn get_todo_by_index(&self, task_id: i64, task_index: i64) -> Result<Todo> {
        let conn = self.db.get_connection();
        let mut stmt = conn.prepare(
            "SELECT id, task_id, task_index, content, status, worktree_requested, created_at, completed_at FROM todos WHERE task_id = ?1 AND task_index = ?2"
        )?;

        let todo = stmt
            .query_row(params![task_id, task_index], |row| {
                Ok(Todo {
                    id: row.get(0)?,
                    task_id: row.get(1)?,
                    task_index: row.get(2)?,
                    content: row.get(3)?,
                    status: row.get(4)?,
                    worktree_requested: row.get(5)?,
                    created_at: row.get::<_, String>(6)?.parse().unwrap(),
                    completed_at: row.get::<_, Option<String>>(7)?.map(|s| s.parse().unwrap()),
                })
            })
            .map_err(|_| {
                TrackError::Other(format!("TODO #{} not found in current task", task_index))
            })?;

        Ok(todo)
    }

    pub fn update_status(&self, todo_id: i64, status: &str) -> Result<()> {
        // Validate status
        TodoStatus::from_str(status).map_err(|_| TrackError::InvalidStatus(status.to_string()))?;

        let completed_at = if status == "done" {
            Some(Utc::now().to_rfc3339())
        } else {
            None
        };

        let conn = self.db.get_connection();
        let affected = conn.execute(
            "UPDATE todos SET status = ?1, completed_at = ?2 WHERE id = ?3",
            params![status, completed_at, todo_id],
        )?;

        if affected == 0 {
            return Err(TrackError::TodoNotFound(todo_id));
        }

        self.db.increment_rev("todos")?;
        Ok(())
    }

    pub fn delete_todo(&self, todo_id: i64) -> Result<()> {
        let conn = self.db.get_connection();
        let affected = conn.execute("DELETE FROM todos WHERE id = ?1", params![todo_id])?;

        if affected == 0 {
            return Err(TrackError::TodoNotFound(todo_id));
        }

        self.db.increment_rev("todos")?;
        Ok(())
    }

    /// Move a TODO to the front (make it the next todo to work on)
    ///
    /// This reorders the task_index so that the specified todo becomes the oldest pending todo.
    /// Only pending todos are affected by the reordering.
    pub fn move_to_next(&self, task_id: i64, task_index: i64) -> Result<()> {
        self.db.with_transaction(|| {
            let conn = self.db.get_connection();

            // Get the todo to move
            let todo = self.get_todo_by_index(task_id, task_index)?;

            // Only allow moving pending todos
            if todo.status != "pending" {
                return Err(TrackError::Other(format!(
                    "Cannot move TODO #{} - only pending todos can be moved (current status: {})",
                    task_index, todo.status
                )));
            }

            // Get all pending todos for this task, ordered by task_index
            let mut stmt = conn.prepare(
                "SELECT id, task_index FROM todos WHERE task_id = ?1 AND status = 'pending' ORDER BY task_index ASC"
            )?;

            let pending_todos: Vec<(i64, i64)> = stmt
                .query_map(params![task_id], |row| {
                    Ok((row.get(0)?, row.get(1)?))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;

            if pending_todos.is_empty() {
                return Err(TrackError::Other("No pending todos found".to_string()));
            }

            // Find the position of the todo to move
            let move_pos = pending_todos.iter().position(|(_, idx)| *idx == task_index);
            if move_pos.is_none() {
                return Err(TrackError::Other(format!("TODO #{} not found in pending todos", task_index)));
            }
            let move_pos = move_pos.unwrap();

            // If already at the front, nothing to do
            if move_pos == 0 {
                return Ok(());
            }

            // Reorder: move the todo to the front
            let mut reordered = pending_todos.clone();
            let moved_todo = reordered.remove(move_pos);
            reordered.insert(0, moved_todo);

            // Get the minimum task_index to use as starting point
            let min_index = pending_todos[0].1;

            // Update all pending todos with new task_index values
            // We use a temporary offset to avoid UNIQUE constraint violations
            let temp_offset = 10000;

            // First, move all to temporary indices
            for (i, (id, _)) in reordered.iter().enumerate() {
                conn.execute(
                    "UPDATE todos SET task_index = ?1 WHERE id = ?2",
                    params![temp_offset + i as i64, id],
                )?;
            }

            // Then, move them to final indices
            for (i, (id, _)) in reordered.iter().enumerate() {
                conn.execute(
                    "UPDATE todos SET task_index = ?1 WHERE id = ?2",
                    params![min_index + i as i64, id],
                )?;
            }

            self.db.increment_rev("todos")?;
            Ok(())
        })
    }

    /// Copies incomplete (pending) todos from one task to another.
    ///
    /// Returns a mapping of old task_index to new task_index for the copied todos.
    /// This mapping is used to copy linked scraps.
    ///
    /// # Arguments
    ///
    /// * `from_task_id` - The task ID to copy from
    /// * `to_task_id` - The task ID to copy to
    ///
    /// # Returns
    ///
    /// A HashMap mapping old task_index values to new task_index values.
    pub fn copy_incomplete_todos(
        &self,
        from_task_id: i64,
        to_task_id: i64,
    ) -> Result<std::collections::HashMap<i64, i64>> {
        use std::collections::HashMap;

        let mut mapping = HashMap::new();

        self.db.with_transaction(|| {
            let conn = self.db.get_connection();

            // Get all pending todos from the source task
            let mut stmt = conn.prepare(
                "SELECT task_index, content FROM todos WHERE task_id = ?1 AND status = 'pending' ORDER BY task_index ASC"
            )?;

            let pending_todos: Vec<(i64, String)> = stmt
                .query_map(params![from_task_id], |row| {
                    Ok((row.get(0)?, row.get(1)?))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;

            // Copy each todo to the new task
            for (old_index, content) in pending_todos {
                // Get next task_index for the destination task
                let next_index: i64 = conn.query_row(
                    "SELECT COALESCE(MAX(task_index), 0) + 1 FROM todos WHERE task_id = ?1",
                    params![to_task_id],
                    |row| row.get(0),
                )?;

                let now = Utc::now().to_rfc3339();

                // Insert the new todo (don't copy worktree_requested)
                conn.execute(
                    "INSERT INTO todos (task_id, task_index, content, status, worktree_requested, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![to_task_id, next_index, content, "pending", 0, now],
                )?;

                // Store the mapping
                mapping.insert(old_index, next_index);
            }

            if !mapping.is_empty() {
                self.db.increment_rev("todos")?;
            }

            Ok(mapping)
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

    fn create_test_task(db: &Database) -> i64 {
        let task_service = TaskService::new(db);
        task_service
            .create_task("Test Task", None, None, None)
            .unwrap()
            .id
    }

    #[test]
    fn test_add_todo_success() {
        let db = setup_db();
        let task_id = create_test_task(&db);
        let service = TodoService::new(&db);

        let todo = service.add_todo(task_id, "Test TODO", false).unwrap();
        assert_eq!(todo.content, "Test TODO");
        assert_eq!(todo.status, "pending");
        assert!(!todo.worktree_requested);
    }

    #[test]
    fn test_add_todo_with_worktree_success() {
        let db = setup_db();
        let task_id = create_test_task(&db);
        let service = TodoService::new(&db);

        let todo = service.add_todo(task_id, "Test TODO", true).unwrap();
        assert_eq!(todo.content, "Test TODO");
        assert_eq!(todo.status, "pending");
        assert!(todo.worktree_requested);
    }

    #[test]
    fn test_get_todo_success() {
        let db = setup_db();
        let task_id = create_test_task(&db);
        let service = TodoService::new(&db);

        let created = service.add_todo(task_id, "Test TODO", false).unwrap();
        let retrieved = service.get_todo(created.id).unwrap();
        assert_eq!(retrieved.id, created.id);
        assert_eq!(retrieved.content, "Test TODO");
    }

    #[test]
    fn test_get_todo_not_found() {
        let db = setup_db();
        let service = TodoService::new(&db);

        let result = service.get_todo(999);
        assert!(matches!(result, Err(TrackError::TodoNotFound(999))));
    }

    #[test]
    fn test_list_todos() {
        let db = setup_db();
        let task_id = create_test_task(&db);
        let service = TodoService::new(&db);

        service.add_todo(task_id, "TODO 1", false).unwrap();
        service.add_todo(task_id, "TODO 2", false).unwrap();

        let todos = service.list_todos(task_id).unwrap();
        assert_eq!(todos.len(), 2);
        assert_eq!(todos[0].content, "TODO 1");
        assert_eq!(todos[1].content, "TODO 2");
    }

    #[test]
    fn test_update_status_success() {
        let db = setup_db();
        let task_id = create_test_task(&db);
        let service = TodoService::new(&db);

        let todo = service.add_todo(task_id, "Test TODO", false).unwrap();
        service.update_status(todo.id, "done").unwrap();

        let updated = service.get_todo(todo.id).unwrap();
        assert_eq!(updated.status, "done");
        assert!(updated.completed_at.is_some());
    }

    #[test]
    fn test_update_status_pending_no_completed_at() {
        let db = setup_db();
        let task_id = create_test_task(&db);
        let service = TodoService::new(&db);

        let todo = service.add_todo(task_id, "Test TODO", false).unwrap();
        service.update_status(todo.id, "done").unwrap();

        // Change back to pending
        service.update_status(todo.id, "pending").unwrap();

        let updated = service.get_todo(todo.id).unwrap();
        assert_eq!(updated.status, "pending");
        assert!(updated.completed_at.is_none());
    }

    #[test]
    fn test_update_status_invalid() {
        let db = setup_db();
        let task_id = create_test_task(&db);
        let service = TodoService::new(&db);

        let todo = service.add_todo(task_id, "Test TODO", false).unwrap();
        let result = service.update_status(todo.id, "invalid_status");
        assert!(matches!(result, Err(TrackError::InvalidStatus(_))));
    }

    #[test]
    fn test_update_status_not_found() {
        let db = setup_db();
        let service = TodoService::new(&db);

        let result = service.update_status(999, "done");
        assert!(matches!(result, Err(TrackError::TodoNotFound(999))));
    }

    #[test]
    fn test_delete_todo_success() {
        let db = setup_db();
        let task_id = create_test_task(&db);
        let service = TodoService::new(&db);

        let todo = service.add_todo(task_id, "Test TODO", false).unwrap();
        service.delete_todo(todo.id).unwrap();

        let result = service.get_todo(todo.id);
        assert!(matches!(result, Err(TrackError::TodoNotFound(_))));
    }

    #[test]
    fn test_delete_todo_not_found() {
        let db = setup_db();
        let service = TodoService::new(&db);

        let result = service.delete_todo(999);
        assert!(matches!(result, Err(TrackError::TodoNotFound(999))));
    }

    #[test]
    fn test_task_index_sequential() {
        let db = setup_db();
        let task_id = create_test_task(&db);
        let service = TodoService::new(&db);

        let todo1 = service.add_todo(task_id, "TODO 1", false).unwrap();
        let todo2 = service.add_todo(task_id, "TODO 2", false).unwrap();
        let todo3 = service.add_todo(task_id, "TODO 3", false).unwrap();

        assert_eq!(todo1.task_index, 1);
        assert_eq!(todo2.task_index, 2);
        assert_eq!(todo3.task_index, 3);
    }

    #[test]
    fn test_task_index_independence() {
        let db = setup_db();
        let task_service = TaskService::new(&db);
        let task1_id = task_service
            .create_task("Task 1", None, None, None)
            .unwrap()
            .id;
        let task2_id = task_service
            .create_task("Task 2", None, None, None)
            .unwrap()
            .id;
        let service = TodoService::new(&db);

        let task1_todo1 = service.add_todo(task1_id, "Task 1 TODO 1", false).unwrap();
        let task2_todo1 = service.add_todo(task2_id, "Task 2 TODO 1", false).unwrap();
        let task1_todo2 = service.add_todo(task1_id, "Task 1 TODO 2", false).unwrap();
        let task2_todo2 = service.add_todo(task2_id, "Task 2 TODO 2", false).unwrap();

        // Each task should have independent indexing starting from 1
        assert_eq!(task1_todo1.task_index, 1);
        assert_eq!(task1_todo2.task_index, 2);
        assert_eq!(task2_todo1.task_index, 1);
        assert_eq!(task2_todo2.task_index, 2);
    }

    #[test]
    fn test_get_todo_by_index_success() {
        let db = setup_db();
        let task_id = create_test_task(&db);
        let service = TodoService::new(&db);

        service.add_todo(task_id, "TODO 1", false).unwrap();
        let created = service.add_todo(task_id, "TODO 2", false).unwrap();
        service.add_todo(task_id, "TODO 3", false).unwrap();

        let retrieved = service.get_todo_by_index(task_id, 2).unwrap();
        assert_eq!(retrieved.id, created.id);
        assert_eq!(retrieved.task_index, 2);
        assert_eq!(retrieved.content, "TODO 2");
    }

    #[test]
    fn test_get_todo_by_index_not_found() {
        let db = setup_db();
        let task_id = create_test_task(&db);
        let service = TodoService::new(&db);

        service.add_todo(task_id, "TODO 1", false).unwrap();

        let result = service.get_todo_by_index(task_id, 5);
        assert!(result.is_err());
    }

    #[test]
    fn test_list_todos_ordered_by_index() {
        let db = setup_db();
        let task_id = create_test_task(&db);
        let service = TodoService::new(&db);

        service.add_todo(task_id, "TODO 1", false).unwrap();
        service.add_todo(task_id, "TODO 2", false).unwrap();
        service.add_todo(task_id, "TODO 3", false).unwrap();

        let todos = service.list_todos(task_id).unwrap();
        assert_eq!(todos.len(), 3);
        assert_eq!(todos[0].task_index, 1);
        assert_eq!(todos[1].task_index, 2);
        assert_eq!(todos[2].task_index, 3);
    }

    #[test]
    fn test_move_to_next_success() {
        let db = setup_db();
        let task_id = create_test_task(&db);
        let service = TodoService::new(&db);

        // Create 4 todos
        service.add_todo(task_id, "TODO 1", false).unwrap();
        service.add_todo(task_id, "TODO 2", false).unwrap();
        service.add_todo(task_id, "TODO 3", false).unwrap();
        service.add_todo(task_id, "TODO 4", false).unwrap();

        // Move TODO #4 to the front
        service.move_to_next(task_id, 4).unwrap();

        let todos = service.list_todos(task_id).unwrap();
        assert_eq!(todos.len(), 4);
        // TODO #4 should now be at index 1 (the front)
        assert_eq!(todos[0].task_index, 1);
        assert_eq!(todos[0].content, "TODO 4");
        // Others should be shifted
        assert_eq!(todos[1].task_index, 2);
        assert_eq!(todos[1].content, "TODO 1");
        assert_eq!(todos[2].task_index, 3);
        assert_eq!(todos[2].content, "TODO 2");
        assert_eq!(todos[3].task_index, 4);
        assert_eq!(todos[3].content, "TODO 3");
    }

    #[test]
    fn test_move_to_next_already_at_front() {
        let db = setup_db();
        let task_id = create_test_task(&db);
        let service = TodoService::new(&db);

        service.add_todo(task_id, "TODO 1", false).unwrap();
        service.add_todo(task_id, "TODO 2", false).unwrap();

        // Try to move TODO #1 to the front (it's already there)
        service.move_to_next(task_id, 1).unwrap();

        let todos = service.list_todos(task_id).unwrap();
        assert_eq!(todos[0].task_index, 1);
        assert_eq!(todos[0].content, "TODO 1");
        assert_eq!(todos[1].task_index, 2);
        assert_eq!(todos[1].content, "TODO 2");
    }

    #[test]
    fn test_move_to_next_with_done_todos() {
        let db = setup_db();
        let task_id = create_test_task(&db);
        let service = TodoService::new(&db);

        // Create todos and mark some as done
        let todo1 = service.add_todo(task_id, "TODO 1", false).unwrap();
        service.add_todo(task_id, "TODO 2", false).unwrap();
        service.add_todo(task_id, "TODO 3", false).unwrap();
        service.add_todo(task_id, "TODO 4", false).unwrap();

        // Mark TODO #1 as done
        service.update_status(todo1.id, "done").unwrap();

        // Move TODO #4 to the front (should be first among pending)
        service.move_to_next(task_id, 4).unwrap();

        let todos = service.list_todos(task_id).unwrap();
        // TODO #1 should still be at index 1 (done)
        assert_eq!(todos[0].task_index, 1);
        assert_eq!(todos[0].content, "TODO 1");
        assert_eq!(todos[0].status, "done");
        // TODO #4 should be at index 2 (first pending)
        assert_eq!(todos[1].task_index, 2);
        assert_eq!(todos[1].content, "TODO 4");
        assert_eq!(todos[1].status, "pending");
    }

    #[test]
    fn test_move_to_next_done_todo_fails() {
        let db = setup_db();
        let task_id = create_test_task(&db);
        let service = TodoService::new(&db);

        let todo1 = service.add_todo(task_id, "TODO 1", false).unwrap();
        service.add_todo(task_id, "TODO 2", false).unwrap();

        // Mark TODO #1 as done
        service.update_status(todo1.id, "done").unwrap();

        // Try to move a done todo
        let result = service.move_to_next(task_id, 1);
        assert!(result.is_err());
    }
}
