//! Database initialization and management for the track CLI.
//!
//! This module handles SQLite database creation, schema initialization, migrations,
//! and application state management. The database stores all task, TODO, link, scrap,
//! and Git repository information.

use crate::utils::Result;
use directories::ProjectDirs;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::PathBuf;

/// Revision numbers for each section, used for change detection.
///
/// Each section has a revision number that is incremented whenever
/// data in that section is modified. This allows efficient change
/// detection without complex queries.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct SectionRevs {
    /// Task metadata (description, ticket, alias) revision
    pub task: i64,
    /// TODOs section revision
    pub todos: i64,
    /// Scraps section revision
    pub scraps: i64,
    /// Links section revision
    pub links: i64,
    /// Repositories section revision
    pub repos: i64,
    /// Worktrees section revision
    pub worktrees: i64,
}

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

            CREATE TABLE IF NOT EXISTS worktrees (
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
                worktree_id INTEGER NOT NULL,
                url TEXT NOT NULL,
                kind TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (worktree_id) REFERENCES worktrees(id) ON DELETE CASCADE
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
            CREATE INDEX IF NOT EXISTS idx_worktrees_task_id ON worktrees(task_id);
            CREATE INDEX IF NOT EXISTS idx_task_repos_task_id ON task_repos(task_id);
            "#,
        )?;

        self.migrate_schema()?;

        Ok(())
    }

    fn migrate_schema(&self) -> Result<()> {
        // Migrate git_items table to worktrees (for existing databases)
        let git_items_exists: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='git_items'",
            [],
            |row| row.get(0),
        )?;

        if git_items_exists > 0 {
            // Drop old indexes before renaming table
            self.conn
                .execute("DROP INDEX IF EXISTS idx_git_items_task_id", [])?;
            self.conn
                .execute("DROP INDEX IF EXISTS idx_git_items_todo_id", [])?;

            // Rename git_items table to worktrees
            self.conn
                .execute("ALTER TABLE git_items RENAME TO worktrees", [])?;

            // Create new indexes with correct names
            self.conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_worktrees_task_id ON worktrees(task_id)",
                [],
            )?;
            self.conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_worktrees_todo_id ON worktrees(todo_id)",
                [],
            )?;
        }

        // Migrate repo_links.git_item_id to worktree_id (for existing databases)
        let git_item_id_exists: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('repo_links') WHERE name='git_item_id'",
            [],
            |row| row.get(0),
        )?;

        if git_item_id_exists > 0 {
            // SQLite doesn't support renaming columns directly in older versions
            // We need to recreate the table
            self.conn.execute_batch(
                r#"
                -- Create new repo_links table with worktree_id
                CREATE TABLE repo_links_new (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    worktree_id INTEGER NOT NULL,
                    url TEXT NOT NULL,
                    kind TEXT NOT NULL,
                    created_at TEXT NOT NULL,
                    FOREIGN KEY (worktree_id) REFERENCES worktrees(id) ON DELETE CASCADE
                );

                -- Copy data from old table
                INSERT INTO repo_links_new (id, worktree_id, url, kind, created_at)
                SELECT id, git_item_id, url, kind, created_at FROM repo_links;

                -- Drop old table
                DROP TABLE repo_links;

                -- Rename new table to repo_links
                ALTER TABLE repo_links_new RENAME TO repo_links;

                -- Recreate index
                CREATE INDEX IF NOT EXISTS idx_repo_links_worktree_id ON repo_links(worktree_id);
                "#,
            )?;
        }

        // Check for todo_id column in worktrees
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('worktrees') WHERE name='todo_id'",
            [],
            |row| row.get(0),
        )?;

        if count == 0 {
            self.conn.execute("ALTER TABLE worktrees ADD COLUMN todo_id INTEGER REFERENCES todos(id) ON DELETE SET NULL", [])?;
        }
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_worktrees_todo_id ON worktrees(todo_id)",
            [],
        )?;

        // Check for is_base column in worktrees
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('worktrees') WHERE name='is_base'",
            [],
            |row| row.get(0),
        )?;

        if count == 0 {
            self.conn.execute(
                "ALTER TABLE worktrees ADD COLUMN is_base INTEGER DEFAULT 0",
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
            self.conn
                .execute("ALTER TABLE todos ADD COLUMN completed_at TEXT", [])?;
        }

        // Check for task_index column in scraps
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('scraps') WHERE name='task_index'",
            [],
            |row| row.get(0),
        )?;

        if count == 0 {
            // Add task_index column
            self.conn
                .execute("ALTER TABLE scraps ADD COLUMN task_index INTEGER", [])?;

            // Populate task_index for existing scraps based on creation order
            self.conn.execute_batch(
                r#"
                WITH numbered_scraps AS (
                    SELECT id, task_id, 
                           ROW_NUMBER() OVER (PARTITION BY task_id ORDER BY created_at) as idx
                    FROM scraps
                )
                UPDATE scraps 
                SET task_index = (
                    SELECT idx FROM numbered_scraps WHERE numbered_scraps.id = scraps.id
                )
                "#,
            )?;

            // Create unique index on (task_id, task_index)
            self.conn.execute(
                "CREATE UNIQUE INDEX idx_scraps_task_index ON scraps(task_id, task_index)",
                [],
            )?;
        }

        // Ensure repo_links index exists (for both new and migrated databases)
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_repo_links_worktree_id ON repo_links(worktree_id)",
            [],
        )?;

        // Check for alias column in tasks
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('tasks') WHERE name='alias'",
            [],
            |row| row.get(0),
        )?;

        if count == 0 {
            // Add column without UNIQUE constraint first (SQLite limitation)
            self.conn
                .execute("ALTER TABLE tasks ADD COLUMN alias TEXT", [])?;
        }

        // Create UNIQUE index for alias column
        self.conn.execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_tasks_alias ON tasks(alias)",
            [],
        )?;

        // Check for task_index column in links
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('links') WHERE name='task_index'",
            [],
            |row| row.get(0),
        )?;

        if count == 0 {
            // Add task_index column
            self.conn
                .execute("ALTER TABLE links ADD COLUMN task_index INTEGER", [])?;

            // Populate task_index for existing links based on creation order
            self.conn.execute_batch(
                r#"
                WITH numbered_links AS (
                    SELECT id, task_id, 
                           ROW_NUMBER() OVER (PARTITION BY task_id ORDER BY created_at) as idx
                    FROM links
                )
                UPDATE links 
                SET task_index = (
                    SELECT idx FROM numbered_links WHERE numbered_links.id = links.id
                )
                "#,
            )?;

            // Create unique index on (task_id, task_index)
            self.conn.execute(
                "CREATE UNIQUE INDEX idx_links_task_index ON links(task_id, task_index)",
                [],
            )?;
        }

        // Check for task_index column in task_repos
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('task_repos') WHERE name='task_index'",
            [],
            |row| row.get(0),
        )?;

        if count == 0 {
            // Add task_index column
            self.conn
                .execute("ALTER TABLE task_repos ADD COLUMN task_index INTEGER", [])?;

            // Populate task_index for existing repos based on creation order
            self.conn.execute_batch(
                r#"
                WITH numbered_repos AS (
                    SELECT id, task_id, 
                           ROW_NUMBER() OVER (PARTITION BY task_id ORDER BY created_at) as idx
                    FROM task_repos
                )
                UPDATE task_repos 
                SET task_index = (
                    SELECT idx FROM numbered_repos WHERE numbered_repos.id = task_repos.id
                )
                "#,
            )?;

            // Create unique index on (task_id, task_index)
            self.conn.execute(
                "CREATE UNIQUE INDEX idx_task_repos_task_index ON task_repos(task_id, task_index)",
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

    /// Increments the revision number for a section and returns the new value.
    ///
    /// # Arguments
    ///
    /// * `section` - The section name (e.g., "todos", "scraps", "links", "repos", "worktrees", "task")
    pub fn increment_rev(&self, section: &str) -> Result<i64> {
        let key = format!("rev:{}", section);
        let current: i64 = self
            .get_app_state(&key)?
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let new_rev = current + 1;
        self.set_app_state(&key, &new_rev.to_string())?;
        Ok(new_rev)
    }

    /// Gets the current revision number for a section.
    ///
    /// # Arguments
    ///
    /// * `section` - The section name
    pub fn get_rev(&self, section: &str) -> Result<i64> {
        let key = format!("rev:{}", section);
        Ok(self
            .get_app_state(&key)?
            .and_then(|s| s.parse().ok())
            .unwrap_or(0))
    }

    /// Gets all section revision numbers at once.
    pub fn get_all_revs(&self) -> Result<SectionRevs> {
        Ok(SectionRevs {
            task: self.get_rev("task")?,
            todos: self.get_rev("todos")?,
            scraps: self.get_rev("scraps")?,
            links: self.get_rev("links")?,
            repos: self.get_rev("repos")?,
            worktrees: self.get_rev("worktrees")?,
        })
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
            "worktrees",
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

    #[test]
    fn test_increment_rev() {
        let db = Database::new_in_memory().unwrap();

        // Initially should be 0
        let rev = db.get_rev("todos").unwrap();
        assert_eq!(rev, 0);

        // Increment should return 1
        let new_rev = db.increment_rev("todos").unwrap();
        assert_eq!(new_rev, 1);

        // Get should return 1
        let rev = db.get_rev("todos").unwrap();
        assert_eq!(rev, 1);

        // Increment again should return 2
        let new_rev = db.increment_rev("todos").unwrap();
        assert_eq!(new_rev, 2);
    }

    #[test]
    fn test_get_rev_default() {
        let db = Database::new_in_memory().unwrap();

        // Non-existent sections should return 0
        let rev = db.get_rev("nonexistent").unwrap();
        assert_eq!(rev, 0);
    }

    #[test]
    fn test_increment_rev_different_sections() {
        let db = Database::new_in_memory().unwrap();

        // Increment different sections
        db.increment_rev("todos").unwrap();
        db.increment_rev("todos").unwrap();
        db.increment_rev("scraps").unwrap();
        db.increment_rev("links").unwrap();
        db.increment_rev("links").unwrap();
        db.increment_rev("links").unwrap();

        // Verify each section has independent rev
        assert_eq!(db.get_rev("todos").unwrap(), 2);
        assert_eq!(db.get_rev("scraps").unwrap(), 1);
        assert_eq!(db.get_rev("links").unwrap(), 3);
        assert_eq!(db.get_rev("repos").unwrap(), 0);
    }

    #[test]
    fn test_get_all_revs() {
        let db = Database::new_in_memory().unwrap();

        // Initially all should be 0
        let revs = db.get_all_revs().unwrap();
        assert_eq!(revs.task, 0);
        assert_eq!(revs.todos, 0);
        assert_eq!(revs.scraps, 0);
        assert_eq!(revs.links, 0);
        assert_eq!(revs.repos, 0);
        assert_eq!(revs.worktrees, 0);

        // Increment some sections
        db.increment_rev("task").unwrap();
        db.increment_rev("todos").unwrap();
        db.increment_rev("todos").unwrap();
        db.increment_rev("worktrees").unwrap();

        // Verify get_all_revs returns correct values
        let revs = db.get_all_revs().unwrap();
        assert_eq!(revs.task, 1);
        assert_eq!(revs.todos, 2);
        assert_eq!(revs.scraps, 0);
        assert_eq!(revs.links, 0);
        assert_eq!(revs.repos, 0);
        assert_eq!(revs.worktrees, 1);
    }

    #[test]
    fn test_section_revs_equality() {
        let db = Database::new_in_memory().unwrap();

        let revs1 = db.get_all_revs().unwrap();
        let revs2 = db.get_all_revs().unwrap();
        assert_eq!(revs1, revs2);

        db.increment_rev("todos").unwrap();
        let revs3 = db.get_all_revs().unwrap();
        assert_ne!(revs1, revs3);
    }
}
