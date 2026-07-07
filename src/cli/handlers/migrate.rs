use crate::cli::handlers::CommandCtx;
use crate::cli::MigrateCommands;
use crate::use_cases::MigrateLegacyWorktreesUseCase;
use crate::utils::Result;

pub fn handle_migrate(ctx: &CommandCtx, command: MigrateCommands) -> Result<()> {
    match command {
        MigrateCommands::LegacyWorktrees { task_ref, dry_run } => {
            let use_case = MigrateLegacyWorktreesUseCase::new(ctx.db);
            let task_id = use_case.resolve_task_id(task_ref.as_deref())?;
            let outcome = use_case.execute(task_id, dry_run)?;

            if outcome.tasks.is_empty() {
                println!("No legacy per-TODO worktree flags found.");
                return Ok(());
            }

            if dry_run {
                println!("Dry run — legacy worktree flags that would be cleared:\n");
            } else {
                println!(
                    "Cleared legacy worktree flags on {} TODO(s).\n",
                    outcome.todos_cleared
                );
            }

            for report in &outcome.tasks {
                println!(
                    "Task #{}: {} (jj slug: {})",
                    report.task_id, report.task_name, report.jj_slug
                );
                println!(
                    "  flagged TODOs: {}, track worktree records: {}",
                    report.flagged_todos, report.track_worktrees
                );
                println!("  next: jj-task start {}", report.jj_slug);
                if report.track_worktrees > 0 {
                    println!(
                        "  note: complete or remove legacy workspaces with `track todo done` / `track archive`"
                    );
                }
                println!();
            }

            if dry_run {
                println!("Run without --dry-run to apply.");
            }
        }
    }

    Ok(())
}
