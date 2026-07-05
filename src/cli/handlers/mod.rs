//! Command handler implementations grouped by domain.

mod alias;
mod completion;
mod config;
mod link;
mod llm_help;
mod repo;
mod scrap;
mod sync;
mod task;
mod todo;

pub use alias::handle_alias;
pub use completion::{handle_complete, handle_completion};
pub use config::handle_config;
pub use link::handle_link;
pub use llm_help::handle_llm_help;
pub use repo::handle_repo;
pub use scrap::handle_scrap;
pub use sync::handle_sync;
pub use task::{
    handle_archive, handle_desc, handle_info, handle_list, handle_new, handle_switch, handle_ticket,
};
pub use todo::handle_todo;

/// Shared database access for command handlers.
use crate::db::Database;

pub struct CommandCtx<'a> {
    pub db: &'a Database,
}

impl<'a> CommandCtx<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }
}
