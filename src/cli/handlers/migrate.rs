use crate::cli::handlers::CommandCtx;
use crate::cli::MigrateCommands;
use crate::use_cases::MigrateLegacyWorktreesUseCase;
use crate::utils::Result;

pub fn handle_migrate(ctx: &CommandCtx, command: MigrateCommands) -> Result<()> {
    match command {
        MigrateCommands::LegacyWorktrees {
            task_ref,
            dry_run,
            force,
        } => {
            let use_case = MigrateLegacyWorktreesUseCase::new(ctx.db);
            let task_id = use_case.resolve_task_id(task_ref.as_deref())?;
            let outcome = use_case.execute(task_id, dry_run, force)?;

            if outcome.tasks.is_empty() {
                println!("No legacy per-TODO worktree flags or worktree records found.");
                return Ok(());
            }

            if dry_run {
                println!("Dry run — legacy migration plan:\n");
            } else {
                println!(
                    "Cleared {} legacy TODO flag(s), removed {} worktree record(s).\n",
                    outcome.todos_cleared, outcome.worktrees_removed
                );
            }

            for report in &outcome.tasks {
                println!(
                    "Task #{}: {} (jj slug: {})",
                    report.task_id, report.task_name, report.jj_slug
                );
                println!(
                    "  flagged TODOs: {}, legacy worktree records: {}",
                    report.flagged_todos, report.legacy_worktrees
                );
                for path in &report.legacy_worktree_paths {
                    println!("  - remove worktree: {path}");
                }
                if !report.worktrees_skipped_dirty.is_empty() {
                    println!("  skipped (uncommitted changes):");
                    for path in &report.worktrees_skipped_dirty {
                        println!("    {path}");
                    }
                    println!("  retry with --force after committing or discarding changes");
                }
                println!("  next: jj-task start {}", report.jj_slug);
                println!();
            }

            if dry_run {
                println!("Run without --dry-run to apply.");
            }
        }
    }

    Ok(())
}
