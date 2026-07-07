//! HTTP route handlers for the WebUI.

use crate::models::TodoStatus;
use crate::services::{LinkService, RepoService, ScrapService, TaskService, TodoService};
use crate::use_cases::{ApplyTodoActionUseCase, GetTaskInfoUseCase};
use crate::utils::TrackError;
use crate::webui::error::WebError;
use crate::webui::state::{AppState, SseEvent};
use crate::webui::templates::SharedTemplates;
use crate::webui::view::{self, format_scraps, format_todos, StatusResponse};
use axum::{
    extract::{Path, State},
    response::Html,
    Form, Json,
};
use serde::Deserialize;

/// Extended application state with templates
#[derive(Clone)]
pub struct WebState {
    pub app: AppState,
    pub templates: SharedTemplates,
}

/// Error response wrapper
pub type AppError = WebError;

/// Form data for adding a todo
#[derive(Deserialize)]
pub struct AddTodoForm {
    pub content: String,
    #[serde(default)]
    pub create_worktree: bool,
    #[serde(default)]
    pub no_workspace: bool,
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

fn render_todo_list_html(
    templates: &crate::webui::templates::Templates,
    db: &crate::db::Database,
    task_id: i64,
) -> Result<String, AppError> {
    let snapshot = GetTaskInfoUseCase::new(db).load(task_id)?;
    let todos = format_todos(&snapshot.todos, &snapshot.worktrees, &snapshot.scraps)?;
    Ok(templates.render(
        "partials/todo_list.html",
        serde_json::json!({ "todos": todos }),
    )?)
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

    let context = view::build_template_context(&db, current_task_id)?;
    let html = state.templates.render("index.html", context)?;
    Ok(Html(html))
}

/// JSON API endpoint for status data
pub async fn api_status(State(state): State<WebState>) -> Result<Json<StatusResponse>, AppError> {
    let db = state.app.db.lock().await;

    let current_task_id = match db.get_current_task_id()? {
        Some(id) => id,
        None => return Ok(Json(StatusResponse::empty())),
    };

    let info = GetTaskInfoUseCase::new(&db);
    let snapshot = info.load(current_task_id)?;
    let response = view::build_api_status(&db, &snapshot)?;

    Ok(Json(response))
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
    let html = render_todo_list_html(&state.templates, &db, current_task_id)?;
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

    if form.create_worktree {
        return Err(TrackError::WorktreeFlagRemoved.into());
    }

    let todo_service = TodoService::new(&db);
    let _todo = todo_service.add_todo(
        current_task_id,
        &form.content,
        crate::models::TodoAddOptions::from_flags(false, form.no_workspace),
    )?;

    // Broadcast SSE event
    state.app.broadcast(SseEvent::Todos);

    let html = render_todo_list_html(&state.templates, &db, current_task_id)?;
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

    if new_status.as_str() == TodoStatus::PENDING {
        todo_service.update_status(todo.id, &new_status)?;
    } else {
        let action = crate::models::TodoAction::from_web_route(&new_status)?;
        ApplyTodoActionUseCase::new(&db).execute(current_task_id, todo_index, action)?;
    }

    // Broadcast SSE event
    state.app.broadcast(SseEvent::Todos);

    let html = render_todo_list_html(&state.templates, &db, current_task_id)?;
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

    let html = render_todo_list_html(&state.templates, &db, current_task_id)?;
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

    let html = render_todo_list_html(&state.templates, &db, current_task_id)?;
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
        .ok_or(TrackError::LinkNotFound(link_index))?;

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
