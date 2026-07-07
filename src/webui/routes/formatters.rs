//! Status and list formatting helpers for WebUI templates and APIs.

use crate::db::Database;
use crate::models::TodoStatus;
use crate::services::agent_context::build_agent_extensions;
use crate::services::{
    LinkService, RepoService, ScrapService, TaskService, TodoService, WorktreeService,
};
use crate::utils::Result;

/// Build status context for templates and JSON APIs.
pub fn build_status_context(db: &Database, task_id: i64) -> Result<serde_json::Value> {
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

    let base_branch = if let Some(base_wt) = worktrees.iter().find(|wt| wt.is_base) {
        base_wt.branch.clone()
    } else if let Some(ref ticket_id) = task.ticket_id {
        format!("task/{}", ticket_id)
    } else {
        format!("task/task-{}", task.id)
    };

    let calendar_id = db.get_app_state("calendar_id").ok().flatten();
    let vcs_mode = db.get_vcs_mode()?;
    let agent = build_agent_extensions(
        vcs_mode,
        &task,
        &todos,
        &worktrees,
        &repos,
        &worktree_service,
    );

    Ok(serde_json::json!({
        "task": task,
        "todos": format_todos(todos, &worktrees, &scraps),
        "links": links,
        "scraps": format_scraps(&scraps),
        "worktrees": worktrees,
        "repos": repos,
        "base_branch": base_branch,
        "calendar_id": calendar_id,
        "workflow": agent.workflow,
        "vcs_mode": agent.vcs_mode,
        "jj": agent.jj,
        "git": agent.git,
        "guardrails": agent.guardrails,
    }))
}

/// Format todos with worktree information and hidden fields.
pub fn format_todos(
    todos: Vec<crate::models::Todo>,
    worktrees: &[crate::models::Worktree],
    scraps: &[crate::models::Scrap],
) -> Vec<serde_json::Value> {
    let oldest_pending_id = todos
        .iter()
        .filter(|t| t.status == TodoStatus::Pending)
        .min_by_key(|t| t.task_index)
        .map(|t| t.id);

    todos
        .into_iter()
        .map(|todo| {
            let todo_worktrees: Vec<String> = worktrees
                .iter()
                .filter(|wt| wt.todo_id == Some(todo.id))
                .map(|wt| wt.path.clone())
                .collect();

            let scrap_count = scraps
                .iter()
                .filter(|s| s.active_todo_id == Some(todo.task_index))
                .count();

            let is_in_progress = oldest_pending_id == Some(todo.id);
            let mut value = serde_json::to_value(&todo).unwrap_or_default();

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
            value
        })
        .collect()
}

/// Format scraps with human-readable timestamps.
pub fn format_scraps(scraps: &[crate::models::Scrap]) -> Vec<serde_json::Value> {
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
