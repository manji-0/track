use rusqlite::{params, OptionalExtension};
use chrono::Utc;
use crate::db::Database;
use crate::models::{Task, TaskStatus};
use crate::utils::{Result, TrackError};

pub struct TaskService<'a> {
    db: &'a Database,
}

impl<'a> TaskService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn create_task(&self, name: &str, ticket_id: Option<&str>, ticket_url: Option<&str>) -> Result<Task> {
        if name.trim().is_empty() {
            return Err(TrackError::EmptyTaskName);
        }

        // Validate ticket ID format if provided
        if let Some(ticket) = ticket_id {
            self.validate_ticket_format(ticket)?;
            
            // Check for duplicate ticket
            if let Some(existing_id) = self.find_task_by_ticket(ticket)? {
                return Err(TrackError::DuplicateTicket(ticket.to_string(), existing_id));
            }
        }

        let now = Utc::now().to_rfc3339();
        let conn = self.db.get_connection();
        
        conn.execute(
            "INSERT INTO tasks (name, status, ticket_id, ticket_url, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![name, TaskStatus::Active.as_str(), ticket_id, ticket_url, now],
        )?;

        let task_id = conn.last_insert_rowid();
        
        // Set as current task
        self.db.set_current_task_id(task_id)?;

        self.get_task(task_id)
    }

    pub fn get_task(&self, task_id: i64) -> Result<Task> {
        let conn = self.db.get_connection();
        let mut stmt = conn.prepare(
            "SELECT id, name, status, ticket_id, ticket_url, created_at FROM tasks WHERE id = ?1"
        )?;

        let task = stmt.query_row(params![task_id], |row| {
            Ok(Task {
                id: row.get(0)?,
                name: row.get(1)?,
                status: row.get(2)?,
                ticket_id: row.get(3)?,
                ticket_url: row.get(4)?,
                created_at: row.get::<_, String>(5)?.parse().unwrap(),
            })
        }).map_err(|_| TrackError::TaskNotFound(task_id))?;

        Ok(task)
    }

    pub fn list_tasks(&self, include_archived: bool) -> Result<Vec<Task>> {
        let conn = self.db.get_connection();
        let query = if include_archived {
            "SELECT id, name, status, ticket_id, ticket_url, created_at FROM tasks ORDER BY created_at DESC"
        } else {
            "SELECT id, name, status, ticket_id, ticket_url, created_at FROM tasks WHERE status = 'active' ORDER BY created_at DESC"
        };

        let mut stmt = conn.prepare(query)?;
        let tasks = stmt.query_map([], |row| {
            Ok(Task {
                id: row.get(0)?,
                name: row.get(1)?,
                status: row.get(2)?,
                ticket_id: row.get(3)?,
                ticket_url: row.get(4)?,
                created_at: row.get::<_, String>(5)?.parse().unwrap(),
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(tasks)
    }

    pub fn switch_task(&self, task_id: i64) -> Result<Task> {
        let task = self.get_task(task_id)?;
        
        if task.status == TaskStatus::Archived.as_str() {
            return Err(TrackError::TaskArchived(task_id));
        }

        self.db.set_current_task_id(task_id)?;
        Ok(task)
    }

    pub fn archive_task(&self, task_id: i64) -> Result<()> {
        let conn = self.db.get_connection();
        
        conn.execute(
            "UPDATE tasks SET status = ?1 WHERE id = ?2",
            params![TaskStatus::Archived.as_str(), task_id],
        )?;

        // Clear current task if it's the archived one
        if let Some(current_id) = self.db.get_current_task_id()? {
            if current_id == task_id {
                self.db.clear_current_task_id()?;
            }
        }

        Ok(())
    }

    pub fn link_ticket(&self, task_id: i64, ticket_id: &str, url: &str) -> Result<()> {
        self.validate_ticket_format(ticket_id)?;
        
        // Check for duplicate ticket (excluding current task)
        if let Some(existing_id) = self.find_task_by_ticket(ticket_id)? {
            if existing_id != task_id {
                return Err(TrackError::DuplicateTicket(ticket_id.to_string(), existing_id));
            }
        }

        let conn = self.db.get_connection();
        conn.execute(
            "UPDATE tasks SET ticket_id = ?1, ticket_url = ?2 WHERE id = ?3",
            params![ticket_id, url, task_id],
        )?;

        Ok(())
    }

    pub fn resolve_task_id(&self, reference: &str) -> Result<i64> {
        // If it starts with "t:", it's a ticket reference
        if let Some(ticket_id) = reference.strip_prefix("t:") {
            self.find_task_by_ticket(ticket_id)?
                .ok_or_else(|| TrackError::Other(format!("No task found with ticket '{}'", ticket_id)))
        } else {
            // Otherwise, parse as task ID
            reference.parse::<i64>()
                .map_err(|_| TrackError::Other(format!("Invalid task reference: {}", reference)))
        }
    }

    fn find_task_by_ticket(&self, ticket_id: &str) -> Result<Option<i64>> {
        let conn = self.db.get_connection();
        let mut stmt = conn.prepare("SELECT id FROM tasks WHERE ticket_id = ?1")?;
        let result = stmt.query_row(params![ticket_id], |row| row.get(0))
            .optional()?;
        Ok(result)
    }

    fn validate_ticket_format(&self, ticket_id: &str) -> Result<()> {
        // Jira format: PROJECT-123
        if ticket_id.contains('-') && ticket_id.chars().any(|c| c.is_ascii_uppercase()) {
            return Ok(());
        }

        // GitHub/GitLab format: owner/repo/123
        let parts: Vec<&str> = ticket_id.split('/').collect();
        if parts.len() == 3 && parts[2].chars().all(|c| c.is_ascii_digit()) {
            return Ok(());
        }

        Err(TrackError::InvalidTicketFormat(ticket_id.to_string()))
    }
}
