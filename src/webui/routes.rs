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
        todos: todos
            .iter()
            .map(|t| serde_json::to_value(t).unwrap_or_default())
            .collect(),
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

/// Add a new todo
pub async fn add_todo(
    State(state): State<WebState>,
    Form(form): Form<AddTodoForm>,
) -> Result<Html<String>, AppError> {
    let db = state.app.db.lock().await;

    let current_task_id = db.get_current_task_id()?.ok_or(TrackError::NoActiveTask)?;

    let todo_service = TodoService::new(&db);
    let todo = todo_service.add_todo(current_task_id, &form.content, false)?;

    // Broadcast SSE event
    state
        .app
        .broadcast(SseEvent::TodoAdded { todo_id: todo.id });

    // Return updated todo list partial
    let todos = todo_service.list_todos(current_task_id)?;
    let html = state.templates.render(
        "partials/todo_list.html",
        serde_json::json!({
            "todos": todos,
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
    state
        .app
        .broadcast(SseEvent::TodoDeleted { todo_id: todo.id });

    // Return updated todo list partial
    let todos = todo_service.list_todos(current_task_id)?;
    let html = state.templates.render(
        "partials/todo_list.html",
        serde_json::json!({
            "todos": todos,
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
    let scrap = scrap_service.add_scrap(current_task_id, &form.content)?;

    // Broadcast SSE event
    state.app.broadcast(SseEvent::ScrapAdded { id: scrap.id });

    // Return updated scrap list partial
    let scraps = scrap_service.list_scraps(current_task_id)?;
    let html = state.templates.render(
        "partials/scrap_list.html",
        serde_json::json!({
            "scraps": scraps,
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
    state.app.broadcast(SseEvent::StatusUpdate);

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
        "todos": todos,
        "links": links,
        "scraps": scraps,
        "worktrees": worktrees,
        "repos": repos,
        "base_branch": base_branch,
    }))
}
