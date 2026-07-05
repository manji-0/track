use super::{Todo, TodoStatus};
use crate::utils::TrackError;
use serde::Serialize;

/// Intent-based operations on a TODO item.
///
/// Handlers map CLI/WebUI input to these actions instead of writing status strings directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TodoAction {
    Complete,
    Cancel,
    MakeNext,
}

impl TodoAction {
    /// Parses a `track todo update` status argument.
    pub fn from_cli_update_status(status: &str) -> Result<Self, TrackError> {
        match status {
            TodoStatus::CANCELLED => Ok(Self::Cancel),
            TodoStatus::DONE => Err(TrackError::TodoCompleteRequiresDoneCommand),
            TodoStatus::PENDING => Err(TrackError::InvalidStatus(
                "pending (reopen is not allowed; add a new TODO instead)".to_string(),
            )),
            other => Err(TrackError::InvalidStatus(other.to_string())),
        }
    }

    /// Parses a WebUI route segment such as `/api/todo/1/done`.
    pub fn from_web_route(status: &str) -> Result<Self, TrackError> {
        match status {
            TodoStatus::DONE => Ok(Self::Complete),
            TodoStatus::CANCELLED => Ok(Self::Cancel),
            TodoStatus::PENDING => Err(TrackError::InvalidStatus(
                "pending (reopen is not allowed)".to_string(),
            )),
            other => Err(TrackError::InvalidStatus(other.to_string())),
        }
    }

    /// Returns actions the caller may invoke for this TODO.
    pub fn allowed_for(todo: &Todo) -> Vec<Self> {
        match todo.status {
            TodoStatus::Pending => vec![Self::MakeNext, Self::Complete, Self::Cancel],
            TodoStatus::Done | TodoStatus::Cancelled => Vec::new(),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Cancel => "cancel",
            Self::MakeNext => "make_next",
        }
    }
}
