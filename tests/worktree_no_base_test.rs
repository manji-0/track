mod common;

use common::jj::JjWorkspace;
use std::fs;
use track::db::Database;
use track::services::{TaskService, TodoService, WorktreeService};

/// Test complete_worktree_for_todo without a base worktree registered in DB
/// This tests the fallback to using base_repo when no base worktree exists
#[test]
fn test_complete_worktree_without_base_worktree() {
    let Some(ws) = JjWorkspace::new() else {
        return;
    };
    let repo_path = ws.repo_path();
    ws.create_bookmark("task/ISSUE-123");

    let db = Database::new_in_memory().unwrap();
    let task_service = TaskService::new(&db);
    let todo_service = TodoService::new(&db);
    let worktree_service = WorktreeService::new(&db);

    let task = task_service
        .create_task("Task", None, Some("ISSUE-123"), None)
        .unwrap();
    let todo = todo_service.add_todo(task.id, "Fix bug", true).unwrap();

    let todo_wt = worktree_service
        .add_worktree(
            task.id,
            &repo_path.to_string_lossy(),
            None,
            Some("ISSUE-123"),
            Some(todo.id),
            false,
        )
        .unwrap();

    fs::write(
        std::path::Path::new(&todo_wt.path).join("fix.txt"),
        "bug fixed",
    )
    .unwrap();
    common::jj::describe_change(std::path::Path::new(&todo_wt.path), "fix bug");
    common::jj::new_change(std::path::Path::new(&todo_wt.path));

    let branch_name = worktree_service
        .complete_worktree_for_todo(todo.id)
        .unwrap();

    assert!(branch_name.is_some());
    assert_eq!(branch_name.unwrap(), todo_wt.branch);

    let worktrees = worktree_service.list_worktrees(task.id).unwrap();
    assert_eq!(worktrees.len(), 0);

    assert!(repo_path.join("fix.txt").exists());
}
