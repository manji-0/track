use crate::models::{Task, TaskRepo, TaskStatus, Todo, TodoStatus, Worktree};
use serde::Serialize;

/// High-level workflow phase for agents and humans.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowPhase {
    Setup,
    SyncRequired,
    Execute,
    TaskComplete,
    Archived,
}

/// Suggested next step for an agent.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct NextAction {
    pub kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    pub reason: String,
}

/// Derived workflow context (not persisted).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct WorkflowContext {
    pub phase: WorkflowPhase,
    pub next_action: NextAction,
}

/// Lifecycle of a TODO's JJ workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceLifecycle {
    NotRequested,
    Requested,
    Ready,
    Merged,
}

/// Agent-oriented view of a TODO item.
#[derive(Debug, Clone, Serialize)]
pub struct TodoAgentView {
    pub todo_id: i64,
    pub content: String,
    pub status: TodoStatus,
    pub is_next: bool,
    pub allowed_actions: Vec<String>,
    pub workspace: WorkspaceAgentView,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceAgentView {
    pub lifecycle: WorkspaceLifecycle,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bookmark: Option<String>,
}

/// Guardrails exposed to agents via JSON status output.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AgentGuardrails {
    pub must_sync_before_code_changes: bool,
    pub complete_requires_jj_merge: bool,
    pub reopen_forbidden: bool,
}

impl Default for AgentGuardrails {
    fn default() -> Self {
        Self {
            must_sync_before_code_changes: true,
            complete_requires_jj_merge: true,
            reopen_forbidden: true,
        }
    }
}

/// Computes the workflow phase from current task state.
pub fn compute_workflow_phase(
    task: &Task,
    todos: &[Todo],
    worktrees: &[Worktree],
    repos: &[TaskRepo],
) -> WorkflowPhase {
    if task.status == TaskStatus::Archived {
        return WorkflowPhase::Archived;
    }

    if repos.is_empty() {
        return WorkflowPhase::Setup;
    }

    let needs_sync = todos.iter().any(|todo| {
        todo.status == TodoStatus::Pending
            && todo.worktree_requested
            && !worktrees.iter().any(|wt| wt.todo_id == Some(todo.id))
    });

    if needs_sync {
        return WorkflowPhase::SyncRequired;
    }

    if todos.iter().any(|todo| todo.status == TodoStatus::Pending) {
        return WorkflowPhase::Execute;
    }

    WorkflowPhase::TaskComplete
}

/// Builds the suggested next action for the current workflow phase.
pub fn build_next_action(
    phase: WorkflowPhase,
    todos: &[Todo],
    worktrees: &[Worktree],
) -> NextAction {
    match phase {
        WorkflowPhase::Setup => NextAction {
            kind: "run_command",
            command: Some("track repo add [path]".to_string()),
            reason: "Register at least one repository for this task".to_string(),
        },
        WorkflowPhase::SyncRequired => NextAction {
            kind: "run_command",
            command: Some("track sync".to_string()),
            reason: "Pending TODOs requested workspaces that do not exist yet".to_string(),
        },
        WorkflowPhase::Execute => {
            let next_todo = oldest_pending_todo(todos);
            if let Some(todo) = next_todo {
                let has_workspace = worktrees.iter().any(|wt| wt.todo_id == Some(todo.id));
                if todo.worktree_requested && has_workspace {
                    NextAction {
                        kind: "run_command",
                        command: Some(format!("track todo workspace {}", todo.task_index)),
                        reason: format!("Work on TODO #{}: {}", todo.task_index, todo.content),
                    }
                } else {
                    NextAction {
                        kind: "execute_todo",
                        command: Some(format!("track todo done {}", todo.task_index)),
                        reason: format!(
                            "Continue with TODO #{}: {}",
                            todo.task_index, todo.content
                        ),
                    }
                }
            } else {
                NextAction {
                    kind: "wait_human",
                    command: None,
                    reason: "No pending TODOs found".to_string(),
                }
            }
        }
        WorkflowPhase::TaskComplete => NextAction {
            kind: "run_command",
            command: Some("track archive".to_string()),
            reason: "All TODOs are done or cancelled".to_string(),
        },
        WorkflowPhase::Archived => NextAction {
            kind: "wait_human",
            command: None,
            reason: "Task is archived".to_string(),
        },
    }
}

pub fn oldest_pending_todo(todos: &[Todo]) -> Option<&Todo> {
    todos
        .iter()
        .filter(|todo| todo.status == TodoStatus::Pending)
        .min_by_key(|todo| todo.task_index)
}

pub fn workspace_lifecycle(todo: &Todo, worktrees: &[Worktree]) -> WorkspaceLifecycle {
    if !todo.worktree_requested {
        return WorkspaceLifecycle::NotRequested;
    }

    if todo.status == TodoStatus::Done {
        return WorkspaceLifecycle::Merged;
    }

    if worktrees.iter().any(|wt| wt.todo_id == Some(todo.id)) {
        WorkspaceLifecycle::Ready
    } else {
        WorkspaceLifecycle::Requested
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn sample_task(status: TaskStatus) -> Task {
        Task {
            id: 1,
            name: "Task".to_string(),
            description: None,
            status,
            ticket_id: Some("PROJ-1".to_string()),
            ticket_url: None,
            alias: None,
            is_today_task: false,
            created_at: Utc::now(),
        }
    }

    fn sample_todo(index: i64, worktree_requested: bool) -> Todo {
        Todo {
            id: index,
            task_id: 1,
            task_index: index,
            content: format!("Todo {}", index),
            status: TodoStatus::Pending,
            worktree_requested,
            created_at: Utc::now(),
            completed_at: None,
        }
    }

    #[test]
    fn workflow_phase_detects_sync_required() {
        let task = sample_task(TaskStatus::Active);
        let todos = vec![sample_todo(1, true)];
        let repos = vec![TaskRepo {
            id: 1,
            task_id: 1,
            task_index: 1,
            repo_path: "/repo".to_string(),
            base_branch: None,
            base_commit_hash: None,
            created_at: Utc::now(),
        }];

        assert_eq!(
            compute_workflow_phase(&task, &todos, &[], &repos),
            WorkflowPhase::SyncRequired
        );
    }

    #[test]
    fn workflow_phase_execute_when_workspaces_exist() {
        let task = sample_task(TaskStatus::Active);
        let todos = vec![sample_todo(1, true)];
        let worktrees = vec![Worktree {
            id: 1,
            task_id: 1,
            path: "/repo/task-1".to_string(),
            branch: "PROJ-1-todo-1".to_string(),
            base_repo: Some("/repo".to_string()),
            status: "active".to_string(),
            created_at: Utc::now(),
            todo_id: Some(1),
            is_base: false,
        }];
        let repos = vec![TaskRepo {
            id: 1,
            task_id: 1,
            task_index: 1,
            repo_path: "/repo".to_string(),
            base_branch: None,
            base_commit_hash: None,
            created_at: Utc::now(),
        }];

        assert_eq!(
            compute_workflow_phase(&task, &todos, &worktrees, &repos),
            WorkflowPhase::Execute
        );
    }
}
