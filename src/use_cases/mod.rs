//! Application use cases that coordinate domain services and external systems.
//!
//! Use cases own multi-step workflows and transaction boundaries where a single
//! service method is not enough.

pub mod archive_task;
pub mod complete_todo;
pub mod create_today_task;

pub use archive_task::{ArchiveTaskOutcome, ArchiveTaskUseCase, DirtyWorkspace};
pub use complete_todo::{CompleteTodoOutcome, CompleteTodoUseCase};
pub use create_today_task::CreateTodayTaskUseCase;
