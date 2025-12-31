use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct Task {
    pub id: i64,
    pub name: String,
    pub status: String,
    pub ticket_id: Option<String>,
    pub ticket_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct Todo {
    pub id: i64,
    pub task_id: i64,
    pub content: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct Link {
    pub id: i64,
    pub task_id: i64,
    pub url: String,
    pub title: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct Scrap {
    pub id: i64,
    pub task_id: i64,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct GitItem {
    pub id: i64,
    pub task_id: i64,
    pub path: String,
    pub branch: String,
    pub base_repo: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub todo_id: Option<i64>,
    pub is_base: bool,
}

#[derive(Debug, Clone)]
pub struct RepoLink {
    pub id: i64,
    pub git_item_id: i64,
    pub url: String,
    pub kind: String,
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
