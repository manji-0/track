//! Data models for the track CLI application.
//!
//! This module defines the core data structures used throughout the application,
//! including tasks, TODOs, links, scraps, and Git-related items.

use chrono::{DateTime, Utc};
use serde::Serialize;
use std::str::FromStr;

/// Represents a development task.
///
/// A task is the primary organizational unit in track. Each task can have multiple TODOs,
/// links, scraps, and associated Git repositories.
#[derive(Debug, Clone, Serialize)]
pub struct Task {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub ticket_id: Option<String>,
    pub ticket_url: Option<String>,
    pub alias: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Represents a TODO item within a task.
///
/// TODOs are task-scoped action items. Each TODO has a task-specific index
/// and can optionally request a Git worktree for isolated development.
#[derive(Debug, Clone, Serialize)]
pub struct Todo {
    #[serde(skip)]
    pub id: i64,
    #[serde(skip)]
    #[allow(dead_code)]
    pub task_id: i64,
    /// Task-scoped sequential ID for this TODO
    #[serde(rename = "todo_id")]
    pub task_index: i64,
    pub content: String,
    pub status: String,
    #[serde(skip)]
    pub worktree_requested: bool,
    #[serde(skip)]
    #[allow(dead_code)]
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl Todo {
    /// Converts the todo content from markdown to HTML.
    ///
    /// This method uses pulldown-cmark to parse the markdown content
    /// and render it as HTML. Plain URLs are automatically converted to clickable links.
    /// All links open in new tabs with target="_blank".
    /// The output is safe for display in web pages.
    pub fn content_html(&self) -> String {
        use pulldown_cmark::{html, Event, Parser, Tag, TagEnd};
        use regex::Regex;

        // Auto-linkify plain URLs that aren't already in markdown link syntax
        // This regex matches URLs and ensures trailing punctuation is not included
        // It allows dots, question marks, etc. within the URL but not at the end
        let url_regex = Regex::new(
            r"(?P<pre>^|[\s\(])(?P<url>https?://[^\s\)<>]+?)(?P<post>[.,;!?]*(?:[\s\)]|$))",
        )
        .unwrap();

        let linkified = url_regex.replace_all(&self.content, |caps: &regex::Captures| {
            let pre = &caps["pre"];
            let url = &caps["url"];
            let post = &caps["post"];

            // Check if this URL is already part of a markdown link [text](url)
            // by looking backwards in the original content
            let cap_start = caps.get(0).unwrap().start();
            if cap_start >= 2 {
                let before = &self.content[..cap_start];
                if before.ends_with("](") {
                    // This is already a markdown link, don't modify
                    return caps.get(0).unwrap().as_str().to_string();
                }
            }

            format!("{}<{}>{}",pre, url, post)
        });

        // Parse markdown and modify link tags to add target="_blank"
        let parser = Parser::new(linkified.as_ref());
        let parser_with_target = parser.map(|event| match event {
            Event::Start(Tag::Link { link_type: _, dest_url, title, id: _ }) => {
                // Create a new link tag with target="_blank" by wrapping in Html event
                // We'll use the original tag and add attributes via HTML
                Event::Html(format!(
                    r#"<a href="{}" target="_blank" rel="noopener noreferrer"{}>"#,
                    html_escape::encode_double_quoted_attribute(&dest_url),
                    if !title.is_empty() {
                        format!(r#" title="{}""#, html_escape::encode_double_quoted_attribute(&title),)
                    } else {
                        String::new()
                    }
                ).into())
            }
            Event::End(TagEnd::Link) => {
                Event::Html("</a>".into())
            }
            _ => event,
        });

        let mut html_output = String::new();
        html::push_html(&mut html_output, parser_with_target);
        html_output
    }
}

/// Represents a link associated with a task.
///
/// Links are URLs with titles that provide context or reference material for a task.
#[derive(Debug, Clone, Serialize)]
pub struct Link {
    #[serde(skip)]
    #[allow(dead_code)]
    pub id: i64,
    #[serde(skip)]
    #[allow(dead_code)]
    pub task_id: i64,
    /// Task-scoped sequential ID for this link
    #[serde(rename = "link_id")]
    pub task_index: i64,
    pub url: String,
    pub title: String,
    #[serde(skip)]
    #[allow(dead_code)]
    pub created_at: DateTime<Utc>,
}

/// Represents a scrap (work note) for a task.
///
/// Scraps are chronological notes that capture progress, decisions, and findings
/// during task execution. They help maintain context and flow of work.
#[derive(Debug, Clone, Serialize)]
pub struct Scrap {
    #[serde(skip)]
    #[allow(dead_code)]
    pub id: i64,
    #[serde(skip)]
    #[allow(dead_code)]
    pub task_id: i64,
    /// Task-scoped sequential ID for this scrap
    pub scrap_id: i64,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

impl Scrap {
    /// Converts the scrap content from markdown to HTML.
    ///
    /// This method uses pulldown-cmark to parse the markdown content
    /// and render it as HTML. Plain URLs are automatically converted to clickable links.
    /// All links open in new tabs with target="_blank".
    /// The output is safe for display in web pages.
    pub fn content_html(&self) -> String {
        use pulldown_cmark::{html, Event, Parser, Tag, TagEnd};
        use regex::Regex;

        // Auto-linkify plain URLs that aren't already in markdown link syntax
        // This regex matches URLs and ensures trailing punctuation is not included
        // It allows dots, question marks, etc. within the URL but not at the end
        let url_regex = Regex::new(
            r"(?P<pre>^|[\s\(])(?P<url>https?://[^\s\)<>]+?)(?P<post>[.,;!?]*(?:[\s\)]|$))",
        )
        .unwrap();

        let linkified = url_regex.replace_all(&self.content, |caps: &regex::Captures| {
            let pre = &caps["pre"];
            let url = &caps["url"];
            let post = &caps["post"];

            // Check if this URL is already part of a markdown link [text](url)
            // by looking backwards in the original content
            let cap_start = caps.get(0).unwrap().start();
            if cap_start >= 2 {
                let before = &self.content[..cap_start];
                if before.ends_with("](") {
                    // This is already a markdown link, don't modify
                    return caps.get(0).unwrap().as_str().to_string();
                }
            }

            format!("{}<{}>{}",pre, url, post)
        });

        // Parse markdown and modify link tags to add target="_blank"
        let parser = Parser::new(linkified.as_ref());
        let parser_with_target = parser.map(|event| match event {
            Event::Start(Tag::Link { link_type: _, dest_url, title, id: _ }) => {
                // Create a new link tag with target="_blank" by wrapping in Html event
                // We'll use the original tag and add attributes via HTML
                Event::Html(format!(
                    r#"<a href="{}" target="_blank" rel="noopener noreferrer"{}>"#,
                    html_escape::encode_double_quoted_attribute(&dest_url),
                    if !title.is_empty() {
                        format!(r#" title="{}""#, html_escape::encode_double_quoted_attribute(&title))
                    } else {
                        String::new()
                    }
                ).into())
            }
            Event::End(TagEnd::Link) => {
                Event::Html("</a>".into())
            }
            _ => event,
        });

        let mut html_output = String::new();
        html::push_html(&mut html_output, parser_with_target);
        html_output
    }
}

/// Represents a Git worktree associated with a task or TODO.
///
/// Worktrees track both base repositories and TODO-specific worktrees,
/// including their paths, branches, and relationships.
#[derive(Debug, Clone, Serialize)]
pub struct Worktree {
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

/// Represents a remote repository link for a worktree.
///
/// RepoLinks store URLs to remote repositories (e.g., GitHub, GitLab)
/// and their types (e.g., "origin", "upstream").
#[derive(Debug, Clone, Serialize)]
pub struct RepoLink {
    #[allow(dead_code)]
    pub id: i64,
    #[allow(dead_code)]
    pub worktree_id: i64,
    pub url: String,
    pub kind: String,
    #[allow(dead_code)]
    pub created_at: DateTime<Utc>,
}

/// Represents a repository associated with a task.
///
/// TaskRepos link Git repositories to tasks, allowing multiple repositories
/// to be managed within a single task context.
#[derive(Debug, Clone, Serialize)]
pub struct TaskRepo {
    #[serde(skip)]
    pub id: i64,
    #[serde(skip)]
    #[allow(dead_code)]
    pub task_id: i64,
    /// Task-scoped sequential ID for this repository
    #[serde(rename = "repo_id")]
    pub task_index: i64,
    pub repo_path: String,
    pub base_branch: Option<String>,
    pub base_commit_hash: Option<String>,
    #[serde(skip)]
    #[allow(dead_code)]
    pub created_at: DateTime<Utc>,
}

/// Status of a task.
///
/// Tasks can be either active (currently being worked on) or archived (completed or abandoned).
#[derive(Debug, Clone)]
pub enum TaskStatus {
    /// Task is currently active and can be worked on
    Active,
    /// Task has been archived and is no longer active
    Archived,
}

impl TaskStatus {
    /// Converts the status to its string representation.
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

/// Status of a TODO item.
///
/// TODOs progress through different states during their lifecycle.
#[derive(Debug, Clone)]
pub enum TodoStatus {
    /// TODO is pending and needs to be completed
    Pending,
    /// TODO has been completed
    Done,
    /// TODO has been cancelled and will not be completed
    Cancelled,
}

impl TodoStatus {
    /// Converts the status to its string representation.
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

    #[test]
    fn test_scrap_content_html_plain_text() {
        let scrap = Scrap {
            id: 1,
            task_id: 1,
            scrap_id: 1,
            content: "This is a plain text scrap.".to_string(),
            created_at: Utc::now(),
        };
        let html = scrap.content_html();
        assert!(html.contains("<p>This is a plain text scrap.</p>"));
    }

    #[test]
    fn test_scrap_content_html_with_markdown() {
        let scrap = Scrap {
            id: 1,
            task_id: 1,
            scrap_id: 1,
            content: "# Heading\n\nThis is **bold** and *italic*.".to_string(),
            created_at: Utc::now(),
        };
        let html = scrap.content_html();
        assert!(html.contains("<h1>Heading</h1>"));
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<em>italic</em>"));
    }

    #[test]
    fn test_scrap_content_html_with_code() {
        let scrap = Scrap {
            id: 1,
            task_id: 1,
            scrap_id: 1,
            content: "Inline `code` and:\n\n```rust\nfn main() {}\n```".to_string(),
            created_at: Utc::now(),
        };
        let html = scrap.content_html();
        assert!(html.contains("<code>code</code>"));
        assert!(html.contains("<pre><code"));
        assert!(html.contains("fn main() {}"));
    }

    #[test]
    fn test_scrap_content_html_with_list() {
        let scrap = Scrap {
            id: 1,
            task_id: 1,
            scrap_id: 1,
            content: "- Item 1\n- Item 2\n- Item 3".to_string(),
            created_at: Utc::now(),
        };
        let html = scrap.content_html();
        assert!(html.contains("<ul>"));
        assert!(html.contains("<li>Item 1</li>"));
        assert!(html.contains("<li>Item 2</li>"));
        assert!(html.contains("<li>Item 3</li>"));
        assert!(html.contains("</ul>"));
    }

    #[test]
    fn test_scrap_content_html_with_link() {
        let scrap = Scrap {
            id: 1,
            task_id: 1,
            scrap_id: 1,
            content: "Check out [this link](https://example.com).".to_string(),
            created_at: Utc::now(),
        };
        let html = scrap.content_html();
        assert!(html.contains("<a href=\"https://example.com\">this link</a>"));
    }

    #[test]
    fn test_scrap_content_html_auto_linkify_plain_url() {
        let scrap = Scrap {
            id: 1,
            task_id: 1,
            scrap_id: 1,
            content: "Check out https://example.com for more info.".to_string(),
            created_at: Utc::now(),
        };
        let html = scrap.content_html();
        assert!(html.contains("<a href=\"https://example.com\">https://example.com</a>"));
    }

    #[test]
    fn test_scrap_content_html_auto_linkify_multiple_urls() {
        let scrap = Scrap {
            id: 1,
            task_id: 1,
            scrap_id: 1,
            content: "See https://example.com and http://test.org".to_string(),
            created_at: Utc::now(),
        };
        let html = scrap.content_html();
        assert!(html.contains("<a href=\"https://example.com\">https://example.com</a>"));
        assert!(html.contains("<a href=\"http://test.org\">http://test.org</a>"));
    }

    #[test]
    fn test_scrap_content_html_auto_linkify_url_with_punctuation() {
        let scrap = Scrap {
            id: 1,
            task_id: 1,
            scrap_id: 1,
            content: "Visit https://example.com/path?query=1, it's great!".to_string(),
            created_at: Utc::now(),
        };
        let html = scrap.content_html();
        // The comma should not be part of the link
        assert!(html.contains(
            "<a href=\"https://example.com/path?query=1\">https://example.com/path?query=1</a>"
        ));
    }

    #[test]
    fn test_scrap_content_html_preserve_markdown_links() {
        let scrap = Scrap {
            id: 1,
            task_id: 1,
            scrap_id: 1,
            content: "Check [my site](https://example.com) and also https://test.com".to_string(),
            created_at: Utc::now(),
        };
        let html = scrap.content_html();
        // Markdown link should work normally
        assert!(html.contains("<a href=\"https://example.com\">my site</a>"));
        // Plain URL should be auto-linkified
        assert!(html.contains("<a href=\"https://test.com\">https://test.com</a>"));
    }

    #[test]
    fn test_todo_content_html_plain_text() {
        let todo = Todo {
            id: 1,
            task_id: 1,
            task_index: 1,
            content: "This is a plain text todo.".to_string(),
            status: "pending".to_string(),
            worktree_requested: false,
            created_at: Utc::now(),
            completed_at: None,
        };
        let html = todo.content_html();
        assert!(html.contains("<p>This is a plain text todo.</p>"));
    }

    #[test]
    fn test_todo_content_html_auto_linkify_url() {
        let todo = Todo {
            id: 1,
            task_id: 1,
            task_index: 1,
            content: "Check https://example.com for details".to_string(),
            status: "pending".to_string(),
            worktree_requested: false,
            created_at: Utc::now(),
            completed_at: None,
        };
        let html = todo.content_html();
        assert!(html.contains("<a href=\"https://example.com\">https://example.com</a>"));
    }

    #[test]
    fn test_todo_content_html_with_markdown() {
        let todo = Todo {
            id: 1,
            task_id: 1,
            task_index: 1,
            content: "Fix **bug** in login".to_string(),
            status: "pending".to_string(),
            worktree_requested: false,
            created_at: Utc::now(),
            completed_at: None,
        };
        let html = todo.content_html();
        assert!(html.contains("<strong>bug</strong>"));
    }
}
