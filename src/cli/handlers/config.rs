use crate::cli::handlers::CommandCtx;
use crate::cli::ConfigCommands;
use crate::utils::Result;

pub fn handle_config(ctx: &CommandCtx, command: ConfigCommands) -> Result<()> {
    match command {
        ConfigCommands::SetCalendar { calendar_id } => {
            ctx.db.set_app_state("calendar_id", &calendar_id)?;
            println!("Set Google Calendar ID: {}", calendar_id);
            println!("\nTo use this calendar in the WebUI:");
            println!("1. Make sure the calendar is shared with appropriate permissions");
            println!("2. The calendar will be displayed in the today task WebUI");
        }
        ConfigCommands::Show => {
            println!("=== Track Configuration ===\n");

            if let Some(calendar_id) = ctx.db.get_app_state("calendar_id")? {
                println!("Google Calendar ID: {}", calendar_id);
            } else {
                println!("Google Calendar ID: (not set)");
                println!("\nTo set a calendar ID, run:");
                println!("  track config set-calendar <calendar-id>");
            }
        }
    }
    Ok(())
}
