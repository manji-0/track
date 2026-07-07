//! Shared SQLite row parsing helpers for domain entities.

use crate::models::{Task, TaskStatus, Todo, TodoStatus};
use chrono::{DateTime, Utc};
use rusqlite::{types::Type, Row};
use std::str::FromStr;

/// Parses an RFC3339 timestamp stored as TEXT.
pub fn parse_datetime(value: String) -> rusqlite::Result<DateTime<Utc>> {
    value
        .parse()
        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(e)))
}

/// Parses a task status column value.
pub fn parse_task_status(value: String) -> rusqlite::Result<TaskStatus> {
    TaskStatus::from_str(&value).map_err(|_| rusqlite::Error::InvalidQuery)
}

/// Parses a todo status column value.
pub fn parse_todo_status(value: String) -> rusqlite::Result<TodoStatus> {
    TodoStatus::from_str(&value).map_err(|_| rusqlite::Error::InvalidQuery)
}

/// Maps a tasks table row into a [`Task`].
pub fn row_to_task(row: &Row<'_>) -> rusqlite::Result<Task> {
    Ok(Task {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        status: parse_task_status(row.get(3)?)?,
        ticket_id: row.get(4)?,
        ticket_url: row.get(5)?,
        alias: row.get(6)?,
        is_today_task: row.get::<_, i64>(7)? != 0,
        created_at: parse_datetime(row.get(8)?)?,
    })
}

/// Maps a todos table row into a [`Todo`].
pub fn row_to_todo(row: &Row<'_>) -> rusqlite::Result<Todo> {
    Ok(Todo {
        id: row.get(0)?,
        task_id: row.get(1)?,
        task_index: row.get(2)?,
        content: row.get(3)?,
        status: parse_todo_status(row.get(4)?)?,
        worktree_requested: row.get::<_, i64>(5)? != 0,
        requires_workspace: row.get::<_, i64>(6)? != 0,
        created_at: parse_datetime(row.get(7)?)?,
        completed_at: row
            .get::<_, Option<String>>(8)?
            .map(parse_datetime)
            .transpose()?,
    })
}
