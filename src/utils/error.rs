//! Error types for the track CLI application.
//!
//! This module defines all error types that can occur during track operations,
//! including database errors, JJ errors, validation errors, and user-facing error messages.

use thiserror::Error;

/// Main error type for the track CLI application.
///
/// This enum encompasses all possible errors that can occur during track operations.
/// Each variant provides a descriptive error message and may contain additional context.
#[derive(Error, Debug)]
pub enum TrackError {
    /// Database operation failed
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("No active task. Run 'track new' or 'track switch' first.")]
    NoActiveTask,

    #[error("Task #{0} not found")]
    TaskNotFound(i64),

    #[error("Task #{0} is archived")]
    TaskArchived(i64),

    #[error("Task name cannot be empty")]
    EmptyTaskName,

    #[error(
        "track sync is deprecated in JJ mode. Use `jj-task start {slug}` instead (or `track sync --legacy` for old per-TODO worktrees)."
    )]
    SyncUseJjTask { slug: String },

    #[error("--worktree was removed. Use one jj-task workspace per task (`jj-task start <slug>`). See `track llm-help`.")]
    WorktreeFlagRemoved,

    #[error("TODO content cannot be empty")]
    EmptyTodoContent,

    #[error("Scrap content cannot be empty")]
    EmptyScrapContent,

    #[error("Workspaces have uncommitted changes: {0:?}")]
    UncommittedWorkspaces(Vec<String>),

    #[error(
        "jj-task workspace '{slug}' is not complete — run `jj-task done {slug}` after merging your PR. Active workspaces: {workspaces:?}"
    )]
    JjTaskNotCompleted {
        slug: String,
        workspaces: Vec<String>,
    },

    #[error("Ticket '{0}' is already linked to task #{1}")]
    DuplicateTicket(String, i64),

    #[error("Invalid ticket ID format: {0}")]
    InvalidTicketFormat(String),

    #[error("TODO #{0} not found")]
    TodoNotFound(i64),

    #[error("Worktree #{0} not found")]
    WorktreeNotFound(i64),

    #[error("Invalid status: {0}")]
    InvalidStatus(String),

    #[error("Cannot transition from '{from}' to '{to}'")]
    InvalidStatusTransition { from: String, to: String },

    #[error("TODO cannot be reopened from '{from}'. Add a new TODO instead.")]
    TodoReopenForbidden { from: String },

    #[error("Use 'track todo done <id>' to complete a TODO (merges JJ workspace if present).")]
    TodoCompleteRequiresDoneCommand,

    #[error(
        "Workspace was merged (bookmark: {bookmark}) but failed to mark TODO #{todo_index} as done: {detail}"
    )]
    TodoCompletionDbFailed {
        todo_index: i64,
        bookmark: String,
        detail: String,
    },

    #[error("JJ error: {0}")]
    Jj(String),

    #[error("Path '{0}' is not a JJ repository")]
    NotJjRepository(String),

    #[error("Bookmark '{0}' already exists")]
    BookmarkExists(String),

    #[error("Invalid URL format: {0}")]
    InvalidUrl(String),

    #[error("No repositories registered for this task")]
    NoRepositoriesRegistered,

    #[error("Failed to check status for {0}")]
    FailedRepoStatusCheck(String),

    #[error("Repository {0} has pending changes in the base workspace. Please clean before sync.")]
    RepoHasPendingChanges(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Operation cancelled by user")]
    #[allow(dead_code)]
    Cancelled,

    #[error("Failed to resolve path: {0}")]
    PathResolutionFailed(String),

    #[error("Current directory is not a registered repository for this task")]
    CurrentDirectoryNotRegistered,

    #[error("Workspace {path} has uncommitted changes. Use --force to recreate.")]
    WorkspaceHasUncommittedChanges { path: String },

    #[error("Bookmark '{bookmark}' not found in {repo_path}")]
    BookmarkNotFound { bookmark: String, repo_path: String },

    #[error("No workspace paths available for this TODO")]
    NoWorkspacePathsAvailable,

    #[error("Workspace cleanup failed: {0:?}")]
    WorkspaceRemovalFailed(Vec<String>),

    #[error("Failed to check workspace status for {path}: {detail}")]
    WorkspaceStatusCheckFailed { path: String, detail: String },

    #[error("JSON serialization failed: {0}")]
    SerializationFailed(String),

    #[error("TODO #{0} not found in current task")]
    TodoIndexNotFound(i64),

    #[error("No pending TODOs to reorder")]
    NoPendingTodos,

    #[error("TODO #{0} is not among pending TODOs")]
    TodoNotPending(i64),

    #[error("Link #{0} not found")]
    LinkNotFound(i64),

    #[error("No task found with reference '{0}'")]
    TaskReferenceNotFound(String),

    #[error("Alias '{alias}' is already in use by task #{task_id}")]
    AliasInUse { alias: String, task_id: i64 },

    #[error("Invalid alias: {0}")]
    InvalidAlias(String),

    #[error("Repository already registered for this task")]
    RepoAlreadyRegistered,

    #[error("Repository #{0} not found")]
    TaskRepoNotFound(i64),

    #[error("Repository #{0} not found in current task")]
    TaskRepoIndexNotFound(i64),

    #[error("Link #{0} not found in current task")]
    LinkIndexNotFound(i64),

    #[error("Path '{0}' is not a git repository")]
    NotGitRepository(String),

    #[error("Git error: {0}")]
    Git(String),

    #[error("Invalid jj-task workspace map at {path}: {detail}")]
    JjTaskMapInvalid { path: String, detail: String },

    #[error("Template '{name}' render failed: {detail}")]
    TemplateRenderFailed { name: String, detail: String },

    #[error("{0}")]
    Other(String),
}

/// Convenience type alias for Results with TrackError.
///
/// This type is used throughout the application for operations that may fail.
pub type Result<T> = std::result::Result<T, TrackError>;
