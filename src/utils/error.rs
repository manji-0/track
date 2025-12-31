use thiserror::Error;

#[derive(Error, Debug)]
pub enum TrackError {
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

    #[error("Git error: {0}")]
    Git(String),

    #[error("Path '{0}' is not a Git repository")]
    NotGitRepository(String),

    #[error("Branch '{0}' already exists")]
    BranchExists(String),

    #[error("Invalid URL format: {0}")]
    InvalidUrl(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Operation cancelled by user")]
    Cancelled,

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, TrackError>;
