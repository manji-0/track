use rusqlite::Connection;
use track::db::Database;
use track::models::TodoStatus;

fn open_legacy_git_items_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        r#"
        CREATE TABLE app_state (key TEXT PRIMARY KEY, value TEXT);
        CREATE TABLE tasks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            description TEXT,
            status TEXT NOT NULL DEFAULT 'active',
            ticket_id TEXT,
            ticket_url TEXT,
            alias TEXT,
            is_today_task INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL
        );
        CREATE TABLE todos (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            task_id INTEGER NOT NULL,
            task_index INTEGER,
            content TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            worktree_requested INTEGER NOT NULL DEFAULT 0,
            requires_workspace INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL,
            completed_at TEXT
        );
        CREATE TABLE git_items (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            task_id INTEGER NOT NULL,
            path TEXT NOT NULL,
            branch TEXT NOT NULL,
            created_at TEXT NOT NULL
        );
        CREATE INDEX idx_git_items_task_id ON git_items(task_id);
        CREATE TABLE task_repos (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            task_id INTEGER NOT NULL,
            repo_path TEXT NOT NULL,
            created_at TEXT NOT NULL
        );
        CREATE TABLE repo_links (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            worktree_id INTEGER NOT NULL,
            url TEXT NOT NULL,
            kind TEXT NOT NULL,
            created_at TEXT NOT NULL
        );
        CREATE TABLE scraps (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            task_id INTEGER NOT NULL,
            content TEXT NOT NULL,
            created_at TEXT NOT NULL
        );
        CREATE TABLE links (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            task_id INTEGER NOT NULL,
            url TEXT NOT NULL,
            created_at TEXT NOT NULL
        );
        "#,
    )
    .unwrap();
    conn
}

#[test]
fn migrate_renames_git_items_to_worktrees() {
    let conn = open_legacy_git_items_db();
    track::db::migrate::migrate_schema(&conn).unwrap();

    let worktrees_exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='worktrees'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(worktrees_exists, 1);

    let git_items_exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='git_items'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(git_items_exists, 0);
}

#[test]
fn fresh_database_initializes_status_constraints() {
    let db = Database::new_in_memory().unwrap();
    let conn = db.get_connection();

    let tasks_sql: String = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type='table' AND name='tasks'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(tasks_sql.contains("CHECK"));

    conn.execute(
        "INSERT INTO tasks (name, status, created_at) VALUES ('bad', 'bogus', datetime('now'))",
        [],
    )
    .unwrap_err();

    conn.execute(
        &format!(
            "INSERT INTO todos (task_id, task_index, content, status, worktree_requested, requires_workspace, created_at)
             VALUES (1, 1, 'x', '{}', 0, 1, datetime('now'))",
            TodoStatus::PENDING
        ),
        [],
    )
    .unwrap_err();
}
