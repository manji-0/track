use crate::db::Database;
use crate::models::{Task, TaskStatus};
use crate::services::link_service::ScrapService;
use crate::services::task_service::TaskService;
use crate::services::todo_service::TodoService;
use crate::utils::Result;
use chrono::{Local, Utc};
use rusqlite::{params, OptionalExtension};

/// Creates or reuses the daily "today" task with atomic todo/scrap inheritance.
pub struct CreateTodayTaskUseCase<'a> {
    db: &'a Database,
}

impl<'a> CreateTodayTaskUseCase<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Returns today's task, creating it and inheriting pending work when needed.
    pub fn get_or_create(&self) -> Result<Task> {
        let today = Local::now().format("%Y-%m-%d").to_string();
        let task_name = format!("Today: {}", today);

        let conn = self.db.get_connection();
        let mut stmt = conn.prepare(
            "SELECT id FROM tasks WHERE is_today_task = 1 AND name = ?1 AND status = 'active'",
        )?;

        if let Some(task_id) = stmt
            .query_row(params![task_name], |row| row.get::<_, i64>(0))
            .optional()?
        {
            return TaskService::new(self.db).get_task(task_id);
        }

        self.create_with_inheritance(&task_name)
    }

    /// Creates a today task and inherits pending todos/scraps inside one transaction.
    pub fn create_with_inheritance(&self, task_name: &str) -> Result<Task> {
        self.db.with_transaction(|| {
            let inherit_from = self.find_today_task_to_inherit_from()?;

            let conn = self.db.get_connection();
            let now = Utc::now().to_rfc3339();

            conn.execute(
                "UPDATE tasks SET is_today_task = 0 WHERE is_today_task = 1",
                [],
            )?;

            conn.execute(
                "INSERT INTO tasks (name, description, status, is_today_task, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    task_name,
                    None::<String>,
                    TaskStatus::Active.as_str(),
                    1,
                    now
                ],
            )?;

            let task_id = conn.last_insert_rowid();

            if let Some(from_task_id) = inherit_from {
                let todo_service = TodoService::new(self.db);
                let scrap_service = ScrapService::new(self.db);
                let mapping =
                    todo_service.copy_incomplete_todos_in_tx(from_task_id, task_id)?;
                scrap_service.copy_linked_scraps_in_tx(from_task_id, task_id, &mapping)?;
            }

            self.db.set_current_task_id(task_id)?;
            self.db.increment_rev("task")?;
            TaskService::new(self.db).get_task(task_id)
        })
    }

    fn find_today_task_to_inherit_from(&self) -> Result<Option<i64>> {
        let conn = self.db.get_connection();
        let mut stmt = conn.prepare(
            "SELECT id FROM tasks WHERE is_today_task = 1 AND status = 'active' ORDER BY created_at DESC LIMIT 1",
        )?;

        stmt.query_row([], |row| row.get(0))
            .optional()
            .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::{ScrapService, TodoService};

    #[test]
    fn create_today_task_inherits_pending_todos_and_scraps() {
        let db = Database::new_in_memory().unwrap();
        let task_service = TaskService::new(&db);
        let todo_service = TodoService::new(&db);
        let scrap_service = ScrapService::new(&db);
        let use_case = CreateTodayTaskUseCase::new(&db);

        let previous = task_service
            .create_task("Today: 2026-01-01", None, None, None)
            .unwrap();
        db.get_connection()
            .execute(
                "UPDATE tasks SET is_today_task = 1 WHERE id = ?1",
                rusqlite::params![previous.id],
            )
            .unwrap();
        db.set_current_task_id(previous.id).unwrap();

        todo_service
            .add_todo(previous.id, "Carry over", false)
            .unwrap();
        scrap_service.add_scrap(previous.id, "linked note").unwrap();

        let today_task = use_case.get_or_create().unwrap();
        assert_ne!(today_task.id, previous.id);
        assert!(today_task.is_today_task);

        let inherited = todo_service.list_todos(today_task.id).unwrap();
        assert_eq!(inherited.len(), 1);
        assert_eq!(inherited[0].content, "Carry over");

        let inherited_scraps = scrap_service.list_scraps(today_task.id).unwrap();
        assert_eq!(inherited_scraps.len(), 1);
        assert_eq!(inherited_scraps[0].content, "linked note");
    }
}
