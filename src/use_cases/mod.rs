//! Application use cases that coordinate domain services and external systems.
//!
//! Use cases own multi-step workflows and transaction boundaries where a single
//! service method is not enough.

pub mod apply_todo_action;
pub mod archive_task;
pub mod complete_todo;
pub mod create_today_task;
pub mod migrate_legacy_worktrees;
pub mod sync_task;
pub mod todo_workspace;

pub use apply_todo_action::ApplyTodoActionUseCase;
pub use archive_task::{ArchiveBlockers, ArchiveTaskOutcome, ArchiveTaskUseCase, DirtyWorkspace};
pub use complete_todo::{CompleteTodoOutcome, CompleteTodoUseCase};
pub use create_today_task::CreateTodayTaskUseCase;
pub use migrate_legacy_worktrees::{
    LegacyWorktreeTaskReport, MigrateLegacyWorktreesOutcome, MigrateLegacyWorktreesUseCase,
};
pub use sync_task::{
    RepoSyncOutcome, SyncTaskOutcome, SyncTaskUseCase, WorkspaceCreateError, WorkspaceCreated,
};
pub use todo_workspace::{TodoWorkspaceOutcome, TodoWorkspaceRequest, TodoWorkspaceUseCase};
