use crate::cli::handlers::CommandCtx;
use crate::cli::AliasCommands;
use crate::services::TaskService;
use crate::utils::{Result, TrackError};

pub fn handle_alias(ctx: &CommandCtx, command: AliasCommands) -> Result<()> {
    let task_service = TaskService::new(ctx.db);

    match command {
        AliasCommands::Set { alias, task, force } => {
            let task_id = match task {
                Some(id) => id,
                None => ctx
                    .db
                    .get_current_task_id()?
                    .ok_or(TrackError::NoActiveTask)?,
            };

            task_service.set_alias(task_id, &alias, force)?;
            let task = task_service.get_task(task_id)?;
            println!("Set alias '{}' for task #{}: {}", alias, task.id, task.name);
        }
        AliasCommands::Remove { task } => {
            let task_id = match task {
                Some(id) => id,
                None => ctx
                    .db
                    .get_current_task_id()?
                    .ok_or(TrackError::NoActiveTask)?,
            };

            let task = task_service.get_task(task_id)?;
            if task.alias.is_none() {
                println!("Task #{} has no alias", task_id);
                return Ok(());
            }

            let removed_alias = task.alias.clone().unwrap();
            task_service.remove_alias(task_id)?;
            println!(
                "Removed alias '{}' from task #{}: {}",
                removed_alias, task.id, task.name
            );
        }
    }

    Ok(())
}
