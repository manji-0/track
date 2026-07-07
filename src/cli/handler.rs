//! Command handler dispatch for the track CLI.

use crate::cli::handlers::CommandCtx;
use crate::cli::Commands;
use crate::db::Database;
use crate::utils::Result;
use clap_complete::Shell;

pub struct CommandHandler {
    db: Database,
}

impl CommandHandler {
    pub fn new() -> Result<Self> {
        let db = Database::new()?;
        Ok(Self { db })
    }

    #[allow(dead_code)]
    pub fn from_db(db: Database) -> Self {
        Self { db }
    }

    /// Returns a reference to the database instance.
    /// This is primarily used for testing.
    #[allow(dead_code)]
    pub fn get_db(&self) -> &Database {
        &self.db
    }

    pub fn handle(&self, command: Commands) -> Result<()> {
        let ctx = CommandCtx::new(&self.db);
        match command {
            Commands::New {
                name,
                description,
                ticket,
                ticket_url,
                template,
            } => super::handlers::handle_new(
                &ctx,
                &name,
                description.as_deref(),
                ticket.as_deref(),
                ticket_url.as_deref(),
                template.as_deref(),
            ),
            Commands::List { all } => super::handlers::handle_list(&ctx, all),
            Commands::Switch { task_ref } => super::handlers::handle_switch(&ctx, &task_ref),
            Commands::Status { id, json, all } => super::handlers::handle_info(&ctx, id, json, all),
            Commands::Desc { description, task } => {
                super::handlers::handle_desc(&ctx, description.as_deref(), task)
            }
            Commands::Ticket {
                ticket_id,
                url,
                task,
            } => super::handlers::handle_ticket(&ctx, &ticket_id, &url, task),
            Commands::Archive { task_ref, force } => {
                super::handlers::handle_archive(&ctx, task_ref.as_deref(), force)
            }
            Commands::Todo(cmd) => super::handlers::handle_todo(&ctx, cmd),
            Commands::Link(cmd) => super::handlers::handle_link(&ctx, cmd),
            Commands::Scrap(cmd) => super::handlers::handle_scrap(&ctx, cmd),
            Commands::Sync { legacy } => super::handlers::handle_sync(&ctx, legacy),
            Commands::Migrate(cmd) => super::handlers::handle_migrate(&ctx, cmd),
            Commands::Repo(cmd) => super::handlers::handle_repo(&ctx, cmd),
            Commands::Alias(cmd) => super::handlers::handle_alias(&ctx, cmd),
            Commands::LlmHelp => super::handlers::handle_llm_help(&ctx),
            Commands::Completion { shell, dynamic } => {
                super::handlers::handle_completion(&ctx, shell, dynamic)
            }
            Commands::Complete { completion_type } => {
                super::handlers::handle_complete(&ctx, completion_type)
            }
            Commands::Config(cmd) => super::handlers::handle_config(&ctx, cmd),
            Commands::Webui { .. } => unreachable!("Webui command is handled in main.rs"),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn handle_llm_help(&self) -> Result<()> {
        super::handlers::handle_llm_help(&CommandCtx::new(&self.db))
    }

    #[allow(dead_code)]
    pub(crate) fn handle_completion(&self, shell: Shell, dynamic: bool) -> Result<()> {
        super::handlers::handle_completion(&CommandCtx::new(&self.db), shell, dynamic)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    #[test]
    fn test_llm_help() {
        let db = Database::new_in_memory().unwrap();
        let handler = CommandHandler::from_db(db);

        let result = handler.handle_llm_help();
        assert!(result.is_ok());
    }

    #[test]
    fn test_completion() {
        let db = Database::new_in_memory().unwrap();
        let handler = CommandHandler::from_db(db);

        for shell in [
            clap_complete::Shell::Bash,
            clap_complete::Shell::Zsh,
            clap_complete::Shell::Fish,
            clap_complete::Shell::PowerShell,
        ] {
            let result = handler.handle_completion(shell, false);
            assert!(
                result.is_ok(),
                "Static completion generation failed for {:?}",
                shell
            );
        }

        for shell in [clap_complete::Shell::Bash, clap_complete::Shell::Zsh] {
            let result = handler.handle_completion(shell, true);
            assert!(
                result.is_ok(),
                "Dynamic completion generation failed for {:?}",
                shell
            );
        }
    }
}
