use crate::models::{
    build_next_action, compute_workflow_phase, oldest_pending_todo, workspace_lifecycle,
    AgentGuardrails, Task, TaskRepo, Todo, TodoAction, TodoAgentView, WorkflowContext,
    WorkspaceAgentView, Worktree,
};
use crate::services::WorktreeService;
use serde::Serialize;

/// Agent-oriented fields shared by `track status --json` and `/api/status`.
#[derive(Debug, Clone, Serialize)]
pub struct AgentStatusExtensions {
    pub workflow: WorkflowContext,
    pub todos_agent: Vec<TodoAgentView>,
    pub guardrails: AgentGuardrails,
}

/// Builds agent-oriented JSON extensions for status endpoints.
pub fn build_agent_extensions(
    task: &Task,
    todos: &[Todo],
    worktrees: &[Worktree],
    repos: &[TaskRepo],
    worktree_service: &WorktreeService<'_>,
) -> AgentStatusExtensions {
    let phase = compute_workflow_phase(task, todos, worktrees, repos);
    let next_todo_id = oldest_pending_todo(todos).map(|todo| todo.id);

    let workflow = WorkflowContext {
        phase,
        next_action: build_next_action(phase, todos, worktrees),
    };

    let todos_agent: Vec<TodoAgentView> = todos
        .iter()
        .map(|todo| {
            let lifecycle = workspace_lifecycle(todo, worktrees);
            let bookmark = if todo.worktree_requested {
                worktree_service
                    .get_todo_branch_name(task.id, task.ticket_id.as_deref(), todo.task_index)
                    .ok()
            } else {
                None
            };
            let path = worktrees
                .iter()
                .find(|wt| wt.todo_id == Some(todo.id))
                .map(|wt| wt.path.clone());

            TodoAgentView {
                todo_id: todo.task_index,
                content: todo.content.clone(),
                status: todo.status,
                is_next: next_todo_id == Some(todo.id),
                allowed_actions: TodoAction::allowed_for(todo)
                    .into_iter()
                    .map(|action| action.as_str().to_string())
                    .chain(if todo.status == crate::models::TodoStatus::Pending {
                        vec!["delete".to_string()]
                    } else {
                        Vec::new()
                    })
                    .collect(),
                workspace: WorkspaceAgentView {
                    lifecycle,
                    path,
                    bookmark,
                },
            }
        })
        .collect();

    AgentStatusExtensions {
        workflow,
        todos_agent,
        guardrails: AgentGuardrails::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::models::WorkflowPhase;
    use crate::services::{TaskService, TodoService};

    #[test]
    fn agent_extensions_include_workflow() {
        let db = Database::new_in_memory().unwrap();
        let task = TaskService::new(&db)
            .create_task("Task", None, None, None)
            .unwrap();
        let todo_service = TodoService::new(&db);
        todo_service.add_todo(task.id, "Work", false).unwrap();

        let todos = todo_service.list_todos(task.id).unwrap();
        let worktree_service = WorktreeService::new(&db);
        let extensions = build_agent_extensions(&task, &todos, &[], &[], &worktree_service);

        assert_eq!(extensions.workflow.phase, WorkflowPhase::Setup);
        assert!(extensions.guardrails.reopen_forbidden);
    }
}
