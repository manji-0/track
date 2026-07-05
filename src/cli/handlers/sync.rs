use crate::cli::handlers::CommandCtx;
use crate::models::TodoStatus;
use crate::services::{RepoService, TaskService, TodoService, WorktreeService};
use crate::utils::{Result, TrackError};
use std::path::{Path, PathBuf};

pub fn handle_sync(ctx: &CommandCtx) -> Result<()> {
    let current_task_id = ctx
        .db
        .get_current_task_id()?
        .ok_or(TrackError::NoActiveTask)?;

    let task_service = TaskService::new(ctx.db);
    let task = task_service.get_task(current_task_id)?;
    let repo_service = RepoService::new(ctx.db);
    let repos = repo_service.list_repos(current_task_id)?;

    if repos.is_empty() {
        return Err(TrackError::Other(
            "No repositories registered for this task".to_string(),
        ));
    }

    // Determine task bookmark name
    let task_bookmark = if let Some(ticket_id) = &task.ticket_id {
        format!("task/{}", ticket_id)
    } else {
        format!("task/task-{}", task.id)
    };

    println!("Syncing task bookmark: {}\n", task_bookmark);

    let worktree_service = WorktreeService::new(ctx.db);
    let existing_worktrees = worktree_service.list_worktrees(current_task_id)?;

    for repo in &repos {
        println!("Repository: {}", repo.repo_path);

        // Check if repository exists
        if !std::path::Path::new(&repo.repo_path).exists() {
            println!("  ⚠ Repository not found, skipping\n");
            continue;
        }

        let status_output = std::process::Command::new("jj")
            .args(["-R", &repo.repo_path, "diff", "--summary"])
            .output()?;

        if !status_output.status.success() {
            return Err(TrackError::Other(format!(
                "Failed to check status for {}",
                repo.repo_path
            )));
        }

        let repo_root = Path::new(&repo.repo_path)
            .canonicalize()
            .map_err(|e| TrackError::Other(format!("Failed to resolve repo path: {}", e)))?;
        let repo_worktrees: Vec<PathBuf> = existing_worktrees
            .iter()
            .filter(|wt| wt.base_repo.as_deref() == Some(repo.repo_path.as_str()))
            .filter_map(|wt| Path::new(&wt.path).canonicalize().ok())
            .filter_map(|wt_path| wt_path.strip_prefix(&repo_root).ok().map(PathBuf::from))
            .collect();

        let status_stdout = String::from_utf8_lossy(&status_output.stdout);
        let mut has_changes = false;

        for line in status_stdout.lines() {
            let path = line.split_whitespace().last().unwrap_or("").trim();
            if path.is_empty() {
                continue;
            }

            let path = Path::new(path);
            let is_worktree = repo_worktrees
                .iter()
                .any(|worktree_path| path.starts_with(worktree_path));

            if !is_worktree {
                has_changes = true;
                break;
            }
        }

        if has_changes {
            return Err(TrackError::Other(format!(
                    "Repository {} has pending changes in the base workspace. Please clean before sync.",
                    repo.repo_path
                )));
        }

        // Check if bookmark exists
        let bookmark_exists =
            worktree_service.bookmark_exists_in_repo(&repo.repo_path, &task_bookmark)?;

        if !bookmark_exists {
            // Determine the base for creating the task bookmark
            let base_ref = if let Some(ref base_branch) = repo.base_branch {
                base_branch.clone()
            } else if let Some(ref base_hash) = repo.base_commit_hash {
                base_hash.clone()
            } else {
                "@".to_string()
            };

            let create_result = std::process::Command::new("jj")
                .args([
                    "-R",
                    &repo.repo_path,
                    "bookmark",
                    "create",
                    &task_bookmark,
                    "-r",
                    &base_ref,
                ])
                .output()?;

            if create_result.status.success() {
                println!("  ✓ Bookmark {} created from {}", task_bookmark, base_ref);
            } else {
                let error = String::from_utf8_lossy(&create_result.stderr);
                println!(
                    "  ✗ Failed to create bookmark {} from {} ({})",
                    task_bookmark,
                    base_ref,
                    error.trim()
                );
                continue;
            }
        } else {
            println!("  ✓ Bookmark {} already exists", task_bookmark);
        }

        let edit_result = std::process::Command::new("jj")
            .args(["-R", &repo.repo_path, "edit", &task_bookmark])
            .status();

        if edit_result.is_ok() && edit_result.unwrap().success() {
            println!("  ✓ Moved workspace to {}\n", task_bookmark);
        } else {
            println!("  ✗ Failed to move workspace to {}\n", task_bookmark);
        }
    }

    // Check for pending worktrees
    println!("Checking for pending workspaces...");
    let todo_service = TodoService::new(ctx.db);
    let todos = todo_service.list_todos(current_task_id)?;

    for todo in todos {
        if todo.worktree_requested && todo.status != TodoStatus::Done {
            // Check if worktree already exists for this TODO
            let worktrees = worktree_service.list_worktrees(current_task_id)?;
            let mut exists = false;
            for wt in worktrees {
                // Check if this worktree is linked to our todo
                // Since list_worktrees returns GitItems which have todo_id, I need to check if that field is accessible
                // Looking at models, GitItem has todo_id: Option<i64>
                if wt.todo_id == Some(todo.id) {
                    exists = true;
                    break;
                }
            }

            if !exists {
                println!(
                    "Creating workspace for TODO #{}: {}",
                    todo.task_index, todo.content
                );
                for repo in &repos {
                    match worktree_service.add_worktree(
                        current_task_id,
                        &repo.repo_path,
                        None,
                        task.ticket_id.as_deref(),
                        Some(todo.id),
                        false,
                    ) {
                        Ok(wt) => println!("  Created {} ({})", wt.path, wt.branch),
                        Err(e) => {
                            eprintln!("  Error creating workspace for {}: {}", repo.repo_path, e)
                        }
                    }
                }
            }
        }
    }

    println!("Sync complete.");
    Ok(())
}
