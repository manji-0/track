use crate::cli::handlers::CommandCtx;
use crate::use_cases::{RepoSyncOutcome, SyncTaskUseCase};
use crate::utils::{Result, TrackError};

pub fn handle_sync(ctx: &CommandCtx) -> Result<()> {
    let current_task_id = ctx
        .db
        .get_current_task_id()?
        .ok_or(TrackError::NoActiveTask)?;

    let outcome = SyncTaskUseCase::new(ctx.db).execute(current_task_id)?;

    println!("Syncing task bookmark: {}\n", outcome.task_bookmark);

    for (repo_path, repo_outcome) in &outcome.repos {
        println!("Repository: {}", repo_path);
        match repo_outcome {
            RepoSyncOutcome::Missing => {
                println!("  ⚠ Repository not found, skipping\n");
            }
            RepoSyncOutcome::BookmarkCreated { base_ref, edit_ok } => {
                println!(
                    "  ✓ Bookmark {} created from {}",
                    outcome.task_bookmark, base_ref
                );
                print_edit_result(&outcome.task_bookmark, *edit_ok);
            }
            RepoSyncOutcome::BookmarkExists { edit_ok } => {
                println!("  ✓ Bookmark {} already exists", outcome.task_bookmark);
                print_edit_result(&outcome.task_bookmark, *edit_ok);
            }
            RepoSyncOutcome::BookmarkCreateFailed { base_ref, detail } => {
                println!(
                    "  ✗ Failed to create bookmark {} from {} ({})",
                    outcome.task_bookmark, base_ref, detail
                );
            }
        }
    }

    println!("Checking for pending workspaces...");

    for created in &outcome.workspaces_created {
        println!(
            "Creating workspace for TODO #{}: {}",
            created.todo_index, created.todo_content
        );
        println!("  Created {} ({})", created.workspace_path, created.branch);
    }

    for err in &outcome.workspace_errors {
        eprintln!(
            "  Error creating workspace for {}: {}",
            err.repo_path, err.detail
        );
    }

    println!("Sync complete.");
    Ok(())
}

fn print_edit_result(task_bookmark: &str, edit_ok: bool) {
    if edit_ok {
        println!("  ✓ Moved workspace to {}\n", task_bookmark);
    } else {
        println!("  ✗ Failed to move workspace to {}\n", task_bookmark);
    }
}
