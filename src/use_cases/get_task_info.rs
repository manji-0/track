use crate::db::Database;
use crate::models::{Link, Scrap, Task, TaskRepo, Todo, VcsMode, Worktree};
use crate::services::agent_context::build_agent_extensions;
use crate::services::{
    LinkService, RepoService, ScrapService, TaskService, TodoService, WorktreeService,
};
use crate::utils::{Result, TrackError};

/// Aggregated task detail for CLI status and agent tooling.
#[derive(Debug, Clone)]
pub struct TaskInfoSnapshot {
    pub task: Task,
    pub todos: Vec<Todo>,
    pub links: Vec<Link>,
    pub scraps: Vec<Scrap>,
    pub worktrees: Vec<Worktree>,
    pub repos: Vec<TaskRepo>,
    pub vcs_mode: VcsMode,
}

/// Loads task detail and builds CLI/JSON status views.
pub struct GetTaskInfoUseCase<'a> {
    db: &'a Database,
}

impl<'a> GetTaskInfoUseCase<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn load(&self, task_id: i64) -> Result<TaskInfoSnapshot> {
        let task_service = TaskService::new(self.db);
        let task = task_service.get_task(task_id)?;

        let todo_service = TodoService::new(self.db);
        let todos = todo_service.list_todos(task_id)?;

        let link_service = LinkService::new(self.db);
        let links = link_service.list_links(task_id)?;

        let scrap_service = ScrapService::new(self.db);
        let scraps = scrap_service.list_scraps(task_id)?;

        let worktree_service = WorktreeService::new(self.db);
        let worktrees = worktree_service.list_worktrees(task_id)?;

        let repo_service = RepoService::new(self.db);
        let repos = repo_service.list_repos(task_id)?;
        let vcs_mode = self.db.get_vcs_mode()?;

        Ok(TaskInfoSnapshot {
            task,
            todos,
            links,
            scraps,
            worktrees,
            repos,
            vcs_mode,
        })
    }

    pub fn base_bookmark(snapshot: &TaskInfoSnapshot) -> String {
        if let Some(base_wt) = snapshot.worktrees.iter().find(|wt| wt.is_base) {
            return base_wt.branch.clone();
        }

        if let Some(ticket_id) = &snapshot.task.ticket_id {
            format!("task/{ticket_id}")
        } else {
            format!("task/task-{}", snapshot.task.id)
        }
    }

    pub fn todo_worktree_branch(&self, snapshot: &TaskInfoSnapshot, todo: &Todo) -> Option<String> {
        if let Some(worktree) = snapshot
            .worktrees
            .iter()
            .find(|wt| wt.todo_id == Some(todo.id))
        {
            return Some(worktree.branch.clone());
        }

        if !todo.worktree_requested {
            return None;
        }

        WorktreeService::new(self.db)
            .get_todo_branch_name(
                snapshot.task.id,
                snapshot.task.ticket_id.as_deref(),
                todo.task_index,
            )
            .ok()
    }

    pub fn orphan_worktrees(snapshot: &TaskInfoSnapshot) -> Vec<&Worktree> {
        snapshot
            .worktrees
            .iter()
            .filter(|wt| wt.todo_id.is_none())
            .collect()
    }

    pub fn to_cli_json(&self, snapshot: &TaskInfoSnapshot) -> Result<serde_json::Value> {
        let worktree_service = WorktreeService::new(self.db);

        let mut todos_json = Vec::with_capacity(snapshot.todos.len());
        for todo in &snapshot.todos {
            let mut todo_val = serde_json::to_value(todo)
                .map_err(|e| TrackError::SerializationFailed(e.to_string()))?;
            if let Some(obj) = todo_val.as_object_mut() {
                let worktree_branch = self.todo_worktree_branch(snapshot, todo);
                obj.insert(
                    "worktree_branch".to_string(),
                    serde_json::to_value(worktree_branch)
                        .map_err(|e| TrackError::SerializationFailed(e.to_string()))?,
                );
            }
            todos_json.push(todo_val);
        }

        let mut worktrees_json = Vec::with_capacity(snapshot.worktrees.len());
        for worktree in &snapshot.worktrees {
            let mut wt_val = serde_json::to_value(worktree)
                .map_err(|e| TrackError::SerializationFailed(e.to_string()))?;
            if let Some(obj) = wt_val.as_object_mut() {
                obj.remove("id");
                obj.remove("is_base");
                obj.remove("task_id");

                let task_scoped_id = worktree.todo_id.and_then(|id| {
                    snapshot
                        .todos
                        .iter()
                        .find(|todo| todo.id == id)
                        .map(|todo| todo.task_index)
                });
                obj.insert(
                    "todo_id".to_string(),
                    serde_json::to_value(task_scoped_id)
                        .map_err(|e| TrackError::SerializationFailed(e.to_string()))?,
                );
            }
            worktrees_json.push(wt_val);
        }

        for todo in &snapshot.todos {
            if todo.worktree_requested
                && !snapshot
                    .worktrees
                    .iter()
                    .any(|wt| wt.todo_id == Some(todo.id))
            {
                let branch_name = self.todo_worktree_branch(snapshot, todo);
                worktrees_json.push(serde_json::json!({
                    "todo_id": todo.task_index,
                    "branch": branch_name,
                    "status": "requested",
                    "path": null,
                    "created_at": null,
                    "base_repo": null
                }));
            }
        }

        let mut output = serde_json::json!({
            "task": snapshot.task,
            "todos": todos_json,
            "links": snapshot.links,
            "scraps": snapshot.scraps,
            "worktrees": worktrees_json,
            "repos": snapshot.repos,
        });

        let agent = build_agent_extensions(
            snapshot.vcs_mode,
            &snapshot.task,
            &snapshot.todos,
            &snapshot.worktrees,
            &snapshot.repos,
            &worktree_service,
        );
        let agent_val = serde_json::to_value(&agent)
            .map_err(|e| TrackError::SerializationFailed(e.to_string()))?;
        if let Some(obj) = output.as_object_mut() {
            if let Some(agent_obj) = agent_val.as_object() {
                for (key, value) in agent_obj {
                    obj.insert(key.clone(), value.clone());
                }
            }
        }

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::TaskService;

    #[test]
    fn base_bookmark_uses_ticket_when_no_base_worktree() {
        let db = Database::new_in_memory().unwrap();
        let task = TaskService::new(&db)
            .create_task("Task", None, Some("ABC-1"), None)
            .unwrap();
        let snapshot = GetTaskInfoUseCase::new(&db).load(task.id).unwrap();

        assert_eq!(GetTaskInfoUseCase::base_bookmark(&snapshot), "task/ABC-1");
    }

    #[test]
    fn cli_json_includes_task_and_todos() {
        let db = Database::new_in_memory().unwrap();
        let task = TaskService::new(&db)
            .create_task("Task", None, None, None)
            .unwrap();
        TodoService::new(&db)
            .add_todo(task.id, "Do thing", false)
            .unwrap();

        let use_case = GetTaskInfoUseCase::new(&db);
        let snapshot = use_case.load(task.id).unwrap();
        let json = use_case.to_cli_json(&snapshot).unwrap();

        assert_eq!(json["task"]["name"], "Task");
        assert_eq!(json["todos"].as_array().map(|t| t.len()), Some(1));
    }
}
