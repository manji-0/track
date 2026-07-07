use crate::models::{
    build_git_context, build_jj_context, build_workflow_context, oldest_pending_todo,
    workspace_lifecycle, AgentGuardrails, GitAgentContext, JjAgentContext, Task, TaskRepo, Todo,
    TodoAction, TodoAgentView, TodoStatus, VcsMode, WorkflowContext, WorkspaceAgentView, Worktree,
};
use crate::services::WorktreeService;
use serde::Serialize;

/// Agent-oriented fields shared by `track status --json` and `/api/status`.
#[derive(Debug, Clone, Serialize)]
pub struct AgentStatusExtensions {
    pub vcs_mode: VcsMode,
    pub workflow: WorkflowContext,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jj: Option<JjAgentContext>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git: Option<GitAgentContext>,
    pub todos_agent: Vec<TodoAgentView>,
    pub guardrails: AgentGuardrails,
}

/// Builds agent-oriented JSON extensions for status endpoints.
pub fn build_agent_extensions(
    vcs_mode: VcsMode,
    task: &Task,
    todos: &[Todo],
    worktrees: &[Worktree],
    repos: &[TaskRepo],
    worktree_service: &WorktreeService<'_>,
) -> AgentStatusExtensions {
    let workflow = build_workflow_context(vcs_mode, task, todos, worktrees, repos);
    let next_todo_id = oldest_pending_todo(todos).map(|todo| todo.id);
    let legacy_merge_required = vcs_mode == VcsMode::Jj
        && todos
            .iter()
            .any(|todo| todo.status == TodoStatus::Pending && todo.worktree_requested);

    let git_ctx = build_git_context(task, repos);
    let git_worktree_path = git_ctx.workspace_path.clone();

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
                .map(|wt| wt.path.clone())
                .or_else(|| {
                    if vcs_mode == VcsMode::Git && todo.status == TodoStatus::Pending {
                        git_worktree_path.clone()
                    } else {
                        None
                    }
                });

            TodoAgentView {
                todo_id: todo.task_index,
                content: todo.content.clone(),
                status: todo.status,
                is_next: next_todo_id == Some(todo.id),
                allowed_actions: TodoAction::allowed_for(todo)
                    .into_iter()
                    .map(|action| action.as_str().to_string())
                    .chain(if todo.status == TodoStatus::Pending {
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

    let guardrails = AgentGuardrails::for_mode(vcs_mode, legacy_merge_required);

    let (jj, git) = match vcs_mode {
        VcsMode::Jj => (Some(build_jj_context(task, repos)), None),
        VcsMode::Git => (None, Some(git_ctx)),
    };

    AgentStatusExtensions {
        vcs_mode,
        workflow,
        jj,
        git,
        todos_agent,
        guardrails,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::models::WorkflowPhase;
    use crate::services::{TaskService, TodoService};

    #[test]
    fn agent_extensions_include_workflow_and_jj() {
        let db = Database::new_in_memory().unwrap();
        let task = TaskService::new(&db)
            .create_task("Task", None, None, None)
            .unwrap();
        let todo_service = TodoService::new(&db);
        todo_service.add_todo(task.id, "Work", false).unwrap();

        let todos = todo_service.list_todos(task.id).unwrap();
        let worktree_service = WorktreeService::new(&db);
        let extensions =
            build_agent_extensions(VcsMode::Jj, &task, &todos, &[], &[], &worktree_service);

        assert_eq!(extensions.workflow.phase, WorkflowPhase::Setup);
        assert_eq!(extensions.vcs_mode, VcsMode::Jj);
        assert_eq!(extensions.jj.as_ref().unwrap().skill, "jj");
        assert!(extensions.git.is_none());
        assert!(extensions.guardrails.must_use_jj_skill);
        assert!(extensions.guardrails.reopen_forbidden);
    }

    #[test]
    fn agent_extensions_use_git_context_in_git_mode() {
        let db = Database::new_in_memory().unwrap();
        db.set_vcs_mode(VcsMode::Git).unwrap();
        let task = TaskService::new(&db)
            .create_task("Task", None, None, None)
            .unwrap();
        let todo_service = TodoService::new(&db);
        todo_service.add_todo(task.id, "Work", false).unwrap();

        let todos = todo_service.list_todos(task.id).unwrap();
        let worktree_service = WorktreeService::new(&db);
        let extensions = build_agent_extensions(
            db.get_vcs_mode().unwrap(),
            &task,
            &todos,
            &[],
            &[],
            &worktree_service,
        );

        assert_eq!(extensions.vcs_mode, VcsMode::Git);
        assert!(extensions.jj.is_none());
        assert_eq!(extensions.git.as_ref().unwrap().branch, "track/task-1");
        assert!(!extensions.guardrails.must_use_jj_skill);
    }
}
