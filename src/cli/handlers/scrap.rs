use crate::cli::ScrapCommands;
use crate::cli::handlers::CommandCtx;
use crate::cli::handlers::json_out::{MutationKind, emit_mutation};
use crate::models::jj_slug;
use crate::services::{ScrapService, TaskRevisionService, TaskService, git_notes, task_workspace};
use crate::utils::{Result, TrackError};
use chrono::Local;

pub fn handle_scrap(ctx: &CommandCtx, command: ScrapCommands) -> Result<()> {
    let current_task_id = ctx
        .db
        .get_current_task_id()?
        .ok_or(TrackError::NoActiveTask)?;
    let scrap_service = ScrapService::new(ctx.db);

    match command {
        ScrapCommands::Add { content, json } => {
            let scrap = scrap_service.add_scrap(current_task_id, &content)?;
            if let Err(err) = sync_aggressive_notes(ctx, current_task_id) {
                eprintln!("warning: git notes not updated: {err}");
            }
            emit_mutation(
                ctx,
                json,
                MutationKind::ScrapAdd,
                Some(scrap.scrap_id.as_i64()),
                Some(current_task_id),
                || {
                    let timestamp = scrap
                        .created_at
                        .with_timezone(&Local)
                        .format("%Y-%m-%d %H:%M:%S");
                    println!("Added scrap at {}", timestamp);
                },
            )?;
        }
        ScrapCommands::List => {
            let scraps = scrap_service.list_scraps(current_task_id)?;
            for scrap in scraps {
                let timestamp = scrap
                    .created_at
                    .with_timezone(&Local)
                    .format("%Y-%m-%d %H:%M:%S");
                println!("[{}]", timestamp);
                println!("  {}", scrap.content);
                println!();
            }
        }
    }

    Ok(())
}

fn sync_aggressive_notes(ctx: &CommandCtx, task_id: crate::models::TaskId) -> Result<()> {
    if !ctx.db.get_aggressive_mode()?.is_on() {
        return Ok(());
    }

    let task = TaskService::new(ctx.db).get_task(task_id)?;
    let slug = jj_slug(&task);
    let scraps = ScrapService::new(ctx.db).list_scraps(task_id)?;
    let revisions = TaskRevisionService::new(ctx.db).list_for_task(task_id)?;

    for rev in revisions {
        let workspace = task_workspace::workspace_path(&rev.repo_path, &slug);
        let root = if task_workspace::workspace_exists(&workspace) {
            workspace.as_str()
        } else {
            rev.repo_path.as_str()
        };
        git_notes::write_scraps(root, &rev.git_commit, &slug, &scraps)?;
    }
    Ok(())
}
