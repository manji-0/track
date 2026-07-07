//! SQLite schema migrations for existing track databases.

use crate::models::{TaskStatus, TodoStatus};
use crate::utils::Result;
use rusqlite::Connection;

/// Adds CHECK constraints on task/todo status columns for existing databases.
pub(crate) fn migrate_status_check_constraints(conn: &Connection) -> Result<()> {
    let tasks_sql: String = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type='table' AND name='tasks'",
            [],
            |row| row.get(0),
        )
        .unwrap_or_default();

    if tasks_sql.contains("CHECK") {
        return Ok(());
    }

    let invalid_tasks: i64 = conn.query_row(
        &format!(
            "SELECT COUNT(*) FROM tasks WHERE status NOT IN ('{}', '{}')",
            TaskStatus::ACTIVE,
            TaskStatus::ARCHIVED
        ),
        [],
        |row| row.get(0),
    )?;
    if invalid_tasks > 0 {
        return Err(crate::utils::TrackError::MigrationBlocked {
            detail: format!("{invalid_tasks} tasks have invalid status values"),
        });
    }

    let invalid_todos: i64 = conn.query_row(
        &format!(
            "SELECT COUNT(*) FROM todos WHERE status NOT IN ('{}', '{}', '{}')",
            TodoStatus::PENDING,
            TodoStatus::DONE,
            TodoStatus::CANCELLED
        ),
        [],
        |row| row.get(0),
    )?;
    if invalid_todos > 0 {
        return Err(crate::utils::TrackError::MigrationBlocked {
            detail: format!("{invalid_todos} todos have invalid status values"),
        });
    }

    let task_check = format!(
        "CHECK (status IN ('{}', '{}'))",
        TaskStatus::ACTIVE,
        TaskStatus::ARCHIVED
    );
    let todo_check = format!(
        "CHECK (status IN ('{}', '{}', '{}'))",
        TodoStatus::PENDING,
        TodoStatus::DONE,
        TodoStatus::CANCELLED
    );

    conn.execute_batch(&format!(
            r#"
            CREATE TABLE tasks_new (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                description TEXT,
                status TEXT NOT NULL DEFAULT '{task_active}' {task_check},
                ticket_id TEXT,
                ticket_url TEXT,
                alias TEXT,
                is_today_task INTEGER DEFAULT 0,
                created_at TEXT NOT NULL
            );
            INSERT INTO tasks_new (id, name, description, status, ticket_id, ticket_url, alias, is_today_task, created_at)
            SELECT id, name, description, status, ticket_id, ticket_url, alias, is_today_task, created_at FROM tasks;
            DROP TABLE tasks;
            ALTER TABLE tasks_new RENAME TO tasks;
            CREATE UNIQUE INDEX IF NOT EXISTS idx_tasks_alias ON tasks(alias);
            CREATE INDEX IF NOT EXISTS idx_tasks_is_today_task ON tasks(is_today_task);

            CREATE TABLE todos_new (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                task_id INTEGER NOT NULL,
                task_index INTEGER,
                content TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT '{todo_pending}' {todo_check},
                worktree_requested INTEGER NOT NULL DEFAULT 0,
                requires_workspace INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL,
                completed_at TEXT,
                FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
            );
            INSERT INTO todos_new (id, task_id, task_index, content, status, worktree_requested, requires_workspace, created_at, completed_at)
            SELECT id, task_id, task_index, content, status, worktree_requested, 1, created_at, completed_at FROM todos;
            DROP TABLE todos;
            ALTER TABLE todos_new RENAME TO todos;
            CREATE INDEX IF NOT EXISTS idx_todos_task_id ON todos(task_id);
            CREATE UNIQUE INDEX IF NOT EXISTS idx_todos_task_index ON todos(task_id, task_index);
            "#,
            task_active = TaskStatus::ACTIVE,
            task_check = task_check,
            todo_pending = TodoStatus::PENDING,
            todo_check = todo_check,
        ))?;

    Ok(())
}

pub fn migrate_schema(conn: &Connection) -> Result<()> {
    // Migrate git_items table to worktrees (for existing databases)
    let git_items_exists: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='git_items'",
        [],
        |row| row.get(0),
    )?;

    if git_items_exists > 0 {
        // Drop old indexes before renaming table
        conn.execute("DROP INDEX IF EXISTS idx_git_items_task_id", [])?;
        conn.execute("DROP INDEX IF EXISTS idx_git_items_todo_id", [])?;

        // Rename git_items table to worktrees
        conn.execute("ALTER TABLE git_items RENAME TO worktrees", [])?;

        // Create new indexes with correct names
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_worktrees_task_id ON worktrees(task_id)",
            [],
        )?;
    }

    // Migrate repo_links.git_item_id to worktree_id (for existing databases)
    let git_item_id_exists: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('repo_links') WHERE name='git_item_id'",
        [],
        |row| row.get(0),
    )?;

    if git_item_id_exists > 0 {
        // SQLite doesn't support renaming columns directly in older versions
        // We need to recreate the table
        conn.execute_batch(
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
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('worktrees') WHERE name='todo_id'",
        [],
        |row| row.get(0),
    )?;

    if count == 0 {
        conn.execute("ALTER TABLE worktrees ADD COLUMN todo_id INTEGER REFERENCES todos(id) ON DELETE SET NULL", [])?;
    }
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_worktrees_todo_id ON worktrees(todo_id)",
        [],
    )?;

    // Check for is_base column in worktrees
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('worktrees') WHERE name='is_base'",
        [],
        |row| row.get(0),
    )?;

    if count == 0 {
        conn.execute(
            "ALTER TABLE worktrees ADD COLUMN is_base INTEGER DEFAULT 0",
            [],
        )?;
    }

    // Check for description column in tasks
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('tasks') WHERE name='description'",
        [],
        |row| row.get(0),
    )?;

    if count == 0 {
        conn.execute("ALTER TABLE tasks ADD COLUMN description TEXT", [])?;
    }

    // Check for task_index column in todos
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('todos') WHERE name='task_index'",
        [],
        |row| row.get(0),
    )?;

    if count == 0 {
        // Add task_index column
        conn.execute("ALTER TABLE todos ADD COLUMN task_index INTEGER", [])?;

        // Populate task_index for existing TODOs based on creation order
        conn.execute_batch(
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
        conn.execute(
            "CREATE UNIQUE INDEX idx_todos_task_index ON todos(task_id, task_index)",
            [],
        )?;
    }

    // Check for worktree_requested column in todos
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('todos') WHERE name='worktree_requested'",
        [],
        |row| row.get(0),
    )?;

    if count == 0 {
        conn.execute(
            "ALTER TABLE todos ADD COLUMN worktree_requested INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }

    // Check for requires_workspace column in todos
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('todos') WHERE name='requires_workspace'",
        [],
        |row| row.get(0),
    )?;

    if count == 0 {
        conn.execute(
            "ALTER TABLE todos ADD COLUMN requires_workspace INTEGER NOT NULL DEFAULT 1",
            [],
        )?;
    }

    // Check for base_branch column in task_repos
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('task_repos') WHERE name='base_branch'",
        [],
        |row| row.get(0),
    )?;

    if count == 0 {
        conn.execute("ALTER TABLE task_repos ADD COLUMN base_branch TEXT", [])?;
    }

    // Check for base_commit_hash column in task_repos
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('task_repos') WHERE name='base_commit_hash'",
        [],
        |row| row.get(0),
    )?;

    if count == 0 {
        conn.execute(
            "ALTER TABLE task_repos ADD COLUMN base_commit_hash TEXT",
            [],
        )?;
    }

    // Check for completed_at column in todos
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('todos') WHERE name='completed_at'",
        [],
        |row| row.get(0),
    )?;

    if count == 0 {
        conn.execute("ALTER TABLE todos ADD COLUMN completed_at TEXT", [])?;
    }

    // Check for task_index column in scraps
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('scraps') WHERE name='task_index'",
        [],
        |row| row.get(0),
    )?;

    if count == 0 {
        // Add task_index column
        conn.execute("ALTER TABLE scraps ADD COLUMN task_index INTEGER", [])?;

        // Populate task_index for existing scraps based on creation order
        conn.execute_batch(
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
        conn.execute(
            "CREATE UNIQUE INDEX idx_scraps_task_index ON scraps(task_id, task_index)",
            [],
        )?;
    }

    // Ensure repo_links index exists (for both new and migrated databases)
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_repo_links_worktree_id ON repo_links(worktree_id)",
        [],
    )?;

    // Check for alias column in tasks
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('tasks') WHERE name='alias'",
        [],
        |row| row.get(0),
    )?;

    if count == 0 {
        // Add column without UNIQUE constraint first (SQLite limitation)
        conn.execute("ALTER TABLE tasks ADD COLUMN alias TEXT", [])?;
    }

    // Create UNIQUE index for alias column
    conn.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_tasks_alias ON tasks(alias)",
        [],
    )?;

    // Check for task_index column in links
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('links') WHERE name='task_index'",
        [],
        |row| row.get(0),
    )?;

    if count == 0 {
        // Add task_index column
        conn.execute("ALTER TABLE links ADD COLUMN task_index INTEGER", [])?;

        // Populate task_index for existing links based on creation order
        conn.execute_batch(
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
        conn.execute(
            "CREATE UNIQUE INDEX idx_links_task_index ON links(task_id, task_index)",
            [],
        )?;
    }

    // Check for task_index column in task_repos
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('task_repos') WHERE name='task_index'",
        [],
        |row| row.get(0),
    )?;

    if count == 0 {
        // Add task_index column
        conn.execute("ALTER TABLE task_repos ADD COLUMN task_index INTEGER", [])?;

        // Populate task_index for existing repos based on creation order
        conn.execute_batch(
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
        conn.execute(
                "CREATE UNIQUE INDEX IF NOT EXISTS idx_task_repos_task_index ON task_repos(task_id, task_index)",
                [],
            )?;
    }

    // Check for active_todo_id column in scraps
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('scraps') WHERE name='active_todo_id'",
        [],
        |row| row.get(0),
    )?;

    if count == 0 {
        // Add active_todo_id column to track which todo was active when scrap was created
        // Active todo = the oldest pending todo at the time of scrap creation
        conn.execute("ALTER TABLE scraps ADD COLUMN active_todo_id INTEGER", [])?;

        // For existing scraps, populate active_todo_id based on the oldest pending todo
        // at the time of scrap creation. We need to find the first todo that was either:
        // 1. Still pending at scrap creation time, OR
        // 2. Completed after scrap creation time
        conn.execute_batch(
            r#"
                UPDATE scraps
                SET active_todo_id = (
                    SELECT task_index
                    FROM todos
                    WHERE todos.task_id = scraps.task_id
                      AND (
                        todos.status = 'pending'
                        OR todos.completed_at IS NULL
                        OR todos.completed_at > scraps.created_at
                      )
                    ORDER BY todos.task_index ASC
                    LIMIT 1
                )
                "#,
        )?;
    }

    // Create index on active_todo_id for efficient lookups
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_scraps_active_todo_id ON scraps(active_todo_id)",
        [],
    )?;

    // Check for is_today_task column in tasks
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('tasks') WHERE name='is_today_task'",
        [],
        |row| row.get(0),
    )?;

    if count == 0 {
        conn.execute(
            "ALTER TABLE tasks ADD COLUMN is_today_task INTEGER DEFAULT 0",
            [],
        )?;
    }

    // Create index on is_today_task for efficient lookups
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_tasks_is_today_task ON tasks(is_today_task)",
        [],
    )?;

    migrate_status_check_constraints(conn)?;

    Ok(())
}
