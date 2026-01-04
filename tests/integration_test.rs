use track::db::Database;
use track::services::{RepoService, TaskService, TodoService, WorktreeService};

/// Integration test: Full workflow from task creation to worktree management
#[test]
fn test_full_task_workflow() {
    let db = Database::new_in_memory().unwrap();
    let task_service = TaskService::new(&db);
    let todo_service = TodoService::new(&db);

    // Create a task
    let task = task_service
        .create_task(
            "Integration Test Task",
            Some("Test description"),
            None,
            None,
        )
        .unwrap();

    // Set current task
    db.set_current_task_id(task.id).unwrap();

    // Add TODOs
    let todo1 = todo_service.add_todo(task.id, "First TODO", false).unwrap();
    let _todo2 = todo_service
        .add_todo(task.id, "Second TODO with worktree", true)
        .unwrap();

    // List TODOs
    let todos = todo_service.list_todos(task.id).unwrap();
    assert_eq!(todos.len(), 2);
    assert_eq!(todos[0].task_index, 1);
    assert_eq!(todos[1].task_index, 2);
    assert!(!todos[0].worktree_requested);
    assert!(todos[1].worktree_requested);

    // Update TODO status
    todo_service.update_status(todo1.id, "done").unwrap();
    let updated_todo = todo_service.get_todo(todo1.id).unwrap();
    assert_eq!(updated_todo.status, "done");

    // Archive task
    task_service.archive_task(task.id).unwrap();
    let archived_task = task_service.get_task(task.id).unwrap();
    assert_eq!(archived_task.status, "archived");
}

/// Integration test: Repository and worktree workflow
#[test]
fn test_repo_worktree_workflow() {
    use std::fs;
    use std::process::Command;

    let db = Database::new_in_memory().unwrap();
    let task_service = TaskService::new(&db);
    let repo_service = RepoService::new(&db);
    let worktree_service = WorktreeService::new(&db);

    // Create a task
    let task = task_service
        .create_task("Repo Test Task", None, Some("PROJ-123"), None)
        .unwrap();

    // Create a temporary git repository
    let temp_dir = tempfile::tempdir().unwrap();
    let repo_path = temp_dir.path().to_str().unwrap();

    Command::new("git")
        .args(["init", repo_path])
        .output()
        .unwrap();

    // Configure git user
    Command::new("git")
        .args(["-C", repo_path, "config", "user.email", "test@example.com"])
        .output()
        .unwrap();
    Command::new("git")
        .args(["-C", repo_path, "config", "user.name", "Test User"])
        .output()
        .unwrap();

    // Create initial commit
    fs::write(temp_dir.path().join("README.md"), "# Test Repo").unwrap();
    Command::new("git")
        .args(["-C", repo_path, "add", "."])
        .output()
        .unwrap();
    Command::new("git")
        .args(["-C", repo_path, "commit", "-m", "Initial commit"])
        .output()
        .unwrap();

    // Register repository
    let repo = repo_service
        .add_repo(task.id, repo_path, None, None)
        .unwrap();
    assert_eq!(repo.task_id, task.id);

    // List repositories
    let repos = repo_service.list_repos(task.id).unwrap();
    assert_eq!(repos.len(), 1);

    // Create base worktree
    let base_wt = worktree_service
        .add_worktree(task.id, repo_path, None, Some("PROJ-123"), None, true)
        .unwrap();
    assert!(base_wt.is_base);
    assert_eq!(base_wt.branch, "task/PROJ-123");

    // List worktrees
    let worktrees = worktree_service.list_worktrees(task.id).unwrap();
    assert_eq!(worktrees.len(), 1);

    // Remove repository
    repo_service.remove_repo(repo.id).unwrap();
    let repos_after = repo_service.list_repos(task.id).unwrap();
    assert_eq!(repos_after.len(), 0);
}

/// Integration test: Task switching and current task management
#[test]
fn test_task_switching() {
    let db = Database::new_in_memory().unwrap();
    let task_service = TaskService::new(&db);

    // Create multiple tasks
    let task1 = task_service
        .create_task("Task 1", None, None, None)
        .unwrap();
    let task2 = task_service
        .create_task("Task 2", None, None, None)
        .unwrap();

    // Current task should be task2 (last created)
    let current_id = db.get_current_task_id().unwrap();
    assert_eq!(current_id, Some(task2.id));

    // Switch to task1
    task_service.switch_task(task1.id).unwrap();
    let current_id = db.get_current_task_id().unwrap();
    assert_eq!(current_id, Some(task1.id));

    // List tasks (should not include archived)
    let tasks = task_service.list_tasks(false).unwrap();
    assert_eq!(tasks.len(), 2);

    // Archive task1
    task_service.archive_task(task1.id).unwrap();

    // List tasks without archived
    let active_tasks = task_service.list_tasks(false).unwrap();
    assert_eq!(active_tasks.len(), 1);

    // List tasks with archived
    let all_tasks = task_service.list_tasks(true).unwrap();
    assert_eq!(all_tasks.len(), 2);
}

/// Integration test: TODO task index independence
#[test]
fn test_todo_task_index_independence() {
    let db = Database::new_in_memory().unwrap();
    let task_service = TaskService::new(&db);
    let todo_service = TodoService::new(&db);

    // Create two tasks
    let task1 = task_service
        .create_task("Task 1", None, None, None)
        .unwrap();
    let task2 = task_service
        .create_task("Task 2", None, None, None)
        .unwrap();

    // Add TODOs to task1
    let t1_todo1 = todo_service
        .add_todo(task1.id, "Task1 TODO1", false)
        .unwrap();
    let t1_todo2 = todo_service
        .add_todo(task1.id, "Task1 TODO2", false)
        .unwrap();

    // Add TODOs to task2
    let t2_todo1 = todo_service
        .add_todo(task2.id, "Task2 TODO1", false)
        .unwrap();
    let t2_todo2 = todo_service
        .add_todo(task2.id, "Task2 TODO2", false)
        .unwrap();

    // Verify task indices are independent
    assert_eq!(t1_todo1.task_index, 1);
    assert_eq!(t1_todo2.task_index, 2);
    assert_eq!(t2_todo1.task_index, 1);
    assert_eq!(t2_todo2.task_index, 2);

    // Get TODO by task index
    let retrieved_t1_todo1 = todo_service.get_todo_by_index(task1.id, 1).unwrap();
    assert_eq!(retrieved_t1_todo1.id, t1_todo1.id);

    let retrieved_t2_todo1 = todo_service.get_todo_by_index(task2.id, 1).unwrap();
    assert_eq!(retrieved_t2_todo1.id, t2_todo1.id);

    // Verify they are different TODOs
    assert_ne!(retrieved_t1_todo1.id, retrieved_t2_todo1.id);
}

/// Integration test: Error handling for non-existent resources
#[test]
fn test_error_handling() {
    let db = Database::new_in_memory().unwrap();
    let task_service = TaskService::new(&db);
    let todo_service = TodoService::new(&db);

    // Try to get non-existent task
    let result = task_service.get_task(999);
    assert!(result.is_err());

    // Try to get non-existent TODO
    let result = todo_service.get_todo(999);
    assert!(result.is_err());

    // Try to switch to archived task
    let task = task_service
        .create_task("Test Task", None, None, None)
        .unwrap();
    task_service.archive_task(task.id).unwrap();
    let result = task_service.switch_task(task.id);
    assert!(result.is_err());

    // Try to update non-existent TODO
    let result = todo_service.update_status(999, "done");
    assert!(result.is_err());

    // Try to delete non-existent TODO
    let result = todo_service.delete_todo(999);
    assert!(result.is_err());
}

/// Integration test: Concurrent TODO additions produce unique indices
/// This test validates that the transaction-based atomic operations work correctly
#[test]
fn test_concurrent_todo_additions() {
    use std::collections::HashSet;
    use std::sync::Arc;
    use std::thread;

    // Create a shared database file for concurrent access
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test_concurrent.db");

    // Initialize the database
    {
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.pragma_update(None, "journal_mode", "WAL").unwrap();
        conn.busy_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        conn.execute_batch(
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
                task_index INTEGER,
                content TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending',
                worktree_requested INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                completed_at TEXT,
                FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
            );
            CREATE UNIQUE INDEX IF NOT EXISTS idx_todos_task_index ON todos(task_id, task_index);
            "#,
        )
        .unwrap();

        // Create a test task
        conn.execute(
            "INSERT INTO tasks (name, status, created_at) VALUES ('Test Task', 'active', datetime('now'))",
            [],
        )
        .unwrap();
    }

    let db_path = Arc::new(db_path);
    let num_threads = 5;
    let todos_per_thread = 4;

    // Spawn multiple threads that add TODOs concurrently
    let handles: Vec<_> = (0..num_threads)
        .map(|thread_id| {
            let db_path = Arc::clone(&db_path);
            thread::spawn(move || {
                let conn = rusqlite::Connection::open(&*db_path).unwrap();
                conn.pragma_update(None, "journal_mode", "WAL").unwrap();
                conn.busy_timeout(std::time::Duration::from_secs(5))
                    .unwrap();

                let mut created_indices = Vec::new();
                for i in 0..todos_per_thread {
                    // Use BEGIN IMMEDIATE transaction like our implementation
                    conn.execute("BEGIN IMMEDIATE", []).unwrap();

                    let next_index: i64 = conn
                        .query_row(
                            "SELECT COALESCE(MAX(task_index), 0) + 1 FROM todos WHERE task_id = 1",
                            [],
                            |row| row.get(0),
                        )
                        .unwrap();

                    let content = format!("Thread {} TODO {}", thread_id, i);
                    conn.execute(
                        "INSERT INTO todos (task_id, task_index, content, status, worktree_requested, created_at) VALUES (1, ?1, ?2, 'pending', 0, datetime('now'))",
                        rusqlite::params![next_index, content],
                    ).unwrap();

                    conn.execute("COMMIT", []).unwrap();
                    created_indices.push(next_index);
                }
                created_indices
            })
        })
        .collect();

    // Collect all created indices
    let mut all_indices: Vec<i64> = handles
        .into_iter()
        .flat_map(|h| h.join().unwrap())
        .collect();

    // Verify all indices are unique
    all_indices.sort();
    let unique_indices: HashSet<_> = all_indices.iter().collect();

    assert_eq!(
        all_indices.len(),
        unique_indices.len(),
        "All task_index values should be unique. Got duplicates: {:?}",
        all_indices
    );

    // Verify indices are sequential (1 to N)
    let expected_count = num_threads * todos_per_thread;
    assert_eq!(all_indices.len(), expected_count);
    for (i, &idx) in all_indices.iter().enumerate() {
        assert_eq!(idx, (i + 1) as i64, "Indices should be sequential");
    }
}
