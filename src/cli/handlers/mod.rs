//! Command handler implementations grouped by domain.

mod alias;
mod completion;
mod config;
mod confirm;
mod hint;
mod import;
mod json_out;
mod link;
mod llm_help;
mod migrate;
mod notes;
mod repo;
mod scrap;
mod sync;
mod task;
mod todo;

pub use alias::handle_alias;
pub use completion::{handle_complete, handle_completion};
pub use config::handle_config;
pub use hint::emit_hint;
pub use import::handle_import;
pub use link::handle_link;
pub use llm_help::handle_llm_help;
pub use migrate::handle_migrate;
pub use notes::handle_notes;
pub use repo::handle_repo;
pub use scrap::handle_scrap;
pub use sync::handle_sync;
pub use task::{
    handle_archive, handle_desc, handle_info, handle_list, handle_new, handle_switch, handle_ticket,
};
pub use todo::handle_todo;

use crate::db::Database;
use crate::models::TaskId;
use crate::services::TaskService;
use crate::utils::Result;

/// Shared database access and the global `--task` reference for command handlers.
pub struct CommandCtx<'a> {
    pub db: &'a Database,
    task_ref: Option<&'a str>,
}

impl<'a> CommandCtx<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db, task_ref: None }
    }

    pub fn with_task_ref(db: &'a Database, task_ref: Option<&'a str>) -> Self {
        Self { db, task_ref }
    }

    /// The global `--task` value, if given.
    pub fn task_ref(&self) -> Option<&'a str> {
        self.task_ref
    }

    /// Task this command operates on (`--task`, else current task in single mode).
    pub fn target_task_id(&self) -> Result<TaskId> {
        TaskService::new(self.db).resolve_target_task_id(self.task_ref)
    }

    /// Like [`Self::target_task_id`], but a command-specific positional reference wins.
    pub fn target_task_id_with(&self, positional: Option<&str>) -> Result<TaskId> {
        TaskService::new(self.db).resolve_target_task_id(positional.or(self.task_ref))
    }

    /// Task for optional context (hints, notes); `None` when nothing is targeted.
    pub fn target_task_id_opt(&self) -> Result<Option<TaskId>> {
        TaskService::new(self.db).resolve_target_task_id_opt(self.task_ref)
    }
}
