use super::alias::TaskAlias;
use super::http_url::HttpUrl;
use super::ids::{
    LinkId, LinkIndex, RepoIndex, RepoLinkId, ScrapId, ScrapIndex, TaskId, TaskRepoId, TodoId,
    TodoIndex, WorktreeId,
};
use super::markdown::render_markdown_with_links;
use super::status::{TaskStatus, TodoStatus};
use super::ticket::TicketId;
use chrono::{DateTime, Utc};
use serde::Serialize;

/// Represents a development task.
///
/// A task is the primary organizational unit in track. Each task can have multiple TODOs,
/// links, scraps, and associated JJ repositories.
#[derive(Debug, Clone, Serialize)]
pub struct Task {
    pub id: TaskId,
    pub name: String,
    pub description: Option<String>,
    pub status: TaskStatus,
    pub ticket_id: Option<TicketId>,
    pub ticket_url: Option<String>,
    pub alias: Option<TaskAlias>,
    pub is_today_task: bool,
    pub created_at: DateTime<Utc>,
}

/// Represents a TODO item within a task.
///
/// TODOs are task-scoped action items. Each TODO has a task-specific index
/// and can optionally request a JJ workspace for isolated development.
#[derive(Debug, Clone, Serialize)]
pub struct Todo {
    #[serde(skip)]
    pub id: TodoId,
    #[serde(skip)]
    #[allow(dead_code)]
    pub task_id: TaskId,
    /// Task-scoped sequential ID for this TODO
    #[serde(rename = "todo_id")]
    pub task_index: TodoIndex,
    pub content: String,
    pub status: TodoStatus,
    #[serde(skip)]
    pub worktree_requested: bool,
    /// When false, this TODO does not require a jj-task/git workspace (e.g. research).
    #[serde(skip)]
    pub requires_workspace: bool,
    #[serde(skip)]
    #[allow(dead_code)]
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl Todo {
    /// Converts the todo content from markdown to HTML.
    pub fn content_html(&self) -> String {
        render_markdown_with_links(&self.content)
    }
}

/// Represents a link associated with a task.
#[derive(Debug, Clone, Serialize)]
pub struct Link {
    #[serde(skip)]
    #[allow(dead_code)]
    pub id: LinkId,
    #[serde(skip)]
    #[allow(dead_code)]
    pub task_id: TaskId,
    /// Task-scoped sequential ID for this link
    #[serde(rename = "link_id")]
    pub task_index: LinkIndex,
    pub url: HttpUrl,
    pub title: String,
    #[serde(skip)]
    #[allow(dead_code)]
    pub created_at: DateTime<Utc>,
}

/// Represents a scrap (work note) for a task.
#[derive(Debug, Clone, Serialize)]
pub struct Scrap {
    #[serde(skip)]
    #[allow(dead_code)]
    pub id: ScrapId,
    #[serde(skip)]
    #[allow(dead_code)]
    pub task_id: TaskId,
    /// Task-scoped sequential ID for this scrap
    pub scrap_id: ScrapIndex,
    pub content: String,
    pub created_at: DateTime<Utc>,
    /// The task-scoped index of the active (oldest pending) TODO when this scrap was created
    pub active_todo_id: Option<TodoIndex>,
}

impl Scrap {
    /// Converts the scrap content from markdown to HTML.
    pub fn content_html(&self) -> String {
        render_markdown_with_links(&self.content)
    }
}

/// Represents a JJ workspace associated with a task or TODO.
#[derive(Debug, Clone, Serialize)]
pub struct Worktree {
    pub id: WorktreeId,
    pub task_id: TaskId,
    pub path: String,
    pub branch: String,
    pub base_repo: Option<String>,
    #[allow(dead_code)]
    pub status: String,
    #[allow(dead_code)]
    pub created_at: DateTime<Utc>,
    #[allow(dead_code)]
    pub todo_id: Option<TodoId>,
    #[allow(dead_code)]
    pub is_base: bool,
}

/// Represents a remote repository link for a worktree.
#[derive(Debug, Clone, Serialize)]
pub struct RepoLink {
    #[allow(dead_code)]
    pub id: RepoLinkId,
    #[allow(dead_code)]
    pub worktree_id: WorktreeId,
    pub url: String,
    pub kind: String,
    #[allow(dead_code)]
    pub created_at: DateTime<Utc>,
}

/// Represents a repository associated with a task.
#[derive(Debug, Clone, Serialize)]
pub struct TaskRepo {
    #[serde(skip)]
    pub id: TaskRepoId,
    #[serde(skip)]
    #[allow(dead_code)]
    pub task_id: TaskId,
    /// Task-scoped sequential ID for this repository
    #[serde(rename = "repo_id")]
    pub task_index: RepoIndex,
    pub repo_path: String,
    pub base_branch: Option<String>,
    pub base_commit_hash: Option<String>,
    #[serde(skip)]
    #[allow(dead_code)]
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_scrap(content: &str) -> Scrap {
        Scrap {
            id: ScrapId::from_i64(1),
            task_id: TaskId::from_i64(1),
            scrap_id: ScrapIndex::from_i64(1),
            content: content.to_string(),
            created_at: Utc::now(),
            active_todo_id: None,
        }
    }

    fn sample_todo(content: &str) -> Todo {
        Todo {
            id: TodoId::from_i64(1),
            task_id: TaskId::from_i64(1),
            task_index: TodoIndex::from_i64(1),
            content: content.to_string(),
            status: TodoStatus::Pending,
            worktree_requested: false,
            requires_workspace: true,
            created_at: Utc::now(),
            completed_at: None,
        }
    }

    #[test]
    fn test_scrap_content_html_plain_text() {
        let html = sample_scrap("This is a plain text scrap.").content_html();
        assert!(html.contains("<p>This is a plain text scrap.</p>"));
    }

    #[test]
    fn test_scrap_content_html_with_markdown() {
        let html = sample_scrap("# Heading\n\nThis is **bold** and *italic*.").content_html();
        assert!(html.contains("<h1>Heading</h1>"));
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<em>italic</em>"));
    }

    #[test]
    fn test_scrap_content_html_with_code() {
        let html = sample_scrap("Inline `code` and:\n\n```rust\nfn main() {}\n```").content_html();
        assert!(html.contains("<code>code</code>"));
        assert!(html.contains("<pre><code"));
        assert!(html.contains("fn main() {}"));
    }

    #[test]
    fn test_scrap_content_html_with_list() {
        let html = sample_scrap("- Item 1\n- Item 2\n- Item 3").content_html();
        assert!(html.contains("<ul>"));
        assert!(html.contains("<li>Item 1</li>"));
        assert!(html.contains("<li>Item 2</li>"));
        assert!(html.contains("<li>Item 3</li>"));
        assert!(html.contains("</ul>"));
    }

    #[test]
    fn test_scrap_content_html_with_link() {
        let html = sample_scrap("[Example](https://example.com)").content_html();
        assert!(html.contains("href=\"https://example.com\""));
        assert!(html.contains("target=\"_blank\""));
        assert!(html.contains("rel=\"noopener noreferrer\""));
    }

    #[test]
    fn test_scrap_content_html_sanitizes_html() {
        let html = sample_scrap("<script>alert('x')</script><b>safe</b>").content_html();
        assert!(!html.contains("<script>"));
        assert!(!html.contains("alert('x')"));
        assert!(html.contains("safe"));
    }

    #[test]
    fn test_scrap_content_html_auto_linkify_plain_url() {
        let html = sample_scrap("Check out https://example.com for more info.").content_html();
        assert!(html.contains("target=\"_blank\""));
        assert!(html.contains("https://example.com"));
    }

    #[test]
    fn test_scrap_content_html_auto_linkify_multiple_urls() {
        let html = sample_scrap("See https://example.com and http://test.org").content_html();
        assert!(html.contains("target=\"_blank\""));
        assert!(html.contains("https://example.com"));
        assert!(html.contains("http://test.org"));
    }

    #[test]
    fn test_scrap_content_html_auto_linkify_url_with_punctuation() {
        let html =
            sample_scrap("Visit https://example.com/path?query=1, it's great!").content_html();
        assert!(html.contains("target=\"_blank\""));
        assert!(html.contains("https://example.com/path?query=1"));
    }

    #[test]
    fn test_scrap_content_html_preserve_markdown_links() {
        let html = sample_scrap("Check [my site](https://example.com) and also https://test.com")
            .content_html();
        assert!(html.contains("target=\"_blank\""));
        assert!(html.contains("my site"));
        assert!(html.contains("https://test.com"));
    }

    #[test]
    fn test_todo_content_html_plain_text() {
        let html = sample_todo("This is a plain text todo.").content_html();
        assert!(html.contains("<p>This is a plain text todo.</p>"));
    }

    #[test]
    fn test_todo_content_html_auto_linkify_url() {
        let html = sample_todo("Check https://example.com for details").content_html();
        assert!(html.contains("target=\"_blank\""));
        assert!(html.contains("https://example.com"));
    }

    #[test]
    fn test_todo_content_html_with_markdown() {
        let html = sample_todo("# Heading\n\nThis is **bold** and *italic*.").content_html();
        assert!(html.contains("<h1>Heading</h1>"));
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<em>italic</em>"));
    }

    #[test]
    fn test_todo_content_html_sanitizes_html() {
        let html = sample_todo("<img src=x onerror=alert(1)><b>safe</b>").content_html();
        assert!(!html.contains("<img"));
        assert!(html.contains("safe"));
    }
}
