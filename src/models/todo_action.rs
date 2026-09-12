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
    pub fn from_cli_update_status(status: TodoStatus) -> Result<Self, TrackError> {
        match status {
            TodoStatus::Cancelled => Ok(Self::Cancel),
            TodoStatus::Done => Err(TrackError::TodoCompleteRequiresDoneCommand),
            TodoStatus::Pending => Err(TrackError::InvalidStatus(
                "pending (reopen is not allowed; add a new TODO instead)".to_string(),
            )),
        }
    }

    /// Parses a WebUI route segment such as `/api/todo/1/done`.
    pub fn from_web_route(status: TodoStatus) -> Result<Self, TrackError> {
        match status {
            TodoStatus::Done => Ok(Self::Complete),
            TodoStatus::Cancelled => Ok(Self::Cancel),
            TodoStatus::Pending => Err(TrackError::InvalidStatus(
                "pending (reopen is not allowed)".to_string(),
            )),
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

/// Actions advertised on a TODO in agent JSON (`todos_agent[].allowed_actions`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TodoAgentAction {
    Complete,
    Cancel,
    MakeNext,
    Delete,
}

impl TodoAgentAction {
    pub fn allowed_for(todo: &Todo) -> Vec<Self> {
        let mut actions: Vec<Self> = TodoAction::allowed_for(todo)
            .into_iter()
            .map(Self::from)
            .collect();
        if todo.status == TodoStatus::Pending {
            actions.push(Self::Delete);
        }
        actions
    }
}

impl From<TodoAction> for TodoAgentAction {
    fn from(action: TodoAction) -> Self {
        match action {
            TodoAction::Complete => Self::Complete,
            TodoAction::Cancel => Self::Cancel,
            TodoAction::MakeNext => Self::MakeNext,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Todo;
    use chrono::Utc;

    fn pending_todo() -> Todo {
        Todo {
            id: crate::models::TodoId::from_i64(1),
            task_id: crate::models::TaskId::from_i64(1),
            task_index: crate::models::TodoIndex::from_i64(1),
            content: "Work".to_string(),
            status: TodoStatus::Pending,
            worktree_requested: false,
            requires_workspace: true,
            created_at: Utc::now(),
            completed_at: None,
        }
    }

    #[test]
    fn agent_actions_include_delete_for_pending() {
        let actions = TodoAgentAction::allowed_for(&pending_todo());
        assert_eq!(
            actions,
            vec![
                TodoAgentAction::MakeNext,
                TodoAgentAction::Complete,
                TodoAgentAction::Cancel,
                TodoAgentAction::Delete,
            ]
        );
    }

    #[test]
    fn agent_actions_serialize_as_snake_case() {
        let json = serde_json::to_value(TodoAgentAction::MakeNext).unwrap();
        assert_eq!(json, "make_next");
        let json = serde_json::to_value(TodoAgentAction::Delete).unwrap();
        assert_eq!(json, "delete");
    }
}
