use crate::cli::NotesCommands;
use crate::cli::handlers::CommandCtx;
use crate::services::git_notes;
use crate::use_cases::project_task_notes_or_warn;
use crate::utils::{Result, TrackError};
use std::env;

pub fn handle_notes(ctx: &CommandCtx, command: NotesCommands) -> Result<()> {
    let cwd = env::current_dir()?;
    let path = cwd
        .to_str()
        .ok_or_else(|| TrackError::PathResolutionFailed(cwd.display().to_string()))?;

    match command {
        NotesCommands::Fetch { remote } => {
            git_notes::fetch_notes(path, &remote)?;
            println!("Fetched {} from {remote}", git_notes::NOTES_REF);
            println!("Run `track import` on the PR branch to restore the task.");
        }
        NotesCommands::Push { remote } => {
            if let Some(task_id) = ctx.db.get_current_task_id()? {
                project_task_notes_or_warn(ctx.db, task_id);
            }
            let _ = git_notes::fetch_notes(path, &remote);
            if let Some(task_id) = ctx.db.get_current_task_id()? {
                project_task_notes_or_warn(ctx.db, task_id);
            }
            git_notes::push_notes(path, &remote)?;
            println!("Pushed {} to {remote}", git_notes::NOTES_REF);
            println!(
                "Others: git fetch {remote} {}:{}",
                git_notes::NOTES_REF,
                git_notes::NOTES_REF
            );
            println!("        track import");
        }
    }
    Ok(())
}
