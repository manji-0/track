use clap::Parser;
use std::process;
use track::cli::{Cli, Commands, handler::CommandHandler};
use track::models::SessionBinding;
use track::webui;

/// Application entry point
fn main() {
    let cli = Cli::parse();

    // Handle webui command separately (requires async runtime)
    if let Commands::Webui { port, open } = cli.command {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
        if let Err(e) = rt.block_on(webui::start_server(port, open)) {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
        return;
    }

    let handler = match CommandHandler::new() {
        Ok(h) => h,
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    };

    let binding = SessionBinding {
        env_task: std::env::var("TRACK_TASK").ok(),
        cwd: std::env::current_dir().ok(),
    };
    let result = handler.handle_with_target(cli.command, cli.task.as_deref(), binding);
    if let Err(e) = result {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}
