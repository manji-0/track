use std::fs;
use std::process::Command;
use track::db::Database;
use track::services::{TaskService, TodoService, WorktreeService};

/// Test complete_worktree_for_todo without a base worktree registered in DB
/// This tests the fallback to using base_repo when no base worktree exists
#[test]
fn test_complete_worktree_without_base_worktree() {
    let db = Database::new_in_memory().unwrap();
    let task_service = TaskService::new(&db);
    let todo_service = TodoService::new(&db);
    let worktree_service = WorktreeService::new(&db);

    let task = task_service
        .create_task("Task", None, Some("ISSUE-123"), None)
        .unwrap();
    let todo = todo_service.add_todo(task.id, "Fix bug", true).unwrap();

    // Setup git repo
    let temp_dir = tempfile::tempdir().unwrap();
    let repo_path = temp_dir.path().to_str().unwrap();

    Command::new("git")
        .args(["init", repo_path])
        .output()
        .unwrap();
    Command::new("git")
        .args(["-C", repo_path, "config", "user.email", "test@test.com"])
        .output()
        .unwrap();
    Command::new("git")
        .args(["-C", repo_path, "config", "user.name", "Test"])
        .output()
        .unwrap();
    fs::write(temp_dir.path().join("README.md"), "init").unwrap();
    Command::new("git")
        .args(["-C", repo_path, "add", "."])
        .output()
        .unwrap();
    Command::new("git")
        .args(["-C", repo_path, "commit", "-m", "init"])
        .output()
        .unwrap();

    // Create task branch in the main repo (simulating `track sync`)
    Command::new("git")
        .args(["-C", repo_path, "checkout", "-b", "task/ISSUE-123"])
        .output()
        .unwrap();

    // Create todo worktree WITHOUT creating a base worktree entry
    // This simulates the scenario where track sync checked out the task branch
    // but didn't register the main repo as a base worktree
    let todo_wt = worktree_service
        .add_worktree(
            task.id,
            repo_path,
            None,
            Some("ISSUE-123"),
            Some(todo.id),
            false, // Not a base worktree
        )
        .unwrap();

    // Note: No base worktree exists in the DB at this point
    // This simulates the issue where track sync doesn't register the main repo


    // Make a commit in todo worktree
    fs::write(
        std::path::Path::new(&todo_wt.path).join("fix.txt"),
        "bug fixed",
    )
    .unwrap();
    Command::new("git")
        .args(["-C", &todo_wt.path, "add", "."])
        .output()
        .unwrap();
    Command::new("git")
        .args(["-C", &todo_wt.path, "commit", "-m", "fix bug"])
        .output()
        .unwrap();

    // Complete the worktree - this should work even without a base worktree in DB
    // It should fall back to using the base_repo path
    let branch_name = worktree_service
        .complete_worktree_for_todo(todo.id)
        .unwrap();

    // Verify branch was returned
    assert!(branch_name.is_some());
    assert_eq!(branch_name.unwrap(), todo_wt.branch);

    // Verify worktree was removed from DB
    let worktrees = worktree_service.list_worktrees(task.id).unwrap();
    assert_eq!(worktrees.len(), 0); // All TODO worktrees removed

    // Verify merge happened (fix.txt should exist in the main repository)
    assert!(std::path::Path::new(repo_path).join("fix.txt").exists());
}
