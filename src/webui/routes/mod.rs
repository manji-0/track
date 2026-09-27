//! HTTP route handlers for the WebUI.

use crate::db::Database;
use crate::models::{
    ScrapIndex, ScrapVisibility, TaskId, TodoAction, TodoAddOptions, TodoIndex, TodoStatus,
};
use crate::services::{LinkService, RepoService, ScrapService, TaskService, TodoService};
use crate::use_cases::{ApplyTodoActionUseCase, GetTaskInfoUseCase, project_task_notes_or_warn};
use crate::utils::TrackError;
use crate::webui::error::WebError;
use crate::webui::state::{AppState, SseEvent};
use crate::webui::templates::SharedTemplates;
use crate::webui::view::{self, StatusResponse, format_scraps, format_todos};
use axum::{
    Form, Json,
    extract::{FromRequestParts, Path, Query, State},
    http::request::Parts,
    response::Html,
};
use serde::Deserialize;
use std::str::FromStr;

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
    #[serde(default)]
    pub share: Option<String>,
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

fn render_task_identity_html(
    templates: &crate::webui::templates::Templates,
    task: &crate::models::Task,
    oob: bool,
) -> Result<String, AppError> {
    Ok(templates.render(
        "partials/task_identity.html",
        serde_json::json!({
            "task": task,
            "oob": oob,
        }),
    )?)
}

fn render_ticket_mutation_html(
    templates: &crate::webui::templates::Templates,
    task: &crate::models::Task,
) -> Result<String, AppError> {
    let ticket = templates.render(
        "partials/ticket.html",
        serde_json::json!({
            "task": task,
        }),
    )?;
    let identity = render_task_identity_html(templates, task, true)?;
    Ok(format!("{ticket}{identity}"))
}

fn render_todo_list_html(
    templates: &crate::webui::templates::Templates,
    db: &crate::db::Database,
    task_id: crate::models::TaskId,
) -> Result<String, AppError> {
    let snapshot = GetTaskInfoUseCase::new(db).load(task_id)?;
    let todos = format_todos(&snapshot.todos, &snapshot.worktrees, &snapshot.scraps)?;
    Ok(templates.render(
        "partials/todo_list.html",
        serde_json::json!({ "todos": todos }),
    )?)
}

/// Request header each task page sets on its HTMX requests (one task per browser tab).
pub const TASK_HEADER: &str = "x-track-task";

#[derive(Deserialize)]
struct TaskQuery {
    task: Option<String>,
}

/// Task a request targets: the `X-Track-Task` header, else the `?task=` query parameter.
///
/// Without either, single context mode falls back to the current task and multi
/// context mode rejects the request (same rule as the CLI `--task`).
pub struct TaskTarget(Option<String>);

impl TaskTarget {
    pub fn resolve(&self, db: &Database) -> crate::utils::Result<TaskId> {
        TaskService::new(db).resolve_target_task_id(self.0.as_deref())
    }
}

impl<S: Send + Sync> FromRequestParts<S> for TaskTarget {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let from_header = parts
            .headers
            .get(TASK_HEADER)
            .and_then(|value| value.to_str().ok())
            .map(str::to_string);
        let reference = from_header
            .or_else(|| {
                Query::<TaskQuery>::try_from_uri(&parts.uri)
                    .ok()
                    .and_then(|query| query.0.task)
            })
            .filter(|reference| !reference.trim().is_empty());
        Ok(Self(reference))
    }
}

/// Tab scope rendered into `base.html` (HTMX header + SSE subscription).
fn page_context(task_id: Option<TaskId>, follow_current: bool) -> serde_json::Value {
    serde_json::json!({
        "task_id": task_id,
        "follow": follow_current,
    })
}

fn render_task_page(
    templates: &crate::webui::templates::Templates,
    db: &Database,
    task_id: TaskId,
    follow_current: bool,
) -> Result<Html<String>, AppError> {
    let mut context = view::build_template_context(db, task_id)?;
    if let Some(obj) = context.as_object_mut() {
        obj.insert(
            "page".to_string(),
            page_context(Some(task_id), follow_current),
        );
    }
    Ok(Html(templates.render("index.html", context)?))
}

fn render_task_list_page(
    templates: &crate::webui::templates::Templates,
    db: &Database,
    follow_current: bool,
) -> Result<Html<String>, AppError> {
    let context_mode = db.get_context_mode()?;
    let current_task_id = TaskService::new(db).resolve_target_task_id_opt(None)?;
    let tasks = TaskService::new(db).list_tasks(false)?;
    Ok(Html(templates.render(
        "tasks.html",
        serde_json::json!({
            "tasks": tasks,
            "current_task_id": current_task_id,
            "context_mode": context_mode,
            "page": page_context(None, follow_current),
        }),
    )?))
}

/// Main dashboard page: the current task in single mode, the task list in multi mode.
pub async fn index(State(state): State<WebState>) -> Result<Html<String>, AppError> {
    let db = state.app.db.lock().await;
    match TaskService::new(&db).resolve_target_task_id_opt(None)? {
        Some(task_id) => render_task_page(&state.templates, &db, task_id, true),
        None => render_task_list_page(&state.templates, &db, !db.get_context_mode()?.is_multi()),
    }
}

/// Task list page; each task opens in its own tab at `/tasks/{id}`.
pub async fn tasks_page(State(state): State<WebState>) -> Result<Html<String>, AppError> {
    let db = state.app.db.lock().await;
    render_task_list_page(&state.templates, &db, false)
}

/// Dashboard pinned to one task (ID, t:<ticket>, a:<alias>, or task name).
pub async fn task_page(
    State(state): State<WebState>,
    Path(task_ref): Path<String>,
) -> Result<Html<String>, AppError> {
    let db = state.app.db.lock().await;
    let task_id = TaskService::new(&db).resolve_target_task_id(Some(&task_ref))?;
    render_task_page(&state.templates, &db, task_id, false)
}

/// JSON API endpoint for status data
pub async fn api_status(
    State(state): State<WebState>,
    target: TaskTarget,
) -> Result<Json<StatusResponse>, AppError> {
    let db = state.app.db.lock().await;

    let task_id = match target.resolve(&db) {
        Ok(id) => id,
        Err(TrackError::NoActiveTask) => return Ok(Json(StatusResponse::empty())),
        Err(err) => return Err(err.into()),
    };

    let info = GetTaskInfoUseCase::new(&db);
    let snapshot = info.load(task_id)?;
    let response = view::build_api_status(&db, &snapshot)?;

    Ok(Json(response))
}

/// Get description card HTML
pub async fn get_description(
    State(state): State<WebState>,
    target: TaskTarget,
) -> Result<Html<String>, AppError> {
    let db = state.app.db.lock().await;
    let task_id = target.resolve(&db)?;

    let task_service = TaskService::new(&db);
    let task = task_service.get_task(task_id)?;

    let html = state.templates.render(
        "partials/description.html",
        serde_json::json!({
            "task": task,
        }),
    )?;

    Ok(Html(html))
}

/// Get ticket card HTML
pub async fn get_ticket(
    State(state): State<WebState>,
    target: TaskTarget,
) -> Result<Html<String>, AppError> {
    let db = state.app.db.lock().await;
    let task_id = target.resolve(&db)?;

    let task_service = TaskService::new(&db);
    let task = task_service.get_task(task_id)?;

    let html = state.templates.render(
        "partials/ticket.html",
        serde_json::json!({
            "task": task,
        }),
    )?;

    Ok(Html(html))
}

/// Get topbar task identity HTML (name, alias, ticket badge)
pub async fn get_identity(
    State(state): State<WebState>,
    target: TaskTarget,
) -> Result<Html<String>, AppError> {
    let db = state.app.db.lock().await;
    let task_id = target.resolve(&db)?;

    let task_service = TaskService::new(&db);
    let task = task_service.get_task(task_id)?;
    let html = render_task_identity_html(&state.templates, &task, false)?;

    Ok(Html(html))
}

/// Get links card HTML
pub async fn get_links(
    State(state): State<WebState>,
    target: TaskTarget,
) -> Result<Html<String>, AppError> {
    let db = state.app.db.lock().await;
    let task_id = target.resolve(&db)?;

    let link_service = LinkService::new(&db);
    let links = link_service.list_links(task_id)?;

    let html = state.templates.render(
        "partials/links.html",
        serde_json::json!({
            "links": links,
        }),
    )?;

    Ok(Html(html))
}

/// Get repos card HTML
pub async fn get_repos(
    State(state): State<WebState>,
    target: TaskTarget,
) -> Result<Html<String>, AppError> {
    let db = state.app.db.lock().await;
    let task_id = target.resolve(&db)?;

    let repo_service = RepoService::new(&db);
    let repos = repo_service.list_repos(task_id)?;

    let html = state.templates.render(
        "partials/repos.html",
        serde_json::json!({
            "repos": repos,
        }),
    )?;

    Ok(Html(html))
}

/// Get todos card HTML
pub async fn get_todos(
    State(state): State<WebState>,
    target: TaskTarget,
) -> Result<Html<String>, AppError> {
    let db = state.app.db.lock().await;
    let task_id = target.resolve(&db)?;
    let html = render_todo_list_html(&state.templates, &db, task_id)?;
    Ok(Html(html))
}

/// Get workflow footer HTML
pub async fn get_workflow(
    State(state): State<WebState>,
    target: TaskTarget,
) -> Result<Html<String>, AppError> {
    let db = state.app.db.lock().await;
    let task_id = target.resolve(&db)?;
    let context = view::build_template_context(&db, task_id)?;
    let html = state.templates.render("partials/workflow.html", context)?;
    Ok(Html(html))
}

/// Get scraps card HTML
pub async fn get_scraps(
    State(state): State<WebState>,
    target: TaskTarget,
) -> Result<Html<String>, AppError> {
    let db = state.app.db.lock().await;
    let task_id = target.resolve(&db)?;

    let scrap_service = ScrapService::new(&db);
    let scraps = scrap_service.list_scraps(task_id)?;

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
    target: TaskTarget,
    Form(form): Form<AddTodoForm>,
) -> Result<Html<String>, AppError> {
    let db = state.app.db.lock().await;
    let task_id = target.resolve(&db)?;

    if form.create_worktree {
        return Err(TrackError::WorktreeFlagRemoved.into());
    }

    let todo_service = TodoService::new(&db);
    let _todo = todo_service.add_todo(
        task_id,
        &form.content,
        TodoAddOptions::from_flags(false, form.no_workspace),
    )?;
    project_task_notes_or_warn(&db, task_id);

    // Broadcast SSE event
    state.app.broadcast(task_id, SseEvent::Todos);

    let html = render_todo_list_html(&state.templates, &db, task_id)?;
    Ok(Html(html))
}

/// Update todo status
pub async fn update_todo_status(
    State(state): State<WebState>,
    target: TaskTarget,
    Path((todo_index, new_status)): Path<(i64, String)>,
) -> Result<Html<String>, AppError> {
    let db = state.app.db.lock().await;
    let task_id = target.resolve(&db)?;

    let status = TodoStatus::from_str(&new_status)
        .map_err(|_| TrackError::InvalidStatus(new_status.clone()))?;
    let action = TodoAction::from_web_route(status)?;
    ApplyTodoActionUseCase::new(&db).execute(task_id, TodoIndex::from_i64(todo_index), action)?;
    project_task_notes_or_warn(&db, task_id);

    // Broadcast SSE event
    state.app.broadcast(task_id, SseEvent::Todos);

    let html = render_todo_list_html(&state.templates, &db, task_id)?;
    Ok(Html(html))
}

/// Delete a todo by task-scoped index
pub async fn delete_todo(
    State(state): State<WebState>,
    target: TaskTarget,
    Path(todo_index): Path<i64>,
) -> Result<Html<String>, AppError> {
    let db = state.app.db.lock().await;
    let task_id = target.resolve(&db)?;

    let todo_service = TodoService::new(&db);
    let todo = todo_service.get_todo_by_index(task_id, TodoIndex::from_i64(todo_index))?;
    todo_service.delete_todo(todo.id)?;
    project_task_notes_or_warn(&db, task_id);

    // Broadcast SSE event
    state.app.broadcast(task_id, SseEvent::Todos);

    let html = render_todo_list_html(&state.templates, &db, task_id)?;
    Ok(Html(html))
}

/// Move a todo to the front (make it the next todo to work on)
pub async fn move_todo_to_next(
    State(state): State<WebState>,
    target: TaskTarget,
    Path(todo_index): Path<i64>,
) -> Result<Html<String>, AppError> {
    let db = state.app.db.lock().await;
    let task_id = target.resolve(&db)?;

    let todo_service = TodoService::new(&db);
    todo_service.move_to_next(task_id, TodoIndex::from_i64(todo_index))?;
    project_task_notes_or_warn(&db, task_id);

    // Broadcast SSE event
    state.app.broadcast(task_id, SseEvent::Todos);

    let html = render_todo_list_html(&state.templates, &db, task_id)?;
    Ok(Html(html))
}

/// Add a new scrap
pub async fn add_scrap(
    State(state): State<WebState>,
    target: TaskTarget,
    Form(form): Form<AddScrapForm>,
) -> Result<Html<String>, AppError> {
    let db = state.app.db.lock().await;
    let task_id = target.resolve(&db)?;

    let scrap_service = ScrapService::new(&db);
    let share = form
        .share
        .as_deref()
        .is_some_and(|v| v == "true" || v == "1" || v == "on");
    let visibility = if share {
        ScrapVisibility::Shared
    } else {
        ScrapVisibility::Local
    };
    let _scrap = scrap_service.add_scrap_with(task_id, &form.content, visibility)?;
    project_task_notes_or_warn(&db, task_id);

    // Broadcast SSE event
    state.app.broadcast(task_id, SseEvent::Scraps);

    // Return updated scrap list partial
    let scraps = scrap_service.list_scraps(task_id)?;
    let html = state.templates.render(
        "partials/scrap_list.html",
        serde_json::json!({
            "scraps": format_scraps(&scraps),
        }),
    )?;

    Ok(Html(html))
}

/// Toggle whether a scrap is included in the published snapshot.
pub async fn set_scrap_visibility(
    State(state): State<WebState>,
    target: TaskTarget,
    Path(scrap_index): Path<i64>,
) -> Result<Html<String>, AppError> {
    let db = state.app.db.lock().await;
    let task_id = target.resolve(&db)?;
    let scrap_service = ScrapService::new(&db);
    let scrap = scrap_service.get_scrap_by_index(task_id, ScrapIndex::from_i64(scrap_index))?;
    let next = if scrap.visibility.is_shared() {
        ScrapVisibility::Local
    } else {
        ScrapVisibility::Shared
    };
    scrap_service.set_visibility(task_id, ScrapIndex::from_i64(scrap_index), next)?;
    project_task_notes_or_warn(&db, task_id);
    state.app.broadcast(task_id, SseEvent::Scraps);
    let scraps = scrap_service.list_scraps(task_id)?;
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
    target: TaskTarget,
    Form(form): Form<UpdateDescriptionForm>,
) -> Result<Html<String>, AppError> {
    let db = state.app.db.lock().await;
    let task_id = target.resolve(&db)?;

    let task_service = TaskService::new(&db);
    task_service.set_description(task_id, &form.description)?;
    project_task_notes_or_warn(&db, task_id);

    // Get updated task
    let task = task_service.get_task(task_id)?;

    // Broadcast SSE event
    state.app.broadcast(task_id, SseEvent::Description);

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
    target: TaskTarget,
    Form(form): Form<UpdateTicketForm>,
) -> Result<Html<String>, AppError> {
    let db = state.app.db.lock().await;
    let task_id = target.resolve(&db)?;

    let task_service = TaskService::new(&db);

    // Clean up ticket_url if empty
    let ticket_url = form.ticket_url.filter(|url| !url.trim().is_empty());
    let ticket_url_str = ticket_url.as_deref().unwrap_or("");

    task_service.link_ticket(task_id, &form.ticket_id, ticket_url_str)?;
    project_task_notes_or_warn(&db, task_id);

    // Get updated task
    let task = task_service.get_task(task_id)?;

    // Broadcast SSE event
    state.app.broadcast(task_id, SseEvent::Ticket);

    // Ticket section plus out-of-band topbar identity so the badge updates immediately
    let html = render_ticket_mutation_html(&state.templates, &task)?;

    Ok(Html(html))
}

/// Add a new link
pub async fn add_link(
    State(state): State<WebState>,
    target: TaskTarget,
    Form(form): Form<AddLinkForm>,
) -> Result<Html<String>, AppError> {
    let db = state.app.db.lock().await;
    let task_id = target.resolve(&db)?;

    let link_service = LinkService::new(&db);

    // Clean up title if empty
    let title = form.title.filter(|t| !t.trim().is_empty());

    link_service.add_link(task_id, &form.url, title.as_deref())?;
    project_task_notes_or_warn(&db, task_id);

    // Broadcast SSE event
    state.app.broadcast(task_id, SseEvent::Links);

    // Return updated links list partial
    let links = link_service.list_links(task_id)?;
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
    target: TaskTarget,
    Path(link_index): Path<i64>,
) -> Result<Html<String>, AppError> {
    let db = state.app.db.lock().await;
    let task_id = target.resolve(&db)?;

    let link_service = LinkService::new(&db);
    let links = link_service.list_links(task_id)?;

    // Find link by task_index
    let link = links
        .iter()
        .find(|l| l.task_index == link_index)
        .ok_or(TrackError::LinkNotFound(link_index))?;

    // Delete link via service
    link_service.delete_link(link.id)?;
    project_task_notes_or_warn(&db, task_id);

    // Broadcast SSE event
    state.app.broadcast(task_id, SseEvent::Links);

    // Return updated links list partial
    let links = link_service.list_links(task_id)?;
    let html = state.templates.render(
        "partials/links.html",
        serde_json::json!({
            "links": links,
        }),
    )?;

    Ok(Html(html))
}
