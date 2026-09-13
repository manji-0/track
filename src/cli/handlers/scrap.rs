use crate::cli::ScrapCommands;
use crate::cli::handlers::CommandCtx;
use crate::cli::handlers::json_out::{MutationKind, emit_mutation};
use crate::models::{ScrapIndex, ScrapVisibility};
use crate::services::ScrapService;
use crate::use_cases::project_task_notes_or_warn;
use crate::utils::{Result, TrackError};
use chrono::Local;

pub fn handle_scrap(ctx: &CommandCtx, command: ScrapCommands) -> Result<()> {
    let current_task_id = ctx
        .db
        .get_current_task_id()?
        .ok_or(TrackError::NoActiveTask)?;
    let scrap_service = ScrapService::new(ctx.db);

    match command {
        ScrapCommands::Add {
            content,
            share,
            json,
        } => {
            let visibility = if share {
                ScrapVisibility::Shared
            } else {
                ScrapVisibility::Local
            };
            let scrap = scrap_service.add_scrap_with(current_task_id, &content, visibility)?;
            project_task_notes_or_warn(ctx.db, current_task_id);
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
                    let vis = scrap.visibility.as_str();
                    println!(
                        "Added scrap #{id} ({vis}) at {timestamp}",
                        id = scrap.scrap_id
                    );
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
                println!(
                    "[{timestamp}] #{} {}",
                    scrap.scrap_id,
                    scrap.visibility.as_str()
                );
                println!("  {}", scrap.content);
                println!();
            }
        }
        ScrapCommands::Share { id, json } => {
            scrap_service.set_visibility(
                current_task_id,
                ScrapIndex::from_i64(id),
                ScrapVisibility::Shared,
            )?;
            project_task_notes_or_warn(ctx.db, current_task_id);
            emit_mutation(
                ctx,
                json,
                MutationKind::ScrapShare,
                Some(id),
                Some(current_task_id),
                || println!("Shared scrap #{id} (included in git notes)"),
            )?;
        }
        ScrapCommands::Unshare { id, json } => {
            scrap_service.set_visibility(
                current_task_id,
                ScrapIndex::from_i64(id),
                ScrapVisibility::Local,
            )?;
            project_task_notes_or_warn(ctx.db, current_task_id);
            emit_mutation(
                ctx,
                json,
                MutationKind::ScrapUnshare,
                Some(id),
                Some(current_task_id),
                || println!("Unshared scrap #{id} (local only)"),
            )?;
        }
    }

    Ok(())
}
