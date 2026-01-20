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

    #[error("JJ error: {0}")]
    Jj(String),

    #[error("Path '{0}' is not a JJ repository")]
    NotJjRepository(String),

    #[error("Bookmark '{0}' already exists")]
    BookmarkExists(String),

    #[error("Invalid URL format: {0}")]
    InvalidUrl(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Operation cancelled by user")]
    #[allow(dead_code)]
    Cancelled,

    #[error("{0}")]
    Other(String),
}

/// Convenience type alias for Results with TrackError.
///
/// This type is used throughout the application for operations that may fail.
pub type Result<T> = std::result::Result<T, TrackError>;
