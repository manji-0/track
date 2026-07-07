//! HTTP route handlers for the WebUI.

mod formatters;

use self::formatters::{build_status_context, format_scraps, format_todos};

use crate::models::TodoStatus;
use crate::models::{
    AgentGuardrails, GitAgentContext, JjAgentContext, TodoAgentView, VcsMode, WorkflowContext,
};
use crate::services::agent_context::build_agent_extensions;
use crate::services::{
    LinkService, RepoService, ScrapService, TaskService, TodoService, WorktreeService,
};
use crate::use_cases::ApplyTodoActionUseCase;
use crate::utils::TrackError;
use crate::webui::error::WebError;
use crate::webui::state::{AppState, SseEvent};
use crate::webui::templates::SharedTemplates;
use axum::{
    extract::{Path, State},
    response::Html,
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
pub type AppError = WebError;

/// Status response for JSON API
#[derive(Serialize)]
pub struct StatusResponse {
    pub task: Option<serde_json::Value>,
    pub todos: Vec<serde_json::Value>,
    pub links: Vec<serde_json::Value>,
    pub scraps: Vec<serde_json::Value>,
    pub worktrees: Vec<serde_json::Value>,
    pub repos: Vec<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow: Option<WorkflowContext>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vcs_mode: Option<VcsMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jj: Option<JjAgentContext>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git: Option<GitAgentContext>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub todos_agent: Option<Vec<TodoAgentView>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guardrails: Option<AgentGuardrails>,
}

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
                workflow: None,
                vcs_mode: None,
                jj: None,
                git: None,
                todos_agent: None,
                guardrails: None,
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

    let vcs_mode = db.get_vcs_mode()?;
    let agent = build_agent_extensions(
        vcs_mode,
        &task,
        &todos,
        &worktrees,
        &repos,
        &worktree_service,
    );

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
        workflow: Some(agent.workflow),
        vcs_mode: Some(agent.vcs_mode),
        jj: agent.jj,
        git: agent.git,
        todos_agent: Some(agent.todos_agent),
        guardrails: Some(agent.guardrails),
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

    if new_status.as_str() == TodoStatus::PENDING {
        todo_service.update_status(todo.id, &new_status)?;
    } else {
        let action = crate::models::TodoAction::from_web_route(&new_status)?;
        ApplyTodoActionUseCase::new(&db).execute(current_task_id, todo_index, action)?;
    }

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
