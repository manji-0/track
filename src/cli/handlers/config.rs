use crate::cli::ConfigCommands;
use crate::cli::handlers::CommandCtx;
use crate::cli::handlers::hint::emit_hint;
use crate::models::{AggressiveMode, VcsMode};
use crate::utils::{Result, TrackError};

pub fn handle_config(ctx: &CommandCtx, command: ConfigCommands) -> Result<()> {
    match command {
        ConfigCommands::Set { key, value } => {
            let normalized = key.trim().to_ascii_lowercase().replace('_', "-");
            match normalized.as_str() {
                "vcs-mode" => {
                    let mode: VcsMode = value.parse().map_err(TrackError::InvalidVcsMode)?;
                    ctx.db.set_vcs_mode(mode)?;
                    println!("Set VCS mode: {mode}");
                    match mode {
                        VcsMode::Git => {
                            println!(
                                "Git mode: track creates `.worktrees/<slug>` on branch `track/<slug>`."
                            );
                            println!(
                                "Register a repo with `track repo add .` (workspace is created automatically)."
                            );
                        }
                        VcsMode::Jj => {
                            println!(
                                "JJ mode: track creates colocated jj workspaces at `.worktrees/<slug>`."
                            );
                            println!(
                                "Bookmark `track/<slug>` is the GitHub PR head (`jj git push --named track/<slug>`)."
                            );
                        }
                    }
                }
                "aggressive-mode" => {
                    let mode: AggressiveMode =
                        value.parse().map_err(TrackError::InvalidAggressiveMode)?;
                    ctx.db.set_aggressive_mode(mode)?;
                    println!("Set aggressive mode: {mode}");
                    if mode.is_on() {
                        println!(
                            "Each task gets a dedicated empty marker revision. `todo done` writes one commit per TODO. Git notes (`refs/notes/track`) on the marker are identity; shared scraps are notes on that TODO's commit. Follow-up work appends a new TODO/commit — published history is never rewritten."
                        );
                        println!(
                            "Share scraps with `track scrap add --share` or `track scrap share N`. Disclose with `track notes push`. Others: `track notes fetch` then `track import` on the PR branch."
                        );
                        println!(
                            "Run `track sync` so existing workspaces get a marker. The marker SHA is never rewritten."
                        );
                    } else {
                        println!("Task revisions and git notes are disabled.");
                    }
                }
                other => return Err(TrackError::UnknownConfigKey(other.to_string())),
            }
            emit_hint(ctx, false, ctx.db.get_current_task_id()?)?;
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
            let aggressive = ctx.db.get_aggressive_mode()?;
            println!(
                "VCS mode: {vcs_mode} (git = default worktrees, jj = colocated jj workspaces)"
            );
            println!("Aggressive mode: {aggressive} (on = published work record in git notes)");

            if let Some(calendar_id) = ctx.db.get_app_state("calendar_id")? {
                println!("Google Calendar ID: {}", calendar_id);
            } else {
                println!("Google Calendar ID: (not set)");
                println!("\nTo set a calendar ID, run:");
                println!("  track config set-calendar <calendar-id>");
            }

            println!("\nTo change settings, run:");
            println!("  track config set vcs-mode git");
            println!("  track config set vcs-mode jj");
            println!("  track config set aggressive-mode on");
            println!("  track config set aggressive-mode off");
        }
    }
    Ok(())
}
