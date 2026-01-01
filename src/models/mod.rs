use chrono::{DateTime, Utc};
use serde::Serialize;
use std::str::FromStr;

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
}

impl FromStr for TaskStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" => Ok(TaskStatus::Active),
            "archived" => Ok(TaskStatus::Archived),
            _ => Err(format!("Invalid TaskStatus: {}", s)),
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
}

impl FromStr for TodoStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(TodoStatus::Pending),
            "done" => Ok(TodoStatus::Done),
            "cancelled" => Ok(TodoStatus::Cancelled),
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
}
