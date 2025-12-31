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
