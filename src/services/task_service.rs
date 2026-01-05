use crate::db::Database;
use crate::models::{Task, TaskStatus};
use crate::utils::{Result, TrackError};
use chrono::Utc;
use rusqlite::{params, OptionalExtension};

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
            "INSERT INTO tasks (name, description, status, ticket_id, ticket_url, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![name, description, TaskStatus::Active.as_str(), ticket_id, ticket_url, now],
        )?;

        let task_id = conn.last_insert_rowid();

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
    pub fn get_task(&self, task_id: i64) -> Result<Task> {
        let conn = self.db.get_connection();
        let mut stmt = conn.prepare(
            "SELECT id, name, description, status, ticket_id, ticket_url, alias, created_at FROM tasks WHERE id = ?1"
        )?;

        let task = stmt
            .query_row(params![task_id], |row| {
                Ok(Task {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    status: row.get(3)?,
                    ticket_id: row.get(4)?,
                    ticket_url: row.get(5)?,
                    alias: row.get(6)?,
                    created_at: row.get::<_, String>(7)?.parse().unwrap(),
                })
            })
            .map_err(|_| TrackError::TaskNotFound(task_id))?;

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
            "SELECT id, name, description, status, ticket_id, ticket_url, alias, created_at FROM tasks ORDER BY created_at DESC"
        } else {
            "SELECT id, name, description, status, ticket_id, ticket_url, alias, created_at FROM tasks WHERE status = 'active' ORDER BY created_at DESC"
        };

        let mut stmt = conn.prepare(query)?;
        let tasks = stmt
            .query_map([], |row| {
                Ok(Task {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    status: row.get(3)?,
                    ticket_id: row.get(4)?,
                    ticket_url: row.get(5)?,
                    alias: row.get(6)?,
                    created_at: row.get::<_, String>(7)?.parse().unwrap(),
                })
            })?
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
    pub fn switch_task(&self, task_id: i64) -> Result<Task> {
        let task = self.get_task(task_id)?;

        if task.status == TaskStatus::Archived.as_str() {
            return Err(TrackError::TaskArchived(task_id));
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
    pub fn link_ticket(&self, task_id: i64, ticket_id: &str, url: &str) -> Result<()> {
        self.validate_ticket_format(ticket_id)?;

        // Check for duplicate ticket (excluding current task)
        if let Some(existing_id) = self.find_task_by_ticket(ticket_id)? {
            if existing_id != task_id {
                return Err(TrackError::DuplicateTicket(
                    ticket_id.to_string(),
                    existing_id,
                ));
            }
        }

        let conn = self.db.get_connection();
        conn.execute(
            "UPDATE tasks SET ticket_id = ?1, ticket_url = ?2 WHERE id = ?3",
            params![ticket_id, url, task_id],
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
    pub fn set_description(&self, task_id: i64, description: &str) -> Result<()> {
        // Validate task exists and is active
        let task = self.get_task(task_id)?;
        if task.status == TaskStatus::Archived.as_str() {
            return Err(TrackError::TaskArchived(task_id));
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
    pub fn resolve_task_id(&self, reference: &str) -> Result<i64> {
        // Priority 1: If it starts with "t:", it's a ticket reference
        if let Some(ticket_id) = reference.strip_prefix("t:") {
            return self.find_task_by_ticket(ticket_id)?.ok_or_else(|| {
                TrackError::Other(format!("No task found with ticket '{}'", ticket_id))
            });
        }

        // Priority 2: Try to parse as numeric task ID
        if let Ok(task_id) = reference.parse::<i64>() {
            return Ok(task_id);
        }

        // Priority 3: Try to find by alias
        if let Some(task_id) = self.get_task_by_alias(reference)? {
            return Ok(task_id);
        }

        Err(TrackError::Other(format!(
            "No task found with reference '{}'",
            reference
        )))
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
    pub fn set_alias(&self, task_id: i64, alias: &str, force: bool) -> Result<()> {
        self.validate_alias(alias)?;

        // Check if alias is already in use
        if let Some(existing_id) = self.get_task_by_alias(alias)? {
            if existing_id != task_id {
                if force {
                    // Remove the alias from the existing task
                    let conn = self.db.get_connection();
                    conn.execute(
                        "UPDATE tasks SET alias = NULL WHERE id = ?1",
                        params![existing_id],
                    )?;
                } else {
                    return Err(TrackError::Other(format!(
                        "Alias '{}' is already in use by task #{}. Use --force to overwrite.",
                        alias, existing_id
                    )));
                }
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
    pub fn remove_alias(&self, task_id: i64) -> Result<()> {
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
    fn get_task_by_alias(&self, alias: &str) -> Result<Option<i64>> {
        let conn = self.db.get_connection();
        let mut stmt = conn.prepare("SELECT id FROM tasks WHERE alias = ?1")?;
        let result = stmt
            .query_row(params![alias], |row| row.get(0))
            .optional()?;
        Ok(result)
    }

    /// Validates an alias format.
    ///
    /// # Arguments
    ///
    /// * `alias` - The alias to validate
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The alias is empty or too long (max 50 characters)
    /// - The alias contains invalid characters (only alphanumeric, hyphens, and underscores allowed)
    /// - The alias is a reserved word
    fn validate_alias(&self, alias: &str) -> Result<()> {
        // Check length
        if alias.is_empty() || alias.len() > 50 {
            return Err(TrackError::Other(
                "Alias must be between 1 and 50 characters".to_string(),
            ));
        }

        // Check format: only alphanumeric, hyphens, and underscores
        if !alias
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(TrackError::Other(
                "Alias can only contain alphanumeric characters, hyphens, and underscores"
                    .to_string(),
            ));
        }

        // Check for reserved words
        let reserved = vec![
            "new", "list", "current", "status", "switch", "archive", "sync", "todo", "scrap",
            "link", "repo", "desc", "ticket", "alias", "help", "webui",
        ];
        if reserved.contains(&alias.to_lowercase().as_str()) {
            return Err(TrackError::Other(format!(
                "Alias '{}' is a reserved word",
                alias
            )));
        }

        Ok(())
    }

    fn find_task_by_ticket(&self, ticket_id: &str) -> Result<Option<i64>> {
        let conn = self.db.get_connection();
        let mut stmt = conn.prepare("SELECT id FROM tasks WHERE ticket_id = ?1")?;
        let result = stmt
            .query_row(params![ticket_id], |row| row.get(0))
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
        assert_eq!(task.status, "active");
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
        assert_eq!(task.ticket_id, Some("PROJ-123".to_string()));
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

        let result = service.get_task(999);
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
        assert_eq!(retrieved.status, "archived");

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
        assert_eq!(retrieved.ticket_id, Some("PROJ-456".to_string()));
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
        let db = setup_db();
        let service = TaskService::new(&db);

        assert!(service.validate_ticket_format("PROJ-123").is_ok());
        assert!(service.validate_ticket_format("ABC-999").is_ok());
    }

    #[test]
    fn test_validate_ticket_format_github() {
        let db = setup_db();
        let service = TaskService::new(&db);

        assert!(service.validate_ticket_format("owner/repo/123").is_ok());
    }

    #[test]
    fn test_validate_ticket_format_invalid() {
        let db = setup_db();
        let service = TaskService::new(&db);

        assert!(matches!(
            service.validate_ticket_format("invalid"),
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
        assert_ne!(task2.id, 1);

        let resolved = service.resolve_task_id(&task2.id.to_string()).unwrap();
        assert_eq!(resolved, task2.id);
    }

    #[test]
    fn test_validate_ticket_format_edge_cases() {
        let db = setup_db();
        let service = TaskService::new(&db);

        // Contains dash but no uppercase (should fail)
        assert!(matches!(
            service.validate_ticket_format("proj-123"),
            Err(TrackError::InvalidTicketFormat(_))
        ));

        // Contains uppercase but no dash (should fail)
        assert!(matches!(
            service.validate_ticket_format("PROJ123"),
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
        assert_eq!(updated.alias, Some("my-alias".to_string()));
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
        assert_eq!(updated.alias, Some("my-alias".to_string()));
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
    fn test_validate_alias_valid() {
        let db = setup_db();
        let service = TaskService::new(&db);

        assert!(service.validate_alias("valid-alias").is_ok());
        assert!(service.validate_alias("valid_alias").is_ok());
        assert!(service.validate_alias("ValidAlias123").is_ok());
    }

    #[test]
    fn test_validate_alias_invalid_chars() {
        let db = setup_db();
        let service = TaskService::new(&db);

        assert!(service.validate_alias("invalid alias").is_err()); // space
        assert!(service.validate_alias("invalid@alias").is_err()); // special char
        assert!(service.validate_alias("invalid.alias").is_err()); // dot
    }

    #[test]
    fn test_validate_alias_length() {
        let db = setup_db();
        let service = TaskService::new(&db);

        assert!(service.validate_alias("").is_err()); // empty
        assert!(service.validate_alias(&"a".repeat(51)).is_err()); // too long
        assert!(service.validate_alias(&"a".repeat(50)).is_ok()); // max length
    }

    #[test]
    fn test_validate_alias_reserved_words() {
        let db = setup_db();
        let service = TaskService::new(&db);

        assert!(service.validate_alias("new").is_err());
        assert!(service.validate_alias("list").is_err());
        assert!(service.validate_alias("status").is_err());
        assert!(service.validate_alias("NEW").is_err()); // case insensitive
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
        assert_eq!(updated1.alias, Some("my-alias".to_string()));

        // Try to set the same alias on task2 with force=true
        service.set_alias(task2.id, "my-alias", true).unwrap();

        // Verify task2 now has the alias
        let updated2 = service.get_task(task2.id).unwrap();
        assert_eq!(updated2.alias, Some("my-alias".to_string()));

        // Verify task1 no longer has the alias
        let updated1_after = service.get_task(task1.id).unwrap();
        assert!(updated1_after.alias.is_none());
    }
}
