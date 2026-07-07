//! WebUI presentation: template context and JSON API responses.

use crate::db::Database;
use crate::models::TodoStatus;
use crate::models::{
    AgentGuardrails, GitAgentContext, JjAgentContext, Scrap, Todo, TodoAgentView, VcsMode,
    WorkflowContext, Worktree,
};
use crate::services::agent_context::{build_agent_extensions, AgentStatusExtensions};
use crate::services::WorktreeService;
use crate::use_cases::{GetTaskInfoUseCase, TaskInfoSnapshot};
use crate::utils::{Result, TrackError};
use serde::Serialize;

/// JSON API payload for `/api/status`.
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

impl StatusResponse {
    pub fn empty() -> Self {
        Self {
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
        }
    }
}

/// Build Minijinja context for the dashboard and status cards.
pub fn build_template_context(db: &Database, task_id: i64) -> Result<serde_json::Value> {
    let info = GetTaskInfoUseCase::new(db);
    let snapshot = info.load(task_id)?;
    let calendar_id = db.get_app_state("calendar_id").ok().flatten();
    to_template_context(db, &snapshot, calendar_id)
}

/// Build `/api/status` response for the active task.
pub fn build_api_status(db: &Database, snapshot: &TaskInfoSnapshot) -> Result<StatusResponse> {
    let agent = agent_extensions(db, snapshot)?;
    let todos = format_todos(&snapshot.todos, &snapshot.worktrees, &snapshot.scraps)?;

    Ok(StatusResponse {
        task: Some(to_json(&snapshot.task)?),
        todos,
        links: snapshot
            .links
            .iter()
            .map(to_json)
            .collect::<Result<Vec<_>>>()?,
        scraps: snapshot
            .scraps
            .iter()
            .map(to_json)
            .collect::<Result<Vec<_>>>()?,
        worktrees: snapshot
            .worktrees
            .iter()
            .map(to_json)
            .collect::<Result<Vec<_>>>()?,
        repos: snapshot
            .repos
            .iter()
            .map(to_json)
            .collect::<Result<Vec<_>>>()?,
        workflow: Some(agent.workflow),
        vcs_mode: Some(agent.vcs_mode),
        jj: agent.jj,
        git: agent.git,
        todos_agent: Some(agent.todos_agent),
        guardrails: Some(agent.guardrails),
    })
}

fn to_template_context(
    db: &Database,
    snapshot: &TaskInfoSnapshot,
    calendar_id: Option<String>,
) -> Result<serde_json::Value> {
    let agent = agent_extensions(db, snapshot)?;

    Ok(serde_json::json!({
        "task": snapshot.task,
        "todos": format_todos(&snapshot.todos, &snapshot.worktrees, &snapshot.scraps)?,
        "links": snapshot.links,
        "scraps": format_scraps(&snapshot.scraps),
        "worktrees": snapshot.worktrees,
        "repos": snapshot.repos,
        "base_branch": GetTaskInfoUseCase::base_bookmark(snapshot),
        "calendar_id": calendar_id,
        "workflow": agent.workflow,
        "vcs_mode": agent.vcs_mode,
        "jj": agent.jj,
        "git": agent.git,
        "guardrails": agent.guardrails,
    }))
}

fn agent_extensions(db: &Database, snapshot: &TaskInfoSnapshot) -> Result<AgentStatusExtensions> {
    let worktree_service = WorktreeService::new(db);
    Ok(build_agent_extensions(
        snapshot.vcs_mode,
        &snapshot.task,
        &snapshot.todos,
        &snapshot.worktrees,
        &snapshot.repos,
        &worktree_service,
    ))
}

fn to_json<T: serde::Serialize>(value: &T) -> Result<serde_json::Value> {
    serde_json::to_value(value).map_err(|err| TrackError::SerializationFailed(err.to_string()))
}

/// Format todos with worktree information and template-only fields.
pub fn format_todos(
    todos: &[Todo],
    worktrees: &[Worktree],
    scraps: &[Scrap],
) -> Result<Vec<serde_json::Value>> {
    let oldest_pending_id = todos
        .iter()
        .filter(|todo| todo.status == TodoStatus::Pending)
        .min_by_key(|todo| todo.task_index)
        .map(|todo| todo.id);

    todos
        .iter()
        .map(|todo| {
            let todo_worktrees: Vec<String> = worktrees
                .iter()
                .filter(|worktree| worktree.todo_id == Some(todo.id))
                .map(|worktree| worktree.path.clone())
                .collect();

            let scrap_count = scraps
                .iter()
                .filter(|scrap| scrap.active_todo_id == Some(todo.task_index))
                .count();

            let is_in_progress = oldest_pending_id == Some(todo.id);
            let mut value = to_json(todo)?;

            if let Some(obj) = value.as_object_mut() {
                obj.insert(
                    "worktree_requested".to_string(),
                    serde_json::Value::Bool(todo.worktree_requested),
                );
                obj.insert(
                    "requires_workspace".to_string(),
                    serde_json::Value::Bool(todo.requires_workspace),
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
                obj.insert(
                    "is_in_progress".to_string(),
                    serde_json::Value::Bool(is_in_progress),
                );
            }

            Ok(value)
        })
        .collect()
}

/// Format scraps with human-readable timestamps for templates.
pub fn format_scraps(scraps: &[Scrap]) -> Vec<serde_json::Value> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::{TaskService, TodoService};

    #[test]
    fn template_context_includes_formatted_todos() {
        let db = Database::new_in_memory().unwrap();
        let task = TaskService::new(&db)
            .create_task("Web", None, None, None)
            .unwrap();
        TodoService::new(&db)
            .add_todo(task.id, "Item", false)
            .unwrap();

        let context = build_template_context(&db, task.id).unwrap();
        assert_eq!(context["task"]["name"], "Web");
        assert_eq!(
            context["todos"].as_array().map(|items| items.len()),
            Some(1)
        );
        assert!(context["todos"][0]["content_html"].is_string());
    }
}
