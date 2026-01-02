//! Database initialization and management for the track CLI.
//!
//! This module handles SQLite database creation, schema initialization, migrations,
//! and application state management. The database stores all task, TODO, link, scrap,
//! and Git repository information.

use crate::utils::Result;
use directories::ProjectDirs;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::PathBuf;

/// Database connection and management.
///
/// Handles SQLite database operations including schema initialization,
/// migrations, and application state persistence.
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Creates a new database instance with the default file location.
    ///
    /// The database file is stored in the platform-specific data directory.
    /// The schema is automatically initialized if the database is new.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The data directory cannot be determined
    /// - The database file cannot be created or opened
    /// - Schema initialization fails
    pub fn new() -> Result<Self> {
        let db_path = Self::get_db_path()?;

        // Create parent directory if it doesn't exist
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(&db_path)?;
        let db = Database { conn };
        db.initialize_schema()?;
        Ok(db)
    }

    /// Creates a new in-memory database (primarily for testing).
    ///
    /// # Errors
    ///
    /// Returns an error if schema initialization fails.
    #[allow(dead_code)]
    pub fn new_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Database { conn };
        db.initialize_schema()?;
        Ok(db)
    }

    fn get_db_path() -> Result<PathBuf> {
        let proj_dirs = ProjectDirs::from("", "", "track").ok_or_else(|| {
            crate::utils::TrackError::Other("Failed to determine data directory".to_string())
        })?;

        Ok(proj_dirs.data_dir().join("track.db"))
    }

    fn initialize_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS app_state (
                key TEXT PRIMARY KEY,
                value TEXT
            );

            CREATE TABLE IF NOT EXISTS tasks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'active',
                ticket_id TEXT UNIQUE,
                ticket_url TEXT,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS todos (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                task_id INTEGER NOT NULL,
                content TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending',
                worktree_requested INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                completed_at TEXT,
                FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS links (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                task_id INTEGER NOT NULL,
                url TEXT NOT NULL,
                title TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS scraps (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                task_id INTEGER NOT NULL,
                content TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS git_items (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                task_id INTEGER NOT NULL,
                path TEXT NOT NULL,
                branch TEXT NOT NULL,
                base_repo TEXT,
                status TEXT NOT NULL DEFAULT 'active',
                created_at TEXT NOT NULL,
                todo_id INTEGER,
                is_base INTEGER DEFAULT 0,
                FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
                FOREIGN KEY (todo_id) REFERENCES todos(id) ON DELETE SET NULL
            );

            CREATE TABLE IF NOT EXISTS repo_links (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                git_item_id INTEGER NOT NULL,
                url TEXT NOT NULL,
                kind TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (git_item_id) REFERENCES git_items(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS task_repos (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                task_id INTEGER NOT NULL,
                repo_path TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
                UNIQUE(task_id, repo_path)
            );

            CREATE INDEX IF NOT EXISTS idx_todos_task_id ON todos(task_id);
            CREATE INDEX IF NOT EXISTS idx_links_task_id ON links(task_id);
            CREATE INDEX IF NOT EXISTS idx_scraps_task_id ON scraps(task_id);
            CREATE INDEX IF NOT EXISTS idx_git_items_task_id ON git_items(task_id);
            CREATE INDEX IF NOT EXISTS idx_repo_links_git_item_id ON repo_links(git_item_id);
            CREATE INDEX IF NOT EXISTS idx_task_repos_task_id ON task_repos(task_id);
            "#,
        )?;

        self.migrate_schema()?;

        Ok(())
    }

    fn migrate_schema(&self) -> Result<()> {
        // Check for todo_id column in git_items
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('git_items') WHERE name='todo_id'",
            [],
            |row| row.get(0),
        )?;

        if count == 0 {
            self.conn.execute("ALTER TABLE git_items ADD COLUMN todo_id INTEGER REFERENCES todos(id) ON DELETE SET NULL", [])?;
        }
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_git_items_todo_id ON git_items(todo_id)",
            [],
        )?;

        // Check for is_base column in git_items
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('git_items') WHERE name='is_base'",
            [],
            |row| row.get(0),
        )?;

        if count == 0 {
            self.conn.execute(
                "ALTER TABLE git_items ADD COLUMN is_base INTEGER DEFAULT 0",
                [],
            )?;
        }

        // Check for description column in tasks
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('tasks') WHERE name='description'",
            [],
            |row| row.get(0),
        )?;

        if count == 0 {
            self.conn
                .execute("ALTER TABLE tasks ADD COLUMN description TEXT", [])?;
        }

        // Check for task_index column in todos
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('todos') WHERE name='task_index'",
            [],
            |row| row.get(0),
        )?;

        if count == 0 {
            // Add task_index column
            self.conn
                .execute("ALTER TABLE todos ADD COLUMN task_index INTEGER", [])?;

            // Populate task_index for existing TODOs based on creation order
            self.conn.execute_batch(
                r#"
                WITH numbered_todos AS (
                    SELECT id, task_id, 
                           ROW_NUMBER() OVER (PARTITION BY task_id ORDER BY created_at) as idx
                    FROM todos
                )
                UPDATE todos 
                SET task_index = (
                    SELECT idx FROM numbered_todos WHERE numbered_todos.id = todos.id
                )
                "#,
            )?;

            // Create unique index on (task_id, task_index)
            self.conn.execute(
                "CREATE UNIQUE INDEX idx_todos_task_index ON todos(task_id, task_index)",
                [],
            )?;
        }

        // Check for worktree_requested column in todos
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('todos') WHERE name='worktree_requested'",
            [],
            |row| row.get(0),
        )?;

        if count == 0 {
            self.conn.execute(
                "ALTER TABLE todos ADD COLUMN worktree_requested INTEGER NOT NULL DEFAULT 0",
                [],
            )?;
        }

        // Check for base_branch column in task_repos
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('task_repos') WHERE name='base_branch'",
            [],
            |row| row.get(0),
        )?;

        if count == 0 {
            self.conn
                .execute("ALTER TABLE task_repos ADD COLUMN base_branch TEXT", [])?;
        }

        // Check for base_commit_hash column in task_repos
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('task_repos') WHERE name='base_commit_hash'",
            [],
            |row| row.get(0),
        )?;

        if count == 0 {
            self.conn.execute(
                "ALTER TABLE task_repos ADD COLUMN base_commit_hash TEXT",
                [],
            )?;
        }

        // Check for completed_at column in todos
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('todos') WHERE name='completed_at'",
            [],
            |row| row.get(0),
        )?;

        if count == 0 {
            self.conn.execute(
                "ALTER TABLE todos ADD COLUMN completed_at TEXT",
                [],
            )?;
        }

        Ok(())
    }

    /// Returns a reference to the underlying SQLite connection.
    pub fn get_connection(&self) -> &Connection {
        &self.conn
    }

    pub fn get_app_state(&self, key: &str) -> Result<Option<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT value FROM app_state WHERE key = ?1")?;
        let result = stmt.query_row(params![key], |row| row.get(0)).optional()?;
        Ok(result)
    }

    pub fn set_app_state(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO app_state (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
        Ok(())
    }

    /// Gets the ID of the current active task.
    ///
    /// # Returns
    ///
    /// `Some(task_id)` if a task is currently active, `None` otherwise.
    pub fn get_current_task_id(&self) -> Result<Option<i64>> {
        match self.get_app_state("current_task_id")? {
            Some(id_str) => Ok(Some(id_str.parse().map_err(|_| {
                crate::utils::TrackError::Other("Invalid task ID in app_state".to_string())
            })?)),
            None => Ok(None),
        }
    }

    /// Sets the current active task.
    ///
    /// # Arguments
    ///
    /// * `task_id` - The ID of the task to set as current
    pub fn set_current_task_id(&self, task_id: i64) -> Result<()> {
        self.set_app_state("current_task_id", &task_id.to_string())
    }

    /// Clears the current active task.
    pub fn clear_current_task_id(&self) -> Result<()> {
        self.conn
            .execute("DELETE FROM app_state WHERE key = 'current_task_id'", [])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_in_memory() {
        let db = Database::new_in_memory().unwrap();
        // Verify connection is valid by querying
        let result: i64 = db
            .get_connection()
            .query_row("SELECT 1", [], |row| row.get(0))
            .unwrap();
        assert_eq!(result, 1);
    }

    #[test]
    fn test_app_state_get_set() {
        let db = Database::new_in_memory().unwrap();

        // Initially should be None
        let value = db.get_app_state("test_key").unwrap();
        assert!(value.is_none());

        // Set a value
        db.set_app_state("test_key", "test_value").unwrap();

        // Get the value back
        let value = db.get_app_state("test_key").unwrap();
        assert_eq!(value, Some("test_value".to_string()));

        // Update the value
        db.set_app_state("test_key", "new_value").unwrap();
        let value = db.get_app_state("test_key").unwrap();
        assert_eq!(value, Some("new_value".to_string()));
    }

    #[test]
    fn test_current_task_id() {
        let db = Database::new_in_memory().unwrap();

        // Initially should be None
        let task_id = db.get_current_task_id().unwrap();
        assert!(task_id.is_none());

        // Set a task ID
        db.set_current_task_id(42).unwrap();

        // Get the task ID back
        let task_id = db.get_current_task_id().unwrap();
        assert_eq!(task_id, Some(42));

        // Clear the task ID
        db.clear_current_task_id().unwrap();
        let task_id = db.get_current_task_id().unwrap();
        assert!(task_id.is_none());
    }

    #[test]
    fn test_schema_initialization() {
        let db = Database::new_in_memory().unwrap();
        let conn = db.get_connection();

        // Verify all tables exist
        let tables = vec![
            "app_state",
            "tasks",
            "todos",
            "links",
            "scraps",
            "git_items",
            "repo_links",
        ];
        for table in tables {
            let result: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                    [table],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(result, 1, "Table {} should exist", table);
        }
    }
}
