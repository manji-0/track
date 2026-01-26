use track::cli::handler::CommandHandler;
use track::cli::{Commands, LinkCommands, RepoCommands, ScrapCommands, TodoCommands};
use track::db::Database;
use track::services::{
    LinkService, RepoService, ScrapService, TaskService, TodoService, WorktreeService,
};

fn jj_available() -> bool {
    std::process::Command::new("jj")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn require_jj() -> bool {
    if !jj_available() {
        eprintln!("Skipping test: jj binary not available");
        return false;
    }
    true
}

fn init_jj_repo(path: &str) {
    let output = std::process::Command::new("jj")
        .args(["git", "init", path])
        .output()
        .expect("failed to run jj git init");
    assert!(
        output.status.success(),
        "jj git init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn describe_change(path: &str, message: &str) {
    let output = std::process::Command::new("jj")
        .args(["-R", path, "describe", "-m", message])
        .output()
        .expect("failed to run jj describe");
    assert!(
        output.status.success(),
        "jj describe failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn new_change(path: &str) {
    let output = std::process::Command::new("jj")
        .args(["-R", path, "new"])
        .output()
        .expect("failed to run jj new");
    assert!(
        output.status.success(),
        "jj new failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn create_bookmark(path: &str, name: &str) {
    let output = std::process::Command::new("jj")
        .args(["-R", path, "bookmark", "create", name, "-r", "@"])
        .output()
        .expect("failed to run jj bookmark create");
    assert!(
        output.status.success(),
        "jj bookmark create failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_handle_new_creates_and_switches_task() {
    let db = Database::new_in_memory().unwrap();
    let handler = CommandHandler::from_db(db);

    let cmd = Commands::New {
        name: "New Task".to_string(),
        description: Some("Desc".to_string()),
        ticket: None,
        ticket_url: None,
        template: None,
    };

    handler.handle(cmd).unwrap();

    let db = handler.get_db();
    let task_service = TaskService::new(db);
    let current_id = db
        .get_current_task_id()
        .unwrap()
        .expect("Should have active task");

    let task = task_service.get_task(current_id).unwrap();
    assert_eq!(task.name, "New Task");
    assert_eq!(task.description.as_deref(), Some("Desc"));
}

#[test]
fn test_handle_switch_changes_task() {
    let db = Database::new_in_memory().unwrap();
    let handler = CommandHandler::from_db(db);
    let db = handler.get_db();
    let task_service = TaskService::new(db);

    let t1 = task_service.create_task("T1", None, None, None).unwrap();
    let t2 = task_service.create_task("T2", None, None, None).unwrap();

    assert_eq!(db.get_current_task_id().unwrap(), Some(t2.id));

    let cmd = Commands::Switch {
        task_ref: t1.id.to_string(),
    };
    handler.handle(cmd).unwrap();

    assert_eq!(db.get_current_task_id().unwrap(), Some(t1.id));
}

#[test]
fn test_handle_todo_add_and_update() {
    let db = Database::new_in_memory().unwrap();
    let handler = CommandHandler::from_db(db);
    let db = handler.get_db();
    let task_service = TaskService::new(db);
    let todo_service = TodoService::new(db);

    let task = task_service.create_task("Task", None, None, None).unwrap();

    let cmd = Commands::Todo(TodoCommands::Add {
        text: "My Todo".to_string(),
        worktree: false,
    });
    handler.handle(cmd).unwrap();

    let todos = todo_service.list_todos(task.id).unwrap();
    assert_eq!(todos.len(), 1);
    assert_eq!(todos[0].content, "My Todo");
    assert_eq!(todos[0].status, "pending");

    let cmd = Commands::Todo(TodoCommands::Update {
        id: 1,
        status: "done".to_string(),
    });
    handler.handle(cmd).unwrap();

    let todo = todo_service.get_todo(todos[0].id).unwrap();
    assert_eq!(todo.status, "done");
}

#[test]
fn test_handle_link_add() {
    let db = Database::new_in_memory().unwrap();
    let handler = CommandHandler::from_db(db);
    let db = handler.get_db();
    let task_service = TaskService::new(db);
    let link_service = LinkService::new(db);

    let task = task_service.create_task("Task", None, None, None).unwrap();

    let cmd = Commands::Link(LinkCommands::Add {
        url: "http://example.com".to_string(),
        title: Some("Example".to_string()),
    });
    handler.handle(cmd).unwrap();

    let links = link_service.list_links(task.id).unwrap();
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].url, "http://example.com");
}

#[test]
fn test_handle_scrap_add() {
    let db = Database::new_in_memory().unwrap();
    let handler = CommandHandler::from_db(db);
    let db = handler.get_db();
    let task_service = TaskService::new(db);
    let scrap_service = ScrapService::new(db);

    let task = task_service.create_task("Task", None, None, None).unwrap();

    let cmd = Commands::Scrap(ScrapCommands::Add {
        content: "My Note".to_string(),
    });
    handler.handle(cmd).unwrap();

    let scraps = scrap_service.list_scraps(task.id).unwrap();
    assert_eq!(scraps.len(), 1);
    assert_eq!(scraps[0].content, "My Note");
}

#[test]
fn test_handle_repo_add_remove() {
    if !require_jj() {
        return;
    }
    let db = Database::new_in_memory().unwrap();
    let handler = CommandHandler::from_db(db);
    let db = handler.get_db();
    let task_service = TaskService::new(db);
    let repo_service = RepoService::new(db);

    let task = task_service.create_task("Task", None, None, None).unwrap();

    let temp_dir = tempfile::tempdir().unwrap();
    let repo_path = temp_dir.path().to_str().unwrap().to_string();
    init_jj_repo(&repo_path);

    // Add initial change so JJ has a base revision
    std::fs::write(std::path::Path::new(&repo_path).join("README.md"), "init").unwrap();
    describe_change(&repo_path, "init");
    new_change(&repo_path);

    let cmd = Commands::Repo(RepoCommands::Add {
        path: Some(repo_path.clone()),
        base: None,
    });
    handler.handle(cmd).unwrap();

    let repos = repo_service.list_repos(task.id).unwrap();
    assert_eq!(repos.len(), 1);
    assert_eq!(repos[0].repo_path, repo_path);

    let cmd = Commands::Repo(RepoCommands::Remove { id: repos[0].id });
    handler.handle(cmd).unwrap();

    let repos = repo_service.list_repos(task.id).unwrap();
    assert_eq!(repos.len(), 0);
}

#[test]
fn test_todo_delete_force() {
    let db = Database::new_in_memory().unwrap();
    let handler = CommandHandler::from_db(db);
    let db = handler.get_db();
    let task_service = TaskService::new(db);
    let todo_service = TodoService::new(db);

    let task = task_service.create_task("Task", None, None, None).unwrap();
    let _todo = todo_service.add_todo(task.id, "To Delete", false).unwrap();

    // Test delete with force=true
    // Should NOT prompt. So valid even with empty stdin.
    let cmd = Commands::Todo(TodoCommands::Delete { id: 1, force: true });

    handler.handle(cmd).unwrap();

    let todos = todo_service.list_todos(task.id).unwrap();
    assert_eq!(todos.len(), 0);
}

#[test]
fn test_todo_workspace_requires_current_repo() {
    if !require_jj() {
        return;
    }
    let db = Database::new_in_memory().unwrap();
    let handler = CommandHandler::from_db(db);
    let db = handler.get_db();
    let task_service = TaskService::new(db);
    let todo_service = TodoService::new(db);
    let repo_service = RepoService::new(db);

    let task = task_service.create_task("Task", None, None, None).unwrap();
    let _todo = todo_service.add_todo(task.id, "Worktree", true).unwrap();

    let temp_dir = tempfile::tempdir().unwrap();
    let repo_path = temp_dir.path().join("repo");
    std::fs::create_dir_all(&repo_path).unwrap();
    init_jj_repo(repo_path.to_str().unwrap());
    std::fs::write(repo_path.join("README.md"), "init").unwrap();
    describe_change(repo_path.to_str().unwrap(), "init");
    new_change(repo_path.to_str().unwrap());

    repo_service
        .add_repo(task.id, repo_path.to_str().unwrap(), None, None)
        .unwrap();

    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(temp_dir.path()).unwrap();

    let cmd = Commands::Todo(TodoCommands::Workspace {
        id: 1,
        recreate: false,
        force: false,
        all: false,
    });
    let result = handler.handle(cmd);

    std::env::set_current_dir(original_dir).unwrap();

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Current directory is not a registered repo"));
}

#[test]
fn test_todo_workspace_accepts_subdir_repo() {
    if !require_jj() {
        return;
    }
    let db = Database::new_in_memory().unwrap();
    let handler = CommandHandler::from_db(db);
    let db = handler.get_db();
    let task_service = TaskService::new(db);
    let todo_service = TodoService::new(db);
    let repo_service = RepoService::new(db);
    let worktree_service = WorktreeService::new(db);

    let task = task_service.create_task("Task", None, None, None).unwrap();
    let _todo = todo_service.add_todo(task.id, "Worktree", false).unwrap();

    let temp_dir = tempfile::tempdir().unwrap();
    let repo_path = temp_dir.path().join("repo");
    std::fs::create_dir_all(&repo_path).unwrap();
    init_jj_repo(repo_path.to_str().unwrap());
    std::fs::write(repo_path.join("README.md"), "init").unwrap();
    describe_change(repo_path.to_str().unwrap(), "init");
    new_change(repo_path.to_str().unwrap());
    create_bookmark(repo_path.to_str().unwrap(), "task/task-1");

    repo_service
        .add_repo(task.id, repo_path.to_str().unwrap(), None, None)
        .unwrap();

    let subdir = repo_path.join("nested");
    std::fs::create_dir_all(&subdir).unwrap();
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&subdir).unwrap();

    let cmd = Commands::Todo(TodoCommands::Workspace {
        id: 1,
        recreate: false,
        force: false,
        all: false,
    });
    handler.handle(cmd).unwrap();

    std::env::set_current_dir(original_dir).unwrap();

    let worktrees = worktree_service.list_worktrees(task.id).unwrap();
    assert_eq!(worktrees.len(), 1);
}

#[test]
fn test_list_repo_links_manual() {
    // Tests WorktreeService::list_repo_links by manually inserting data
    use chrono::Utc;
    use rusqlite::params;
    use track::services::WorktreeService;

    let db = Database::new_in_memory().unwrap();
    let worktree_service = WorktreeService::new(&db);

    // Insert dummy worktree
    let conn = db.get_connection();
    conn.execute(
        "INSERT INTO tasks (name, status, created_at) VALUES ('T', 'active', 'now')",
        [],
    )
    .unwrap();
    let task_id = conn.last_insert_rowid();

    conn.execute(
        "INSERT INTO worktrees (task_id, path, branch, status, created_at) VALUES (?1, 'p', 'b', 'active', 'now')",
        params![task_id]
    ).unwrap();
    let worktree_id = conn.last_insert_rowid();

    // Manually insert repo_link
    conn.execute(
        "INSERT INTO repo_links (worktree_id, url, kind, created_at) VALUES (?1, 'http://url', 'github', ?2)",
        params![worktree_id, Utc::now().to_rfc3339()]
    ).unwrap();

    let links = worktree_service.list_repo_links(worktree_id).unwrap();
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].url, "http://url");
}

#[test]
fn test_handle_archive_clean_worktree() {
    if !require_jj() {
        return;
    }
    let db = Database::new_in_memory().unwrap();
    let handler = CommandHandler::from_db(db);
    let db = handler.get_db();
    let task_service = TaskService::new(db);
    let worktree_service = WorktreeService::new(db);

    let task = task_service
        .create_task("Task", None, Some("PROJ-999"), None)
        .unwrap();

    // Create worktree
    let temp_dir = tempfile::tempdir().unwrap();
    let repo_path = temp_dir.path().to_str().unwrap().to_string();

    init_jj_repo(&repo_path);
    std::fs::write(std::path::Path::new(&repo_path).join("README.md"), "init").unwrap();
    describe_change(&repo_path, "init");
    new_change(&repo_path);

    worktree_service
        .add_worktree(task.id, &repo_path, None, Some("PROJ-999"), None, true)
        .unwrap();

    // Worktree is clean.
    // Call archive.
    // If mutants (invert clean check) trigger, it prompts. If we pass EOF, it cancels. Task remains active.
    // If valid: it proceeds (no prompt). Task becomes archived.

    let cmd = Commands::Archive {
        task_ref: Some(task.id.to_string()),
    };

    // Note: This relies on stdin being empty in test env.
    handler.handle(cmd).unwrap();

    let t = task_service.get_task(task.id).unwrap();
    assert_eq!(t.status, "archived");

    // Verify worktree removed
    let wts = worktree_service.list_worktrees(task.id).unwrap();
    assert!(wts.is_empty());
}

#[test]
fn test_handle_sync() {
    if !require_jj() {
        return;
    }
    let db = Database::new_in_memory().unwrap();
    let handler = CommandHandler::from_db(db);
    let db = handler.get_db();
    let task_service = TaskService::new(db);
    let repo_service = RepoService::new(db);

    let task = task_service
        .create_task("Task Sync", None, Some("SYNC-123"), None)
        .unwrap();

    // Setup JJ repo
    let temp_dir = tempfile::tempdir().unwrap();
    let repo_path = temp_dir.path().to_str().unwrap().to_string();
    init_jj_repo(&repo_path);
    std::fs::write(std::path::Path::new(&repo_path).join("README.md"), "init").unwrap();
    describe_change(&repo_path, "init");
    new_change(&repo_path);

    // Register repo
    repo_service
        .add_repo(task.id, &repo_path, None, None)
        .unwrap();

    // Add TODO with worktree
    let todo_service = TodoService::new(db);
    todo_service.add_todo(task.id, "Todo WT", true).unwrap();

    // Call Sync
    let cmd = Commands::Sync;
    handler.handle(cmd).unwrap();

    // Verify bookmarks created
    let output = std::process::Command::new("jj")
        .args(["-R", &repo_path, "bookmark", "list", "task/SYNC-123"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("task/SYNC-123"));

    let todo_output = std::process::Command::new("jj")
        .args(["-R", &repo_path, "bookmark", "list", "SYNC-123-todo-1"])
        .output()
        .unwrap();
    let todo_stdout = String::from_utf8_lossy(&todo_output.stdout);
    assert!(todo_stdout.contains("SYNC-123-todo-1"));

    // Verify worktrees created in DB
    let worktree_service = WorktreeService::new(db);
    let worktrees = worktree_service.list_worktrees(task.id).unwrap();
    assert_eq!(worktrees.len(), 1); // Only the TODO worktree, specifically?
                                    // Wait, sync creates base branch (task/SYNC-123) but does it create base WORKTREE?
                                    // "Cycles through repos... creates task branch... checks out task branch."
                                    // It creates branch, checks it out (which updates HEAD of repo_path).
                                    // Then "for todo in todos... create worktree".
                                    // It does NOT auto-create a worktree for the task base unless requested?
                                    // Looking at sync code: `worktree_service.add_worktree` is only called inside todo loop.

    // So 1 worktree expected (from todo).
    assert_eq!(worktrees.len(), 1);
    assert!(worktrees[0].branch.contains("SYNC-123-todo-1"));
}

#[test]
fn test_handle_sync_dirty_repo() {
    if !require_jj() {
        return;
    }
    let db = Database::new_in_memory().unwrap();
    let handler = CommandHandler::from_db(db);
    let db = handler.get_db();
    let task_service = TaskService::new(db);
    let repo_service = RepoService::new(db);

    let task = task_service
        .create_task("Task", None, Some("SYNC-DIRTY"), None)
        .unwrap();

    let temp_dir = tempfile::tempdir().unwrap();
    let repo_path = temp_dir.path().to_str().unwrap().to_string();
    init_jj_repo(&repo_path);
    std::fs::write(std::path::Path::new(&repo_path).join("README.md"), "init").unwrap();
    describe_change(&repo_path, "init");
    new_change(&repo_path);

    repo_service
        .add_repo(task.id, &repo_path, None, None)
        .unwrap();

    std::fs::write(std::path::Path::new(&repo_path).join("README.md"), "dirty").unwrap();

    let cmd = Commands::Sync;
    let result = handler.handle(cmd);

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("pending changes"));
}

#[test]
fn test_handle_sync_repo_not_found() {
    if !require_jj() {
        return;
    }
    let db = Database::new_in_memory().unwrap();
    let handler = CommandHandler::from_db(db);
    let db = handler.get_db();
    let task_service = TaskService::new(db);
    let repo_service = RepoService::new(db);

    let task = task_service
        .create_task("Task", None, Some("SYNC-404"), None)
        .unwrap();

    // Register non-existent repo
    // Note: RepoService validates JJ repos, so we can't directly add invalid ones
    // Instead, we'll add a valid one and then delete the directory

    let temp_dir = tempfile::tempdir().unwrap();
    let repo_path = temp_dir.path().to_str().unwrap().to_string();
    init_jj_repo(&repo_path);

    repo_service
        .add_repo(task.id, &repo_path, None, None)
        .unwrap();

    // Delete the repo directory
    drop(temp_dir); // This removes the directory

    // Sync should handle this gracefully (skip non-existent repos)
    let cmd = Commands::Sync;
    let result = handler.handle(cmd);

    // Should succeed (just skip the missing repo)
    assert!(result.is_ok());
}

#[test]
fn test_handle_sync_branch_already_exists() {
    if !require_jj() {
        return;
    }
    let db = Database::new_in_memory().unwrap();
    let handler = CommandHandler::from_db(db);
    let db = handler.get_db();
    let task_service = TaskService::new(db);
    let repo_service = RepoService::new(db);

    let task = task_service
        .create_task("Task", None, Some("SYNC-200"), None)
        .unwrap();

    // Setup JJ repo
    let temp_dir = tempfile::tempdir().unwrap();
    let repo_path = temp_dir.path().to_str().unwrap().to_string();
    init_jj_repo(&repo_path);
    std::fs::write(std::path::Path::new(&repo_path).join("README.md"), "init").unwrap();
    describe_change(&repo_path, "init");
    new_change(&repo_path);

    // Pre-create the task bookmark
    create_bookmark(&repo_path, "task/SYNC-200");

    repo_service
        .add_repo(task.id, &repo_path, None, None)
        .unwrap();

    // Call Sync - should detect existing branch and checkout
    let cmd = Commands::Sync;
    handler.handle(cmd).unwrap();

    // Verify task bookmark exists
    let output = std::process::Command::new("jj")
        .args(["-R", &repo_path, "bookmark", "list", "task/SYNC-200"])
        .output()
        .unwrap();
    let current_bookmarks = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert!(current_bookmarks.contains("task/SYNC-200"));
}

#[test]
fn test_handle_sync_worktree_already_exists() {
    if !require_jj() {
        return;
    }
    let db = Database::new_in_memory().unwrap();
    let handler = CommandHandler::from_db(db);
    let db = handler.get_db();
    let task_service = TaskService::new(db);
    let repo_service = RepoService::new(db);
    let todo_service = TodoService::new(db);
    let worktree_service = WorktreeService::new(db);

    let task = task_service
        .create_task("Task", None, Some("SYNC-300"), None)
        .unwrap();

    // Setup JJ repo
    let temp_dir = tempfile::tempdir().unwrap();
    let repo_path = temp_dir.path().to_str().unwrap().to_string();
    init_jj_repo(&repo_path);
    std::fs::write(std::path::Path::new(&repo_path).join("README.md"), "init").unwrap();
    describe_change(&repo_path, "init");
    new_change(&repo_path);
    create_bookmark(&repo_path, "task/SYNC-300");

    repo_service
        .add_repo(task.id, &repo_path, None, None)
        .unwrap();

    // Add TODO with worktree request
    let todo = todo_service.add_todo(task.id, "Todo", true).unwrap();

    // Pre-create the worktree manually
    worktree_service
        .add_worktree(
            task.id,
            &repo_path,
            None,
            Some("SYNC-300"),
            Some(todo.id),
            false,
        )
        .unwrap();

    // Call Sync - should detect existing worktree and NOT create duplicate
    let cmd = Commands::Sync;
    handler.handle(cmd).unwrap();

    // Verify only 1 worktree exists (not duplicated)
    let worktrees = worktree_service.list_worktrees(task.id).unwrap();
    assert_eq!(worktrees.len(), 1);
}

#[test]
fn test_handle_sync_skip_done_todos() {
    if !require_jj() {
        return;
    }
    let db = Database::new_in_memory().unwrap();
    let handler = CommandHandler::from_db(db);
    let db = handler.get_db();
    let task_service = TaskService::new(db);
    let repo_service = RepoService::new(db);
    let todo_service = TodoService::new(db);
    let worktree_service = WorktreeService::new(db);

    let task = task_service
        .create_task("Task", None, Some("SYNC-400"), None)
        .unwrap();

    // Setup JJ repo
    let temp_dir = tempfile::tempdir().unwrap();
    let repo_path = temp_dir.path().to_str().unwrap().to_string();
    init_jj_repo(&repo_path);
    std::fs::write(std::path::Path::new(&repo_path).join("README.md"), "init").unwrap();
    describe_change(&repo_path, "init");
    new_change(&repo_path);

    repo_service
        .add_repo(task.id, &repo_path, None, None)
        .unwrap();

    // Add TODO with worktree request but mark as done
    let todo = todo_service.add_todo(task.id, "Done Todo", true).unwrap();
    todo_service.update_status(todo.id, "done").unwrap();

    // Call Sync - should NOT create worktree for done TODO
    let cmd = Commands::Sync;
    handler.handle(cmd).unwrap();

    // Verify no worktrees created
    let worktrees = worktree_service.list_worktrees(task.id).unwrap();
    assert_eq!(worktrees.len(), 0);
}

#[test]
fn test_handle_sync_failed_branch_create() {
    if !require_jj() {
        return;
    }
    // Test scenario where bookmark creation fails
    let db = Database::new_in_memory().unwrap();
    let handler = CommandHandler::from_db(db);
    let db = handler.get_db();
    let task_service = TaskService::new(db);
    let repo_service = RepoService::new(db);

    let task = task_service
        .create_task("Task", None, Some("SYNC-500"), None)
        .unwrap();

    // Use an invalid repo path to cause JJ failures
    let temp_dir = tempfile::tempdir().unwrap();
    let repo_path = temp_dir.path().to_str().unwrap().to_string();
    init_jj_repo(&repo_path);
    std::fs::write(std::path::Path::new(&repo_path).join("README.md"), "init").unwrap();
    describe_change(&repo_path, "init");
    new_change(&repo_path);

    repo_service
        .add_repo(task.id, &repo_path, None, None)
        .unwrap();

    // Make the .jj directory read-only to cause failures
    let jj_dir = std::path::Path::new(&repo_path).join(".jj");
    let mut perms = std::fs::metadata(&jj_dir).unwrap().permissions();
    perms.set_readonly(true);
    std::fs::set_permissions(&jj_dir, perms).ok(); // May fail on some systems

    // Sync should handle failures gracefully (they cause continue, not panic)
    let cmd = Commands::Sync;
    let result = handler.handle(cmd);

    // Should succeed even if JJ operations failed
    assert!(result.is_ok());
}

#[test]
fn test_worktree_complete_with_base_only() {
    use track::services::{TodoService, WorktreeService};

    if !require_jj() {
        return;
    }

    let db = Database::new_in_memory().unwrap();
    let task_service = TaskService::new(&db);
    let todo_service = TodoService::new(&db);
    let worktree_service = WorktreeService::new(&db);

    let task = task_service
        .create_task("Task", None, Some("WTB-100"), None)
        .unwrap();
    let todo = todo_service.add_todo(task.id, "Test TODO", true).unwrap();

    let temp_dir = tempfile::tempdir().unwrap();
    let repo_path = temp_dir.path().to_str().unwrap().to_string();
    init_jj_repo(&repo_path);
    std::fs::write(std::path::Path::new(&repo_path).join("README.md"), "init").unwrap();
    describe_change(&repo_path, "init");
    new_change(&repo_path);

    // Create only base worktree (is_base=true), no TODO-specific worktree
    let _base_wt = worktree_service
        .add_worktree(task.id, &repo_path, None, Some("WTB-100"), None, true)
        .unwrap();

    // Try to complete worktree for TODO - should return None because no TODO-specific worktree exists
    // This exercises get_worktree_by_todo internally, which checks is_base != 0
    let result = worktree_service
        .complete_worktree_for_todo(todo.id)
        .unwrap();
    assert!(result.is_none());
}

#[test]
fn test_handle_sync_multiple_todos_different_worktrees() {
    if !require_jj() {
        return;
    }
    let db = Database::new_in_memory().unwrap();
    let handler = CommandHandler::from_db(db);
    let db = handler.get_db();
    let task_service = TaskService::new(db);
    let repo_service = RepoService::new(db);
    let todo_service = TodoService::new(db);
    let worktree_service = WorktreeService::new(db);

    let task = task_service
        .create_task("Task", None, Some("SYNC-600"), None)
        .unwrap();

    let temp_dir = tempfile::tempdir().unwrap();
    let repo_path = temp_dir.path().to_str().unwrap().to_string();
    init_jj_repo(&repo_path);
    std::fs::write(std::path::Path::new(&repo_path).join("README.md"), "init").unwrap();
    describe_change(&repo_path, "init");
    new_change(&repo_path);
    create_bookmark(&repo_path, "task/SYNC-600");

    repo_service
        .add_repo(task.id, &repo_path, None, None)
        .unwrap();

    // Create two TODOs with worktree requests
    let todo1 = todo_service.add_todo(task.id, "Todo 1", true).unwrap();
    let todo2 = todo_service.add_todo(task.id, "Todo 2", true).unwrap();

    // Pre-create worktree for todo1
    worktree_service
        .add_worktree(
            task.id,
            &repo_path,
            None,
            Some("SYNC-600"),
            Some(todo1.id),
            false,
        )
        .unwrap();

    // Call Sync - should create worktree for todo2, but NOT todo1 (exists check)
    let cmd = Commands::Sync;
    handler.handle(cmd).unwrap();

    // Verify both worktrees exist with correct todo_id linkage
    let worktrees = worktree_service.list_worktrees(task.id).unwrap();
    assert_eq!(worktrees.len(), 2);

    // Verify TODO IDs match (this tests line 611: wt.todo_id == Some(todo.id))
    let wt1 = worktrees
        .iter()
        .find(|w| w.todo_id == Some(todo1.id))
        .expect("Todo1 worktree not found");
    let wt2 = worktrees
        .iter()
        .find(|w| w.todo_id == Some(todo2.id))
        .expect("Todo2 worktree not found");

    assert_eq!(wt1.todo_id, Some(todo1.id));
    assert_eq!(wt2.todo_id, Some(todo2.id));
}

#[test]
fn test_handle_archive_default_current_task() {
    let db = Database::new_in_memory().unwrap();
    let handler = CommandHandler::from_db(db);
    let db = handler.get_db();
    let task_service = TaskService::new(db);

    let t1 = task_service
        .create_task("Task To Archive", None, None, None)
        .unwrap();

    // Verify it is current task
    assert_eq!(db.get_current_task_id().unwrap(), Some(t1.id));

    // Archive without specifying task_ref (None)
    let cmd = Commands::Archive { task_ref: None };
    handler.handle(cmd).unwrap();

    let t = task_service.get_task(t1.id).unwrap();
    assert_eq!(t.status, "archived");
}

#[test]
fn test_handle_status_explicit_id() {
    let db = Database::new_in_memory().unwrap();
    let handler = CommandHandler::from_db(db);
    let db = handler.get_db();
    let task_service = TaskService::new(db);

    let t1 = task_service
        .create_task("Task 1", None, None, None)
        .unwrap();
    let _t2 = task_service
        .create_task("Task 2", None, None, None)
        .unwrap();

    // Call status for t1 while t2 is active
    let cmd = Commands::Status {
        id: Some(t1.id.to_string()),
        json: false,
        all: false,
    };

    // Should succeed
    handler.handle(cmd).unwrap();
}
