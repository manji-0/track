use crate::cli::handlers::CommandCtx;
use crate::cli::ConfigCommands;
use crate::models::VcsMode;
use crate::utils::{Result, TrackError};

pub fn handle_config(ctx: &CommandCtx, command: ConfigCommands) -> Result<()> {
    match command {
        ConfigCommands::Set { key, value } => {
            let normalized = key.trim().to_ascii_lowercase().replace('_', "-");
            match normalized.as_str() {
                "vcs-mode" => {
                    let mode: VcsMode = value
                        .parse()
                        .map_err(|err: String| TrackError::Other(err))?;
                    ctx.db.set_vcs_mode(mode)?;
                    println!("Set VCS mode: {mode}");
                    match mode {
                        VcsMode::Jj => {
                            println!("\nJJ mode uses agent-skill-jj (jj-task + $jj skill).");
                            println!("Run `jj-task start <slug>` to begin work.");
                            println!("(`track sync` is legacy-only — see `track migrate legacy-worktrees`)");
                        }
                        VcsMode::Git => {
                            println!("\nGit mode uses plain git worktrees and branches.");
                            println!("Run `track sync` to create `.worktrees/<slug>/` workspaces.");
                        }
                    }
                }
                other => {
                    return Err(TrackError::Other(format!(
                        "unknown config key '{other}' (supported: vcs-mode)"
                    )));
                }
            }
        }
        ConfigCommands::SetCalendar { calendar_id } => {
            ctx.db.set_app_state("calendar_id", &calendar_id)?;
            println!("Set Google Calendar ID: {}", calendar_id);
            println!("\nTo use this calendar in the WebUI:");
            println!("1. Make sure the calendar is shared with appropriate permissions");
            println!("2. The calendar will be displayed in the today task WebUI");
        }
        ConfigCommands::Show => {
            println!("=== Track Configuration ===\n");

            let vcs_mode = ctx.db.get_vcs_mode()?;
            println!("VCS mode: {vcs_mode} (jj = agent-skill-jj, git = plain git worktrees)");

            if let Some(calendar_id) = ctx.db.get_app_state("calendar_id")? {
                println!("Google Calendar ID: {}", calendar_id);
            } else {
                println!("Google Calendar ID: (not set)");
                println!("\nTo set a calendar ID, run:");
                println!("  track config set-calendar <calendar-id>");
            }

            println!("\nTo change VCS mode, run:");
            println!("  track config set vcs-mode jj");
            println!("  track config set vcs-mode git");
        }
    }
    Ok(())
}
