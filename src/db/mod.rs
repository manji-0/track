use rusqlite::{Connection, params, OptionalExtension};
use std::path::PathBuf;
use directories::ProjectDirs;
use crate::utils::Result;

pub struct Database {
    conn: Connection,
}

impl Database {
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

    fn get_db_path() -> Result<PathBuf> {
        let proj_dirs = ProjectDirs::from("", "", "track")
            .ok_or_else(|| crate::utils::TrackError::Other("Failed to determine data directory".to_string()))?;
        
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
                created_at TEXT NOT NULL,
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

            CREATE INDEX IF NOT EXISTS idx_todos_task_id ON todos(task_id);
            CREATE INDEX IF NOT EXISTS idx_links_task_id ON links(task_id);
            CREATE INDEX IF NOT EXISTS idx_scraps_task_id ON scraps(task_id);
            CREATE INDEX IF NOT EXISTS idx_git_items_task_id ON git_items(task_id);
            CREATE INDEX IF NOT EXISTS idx_repo_links_git_item_id ON repo_links(git_item_id);
            "#
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
        self.conn.execute("CREATE INDEX IF NOT EXISTS idx_git_items_todo_id ON git_items(todo_id)", [])?;

        // Check for is_base column in git_items
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('git_items') WHERE name='is_base'",
            [],
            |row| row.get(0),
        )?;

        if count == 0 {
            self.conn.execute("ALTER TABLE git_items ADD COLUMN is_base INTEGER DEFAULT 0", [])?;
        }

        Ok(())
    }

    pub fn get_connection(&self) -> &Connection {
        &self.conn
    }

    pub fn get_app_state(&self, key: &str) -> Result<Option<String>> {
        let mut stmt = self.conn.prepare("SELECT value FROM app_state WHERE key = ?1")?;
        let result = stmt.query_row(params![key], |row| row.get(0))
            .optional()?;
        Ok(result)
    }

    pub fn set_app_state(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO app_state (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn get_current_task_id(&self) -> Result<Option<i64>> {
        match self.get_app_state("current_task_id")? {
            Some(id_str) => Ok(Some(id_str.parse().map_err(|_| {
                crate::utils::TrackError::Other("Invalid task ID in app_state".to_string())
            })?)),
            None => Ok(None),
        }
    }

    pub fn set_current_task_id(&self, task_id: i64) -> Result<()> {
        self.set_app_state("current_task_id", &task_id.to_string())
    }

    pub fn clear_current_task_id(&self) -> Result<()> {
        self.conn.execute("DELETE FROM app_state WHERE key = 'current_task_id'", [])?;
        Ok(())
    }
}
