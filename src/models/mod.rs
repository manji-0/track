use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Task {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub ticket_id: Option<String>,
    pub ticket_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Todo {
    pub id: i64,
    #[allow(dead_code)]
    pub task_id: i64,
    pub task_index: i64,
    pub content: String,
    pub status: String,
    #[serde(default)]
    pub worktree_requested: bool,
    #[allow(dead_code)]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Link {
    #[allow(dead_code)]
    pub id: i64,
    #[allow(dead_code)]
    pub task_id: i64,
    pub url: String,
    pub title: String,
    #[allow(dead_code)]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Scrap {
    #[allow(dead_code)]
    pub id: i64,
    #[allow(dead_code)]
    pub task_id: i64,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GitItem {
    pub id: i64,
    pub task_id: i64,
    pub path: String,
    pub branch: String,
    pub base_repo: Option<String>,
    #[allow(dead_code)]
    pub status: String,
    #[allow(dead_code)]
    pub created_at: DateTime<Utc>,
    #[allow(dead_code)]
    pub todo_id: Option<i64>,
    #[allow(dead_code)]
    pub is_base: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct RepoLink {
    #[allow(dead_code)]
    pub id: i64,
    #[allow(dead_code)]
    pub git_item_id: i64,
    pub url: String,
    pub kind: String,
    #[allow(dead_code)]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskRepo {
    pub id: i64,
    #[allow(dead_code)]
    pub task_id: i64,
    pub repo_path: String,
    #[allow(dead_code)]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub enum TaskStatus {
    Active,
    Archived,
}

impl TaskStatus {
    pub fn as_str(&self) -> &str {
        match self {
            TaskStatus::Active => "active",
            TaskStatus::Archived => "archived",
        }
    }

    #[allow(dead_code)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "active" => Some(TaskStatus::Active),
            "archived" => Some(TaskStatus::Archived),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum TodoStatus {
    Pending,
    Done,
    Cancelled,
}

impl TodoStatus {
    pub fn as_str(&self) -> &str {
        match self {
            TodoStatus::Pending => "pending",
            TodoStatus::Done => "done",
            TodoStatus::Cancelled => "cancelled",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(TodoStatus::Pending),
            "done" => Some(TodoStatus::Done),
            "cancelled" => Some(TodoStatus::Cancelled),
            _ => None,
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
        assert!(matches!(TaskStatus::from_str("active"), Some(TaskStatus::Active)));
        assert!(matches!(TaskStatus::from_str("archived"), Some(TaskStatus::Archived)));
        assert!(TaskStatus::from_str("invalid").is_none());
    }

    #[test]
    fn test_todo_status_as_str() {
        assert_eq!(TodoStatus::Pending.as_str(), "pending");
        assert_eq!(TodoStatus::Done.as_str(), "done");
        assert_eq!(TodoStatus::Cancelled.as_str(), "cancelled");
    }

    #[test]
    fn test_todo_status_from_str() {
        assert!(matches!(TodoStatus::from_str("pending"), Some(TodoStatus::Pending)));
        assert!(matches!(TodoStatus::from_str("done"), Some(TodoStatus::Done)));
        assert!(matches!(TodoStatus::from_str("cancelled"), Some(TodoStatus::Cancelled)));
        assert!(TodoStatus::from_str("invalid").is_none());
    }
}

