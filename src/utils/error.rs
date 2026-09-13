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
        "--worktree was removed. Use one workspace per task (`track sync`). See `track llm-help`."
    )]
    WorktreeFlagRemoved,

    #[error("TODO content cannot be empty")]
    EmptyTodoContent,

    #[error("Scrap content cannot be empty")]
    EmptyScrapContent,

    #[error("Scrap #{0} not found in current task")]
    ScrapIndexNotFound(i64),

    #[error(
        "No track task snapshot in git notes (fetch `refs/notes/track` and run from the PR branch)"
    )]
    NoTaskNotes,

    #[error("Track task snapshot could not be parsed: {0}")]
    TaskNotesParse(String),

    #[error(
        "Cannot rewrite published commits on {branch}. Add a follow-up TODO instead of amending history."
    )]
    CannotRewritePublishedHistory { branch: String },

    #[error("Task branch {branch} has diverged from the published tip")]
    HistoryDiverged { branch: String },

    #[error("Workspaces have uncommitted changes: {0:?}")]
    UncommittedWorkspaces(Vec<String>),

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

    #[error("Path '{0}' is not a git or jj repository")]
    NotVcsRepository(String),

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

    #[error("Confirmation required (stdin is not a TTY). {hint}")]
    ConfirmationRequired { hint: String },

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

    #[error("Template '{name}' render failed: {detail}")]
    TemplateRenderFailed { name: String, detail: String },

    #[error("Failed to determine data directory")]
    DataDirectoryUnavailable,

    #[error("Invalid VCS mode: {0}")]
    InvalidVcsMode(String),

    #[error(
        "Workspace at {path} is {actual} but vcs-mode is {expected}. Switch back with `track config set vcs-mode {actual}` or remove the workspace and re-run `track sync`."
    )]
    WorkspaceVcsMismatch {
        path: String,
        expected: String,
        actual: String,
    },

    #[error("Invalid aggressive mode: {0}")]
    InvalidAggressiveMode(String),

    #[error("Invalid app_state value for '{key}': {detail}")]
    InvalidAppStateValue { key: String, detail: String },

    #[error("Migration blocked: {detail}")]
    MigrationBlocked { detail: String },

    #[error("Unknown config key '{0}' (supported: vcs-mode, aggressive-mode)")]
    UnknownConfigKey(String),
}

/// Convenience type alias for Results with TrackError.
///
/// This type is used throughout the application for operations that may fail.
pub type Result<T> = std::result::Result<T, TrackError>;
