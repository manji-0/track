mod common;

use common::jj::{self, JjWorkspace};
use std::fs;
use track::db::Database;
use track::services::{TaskService, TodoService, WorktreeService};

/// Test complete_worktree_for_todo full workflow
#[test]
fn test_complete_worktree_for_todo_full_workflow() {
    let Some(ws) = JjWorkspace::new() else {
        return;
    };
    let repo_path = ws.repo_path();

    let db = Database::new_in_memory().unwrap();
    let task_service = TaskService::new(&db);
    let todo_service = TodoService::new(&db);
    let worktree_service = WorktreeService::new(&db);

    let task = task_service
        .create_task("Task", None, Some("WT-100"), None)
        .unwrap();
    let todo = todo_service.add_todo(task.id, "Todo WT", true).unwrap();

    let base_wt = worktree_service
        .add_worktree(
            task.id,
            &repo_path.to_string_lossy(),
            None,
            Some("WT-100"),
            None,
            true,
        )
        .unwrap();

    let todo_wt = worktree_service
        .add_worktree(
            task.id,
            &repo_path.to_string_lossy(),
            None,
            Some("WT-100"),
            Some(todo.id),
            false,
        )
        .unwrap();

    fs::write(
        std::path::Path::new(&todo_wt.path).join("feature.txt"),
        "new feature",
    )
    .unwrap();
    jj::describe_change(std::path::Path::new(&todo_wt.path), "add feature");
    jj::new_change(std::path::Path::new(&todo_wt.path));

    let branch_name = worktree_service
        .complete_worktree_for_todo(todo.id)
        .unwrap();

    assert!(branch_name.is_some());
    assert_eq!(branch_name.unwrap(), todo_wt.branch);

    let worktrees = worktree_service.list_worktrees(task.id).unwrap();
    assert_eq!(worktrees.len(), 1);
    assert!(worktrees[0].is_base);

    assert!(std::path::Path::new(&base_wt.path)
        .join("feature.txt")
        .exists());
}

/// Test complete_worktree_for_todo with uncommitted changes
#[test]
fn test_complete_worktree_with_uncommitted_changes() {
    let Some(ws) = JjWorkspace::new() else {
        return;
    };
    let repo_path = ws.repo_path();

    let db = Database::new_in_memory().unwrap();
    let task_service = TaskService::new(&db);
    let todo_service = TodoService::new(&db);
    let worktree_service = WorktreeService::new(&db);

    let task = task_service
        .create_task("Task", None, Some("WT-101"), None)
        .unwrap();
    let todo = todo_service.add_todo(task.id, "Todo WT", true).unwrap();

    worktree_service
        .add_worktree(
            task.id,
            &repo_path.to_string_lossy(),
            None,
            Some("WT-101"),
            None,
            true,
        )
        .unwrap();
    let todo_wt = worktree_service
        .add_worktree(
            task.id,
            &repo_path.to_string_lossy(),
            None,
            Some("WT-101"),
            Some(todo.id),
            false,
        )
        .unwrap();

    fs::write(
        std::path::Path::new(&todo_wt.path).join("dirty.txt"),
        "uncommitted",
    )
    .unwrap();

    let result = worktree_service.complete_worktree_for_todo(todo.id);
    assert!(result.is_err());

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

    let result = worktree_service
        .complete_worktree_for_todo(todo.id)
        .unwrap();

    assert!(result.is_none());
}

/// Test removing a workspace actually removes the files
#[test]
fn test_remove_git_worktree_actually_removes() {
    let Some(ws) = JjWorkspace::new() else {
        return;
    };
    let repo_path = ws.repo_path();

    let db = Database::new_in_memory().unwrap();
    let task_service = TaskService::new(&db);
    let worktree_service = WorktreeService::new(&db);

    let task = task_service
        .create_task("Task", None, Some("RM-100"), None)
        .unwrap();

    let wt = worktree_service
        .add_worktree(
            task.id,
            &repo_path.to_string_lossy(),
            None,
            Some("RM-100"),
            None,
            true,
        )
        .unwrap();

    assert!(std::path::Path::new(&wt.path).exists());

    worktree_service.remove_worktree(wt.id, false).unwrap();

    assert!(!std::path::Path::new(&wt.path).exists());
}
