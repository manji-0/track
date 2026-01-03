mod cli;
mod db;
mod models;
mod services;
mod utils;
mod webui;

use clap::Parser;
use cli::{handler::CommandHandler, Cli, Commands};
use std::process;

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

    if let Err(e) = handler.handle(cli.command) {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

