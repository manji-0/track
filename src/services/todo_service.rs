use rusqlite::params;
use chrono::Utc;
use crate::db::Database;
use crate::models::{Todo, TodoStatus};
use crate::utils::{Result, TrackError};

pub struct TodoService<'a> {
    db: &'a Database,
}

impl<'a> TodoService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn add_todo(&self, task_id: i64, content: &str) -> Result<Todo> {
        let now = Utc::now().to_rfc3339();
        let conn = self.db.get_connection();

        conn.execute(
            "INSERT INTO todos (task_id, content, status, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![task_id, content, TodoStatus::Pending.as_str(), now],
        )?;

        let todo_id = conn.last_insert_rowid();
        self.get_todo(todo_id)
    }

    pub fn get_todo(&self, todo_id: i64) -> Result<Todo> {
        let conn = self.db.get_connection();
        let mut stmt = conn.prepare(
            "SELECT id, task_id, content, status, created_at FROM todos WHERE id = ?1"
        )?;

        let todo = stmt.query_row(params![todo_id], |row| {
            Ok(Todo {
                id: row.get(0)?,
                task_id: row.get(1)?,
                content: row.get(2)?,
                status: row.get(3)?,
                created_at: row.get::<_, String>(4)?.parse().unwrap(),
            })
        }).map_err(|_| TrackError::TodoNotFound(todo_id))?;

        Ok(todo)
    }

    pub fn list_todos(&self, task_id: i64) -> Result<Vec<Todo>> {
        let conn = self.db.get_connection();
        let mut stmt = conn.prepare(
            "SELECT id, task_id, content, status, created_at FROM todos WHERE task_id = ?1 ORDER BY created_at ASC"
        )?;

        let todos = stmt.query_map(params![task_id], |row| {
            Ok(Todo {
                id: row.get(0)?,
                task_id: row.get(1)?,
                content: row.get(2)?,
                status: row.get(3)?,
                created_at: row.get::<_, String>(4)?.parse().unwrap(),
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(todos)
    }

    pub fn update_status(&self, todo_id: i64, status: &str) -> Result<()> {
        // Validate status
        TodoStatus::from_str(status)
            .ok_or_else(|| TrackError::InvalidStatus(status.to_string()))?;

        let conn = self.db.get_connection();
        let affected = conn.execute(
            "UPDATE todos SET status = ?1 WHERE id = ?2",
            params![status, todo_id],
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
        task_service.create_task("Test Task", None, None, None).unwrap().id
    }

    #[test]
    fn test_add_todo_success() {
        let db = setup_db();
        let task_id = create_test_task(&db);
        let service = TodoService::new(&db);

        let todo = service.add_todo(task_id, "Test TODO").unwrap();
        assert_eq!(todo.content, "Test TODO");
        assert_eq!(todo.status, "pending");
    }

    #[test]
    fn test_get_todo_success() {
        let db = setup_db();
        let task_id = create_test_task(&db);
        let service = TodoService::new(&db);

        let created = service.add_todo(task_id, "Test TODO").unwrap();
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

        service.add_todo(task_id, "TODO 1").unwrap();
        service.add_todo(task_id, "TODO 2").unwrap();

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

        let todo = service.add_todo(task_id, "Test TODO").unwrap();
        service.update_status(todo.id, "done").unwrap();

        let updated = service.get_todo(todo.id).unwrap();
        assert_eq!(updated.status, "done");
    }

    #[test]
    fn test_update_status_invalid() {
        let db = setup_db();
        let task_id = create_test_task(&db);
        let service = TodoService::new(&db);

        let todo = service.add_todo(task_id, "Test TODO").unwrap();
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

        let todo = service.add_todo(task_id, "Test TODO").unwrap();
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
}

