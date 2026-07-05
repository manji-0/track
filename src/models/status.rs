use serde::Serialize;
use std::str::FromStr;

/// Status of a task.
///
/// Tasks can be either active (currently being worked on) or archived (completed or abandoned).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    /// Task is currently active and can be worked on
    Active,
    /// Task has been archived and is no longer active
    Archived,
}

impl TaskStatus {
    pub const ACTIVE: &'static str = "active";
    pub const ARCHIVED: &'static str = "archived";

    /// Converts the status to its string representation.
    pub fn as_str(&self) -> &str {
        match self {
            TaskStatus::Active => Self::ACTIVE,
            TaskStatus::Archived => Self::ARCHIVED,
        }
    }

    /// Returns whether a transition from `self` to `target` is allowed.
    pub fn can_transition_to(self, target: Self) -> bool {
        matches!(
            (self, target),
            (TaskStatus::Active, TaskStatus::Active)
                | (TaskStatus::Active, TaskStatus::Archived)
                | (TaskStatus::Archived, TaskStatus::Archived)
        )
    }
}

impl FromStr for TaskStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            Self::ACTIVE => Ok(TaskStatus::Active),
            Self::ARCHIVED => Ok(TaskStatus::Archived),
            _ => Err(format!("Invalid TaskStatus: {}", s)),
        }
    }
}

/// Status of a TODO item.
///
/// TODOs progress through different states during their lifecycle.
/// Reopening a completed or cancelled TODO (transition back to `Pending`) is not allowed;
/// add a new TODO instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TodoStatus {
    /// TODO is pending and needs to be completed
    Pending,
    /// TODO has been completed
    Done,
    /// TODO has been cancelled and will not be completed
    Cancelled,
}

impl TodoStatus {
    pub const PENDING: &'static str = "pending";
    pub const DONE: &'static str = "done";
    pub const CANCELLED: &'static str = "cancelled";

    /// Converts the status to its string representation.
    pub fn as_str(&self) -> &str {
        match self {
            TodoStatus::Pending => Self::PENDING,
            TodoStatus::Done => Self::DONE,
            TodoStatus::Cancelled => Self::CANCELLED,
        }
    }

    /// Returns whether a transition from `self` to `target` is allowed.
    pub fn can_transition_to(self, target: Self) -> bool {
        if Self::is_reopen_attempt(self, target) {
            return false;
        }

        matches!(
            (self, target),
            (TodoStatus::Pending, TodoStatus::Pending)
                | (TodoStatus::Pending, TodoStatus::Done)
                | (TodoStatus::Pending, TodoStatus::Cancelled)
                | (TodoStatus::Done, TodoStatus::Done)
                | (TodoStatus::Cancelled, TodoStatus::Cancelled)
        )
    }

    /// Returns true when moving a terminal TODO back to pending (reopen).
    pub fn is_reopen_attempt(from: Self, to: Self) -> bool {
        from != Self::Pending && to == Self::Pending
    }

    /// Returns true when the TODO can no longer be worked on.
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Done | Self::Cancelled)
    }
}

impl FromStr for TodoStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            Self::PENDING => Ok(TodoStatus::Pending),
            Self::DONE => Ok(TodoStatus::Done),
            Self::CANCELLED => Ok(TodoStatus::Cancelled),
            _ => Err(format!("Invalid TodoStatus: {}", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_status_as_str() {
        assert_eq!(TaskStatus::Active.as_str(), "active");
        assert_eq!(TaskStatus::Archived.as_str(), "archived");
    }

    #[test]
    fn test_task_status_from_str() {
        assert!(matches!(
            "active".parse::<TaskStatus>(),
            Ok(TaskStatus::Active)
        ));
        assert!(matches!(
            "archived".parse::<TaskStatus>(),
            Ok(TaskStatus::Archived)
        ));
        assert!("invalid".parse::<TaskStatus>().is_err());
    }

    #[test]
    fn test_task_status_transitions() {
        assert!(TaskStatus::Active.can_transition_to(TaskStatus::Archived));
        assert!(!TaskStatus::Archived.can_transition_to(TaskStatus::Active));
        assert!(TaskStatus::Archived.can_transition_to(TaskStatus::Archived));
    }

    #[test]
    fn test_todo_status_as_str() {
        assert_eq!(TodoStatus::Pending.as_str(), "pending");
        assert_eq!(TodoStatus::Done.as_str(), "done");
        assert_eq!(TodoStatus::Cancelled.as_str(), "cancelled");
    }

    #[test]
    fn test_todo_status_from_str() {
        assert!(matches!(
            "pending".parse::<TodoStatus>(),
            Ok(TodoStatus::Pending)
        ));
        assert!(matches!("done".parse::<TodoStatus>(), Ok(TodoStatus::Done)));
        assert!(matches!(
            "cancelled".parse::<TodoStatus>(),
            Ok(TodoStatus::Cancelled)
        ));
        assert!("invalid".parse::<TodoStatus>().is_err());
    }

    #[test]
    fn test_todo_status_reopen_is_forbidden() {
        assert!(TodoStatus::is_reopen_attempt(
            TodoStatus::Done,
            TodoStatus::Pending
        ));
        assert!(TodoStatus::is_reopen_attempt(
            TodoStatus::Cancelled,
            TodoStatus::Pending
        ));
        assert!(!TodoStatus::is_reopen_attempt(
            TodoStatus::Pending,
            TodoStatus::Pending
        ));
        assert!(!TodoStatus::can_transition_to(
            TodoStatus::Done,
            TodoStatus::Pending
        ));
        assert!(!TodoStatus::can_transition_to(
            TodoStatus::Cancelled,
            TodoStatus::Pending
        ));
        assert!(TodoStatus::Done.is_terminal());
        assert!(TodoStatus::Cancelled.is_terminal());
        assert!(!TodoStatus::Pending.is_terminal());
    }
}
