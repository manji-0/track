mod cli;
mod db;
mod models;
mod services;
mod utils;

use clap::Parser;
use cli::{Cli, handler::CommandHandler};
use std::process;

fn main() {
    let cli = Cli::parse();

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
