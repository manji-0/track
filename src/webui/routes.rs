//! HTTP route handlers for the WebUI.

use crate::db::Database;
use crate::services::{
    LinkService, RepoService, ScrapService, TaskService, TodoService, WorktreeService,
};
use crate::utils::TrackError;
use crate::webui::state::{AppState, SseEvent};
use crate::webui::templates::SharedTemplates;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    Form, Json,
};
use serde::{Deserialize, Serialize};

/// Extended application state with templates
#[derive(Clone)]
pub struct WebState {
    pub app: AppState,
    pub templates: SharedTemplates,
}

/// Error response wrapper
pub struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error: {}", self.0),
        )
            .into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

/// Status response for JSON API
#[derive(Serialize)]
pub struct StatusResponse {
    pub task: Option<serde_json::Value>,
    pub todos: Vec<serde_json::Value>,
    pub links: Vec<serde_json::Value>,
    pub scraps: Vec<serde_json::Value>,
    pub worktrees: Vec<serde_json::Value>,
    pub repos: Vec<serde_json::Value>,
}

/// Form data for adding a todo
#[derive(Deserialize)]
pub struct AddTodoForm {
    pub content: String,
    #[serde(default)]
    pub create_worktree: bool,
}

/// Form data for adding a scrap
#[derive(Deserialize)]
pub struct AddScrapForm {
    pub content: String,
}

/// Form data for updating description
#[derive(Deserialize)]
pub struct UpdateDescriptionForm {
    pub description: String,
}

/// Form data for updating ticket
#[derive(Deserialize)]
pub struct UpdateTicketForm {
    pub ticket_id: String,
    pub ticket_url: Option<String>,
}

/// Form data for adding a link
#[derive(Deserialize)]
pub struct AddLinkForm {
    pub url: String,
    pub title: Option<String>,
}

/// Main dashboard page
pub async fn index(State(state): State<WebState>) -> Result<Html<String>, AppError> {
    let db = state.app.db.lock().await;

    let current_task_id = match db.get_current_task_id()? {
        Some(id) => id,
        None => {
            let html = state.templates.render(
                "index.html",
                serde_json::json!({
                    "task": null,
                    "todos": [],
                    "scraps": [],
                    "links": [],
                    "repos": [],
                    "worktrees": [],
                }),
            )?;
            return Ok(Html(html));
        }
    };

    let context = build_status_context(&db, current_task_id)?;
    let html = state.templates.render("index.html", context)?;
    Ok(Html(html))
}

/// JSON API endpoint for status data
pub async fn api_status(State(state): State<WebState>) -> Result<Json<StatusResponse>, AppError> {
    let db = state.app.db.lock().await;

    let current_task_id = match db.get_current_task_id()? {
        Some(id) => id,
        None => {
            return Ok(Json(StatusResponse {
                task: None,
                todos: vec![],
                links: vec![],
                scraps: vec![],
                worktrees: vec![],
                repos: vec![],
            }));
        }
    };

    let task_service = TaskService::new(&db);
    let task = task_service.get_task(current_task_id)?;

    let todo_service = TodoService::new(&db);
    let todos = todo_service.list_todos(current_task_id)?;

    let link_service = LinkService::new(&db);
    let links = link_service.list_links(current_task_id)?;

    let scrap_service = ScrapService::new(&db);
    let scraps = scrap_service.list_scraps(current_task_id)?;

    let worktree_service = WorktreeService::new(&db);
    let worktrees = worktree_service.list_worktrees(current_task_id)?;

    let repo_service = RepoService::new(&db);
    let repos = repo_service.list_repos(current_task_id)?;

    Ok(Json(StatusResponse {
        task: Some(serde_json::to_value(&task)?),
        todos: format_todos(todos, &worktrees, &scraps),
        links: links
            .iter()
            .map(|l| serde_json::to_value(l).unwrap_or_default())
            .collect(),
        scraps: scraps
            .iter()
            .map(|s| serde_json::to_value(s).unwrap_or_default())
            .collect(),
        worktrees: worktrees
            .iter()
            .map(|w| serde_json::to_value(w).unwrap_or_default())
            .collect(),
        repos: repos
            .iter()
            .map(|r| serde_json::to_value(r).unwrap_or_default())
            .collect(),
    }))
}

/// Get description card HTML
pub async fn get_description(State(state): State<WebState>) -> Result<Html<String>, AppError> {
    let db = state.app.db.lock().await;
    let current_task_id = db.get_current_task_id()?.ok_or(TrackError::NoActiveTask)?;

    let task_service = TaskService::new(&db);
    let task = task_service.get_task(current_task_id)?;

    let html = state.templates.render(
        "partials/description.html",
        serde_json::json!({
            "task": task,
        }),
    )?;

    Ok(Html(html))
}

/// Get ticket card HTML
pub async fn get_ticket(State(state): State<WebState>) -> Result<Html<String>, AppError> {
    let db = state.app.db.lock().await;
    let current_task_id = db.get_current_task_id()?.ok_or(TrackError::NoActiveTask)?;

    let task_service = TaskService::new(&db);
    let task = task_service.get_task(current_task_id)?;

    let html = state.templates.render(
        "partials/ticket.html",
        serde_json::json!({
            "task": task,
        }),
    )?;

    Ok(Html(html))
}

/// Get links card HTML
pub async fn get_links(State(state): State<WebState>) -> Result<Html<String>, AppError> {
    let db = state.app.db.lock().await;
    let current_task_id = db.get_current_task_id()?.ok_or(TrackError::NoActiveTask)?;

    let link_service = LinkService::new(&db);
    let links = link_service.list_links(current_task_id)?;

    let html = state.templates.render(
        "partials/links.html",
        serde_json::json!({
            "links": links,
        }),
    )?;

    Ok(Html(html))
}

/// Get repos card HTML
pub async fn get_repos(State(state): State<WebState>) -> Result<Html<String>, AppError> {
    let db = state.app.db.lock().await;
    let current_task_id = db.get_current_task_id()?.ok_or(TrackError::NoActiveTask)?;

    let repo_service = RepoService::new(&db);
    let repos = repo_service.list_repos(current_task_id)?;

    let html = state.templates.render(
        "partials/repos.html",
        serde_json::json!({
            "repos": repos,
        }),
    )?;

    Ok(Html(html))
}

/// Get todos card HTML
pub async fn get_todos(State(state): State<WebState>) -> Result<Html<String>, AppError> {
    let db = state.app.db.lock().await;
    let current_task_id = db.get_current_task_id()?.ok_or(TrackError::NoActiveTask)?;

    let todo_service = TodoService::new(&db);
    let todos = todo_service.list_todos(current_task_id)?;

    let scrap_service = ScrapService::new(&db);
    let scraps = scrap_service.list_scraps(current_task_id)?;

    let worktree_service = WorktreeService::new(&db);
    let worktrees = worktree_service.list_worktrees(current_task_id)?;

    let html = state.templates.render(
        "partials/todo_list.html",
        serde_json::json!({
            "todos": format_todos(todos, &worktrees, &scraps),
        }),
    )?;

    Ok(Html(html))
}

/// Get scraps card HTML
pub async fn get_scraps(State(state): State<WebState>) -> Result<Html<String>, AppError> {
    let db = state.app.db.lock().await;
    let current_task_id = db.get_current_task_id()?.ok_or(TrackError::NoActiveTask)?;

    let scrap_service = ScrapService::new(&db);
    let scraps = scrap_service.list_scraps(current_task_id)?;

    let html = state.templates.render(
        "partials/scrap_list.html",
        serde_json::json!({
            "scraps": format_scraps(&scraps),
        }),
    )?;

    Ok(Html(html))
}

/// Add a new todo
pub async fn add_todo(
    State(state): State<WebState>,
    Form(form): Form<AddTodoForm>,
) -> Result<Html<String>, AppError> {
    let db = state.app.db.lock().await;

    let current_task_id = db.get_current_task_id()?.ok_or(TrackError::NoActiveTask)?;

    let todo_service = TodoService::new(&db);
    let _todo = todo_service.add_todo(current_task_id, &form.content, form.create_worktree)?;

    // Broadcast SSE event
    state.app.broadcast(SseEvent::Todos);

    // Return updated todo list partial
    let todos = todo_service.list_todos(current_task_id)?;
    let scrap_service = ScrapService::new(&db);
    let scraps = scrap_service.list_scraps(current_task_id)?;
    let worktree_service = WorktreeService::new(&db);
    let worktrees = worktree_service.list_worktrees(current_task_id)?;

    let html = state.templates.render(
        "partials/todo_list.html",
        serde_json::json!({
            "todos": format_todos(todos, &worktrees, &scraps),
        }),
    )?;

    Ok(Html(html))
}

/// Update todo status
pub async fn update_todo_status(
    State(state): State<WebState>,
    Path((todo_index, new_status)): Path<(i64, String)>,
) -> Result<Html<String>, AppError> {
    let db = state.app.db.lock().await;

    let current_task_id = db.get_current_task_id()?.ok_or(TrackError::NoActiveTask)?;

    let todo_service = TodoService::new(&db);
    let todo = todo_service.get_todo_by_index(current_task_id, todo_index)?;
    todo_service.update_status(todo.id, &new_status)?;

    // Broadcast SSE event
    state.app.broadcast(SseEvent::Todos);

    // Return updated todo list partial
    let todos = todo_service.list_todos(current_task_id)?;
    let scrap_service = ScrapService::new(&db);
    let scraps = scrap_service.list_scraps(current_task_id)?;
    let worktree_service = WorktreeService::new(&db);
    let worktrees = worktree_service.list_worktrees(current_task_id)?;

    let html = state.templates.render(
        "partials/todo_list.html",
        serde_json::json!({
            "todos": format_todos(todos, &worktrees, &scraps),
        }),
    )?;

    Ok(Html(html))
}

/// Delete a todo by task-scoped index
pub async fn delete_todo(
    State(state): State<WebState>,
    Path(todo_index): Path<i64>,
) -> Result<Html<String>, AppError> {
    let db = state.app.db.lock().await;

    let current_task_id = db.get_current_task_id()?.ok_or(TrackError::NoActiveTask)?;

    let todo_service = TodoService::new(&db);
    let todo = todo_service.get_todo_by_index(current_task_id, todo_index)?;
    todo_service.delete_todo(todo.id)?;

    // Broadcast SSE event
    state.app.broadcast(SseEvent::Todos);

    // Return updated todo list partial
    let todos = todo_service.list_todos(current_task_id)?;
    let scrap_service = ScrapService::new(&db);
    let scraps = scrap_service.list_scraps(current_task_id)?;
    let worktree_service = WorktreeService::new(&db);
    let worktrees = worktree_service.list_worktrees(current_task_id)?;

    let html = state.templates.render(
        "partials/todo_list.html",
        serde_json::json!({
            "todos": format_todos(todos, &worktrees, &scraps),
        }),
    )?;

    Ok(Html(html))
}

/// Move a todo to the front (make it the next todo to work on)
pub async fn move_todo_to_next(
    State(state): State<WebState>,
    Path(todo_index): Path<i64>,
) -> Result<Html<String>, AppError> {
    let db = state.app.db.lock().await;

    let current_task_id = db.get_current_task_id()?.ok_or(TrackError::NoActiveTask)?;

    let todo_service = TodoService::new(&db);
    todo_service.move_to_next(current_task_id, todo_index)?;

    // Broadcast SSE event
    state.app.broadcast(SseEvent::Todos);

    // Return updated todo list partial
    let todos = todo_service.list_todos(current_task_id)?;
    let scrap_service = ScrapService::new(&db);
    let scraps = scrap_service.list_scraps(current_task_id)?;
    let worktree_service = WorktreeService::new(&db);
    let worktrees = worktree_service.list_worktrees(current_task_id)?;

    let html = state.templates.render(
        "partials/todo_list.html",
        serde_json::json!({
            "todos": format_todos(todos, &worktrees, &scraps),
        }),
    )?;

    Ok(Html(html))
}

/// Add a new scrap
pub async fn add_scrap(
    State(state): State<WebState>,
    Form(form): Form<AddScrapForm>,
) -> Result<Html<String>, AppError> {
    let db = state.app.db.lock().await;

    let current_task_id = db.get_current_task_id()?.ok_or(TrackError::NoActiveTask)?;

    let scrap_service = ScrapService::new(&db);
    let _scrap = scrap_service.add_scrap(current_task_id, &form.content)?;

    // Broadcast SSE event
    state.app.broadcast(SseEvent::Scraps);

    // Return updated scrap list partial
    let scraps = scrap_service.list_scraps(current_task_id)?;
    let html = state.templates.render(
        "partials/scrap_list.html",
        serde_json::json!({
            "scraps": format_scraps(&scraps),
        }),
    )?;

    Ok(Html(html))
}

/// Update task description
pub async fn update_description(
    State(state): State<WebState>,
    Form(form): Form<UpdateDescriptionForm>,
) -> Result<Html<String>, AppError> {
    let db = state.app.db.lock().await;

    let current_task_id = db.get_current_task_id()?.ok_or(TrackError::NoActiveTask)?;

    let task_service = TaskService::new(&db);
    task_service.set_description(current_task_id, &form.description)?;

    // Get updated task
    let task = task_service.get_task(current_task_id)?;

    // Broadcast SSE event
    state.app.broadcast(SseEvent::Description);

    // Return updated description section
    let html = state.templates.render(
        "partials/description.html",
        serde_json::json!({
            "task": task,
        }),
    )?;

    Ok(Html(html))
}

/// Build status context for templates
fn build_status_context(db: &Database, task_id: i64) -> anyhow::Result<serde_json::Value> {
    let task_service = TaskService::new(db);
    let task = task_service.get_task(task_id)?;

    let todo_service = TodoService::new(db);
    let todos = todo_service.list_todos(task_id)?;

    let link_service = LinkService::new(db);
    let links = link_service.list_links(task_id)?;

    let scrap_service = ScrapService::new(db);
    let scraps = scrap_service.list_scraps(task_id)?;

    let worktree_service = WorktreeService::new(db);
    let worktrees = worktree_service.list_worktrees(task_id)?;

    let repo_service = RepoService::new(db);
    let repos = repo_service.list_repos(task_id)?;

    // Calculate base branch
    let base_branch = if let Some(base_wt) = worktrees.iter().find(|wt| wt.is_base) {
        base_wt.branch.clone()
    } else if let Some(ref ticket_id) = task.ticket_id {
        format!("task/{}", ticket_id)
    } else {
        format!("task/task-{}", task.id)
    };

    Ok(serde_json::json!({
        "task": task,
        "todos": format_todos(todos, &worktrees, &scraps),
        "links": links,
        "scraps": format_scraps(&scraps),
        "worktrees": worktrees,
        "repos": repos,
        "base_branch": base_branch,
    }))
}

/// Format todos with worktree information and hidden fields
fn format_todos(
    todos: Vec<crate::models::Todo>,
    worktrees: &[crate::models::Worktree],
    scraps: &[crate::models::Scrap],
) -> Vec<serde_json::Value> {
    todos
        .into_iter()
        .map(|todo| {
            let todo_worktrees: Vec<String> = worktrees
                .iter()
                .filter(|wt| wt.todo_id == Some(todo.id))
                .map(|wt| wt.path.clone())
                .collect();

            // Count scraps associated with this todo
            let scrap_count = scraps
                .iter()
                .filter(|s| s.active_todo_id == Some(todo.task_index))
                .count();

            let mut value = serde_json::to_value(&todo).unwrap_or_default();

            if let Some(obj) = value.as_object_mut() {
                // Reinject skipped fields that are needed for UI
                obj.insert(
                    "worktree_requested".to_string(),
                    serde_json::Value::Bool(todo.worktree_requested),
                );
                obj.insert(
                    "worktree_paths".to_string(),
                    serde_json::json!(todo_worktrees),
                );
                obj.insert(
                    "content_html".to_string(),
                    serde_json::Value::String(todo.content_html()),
                );
                obj.insert(
                    "has_scraps".to_string(),
                    serde_json::Value::Bool(scrap_count > 0),
                );
            }
            value
        })
        .collect()
}

/// Format scraps with human-readable timestamps
fn format_scraps(scraps: &[crate::models::Scrap]) -> Vec<serde_json::Value> {
    use chrono::Local;

    scraps
        .iter()
        .map(|scrap| {
            let formatted_time = scrap
                .created_at
                .with_timezone(&Local)
                .format("%Y-%m-%d %H:%M:%S")
                .to_string();

            serde_json::json!({
                "scrap_id": scrap.scrap_id,
                "content": scrap.content,
                "content_html": scrap.content_html(),
                "created_at": formatted_time,
                "active_todo_id": scrap.active_todo_id,
            })
        })
        .collect()
}

/// Update task ticket
pub async fn update_ticket(
    State(state): State<WebState>,
    Form(form): Form<UpdateTicketForm>,
) -> Result<Html<String>, AppError> {
    let db = state.app.db.lock().await;

    let current_task_id = db.get_current_task_id()?.ok_or(TrackError::NoActiveTask)?;

    let task_service = TaskService::new(&db);

    // Clean up ticket_url if empty
    let ticket_url = form.ticket_url.filter(|url| !url.trim().is_empty());
    let ticket_url_str = ticket_url.as_deref().unwrap_or("");

    task_service.link_ticket(current_task_id, &form.ticket_id, ticket_url_str)?;

    // Get updated task
    let task = task_service.get_task(current_task_id)?;

    // Broadcast SSE event
    state.app.broadcast(SseEvent::Ticket);

    // Return updated ticket section
    let html = state.templates.render(
        "partials/ticket.html",
        serde_json::json!({
            "task": task,
        }),
    )?;

    Ok(Html(html))
}

/// Add a new link
pub async fn add_link(
    State(state): State<WebState>,
    Form(form): Form<AddLinkForm>,
) -> Result<Html<String>, AppError> {
    let db = state.app.db.lock().await;

    let current_task_id = db.get_current_task_id()?.ok_or(TrackError::NoActiveTask)?;

    let link_service = LinkService::new(&db);

    // Clean up title if empty
    let title = form.title.filter(|t| !t.trim().is_empty());

    link_service.add_link(current_task_id, &form.url, title.as_deref())?;

    // Broadcast SSE event
    state.app.broadcast(SseEvent::Links);

    // Return updated links list partial
    let links = link_service.list_links(current_task_id)?;
    let html = state.templates.render(
        "partials/links.html",
        serde_json::json!({
            "links": links,
        }),
    )?;

    Ok(Html(html))
}

/// Delete a link by task-scoped index
pub async fn delete_link(
    State(state): State<WebState>,
    Path(link_index): Path<i64>,
) -> Result<Html<String>, AppError> {
    let db = state.app.db.lock().await;

    let current_task_id = db.get_current_task_id()?.ok_or(TrackError::NoActiveTask)?;

    let link_service = LinkService::new(&db);
    let links = link_service.list_links(current_task_id)?;

    // Find link by task_index
    let link = links
        .iter()
        .find(|l| l.task_index == link_index)
        .ok_or_else(|| TrackError::Other(format!("Link #{} not found", link_index)))?;

    // Delete link via service
    link_service.delete_link(link.id)?;

    // Broadcast SSE event
    state.app.broadcast(SseEvent::Links);

    // Return updated links list partial
    let links = link_service.list_links(current_task_id)?;
    let html = state.templates.render(
        "partials/links.html",
        serde_json::json!({
            "links": links,
        }),
    )?;

    Ok(Html(html))
}
