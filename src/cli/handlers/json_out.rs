//! Agent-oriented JSON output for CLI mutations and task lists.

use crate::cli::handlers::CommandCtx;
use crate::db::Database;
use crate::models::{Task, TaskId};
use crate::services::TaskService;
use crate::use_cases::GetTaskInfoUseCase;
use crate::utils::{Result, TrackError};
use serde::Serialize;
use serde_json::{json, Value};

/// Kind of CLI mutation encoded in `--json` responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MutationKind {
    TaskNew,
    Switch,
    TodoAdd,
    TodoDone,
    TodoUpdate,
    TodoDelete,
    TodoNext,
    ScrapAdd,
    RepoAdd,
    Archive,
}

/// Envelope added on top of `track status --json`.
#[derive(Debug, Clone, Serialize)]
pub struct Mutation {
    pub kind: MutationKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
}

/// Builds status JSON for `task_id`, or an empty snapshot when the id is missing.
pub fn status_json(db: &Database, task_id: Option<TaskId>) -> Result<Value> {
    match task_id {
        Some(id) => {
            let info = GetTaskInfoUseCase::new(db);
            let snapshot = info.load(id)?;
            info.to_cli_json(&snapshot)
        }
        None => Ok(json!({
            "task": null,
            "todos": [],
            "links": [],
            "scraps": [],
            "worktrees": [],
            "repos": []
        })),
    }
}

/// Status snapshot plus `ok` and `mutation` for a successful write.
pub fn mutation_json(
    db: &Database,
    kind: MutationKind,
    id: Option<i64>,
    task_id: Option<TaskId>,
) -> Result<Value> {
    let resolved = match task_id {
        Some(id) => Some(id),
        None => db.get_current_task_id()?,
    };
    let mut output = status_json(db, resolved)?;
    let mutation = serde_json::to_value(Mutation { kind, id })
        .map_err(|err| TrackError::SerializationFailed(err.to_string()))?;
    if let Some(obj) = output.as_object_mut() {
        obj.insert("ok".to_string(), Value::Bool(true));
        obj.insert("mutation".to_string(), mutation);
    }
    Ok(output)
}

pub fn print_json(value: &Value) -> Result<()> {
    let json = serde_json::to_string_pretty(value)
        .map_err(|err| TrackError::SerializationFailed(err.to_string()))?;
    println!("{json}");
    Ok(())
}

pub fn emit_mutation(
    ctx: &CommandCtx<'_>,
    json: bool,
    kind: MutationKind,
    id: Option<i64>,
    task_id: Option<TaskId>,
    human: impl FnOnce(),
) -> Result<()> {
    if json {
        let value = mutation_json(ctx.db, kind, id, task_id)?;
        print_json(&value)
    } else {
        human();
        Ok(())
    }
}

pub fn list_json(db: &Database, include_archived: bool) -> Result<Value> {
    let task_service = TaskService::new(db);
    let tasks = task_service.list_tasks(include_archived)?;
    let current_task_id = db.get_current_task_id()?;

    let mut rows = Vec::with_capacity(tasks.len());
    for task in tasks {
        let is_current = current_task_id == Some(task.id);
        rows.push(task_list_row(&task, is_current)?);
    }

    Ok(json!({
        "current_task_id": current_task_id,
        "tasks": rows,
    }))
}

fn task_list_row(task: &Task, is_current: bool) -> Result<Value> {
    let mut value = serde_json::to_value(task)
        .map_err(|err| TrackError::SerializationFailed(err.to_string()))?;
    if let Some(obj) = value.as_object_mut() {
        obj.insert("is_current".to_string(), Value::Bool(is_current));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::TodoService;

    #[test]
    fn mutation_json_reuses_status_snapshot() {
        let db = Database::new_in_memory().unwrap();
        let task = TaskService::new(&db)
            .create_task("Task", None, None, None)
            .unwrap();
        TodoService::new(&db)
            .add_todo(task.id, "Work", false)
            .unwrap();

        let value = mutation_json(&db, MutationKind::TodoAdd, Some(1), None).unwrap();
        assert_eq!(value["ok"], true);
        assert_eq!(value["mutation"]["kind"], "todo_add");
        assert_eq!(value["mutation"]["id"], 1);
        assert_eq!(value["task"]["name"], "Task");
        assert!(value["workflow"]["phase"].is_string());
        assert!(value["todos_agent"].as_array().is_some());
        assert!(value["guardrails"]["reopen_forbidden"].as_bool().unwrap());
    }

    #[test]
    fn mutation_json_can_load_archived_task() {
        let db = Database::new_in_memory().unwrap();
        let task_service = TaskService::new(&db);
        let task = task_service.create_task("Done", None, None, None).unwrap();
        task_service.archive_task(task.id).unwrap();
        assert!(db.get_current_task_id().unwrap().is_none());

        let value = mutation_json(
            &db,
            MutationKind::Archive,
            Some(task.id.as_i64()),
            Some(task.id),
        )
        .unwrap();
        assert_eq!(value["mutation"]["kind"], "archive");
        assert_eq!(value["task"]["status"], "archived");
        assert_eq!(value["workflow"]["phase"], "archived");
    }

    #[test]
    fn list_json_marks_current_task() {
        let db = Database::new_in_memory().unwrap();
        let task_service = TaskService::new(&db);
        let first = task_service.create_task("One", None, None, None).unwrap();
        let second = task_service.create_task("Two", None, None, None).unwrap();

        let value = list_json(&db, false).unwrap();
        assert!(value.get("ok").is_none());
        assert!(value.get("mutation").is_none());
        assert_eq!(value["current_task_id"], second.id.as_i64());
        let tasks = value["tasks"].as_array().unwrap();
        assert_eq!(tasks.len(), 2);
        let one = tasks
            .iter()
            .find(|row| row["id"] == first.id.as_i64())
            .unwrap();
        let two = tasks
            .iter()
            .find(|row| row["id"] == second.id.as_i64())
            .unwrap();
        assert_eq!(one["is_current"], false);
        assert_eq!(two["is_current"], true);
        assert_eq!(two["name"], "Two");
    }
}
