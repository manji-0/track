use std::fs;
use std::process::Command;
use track::db::Database;
use track::services::{TaskService, TodoService, WorktreeService};

fn init_jj_repo(path: &str) {
    Command::new("jj")
        .args(["git", "init", path])
        .output()
        .unwrap();
}

fn describe_change(path: &str, message: &str) {
    Command::new("jj")
        .args(["-R", path, "describe", "-m", message])
        .output()
        .unwrap();
}

fn new_change(path: &str) {
    Command::new("jj")
        .args(["-R", path, "new"])
        .output()
        .unwrap();
}

/// Test complete_worktree_for_todo full workflow
#[test]
fn test_complete_worktree_for_todo_full_workflow() {
    let db = Database::new_in_memory().unwrap();
    let task_service = TaskService::new(&db);
    let todo_service = TodoService::new(&db);
    let worktree_service = WorktreeService::new(&db);

    let task = task_service
        .create_task("Task", None, Some("WT-100"), None)
        .unwrap();
    let todo = todo_service.add_todo(task.id, "Todo WT", true).unwrap();

    // Setup JJ repo
    let temp_dir = tempfile::tempdir().unwrap();
    let repo_path = temp_dir.path().to_str().unwrap();

    init_jj_repo(repo_path);
    fs::write(temp_dir.path().join("README.md"), "init").unwrap();
    describe_change(repo_path, "init");
    new_change(repo_path);

    // Create base worktree
    let base_wt = worktree_service
        .add_worktree(task.id, repo_path, None, Some("WT-100"), None, true)
        .unwrap();

    // Create todo worktree
    let todo_wt = worktree_service
        .add_worktree(
            task.id,
            repo_path,
            None,
            Some("WT-100"),
            Some(todo.id),
            false,
        )
        .unwrap();

    // Make a commit in todo worktree
    fs::write(
        std::path::Path::new(&todo_wt.path).join("feature.txt"),
        "new feature",
    )
    .unwrap();
    describe_change(&todo_wt.path, "add feature");
    new_change(&todo_wt.path);

    // Complete the worktree
    let branch_name = worktree_service
        .complete_worktree_for_todo(todo.id)
        .unwrap();

    // Verify branch was returned
    assert!(branch_name.is_some());
    assert_eq!(branch_name.unwrap(), todo_wt.branch);

    // Verify worktree was removed from DB
    let worktrees = worktree_service.list_worktrees(task.id).unwrap();
    assert_eq!(worktrees.len(), 1); // Only base remains
    assert!(worktrees[0].is_base);

    // Verify rebase integration happened (feature.txt should exist in base worktree)
    assert!(std::path::Path::new(&base_wt.path)
        .join("feature.txt")
        .exists());
}

/// Test complete_worktree_for_todo with uncommitted changes
#[test]
fn test_complete_worktree_with_uncommitted_changes() {
    let db = Database::new_in_memory().unwrap();
    let task_service = TaskService::new(&db);
    let todo_service = TodoService::new(&db);
    let worktree_service = WorktreeService::new(&db);

    let task = task_service
        .create_task("Task", None, Some("WT-101"), None)
        .unwrap();
    let todo = todo_service.add_todo(task.id, "Todo WT", true).unwrap();

    // Setup JJ repo
    let temp_dir = tempfile::tempdir().unwrap();
    let repo_path = temp_dir.path().to_str().unwrap();

    init_jj_repo(repo_path);
    fs::write(temp_dir.path().join("README.md"), "init").unwrap();
    describe_change(repo_path, "init");
    new_change(repo_path);

    // Create base and todo worktrees
    worktree_service
        .add_worktree(task.id, repo_path, None, Some("WT-101"), None, true)
        .unwrap();
    let todo_wt = worktree_service
        .add_worktree(
            task.id,
            repo_path,
            None,
            Some("WT-101"),
            Some(todo.id),
            false,
        )
        .unwrap();

    // Make UNCOMMITTED change
    fs::write(
        std::path::Path::new(&todo_wt.path).join("dirty.txt"),
        "uncommitted",
    )
    .unwrap();

    // Try to complete - should fail
    let result = worktree_service.complete_worktree_for_todo(todo.id);
    assert!(result.is_err());

    // Verify worktree still exists
    let worktrees = worktree_service.list_worktrees(task.id).unwrap();
    assert_eq!(worktrees.len(), 2);
}

/// Test complete_worktree_for_todo when no worktree exists
#[test]
fn test_complete_worktree_no_worktree() {
    let db = Database::new_in_memory().unwrap();
    let task_service = TaskService::new(&db);
    let todo_service = TodoService::new(&db);
    let worktree_service = WorktreeService::new(&db);

    let task = task_service.create_task("Task", None, None, None).unwrap();
    let todo = todo_service.add_todo(task.id, "Todo No WT", false).unwrap();

    // No worktree created for this TODO
    let result = worktree_service
        .complete_worktree_for_todo(todo.id)
        .unwrap();

    // Should return None
    assert!(result.is_none());
}

/// Test removing a workspace actually removes the files
#[test]
fn test_remove_git_worktree_actually_removes() {
    let db = Database::new_in_memory().unwrap();
    let task_service = TaskService::new(&db);
    let worktree_service = WorktreeService::new(&db);

    let task = task_service
        .create_task("Task", None, Some("RM-100"), None)
        .unwrap();

    // Setup JJ repo
    let temp_dir = tempfile::tempdir().unwrap();
    let repo_path = temp_dir.path().to_str().unwrap();

    init_jj_repo(repo_path);
    fs::write(temp_dir.path().join("README.md"), "init").unwrap();
    describe_change(repo_path, "init");
    new_change(repo_path);

    // Create worktree
    let wt = worktree_service
        .add_worktree(task.id, repo_path, None, Some("RM-100"), None, true)
        .unwrap();

    // Verify worktree path exists
    assert!(std::path::Path::new(&wt.path).exists());

    // Remove worktree (keep_files=false)
    worktree_service.remove_worktree(wt.id, false).unwrap();

    // Verify worktree path no longer exists
    assert!(!std::path::Path::new(&wt.path).exists());
}
