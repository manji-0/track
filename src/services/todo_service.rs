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
        let conn = self.db.get_connection();

        // Get next task_index for this task
        let next_index: i64 = conn.query_row(
            "SELECT COALESCE(MAX(task_index), 0) + 1 FROM todos WHERE task_id = ?1",
            params![task_id],
            |row| row.get(0),
        )?;

        conn.execute(
            "INSERT INTO todos (task_id, task_index, content, status, worktree_requested, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![task_id, next_index, content, TodoStatus::Pending.as_str(), worktree_requested, now],
        )?;

        let todo_id = conn.last_insert_rowid();
        self.get_todo(todo_id)
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

        Ok(())
    }

    pub fn delete_todo(&self, todo_id: i64) -> Result<()> {
        let conn = self.db.get_connection();
        let affected = conn.execute("DELETE FROM todos WHERE id = ?1", params![todo_id])?;

        if affected == 0 {
            return Err(TrackError::TodoNotFound(todo_id));
        }

        Ok(())
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
        assert_eq!(todo.worktree_requested, false);
    }

    #[test]
    fn test_add_todo_with_worktree_success() {
        let db = setup_db();
        let task_id = create_test_task(&db);
        let service = TodoService::new(&db);

        let todo = service.add_todo(task_id, "Test TODO", true).unwrap();
        assert_eq!(todo.content, "Test TODO");
        assert_eq!(todo.status, "pending");
        assert_eq!(todo.worktree_requested, true);
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
}
