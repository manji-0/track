use crate::cli::handlers::CommandCtx;
use crate::cli::ScrapCommands;
use crate::services::ScrapService;
use crate::utils::{Result, TrackError};
use chrono::Local;

pub fn handle_scrap(ctx: &CommandCtx, command: ScrapCommands) -> Result<()> {
    let current_task_id = ctx
        .db
        .get_current_task_id()?
        .ok_or(TrackError::NoActiveTask)?;
    let scrap_service = ScrapService::new(ctx.db);

    match command {
        ScrapCommands::Add { content } => {
            let scrap = scrap_service.add_scrap(current_task_id, &content)?;
            let timestamp = scrap
                .created_at
                .with_timezone(&Local)
                .format("%Y-%m-%d %H:%M:%S");
            println!("Added scrap at {}", timestamp);
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
