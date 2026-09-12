use crate::db::Database;
use crate::db::row_mapping::row_to_task;
use crate::models::{Task, TaskAlias, TaskId, TaskStatus, TicketId};
use crate::utils::{Result, TrackError};
use chrono::Utc;
use rusqlite::{OptionalExtension, params};

/// Service for managing development tasks.
///
/// TaskService provides operations for creating, retrieving, updating, and archiving tasks.
/// It handles task lifecycle management, ticket linking, and task switching.
pub struct TaskService<'a> {
    db: &'a Database,
}

impl<'a> TaskService<'a> {
    /// Creates a new TaskService instance.
    ///
    /// # Arguments
    ///
    /// * `db` - Reference to the database connection
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Creates a new task and sets it as the current task.
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the task (cannot be empty)
    /// * `description` - Optional task description
    /// * `ticket_id` - Optional ticket ID (e.g., "PROJ-123" or "owner/repo/456")
    /// * `ticket_url` - Optional URL to the ticket
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The task name is empty
    /// - The ticket ID format is invalid
    /// - A task with the same ticket ID already exists
    pub fn create_task(
        &self,
        name: &str,
        description: Option<&str>,
        ticket_id: Option<&str>,
        ticket_url: Option<&str>,
    ) -> Result<Task> {
        if name.trim().is_empty() {
            return Err(TrackError::EmptyTaskName);
        }

        let ticket = ticket_id.map(TicketId::parse).transpose()?;
        if let Some(ticket) = ticket.as_ref()
            && let Some(existing_id) = self.find_task_by_ticket(ticket)?
        {
            return Err(TrackError::DuplicateTicket(
                ticket.to_string(),
                existing_id.as_i64(),
            ));
        }

        let now = Utc::now().to_rfc3339();
        let conn = self.db.get_connection();

        conn.execute(
            "INSERT INTO tasks (name, description, status, ticket_id, ticket_url, is_today_task, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![name, description, TaskStatus::Active.as_str(), ticket.as_ref().map(|t| t.as_str()), ticket_url, 0, now],
        )?;

        let task_id = TaskId::from_i64(conn.last_insert_rowid());

        // Set as current task
        self.db.set_current_task_id(task_id)?;

        self.get_task(task_id)
    }

    /// Retrieves a task by its ID.
    ///
    /// # Arguments
    ///
    /// * `task_id` - The ID of the task to retrieve
    ///
    /// # Errors
    ///
    /// Returns `TrackError::TaskNotFound` if the task does not exist.
    pub fn get_task(&self, task_id: crate::models::TaskId) -> Result<Task> {
        let conn = self.db.get_connection();
        let mut stmt = conn.prepare(
            "SELECT id, name, description, status, ticket_id, ticket_url, alias, is_today_task, created_at FROM tasks WHERE id = ?1"
        )?;

        let task = stmt
            .query_row(params![task_id], row_to_task)
            .map_err(|_| TrackError::TaskNotFound(task_id.as_i64()))?;

        Ok(task)
    }

    /// Lists all tasks, optionally including archived tasks.
    ///
    /// # Arguments
    ///
    /// * `include_archived` - If true, includes archived tasks in the results
    ///
    /// # Returns
    ///
    /// A vector of tasks ordered by creation date (newest first).
    pub fn list_tasks(&self, include_archived: bool) -> Result<Vec<Task>> {
        let conn = self.db.get_connection();
        let query = if include_archived {
            "SELECT id, name, description, status, ticket_id, ticket_url, alias, is_today_task, created_at FROM tasks ORDER BY created_at DESC".to_string()
        } else {
            format!(
                "SELECT id, name, description, status, ticket_id, ticket_url, alias, is_today_task, created_at FROM tasks WHERE status = '{}' ORDER BY created_at DESC",
                TaskStatus::ACTIVE
            )
        };

        let mut stmt = conn.prepare(&query)?;
        let tasks = stmt
            .query_map([], row_to_task)?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(tasks)
    }

    /// Switches to a different task, making it the current active task.
    ///
    /// # Arguments
    ///
    /// * `task_id` - The ID of the task to switch to
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The task does not exist
    /// - The task is archived
    pub fn switch_task(&self, task_id: crate::models::TaskId) -> Result<Task> {
        let task = self.get_task(task_id)?;

        if task.status == TaskStatus::Archived {
            return Err(TrackError::TaskArchived(task_id.as_i64()));
        }

        self.db.set_current_task_id(task_id)?;
        Ok(task)
    }

    /// Archives a task, marking it as completed or abandoned.
    ///
    /// If the archived task is the current task, the current task is cleared.
    ///
    /// # Arguments
    ///
    /// * `task_id` - The ID of the task to archive
    pub fn archive_task(&self, task_id: crate::models::TaskId) -> Result<()> {
        let task = self.get_task(task_id)?;
        task.status.archive()?;

        let conn = self.db.get_connection();

        conn.execute(
            "UPDATE tasks SET status = ?1 WHERE id = ?2",
            params![TaskStatus::Archived.as_str(), task_id],
        )?;

        // Clear current task if it's the archived one
        if let Some(current_id) = self.db.get_current_task_id()?
            && current_id == task_id
        {
            self.db.clear_current_task_id()?;
        }

        self.db.increment_rev("task")?;
        Ok(())
    }

    /// Links a ticket to an existing task.
    ///
    /// # Arguments
    ///
    /// * `task_id` - The ID of the task to link the ticket to
    /// * `ticket_id` - The ticket ID (e.g., "PROJ-123" or "owner/repo/456")
    /// * `url` - The URL to the ticket
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The ticket ID format is invalid
    /// - Another task is already linked to this ticket
    pub fn link_ticket(
        &self,
        task_id: crate::models::TaskId,
        ticket_id: &str,
        url: &str,
    ) -> Result<()> {
        let ticket = TicketId::parse(ticket_id)?;

        if let Some(existing_id) = self.find_task_by_ticket(&ticket)?
            && existing_id != task_id
        {
            return Err(TrackError::DuplicateTicket(
                ticket.to_string(),
                existing_id.as_i64(),
            ));
        }

        let conn = self.db.get_connection();
        conn.execute(
            "UPDATE tasks SET ticket_id = ?1, ticket_url = ?2 WHERE id = ?3",
            params![ticket.as_str(), url, task_id],
        )?;

        self.db.increment_rev("task")?;
        Ok(())
    }

    /// Sets or updates the description of a task.
    ///
    /// # Arguments
    ///
    /// * `task_id` - The ID of the task to update
    /// * `description` - The new description text
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The task does not exist
    /// - The task is archived
    pub fn set_description(&self, task_id: crate::models::TaskId, description: &str) -> Result<()> {
        // Validate task exists and is active
        let task = self.get_task(task_id)?;
        if task.status == TaskStatus::Archived {
            return Err(TrackError::TaskArchived(task_id.as_i64()));
        }

        let conn = self.db.get_connection();
        conn.execute(
            "UPDATE tasks SET description = ?1 WHERE id = ?2",
            params![description, task_id],
        )?;

        self.db.increment_rev("task")?;
        Ok(())
    }

    /// Resolves a task reference to a task ID.
    ///
    /// Accepts a numeric task ID, a ticket reference prefixed with "t:", or an alias.
    ///
    /// # Arguments
    ///
    /// * `reference` - Either a task ID (e.g., "1"), ticket reference (e.g., "t:PROJ-123"), or alias (e.g., "daily-work")
    ///
    /// # Returns
    ///
    /// The resolved task ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the reference is invalid or no matching task is found.
    pub fn resolve_task_id(&self, reference: &str) -> Result<TaskId> {
        if let Some(ticket_id) = reference.strip_prefix("t:") {
            let ticket = TicketId::parse(ticket_id)?;
            return self
                .find_task_by_ticket(&ticket)?
                .ok_or_else(|| TrackError::TaskReferenceNotFound(format!("t:{ticket_id}")));
        }

        if let Ok(task_id) = reference.parse::<i64>() {
            return Ok(TaskId::from_i64(task_id));
        }

        if let Ok(alias) = TaskAlias::parse(reference)
            && let Some(task_id) = self.get_task_by_alias(&alias)?
        {
            return Ok(task_id);
        }

        Err(TrackError::TaskReferenceNotFound(reference.to_string()))
    }

    /// Sets an alias for a task.
    ///
    /// # Arguments
    ///
    /// * `task_id` - The ID of the task to set the alias for
    /// * `alias` - The alias to set (must be unique and valid)
    /// * `force` - If true, removes the alias from any existing task before setting it
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The alias format is invalid
    /// - The alias is already in use by another task (when force is false)
    /// - The task does not exist
    pub fn set_alias(
        &self,
        task_id: crate::models::TaskId,
        alias: &str,
        force: bool,
    ) -> Result<()> {
        let alias = TaskAlias::parse(alias)?;

        // Check if alias is already in use
        if let Some(existing_id) = self.get_task_by_alias(&alias)?
            && existing_id != task_id
        {
            if force {
                let conn = self.db.get_connection();
                conn.execute(
                    "UPDATE tasks SET alias = NULL WHERE id = ?1",
                    params![existing_id],
                )?;
            } else {
                return Err(TrackError::AliasInUse {
                    alias: alias.to_string(),
                    task_id: existing_id.as_i64(),
                });
            }
        }

        let conn = self.db.get_connection();
        conn.execute(
            "UPDATE tasks SET alias = ?1 WHERE id = ?2",
            params![alias, task_id],
        )?;

        self.db.increment_rev("task")?;
        Ok(())
    }

    /// Removes the alias from a task.
    ///
    /// # Arguments
    ///
    /// * `task_id` - The ID of the task to remove the alias from
    pub fn remove_alias(&self, task_id: crate::models::TaskId) -> Result<()> {
        let conn = self.db.get_connection();
        conn.execute(
            "UPDATE tasks SET alias = NULL WHERE id = ?1",
            params![task_id],
        )?;
        Ok(())
    }

    /// Finds a task ID by its alias.
    ///
    /// # Arguments
    ///
    /// * `alias` - The alias to search for
    ///
    /// # Returns
    ///
    /// `Some(task_id)` if a task with the alias exists, `None` otherwise.
    fn get_task_by_alias(&self, alias: &TaskAlias) -> Result<Option<TaskId>> {
        let conn = self.db.get_connection();
        let mut stmt = conn.prepare("SELECT id FROM tasks WHERE alias = ?1")?;
        let result = stmt
            .query_row(params![alias], |row| row.get(0))
            .optional()?;
        Ok(result)
    }

    fn find_task_by_ticket(&self, ticket_id: &TicketId) -> Result<Option<TaskId>> {
        let conn = self.db.get_connection();
        let mut stmt = conn.prepare("SELECT id FROM tasks WHERE ticket_id = ?1")?;
        let result = stmt
            .query_row(params![ticket_id], |row| row.get(0))
            .optional()?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    fn setup_db() -> Database {
        Database::new_in_memory().unwrap()
    }

    #[test]
    fn test_create_task_success() {
        let db = setup_db();
        let service = TaskService::new(&db);

        let task = service.create_task("Test Task", None, None, None).unwrap();
        assert_eq!(task.name, "Test Task");
        assert_eq!(task.status, TaskStatus::Active);
        assert!(task.ticket_id.is_none());
    }

    #[test]
    fn test_create_task_with_ticket() {
        let db = setup_db();
        let service = TaskService::new(&db);

        let task = service
            .create_task(
                "Test Task",
                None,
                Some("PROJ-123"),
                Some("https://example.com"),
            )
            .unwrap();
        assert_eq!(task.ticket_id.as_deref(), Some("PROJ-123"));
        assert_eq!(task.ticket_url, Some("https://example.com".to_string()));
    }

    #[test]
    fn test_create_task_empty_name() {
        let db = setup_db();
        let service = TaskService::new(&db);

        let result = service.create_task("", None, None, None);
        assert!(matches!(result, Err(TrackError::EmptyTaskName)));
    }

    #[test]
    fn test_create_task_duplicate_ticket() {
        let db = setup_db();
        let service = TaskService::new(&db);

        service
            .create_task("Task 1", None, Some("PROJ-123"), None)
            .unwrap();
        let result = service.create_task("Task 2", None, Some("PROJ-123"), None);
        assert!(matches!(result, Err(TrackError::DuplicateTicket(_, _))));
    }

    #[test]
    fn test_get_task_success() {
        let db = setup_db();
        let service = TaskService::new(&db);

        let created = service.create_task("Test Task", None, None, None).unwrap();
        let retrieved = service.get_task(created.id).unwrap();
        assert_eq!(retrieved.id, created.id);
        assert_eq!(retrieved.name, "Test Task");
    }

    #[test]
    fn test_get_task_not_found() {
        let db = setup_db();
        let service = TaskService::new(&db);

        let result = service.get_task(TaskId::from_i64(999));
        assert!(matches!(result, Err(TrackError::TaskNotFound(999))));
    }

    #[test]
    fn test_list_tasks() {
        let db = setup_db();
        let service = TaskService::new(&db);

        service.create_task("Task 1", None, None, None).unwrap();
        service.create_task("Task 2", None, None, None).unwrap();

        let tasks = service.list_tasks(false).unwrap();
        assert_eq!(tasks.len(), 2);
    }

    #[test]
    fn test_list_tasks_exclude_archived() {
        let db = setup_db();
        let service = TaskService::new(&db);

        let task1 = service.create_task("Task 1", None, None, None).unwrap();
        service.create_task("Task 2", None, None, None).unwrap();
        service.archive_task(task1.id).unwrap();

        let tasks = service.list_tasks(false).unwrap();
        assert_eq!(tasks.len(), 1);

        let all_tasks = service.list_tasks(true).unwrap();
        assert_eq!(all_tasks.len(), 2);
    }

    #[test]
    fn test_switch_task_success() {
        let db = setup_db();
        let service = TaskService::new(&db);

        let task = service.create_task("Task 1", None, None, None).unwrap();
        let switched = service.switch_task(task.id).unwrap();
        assert_eq!(switched.id, task.id);

        let current_id = db.get_current_task_id().unwrap();
        assert_eq!(current_id, Some(task.id));
    }

    #[test]
    fn test_switch_task_archived() {
        let db = setup_db();
        let service = TaskService::new(&db);

        let task = service.create_task("Task 1", None, None, None).unwrap();
        service.archive_task(task.id).unwrap();

        let result = service.switch_task(task.id);
        assert!(matches!(result, Err(TrackError::TaskArchived(_))));
    }

    #[test]
    fn test_archive_task() {
        let db = setup_db();
        let service = TaskService::new(&db);

        let task = service.create_task("Task 1", None, None, None).unwrap();
        service.archive_task(task.id).unwrap();

        let retrieved = service.get_task(task.id).unwrap();
        assert_eq!(retrieved.status, TaskStatus::Archived);

        // Current task should be cleared
        let current_id = db.get_current_task_id().unwrap();
        assert!(current_id.is_none());
    }

    #[test]
    fn test_link_ticket_success() {
        let db = setup_db();
        let service = TaskService::new(&db);

        let task = service.create_task("Task 1", None, None, None).unwrap();
        service
            .link_ticket(task.id, "PROJ-456", "https://example.com")
            .unwrap();

        let retrieved = service.get_task(task.id).unwrap();
        assert_eq!(retrieved.ticket_id.as_deref(), Some("PROJ-456"));
    }

    #[test]
    fn test_link_ticket_duplicate() {
        let db = setup_db();
        let service = TaskService::new(&db);

        service
            .create_task("Task 1", None, Some("PROJ-123"), None)
            .unwrap();
        let task2 = service.create_task("Task 2", None, None, None).unwrap();

        let result = service.link_ticket(task2.id, "PROJ-123", "https://example.com");
        assert!(matches!(result, Err(TrackError::DuplicateTicket(_, _))));
    }

    #[test]
    fn test_resolve_task_id_by_id() {
        let db = setup_db();
        let service = TaskService::new(&db);

        let task = service.create_task("Task 1", None, None, None).unwrap();
        let resolved = service.resolve_task_id(&task.id.to_string()).unwrap();
        assert_eq!(resolved, task.id);
    }

    #[test]
    fn test_resolve_task_id_by_ticket() {
        let db = setup_db();
        let service = TaskService::new(&db);

        let task = service
            .create_task("Task 1", None, Some("PROJ-789"), None)
            .unwrap();
        let resolved = service.resolve_task_id("t:PROJ-789").unwrap();
        assert_eq!(resolved, task.id);
    }

    #[test]
    fn test_validate_ticket_format_jira() {
        assert!(TicketId::parse("PROJ-123").is_ok());
        assert!(TicketId::parse("ABC-999").is_ok());
    }

    #[test]
    fn test_validate_ticket_format_github() {
        assert!(TicketId::parse("owner/repo/123").is_ok());
    }

    #[test]
    fn test_validate_ticket_format_invalid() {
        assert!(matches!(
            TicketId::parse("invalid"),
            Err(TrackError::InvalidTicketFormat(_))
        ));
    }

    #[test]
    fn test_create_task_with_description() {
        let db = setup_db();
        let service = TaskService::new(&db);

        let task = service
            .create_task("Test Task", Some("This is a test description"), None, None)
            .unwrap();
        assert_eq!(
            task.description,
            Some("This is a test description".to_string())
        );
    }

    #[test]
    fn test_set_description() {
        let db = setup_db();
        let service = TaskService::new(&db);

        let task = service.create_task("Test Task", None, None, None).unwrap();
        assert!(task.description.is_none());

        service.set_description(task.id, "New description").unwrap();
        let updated = service.get_task(task.id).unwrap();
        assert_eq!(updated.description, Some("New description".to_string()));
    }

    #[test]
    fn test_set_description_archived_task() {
        let db = setup_db();
        let service = TaskService::new(&db);

        let task = service.create_task("Test Task", None, None, None).unwrap();
        service.archive_task(task.id).unwrap();

        let result = service.set_description(task.id, "New description");
        assert!(matches!(result, Err(TrackError::TaskArchived(_))));
    }

    #[test]
    fn test_description_persists() {
        let db = setup_db();
        let service = TaskService::new(&db);

        let task = service
            .create_task("Test Task", Some("Original description"), None, None)
            .unwrap();
        let task_id = task.id;

        // Retrieve again to ensure it persists
        let retrieved = service.get_task(task_id).unwrap();
        assert_eq!(
            retrieved.description,
            Some("Original description".to_string())
        );
    }

    #[test]
    fn test_resolve_task_id_not_one() {
        let db = setup_db();
        let service = TaskService::new(&db);

        let _ = service.create_task("Task 1", None, None, None).unwrap();
        let task2 = service.create_task("Task 2", None, None, None).unwrap();

        // Ensure ID is not 1
        assert_ne!(task2.id, TaskId::from_i64(1));

        let resolved = service.resolve_task_id(&task2.id.to_string()).unwrap();
        assert_eq!(resolved, task2.id);
    }

    #[test]
    fn test_validate_ticket_format_edge_cases() {
        assert!(matches!(
            TicketId::parse("proj-123"),
            Err(TrackError::InvalidTicketFormat(_))
        ));
        assert!(matches!(
            TicketId::parse("PROJ123"),
            Err(TrackError::InvalidTicketFormat(_))
        ));
    }

    #[test]
    fn test_set_alias() {
        let db = setup_db();
        let service = TaskService::new(&db);

        let task = service.create_task("Task 1", None, None, None).unwrap();
        service.set_alias(task.id, "my-alias", false).unwrap();

        let updated = service.get_task(task.id).unwrap();
        assert_eq!(updated.alias.as_deref(), Some("my-alias"));
    }

    #[test]
    fn test_set_alias_duplicate() {
        let db = setup_db();
        let service = TaskService::new(&db);

        let task1 = service.create_task("Task 1", None, None, None).unwrap();
        let task2 = service.create_task("Task 2", None, None, None).unwrap();

        service.set_alias(task1.id, "my-alias", false).unwrap();

        // Try to set the same alias on a different task
        let result = service.set_alias(task2.id, "my-alias", false);
        assert!(result.is_err());
    }

    #[test]
    fn test_set_alias_same_task_twice() {
        let db = setup_db();
        let service = TaskService::new(&db);

        let task = service.create_task("Task 1", None, None, None).unwrap();

        service.set_alias(task.id, "my-alias", false).unwrap();

        // Setting the same alias on the same task should succeed
        service.set_alias(task.id, "my-alias", false).unwrap();

        let updated = service.get_task(task.id).unwrap();
        assert_eq!(updated.alias.as_deref(), Some("my-alias"));
    }

    #[test]
    fn test_remove_alias() {
        let db = setup_db();
        let service = TaskService::new(&db);

        let task = service.create_task("Task 1", None, None, None).unwrap();
        service.set_alias(task.id, "my-alias", false).unwrap();
        service.remove_alias(task.id).unwrap();

        let updated = service.get_task(task.id).unwrap();
        assert!(updated.alias.is_none());
    }

    #[test]
    fn test_resolve_task_id_by_alias() {
        let db = setup_db();
        let service = TaskService::new(&db);

        let task = service.create_task("Task 1", None, None, None).unwrap();
        service.set_alias(task.id, "my-alias", false).unwrap();

        let resolved = service.resolve_task_id("my-alias").unwrap();
        assert_eq!(resolved, task.id);
    }

    #[test]
    fn test_resolve_task_id_priority() {
        let db = setup_db();
        let service = TaskService::new(&db);

        // Create tasks with different reference types
        let task1 = service.create_task("Task 1", None, None, None).unwrap();
        let task2 = service
            .create_task("Task 2", None, Some("PROJ-123"), None)
            .unwrap();
        let task3 = service.create_task("Task 3", None, None, None).unwrap();
        service.set_alias(task3.id, "my-alias", false).unwrap();

        // Test numeric ID (priority 1)
        let resolved = service.resolve_task_id(&task1.id.to_string()).unwrap();
        assert_eq!(resolved, task1.id);

        // Test ticket reference (priority 2)
        let resolved = service.resolve_task_id("t:PROJ-123").unwrap();
        assert_eq!(resolved, task2.id);

        // Test alias (priority 3)
        let resolved = service.resolve_task_id("my-alias").unwrap();
        assert_eq!(resolved, task3.id);
    }

    #[test]
    fn test_duplicate_alias() {
        let db = setup_db();
        let service = TaskService::new(&db);

        let task1 = service.create_task("Task 1", None, None, None).unwrap();
        let task2 = service.create_task("Task 2", None, None, None).unwrap();

        service.set_alias(task1.id, "my-alias", false).unwrap();
        let result = service.set_alias(task2.id, "my-alias", false);

        assert!(result.is_err());
    }

    #[test]
    fn test_set_alias_force_overwrite() {
        let db = setup_db();
        let service = TaskService::new(&db);

        let task1 = service.create_task("Task 1", None, None, None).unwrap();
        let task2 = service.create_task("Task 2", None, None, None).unwrap();

        // Set alias on task1
        service.set_alias(task1.id, "my-alias", false).unwrap();

        // Verify task1 has the alias
        let updated1 = service.get_task(task1.id).unwrap();
        assert_eq!(updated1.alias.as_deref(), Some("my-alias"));

        // Try to set the same alias on task2 with force=true
        service.set_alias(task2.id, "my-alias", true).unwrap();

        // Verify task2 now has the alias
        let updated2 = service.get_task(task2.id).unwrap();
        assert_eq!(updated2.alias.as_deref(), Some("my-alias"));

        // Verify task1 no longer has the alias
        let updated1_after = service.get_task(task1.id).unwrap();
        assert!(updated1_after.alias.is_none());
    }
}
