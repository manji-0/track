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
use crate::models::{SessionBinding, TargetSource, TaskId};
use crate::services::TaskService;
use crate::utils::Result;

/// Shared database access, the global `--task` reference, and the session binding.
pub struct CommandCtx<'a> {
    pub db: &'a Database,
    task_ref: Option<&'a str>,
    binding: SessionBinding,
}

impl<'a> CommandCtx<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self::with_task_ref(db, None)
    }

    pub fn with_task_ref(db: &'a Database, task_ref: Option<&'a str>) -> Self {
        Self::with_binding(db, task_ref, SessionBinding::default())
    }

    pub fn with_binding(
        db: &'a Database,
        task_ref: Option<&'a str>,
        binding: SessionBinding,
    ) -> Self {
        Self {
            db,
            task_ref,
            binding,
        }
    }

    /// The global `--task` value, if given.
    pub fn task_ref(&self) -> Option<&'a str> {
        self.task_ref
    }

    /// Task this command operates on: `--task`, `TRACK_TASK`, the workspace under the
    /// working directory, then (single mode) the current task.
    pub fn target_task_id(&self) -> Result<TaskId> {
        self.target_task_id_with(None)
    }

    /// Like [`Self::target_task_id`], but a command-specific positional reference wins.
    pub fn target_task_id_with(&self, positional: Option<&str>) -> Result<TaskId> {
        Ok(self.target_with_source(positional)?.0)
    }

    /// [`Self::target_task_id_with`] plus where the answer came from.
    pub fn target_with_source(&self, positional: Option<&str>) -> Result<(TaskId, TargetSource)> {
        match self.resolve(positional)? {
            Some(found) => Ok(found),
            None => Err(TaskService::new(self.db).no_target_error()?),
        }
    }

    /// Task for optional context (hints, notes); `None` when nothing is targeted.
    pub fn target_task_id_opt(&self) -> Result<Option<TaskId>> {
        Ok(self.resolve(None)?.map(|(id, _)| id))
    }

    /// An env ref that does not resolve is an error: the user set it deliberately.
    fn resolve(&self, positional: Option<&str>) -> Result<Option<(TaskId, TargetSource)>> {
        let service = TaskService::new(self.db);
        let explicit = positional
            .or(self.task_ref)
            .map(|r| (r, TargetSource::Flag));
        let bound = self.binding.env_task_ref().map(|r| (r, TargetSource::Env));
        if let Some((reference, source)) = explicit.or(bound) {
            let id = service.resolve_target_task_id(Some(reference))?;
            return Ok(Some((id, source)));
        }
        if let Some(cwd) = &self.binding.cwd
            && let Some(id) = service.find_task_by_workspace(cwd)?
        {
            return Ok(Some((id, TargetSource::Workspace)));
        }
        Ok(service
            .resolve_target_task_id_opt(None)?
            .map(|id| (id, TargetSource::Current)))
    }
}
