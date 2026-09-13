//! Versioned task snapshot stored in `refs/notes/track`.
//!
//! This is a boundary DTO (git notes JSON), not a domain entity. Parse with
//! [`parse_notes`], then convert into domain values at the import use case.

use crate::models::{Link, Scrap, Task, Todo};
use crate::utils::{Result, TrackError};
use serde::{Deserialize, Serialize};

pub const FORMAT: &str = "track-task";
pub const VERSION: u32 = 3;
pub const TODO_FORMAT: &str = "track-todo";
pub const TODO_VERSION: u32 = 1;
pub const TASK_TODO_TRAILER: &str = "Task-Todo";
pub const TASK_SLUG_TRAILER: &str = "Task-Slug";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskNotesDto {
    pub format: String,
    pub version: u32,
    pub slug: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ticket_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ticket_url: Option<String>,
    #[serde(default)]
    pub todos: Vec<TodoNotesDto>,
    #[serde(default)]
    pub links: Vec<LinkNotesDto>,
    #[serde(default)]
    pub scraps: Vec<ScrapNotesDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TodoNotesDto {
    pub index: i64,
    pub content: String,
    pub status: String,
    #[serde(default = "default_requires_workspace")]
    pub requires_workspace: bool,
}

fn default_requires_workspace() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LinkNotesDto {
    pub index: i64,
    pub url: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScrapNotesDto {
    pub index: i64,
    pub content: String,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_todo_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TodoCommitNotesDto {
    pub format: String,
    pub version: u32,
    pub slug: String,
    pub todo_index: i64,
    pub content: String,
    pub status: String,
    #[serde(default)]
    pub scraps: Vec<ScrapNotesDto>,
}

/// Task identity on the marker. TODOs and scraps live on per-TODO commits.
pub fn build_marker_snapshot(slug: &str, task: &Task, links: &[Link]) -> TaskNotesDto {
    TaskNotesDto {
        format: FORMAT.to_string(),
        version: VERSION,
        slug: slug.to_string(),
        name: task.name.clone(),
        description: task.description.clone(),
        ticket_id: task.ticket_id.as_ref().map(|t| t.to_string()),
        ticket_url: task.ticket_url.clone(),
        todos: Vec::new(),
        links: links
            .iter()
            .map(|link| LinkNotesDto {
                index: link.task_index.as_i64(),
                url: link.url.to_string(),
                title: link.title.clone(),
            })
            .collect(),
        scraps: Vec::new(),
    }
}

pub fn build_todo_notes(slug: &str, todo: &Todo, scraps: &[Scrap]) -> TodoCommitNotesDto {
    TodoCommitNotesDto {
        format: TODO_FORMAT.to_string(),
        version: TODO_VERSION,
        slug: slug.to_string(),
        todo_index: todo.task_index.as_i64(),
        content: todo.content.clone(),
        status: todo.status.as_str().to_string(),
        scraps: scraps
            .iter()
            .filter(|scrap| {
                scrap.visibility.is_shared() && scrap.active_todo_id == Some(todo.task_index)
            })
            .map(|scrap| ScrapNotesDto {
                index: scrap.scrap_id.as_i64(),
                content: scrap.content.clone(),
                created_at: scrap.created_at.to_rfc3339(),
                active_todo_id: scrap.active_todo_id.map(|id| id.as_i64()),
            })
            .collect(),
    }
}

pub fn todo_commit_message(index: i64, content: &str, slug: &str) -> String {
    format!("{content}\n\n{TASK_TODO_TRAILER}: {index}\n{TASK_SLUG_TRAILER}: {slug}\n")
}

pub fn parse_task_todo_index(message: &str) -> Option<i64> {
    for line in message.lines() {
        let line = line.trim();
        if let Some(rest) = line
            .strip_prefix("Task-Todo:")
            .or_else(|| line.strip_prefix("task-todo:"))
        {
            return rest.trim().parse().ok();
        }
    }
    None
}

/// Legacy whole-task blob (v2). Kept for import of older notes.
pub fn build_snapshot(
    slug: &str,
    task: &Task,
    todos: &[Todo],
    links: &[Link],
    scraps: &[Scrap],
) -> TaskNotesDto {
    TaskNotesDto {
        format: FORMAT.to_string(),
        version: VERSION,
        slug: slug.to_string(),
        name: task.name.clone(),
        description: task.description.clone(),
        ticket_id: task.ticket_id.as_ref().map(|t| t.to_string()),
        ticket_url: task.ticket_url.clone(),
        todos: todos
            .iter()
            .map(|todo| TodoNotesDto {
                index: todo.task_index.as_i64(),
                content: todo.content.clone(),
                status: todo.status.as_str().to_string(),
                requires_workspace: todo.requires_workspace,
            })
            .collect(),
        links: links
            .iter()
            .map(|link| LinkNotesDto {
                index: link.task_index.as_i64(),
                url: link.url.to_string(),
                title: link.title.clone(),
            })
            .collect(),
        scraps: scraps
            .iter()
            .filter(|scrap| scrap.visibility.is_shared())
            .map(|scrap| ScrapNotesDto {
                index: scrap.scrap_id.as_i64(),
                content: scrap.content.clone(),
                created_at: scrap.created_at.to_rfc3339(),
                active_todo_id: scrap.active_todo_id.map(|id| id.as_i64()),
            })
            .collect(),
    }
}

pub fn format_snapshot(snapshot: &TaskNotesDto) -> Result<String> {
    serde_json::to_string_pretty(snapshot)
        .map_err(|err| TrackError::SerializationFailed(err.to_string()))
}

/// Parses v2 JSON, or the legacy `track-scraps v1` text format.
pub fn parse_notes(body: &str) -> Result<TaskNotesDto> {
    let trimmed = body.trim();
    if trimmed.starts_with('{') {
        return parse_v2(trimmed);
    }
    if trimmed.starts_with("track-scraps v1") {
        return parse_v1(trimmed);
    }
    Err(TrackError::TaskNotesParse(
        "expected track-task JSON or track-scraps v1".to_string(),
    ))
}

fn parse_v2(body: &str) -> Result<TaskNotesDto> {
    let value: serde_json::Value =
        serde_json::from_str(body).map_err(|err| TrackError::TaskNotesParse(err.to_string()))?;
    if value.get("format").and_then(|v| v.as_str()) == Some(TODO_FORMAT) {
        return Err(TrackError::TaskNotesParse(
            "track-todo notes are not a task snapshot".to_string(),
        ));
    }
    let dto: TaskNotesDto =
        serde_json::from_value(value).map_err(|err| TrackError::TaskNotesParse(err.to_string()))?;
    if dto.format != FORMAT {
        return Err(TrackError::TaskNotesParse(format!(
            "unknown format '{}'",
            dto.format
        )));
    }
    if dto.version < 2 {
        return Err(TrackError::TaskNotesParse(format!(
            "unsupported version {}",
            dto.version
        )));
    }
    if dto.slug.trim().is_empty() || dto.name.trim().is_empty() {
        return Err(TrackError::TaskNotesParse(
            "slug and name are required".to_string(),
        ));
    }
    Ok(dto)
}

pub fn format_todo_notes(notes: &TodoCommitNotesDto) -> Result<String> {
    serde_json::to_string_pretty(notes)
        .map_err(|err| TrackError::SerializationFailed(err.to_string()))
}

pub fn parse_todo_notes(body: &str) -> Result<TodoCommitNotesDto> {
    let dto: TodoCommitNotesDto = serde_json::from_str(body.trim())
        .map_err(|err| TrackError::TaskNotesParse(err.to_string()))?;
    if dto.format != TODO_FORMAT {
        return Err(TrackError::TaskNotesParse(format!(
            "unknown format '{}'",
            dto.format
        )));
    }
    Ok(dto)
}

fn parse_v1(body: &str) -> Result<TaskNotesDto> {
    let mut slug = String::new();
    let mut scraps = Vec::new();
    let mut current_header: Option<(String, i64)> = None;
    let mut current_body = String::new();
    let mut index = 0i64;

    for line in body.lines() {
        if let Some(rest) = line.strip_prefix("task:") {
            slug = rest.trim().to_string();
            continue;
        }
        if let Some((ts, id)) = parse_v1_scrap_header(line) {
            flush_v1_scrap(
                &mut scraps,
                &mut index,
                current_header.take(),
                &mut current_body,
            );
            current_header = Some((ts, id));
            continue;
        }
        if current_header.is_some() {
            if !current_body.is_empty() {
                current_body.push('\n');
            }
            current_body.push_str(line);
        }
    }
    flush_v1_scrap(
        &mut scraps,
        &mut index,
        current_header.take(),
        &mut current_body,
    );

    if slug.is_empty() {
        return Err(TrackError::TaskNotesParse(
            "v1 notes missing task slug".to_string(),
        ));
    }

    Ok(TaskNotesDto {
        format: FORMAT.to_string(),
        version: VERSION,
        name: slug.clone(),
        slug,
        description: None,
        ticket_id: None,
        ticket_url: None,
        todos: Vec::new(),
        links: Vec::new(),
        scraps,
    })
}

fn parse_v1_scrap_header(line: &str) -> Option<(String, i64)> {
    let line = line.trim();
    if !line.starts_with('[') {
        return None;
    }
    let close = line.find(']')?;
    let ts = line[1..close].to_string();
    let rest = line[close + 1..].trim();
    let id = rest.strip_prefix('#')?.parse().ok()?;
    Some((ts, id))
}

fn flush_v1_scrap(
    scraps: &mut Vec<ScrapNotesDto>,
    fallback_index: &mut i64,
    header: Option<(String, i64)>,
    body: &mut String,
) {
    let Some((created_at, index)) = header else {
        body.clear();
        return;
    };
    let content = body.trim().to_string();
    body.clear();
    if content.is_empty() {
        return;
    }
    let index = if index > 0 {
        index
    } else {
        *fallback_index += 1;
        *fallback_index
    };
    scraps.push(ScrapNotesDto {
        index,
        content,
        created_at,
        active_todo_id: None,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        Scrap, ScrapId, ScrapIndex, ScrapVisibility, Task, TaskId, TaskStatus, Todo, TodoId,
        TodoIndex, TodoStatus,
    };
    use chrono::Utc;

    fn sample_task(name: &str) -> Task {
        Task {
            id: TaskId::from_i64(1),
            name: name.to_string(),
            description: Some("scope".to_string()),
            status: TaskStatus::Active,
            ticket_id: None,
            ticket_url: None,
            alias: None,
            is_today_task: false,
            created_at: Utc::now(),
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

    fn sample_scrap(content: &str, visibility: ScrapVisibility) -> Scrap {
        Scrap {
            id: ScrapId::from_i64(1),
            task_id: TaskId::from_i64(1),
            scrap_id: ScrapIndex::from_i64(1),
            content: content.to_string(),
            created_at: Utc::now(),
            active_todo_id: None,
            visibility,
        }
    }

    #[test]
    fn todo_notes_only_include_that_todos_shared_scraps() {
        let mut todo = sample_todo("Implement");
        todo.status = TodoStatus::Done;
        let mut other_todo = sample_todo("Review");
        other_todo.task_index = TodoIndex::from_i64(2);
        let mut for_impl = sample_scrap("chose bcrypt", ScrapVisibility::Shared);
        for_impl.active_todo_id = Some(todo.task_index);
        let mut for_review = sample_scrap("nits later", ScrapVisibility::Shared);
        for_review.id = ScrapId::from_i64(2);
        for_review.scrap_id = ScrapIndex::from_i64(2);
        for_review.active_todo_id = Some(other_todo.task_index);
        let local = sample_scrap("scratch", ScrapVisibility::Local);
        let notes = build_todo_notes("auth", &todo, &[for_impl, for_review, local]);
        assert_eq!(notes.todo_index, 1);
        assert_eq!(notes.scraps.len(), 1);
        assert_eq!(notes.scraps[0].content, "chose bcrypt");
        let parsed = parse_todo_notes(&format_todo_notes(&notes).unwrap()).unwrap();
        assert_eq!(parsed.format, TODO_FORMAT);
    }

    #[test]
    fn parse_task_todo_trailer() {
        let message = todo_commit_message(3, "Fix nits", "auth");
        assert_eq!(parse_task_todo_index(&message), Some(3));
        assert!(message.contains("Task-Slug: auth"));
    }

    #[test]
    fn snapshot_omits_local_scraps() {
        let task = sample_task("Auth");
        let todos = [sample_todo("Implement")];
        let scraps = [
            sample_scrap("secret scratch", ScrapVisibility::Local),
            sample_scrap("chose bcrypt", ScrapVisibility::Shared),
        ];
        let dto = build_snapshot("auth", &task, &todos, &[], &scraps);
        assert_eq!(dto.scraps.len(), 1);
        assert_eq!(dto.scraps[0].content, "chose bcrypt");
        assert_eq!(dto.todos[0].content, "Implement");
        let json = format_snapshot(&dto).unwrap();
        let parsed = parse_notes(&json).unwrap();
        assert_eq!(parsed.slug, "auth");
        assert_eq!(parsed.scraps.len(), 1);
    }

    #[test]
    fn parse_legacy_v1_notes() {
        let body =
            "track-scraps v1\ntask: fix-auth\n\n[2026-09-13T00:00:00+00:00] #1\nchose oauth\n";
        let dto = parse_notes(body).unwrap();
        assert_eq!(dto.slug, "fix-auth");
        assert_eq!(dto.scraps.len(), 1);
        assert_eq!(dto.scraps[0].content, "chose oauth");
        assert!(dto.todos.is_empty());
    }
}
