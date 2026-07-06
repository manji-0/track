use crate::models::jj::jj_slug;
use crate::models::{Task, TaskRepo, TaskStatus, Todo, TodoStatus, Worktree};
use crate::services::jj_task;
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

/// jj-task / agent-skill-jj context for the current task.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct JjAgentContext {
    pub slug: String,
    pub skill: &'static str,
    pub workspace_registered: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_path: Option<String>,
    pub start_command: String,
    pub path_command: String,
    pub repo_init_command: &'static str,
}

/// Guardrails exposed to agents via JSON status output.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AgentGuardrails {
    /// Commits, squash, push, and PR workflow belong to the `$jj` skill.
    pub must_use_jj_skill: bool,
    pub jj_skill_name: &'static str,
    pub reopen_forbidden: bool,
    /// True only for legacy per-TODO `--worktree` items managed by track.
    pub complete_requires_jj_merge: bool,
}

impl Default for AgentGuardrails {
    fn default() -> Self {
        Self {
            must_use_jj_skill: true,
            jj_skill_name: "jj",
            reopen_forbidden: true,
            complete_requires_jj_merge: false,
        }
    }
}

fn repo_paths(repos: &[TaskRepo]) -> Vec<String> {
    repos.iter().map(|repo| repo.repo_path.clone()).collect()
}

fn legacy_worktree_sync_needed(todos: &[Todo], worktrees: &[Worktree]) -> bool {
    todos.iter().any(|todo| {
        todo.status == TodoStatus::Pending
            && todo.worktree_requested
            && !worktrees.iter().any(|wt| wt.todo_id == Some(todo.id))
    })
}

fn jj_workspace_needed(task: &Task, todos: &[Todo], repos: &[TaskRepo]) -> bool {
    if repos.is_empty() {
        return false;
    }
    let has_pending = todos.iter().any(|todo| todo.status == TodoStatus::Pending);
    if !has_pending {
        return false;
    }
    let slug = jj_slug(task);
    !jj_task::slug_registered(&slug, &repo_paths(repos))
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

    if legacy_worktree_sync_needed(todos, worktrees) || jj_workspace_needed(task, todos, repos) {
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
    task: &Task,
    todos: &[Todo],
    worktrees: &[Worktree],
    repos: &[TaskRepo],
) -> NextAction {
    let slug = jj_slug(task);
    let paths = repo_paths(repos);

    match phase {
        WorkflowPhase::Setup => NextAction {
            kind: "run_command",
            command: Some("track repo add [path]".to_string()),
            reason: "Register at least one repository, then run jj-task repo init from the main workspace".to_string(),
        },
        WorkflowPhase::SyncRequired => {
            if legacy_worktree_sync_needed(todos, worktrees) {
                NextAction {
                    kind: "run_command",
                    command: Some("track sync".to_string()),
                    reason: "Legacy per-TODO --worktree workspaces are pending (prefer jj-task for new tasks)".to_string(),
                }
            } else {
                NextAction {
                    kind: "run_command",
                    command: Some(format!("jj-task start {slug}")),
                    reason: format!(
                        "Start a jj-task workspace at .worktrees/{slug}. Run jj-task repo init once from the main repo if needed. Load the $jj skill for commits and PR."
                    ),
                }
            }
        }
        WorkflowPhase::Execute => {
            let next_todo = oldest_pending_todo(todos);
            if let Some(todo) = next_todo {
                if jj_task::slug_registered(&slug, &paths) {
                    return NextAction {
                        kind: "run_command",
                        command: Some(format!("cd \"$(jj-task path {slug})\"")),
                        reason: format!(
                            "Work on TODO #{} in jj-task workspace. Use $jj skill for jj commit/squash/push — not jj describe alone.",
                            todo.task_index
                        ),
                    };
                }

                let has_workspace = worktrees.iter().any(|wt| wt.todo_id == Some(todo.id));
                if todo.worktree_requested && has_workspace {
                    return NextAction {
                        kind: "run_command",
                        command: Some(format!("track todo workspace {}", todo.task_index)),
                        reason: format!(
                            "Legacy TODO workspace for #{}: {}",
                            todo.task_index, todo.content
                        ),
                    };
                }

                NextAction {
                    kind: "execute_todo",
                    command: Some(format!("track todo done {}", todo.task_index)),
                    reason: format!("Continue with TODO #{}: {}", todo.task_index, todo.content),
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
            command: Some(format!(
                "jj-task done {slug}; track archive"
            )),
            reason: "All TODOs done — use $jj skill to push/merge PR, then jj-task done and track archive".to_string(),
        },
        WorkflowPhase::Archived => NextAction {
            kind: "wait_human",
            command: None,
            reason: "Task is archived".to_string(),
        },
    }
}

pub fn build_jj_context(task: &Task, repos: &[TaskRepo]) -> JjAgentContext {
    let slug = jj_slug(task);
    let paths = repo_paths(repos);
    let workspace_registered = jj_task::slug_registered(&slug, &paths);
    let workspace_path = if workspace_registered {
        paths
            .iter()
            .find_map(|repo_path| jj_task::workspace_path(repo_path, &slug))
    } else {
        paths
            .first()
            .map(|first_repo| jj_task::expected_workspace_path(first_repo, &slug))
    };

    JjAgentContext {
        slug: slug.clone(),
        skill: "jj",
        workspace_registered,
        workspace_path,
        start_command: format!("jj-task start {slug}"),
        path_command: format!("jj-task path {slug}"),
        repo_init_command: "jj-task repo init",
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
    fn workflow_phase_detects_legacy_sync_required() {
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
    fn workflow_phase_jj_needs_workspace_without_map() {
        let task = sample_task(TaskStatus::Active);
        let todos = vec![sample_todo(1, false)];
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
    fn sync_required_action_prefers_jj_task_start() {
        let task = sample_task(TaskStatus::Active);
        let todos = vec![sample_todo(1, false)];
        let repos = vec![TaskRepo {
            id: 1,
            task_id: 1,
            task_index: 1,
            repo_path: "/repo".to_string(),
            base_branch: None,
            base_commit_hash: None,
            created_at: Utc::now(),
        }];

        let action = build_next_action(WorkflowPhase::SyncRequired, &task, &todos, &[], &repos);
        assert_eq!(action.command.as_deref(), Some("jj-task start proj-1"));
    }

    #[test]
    fn build_jj_context_includes_slug_and_commands() {
        let task = sample_task(TaskStatus::Active);
        let ctx = build_jj_context(&task, &[]);
        assert_eq!(ctx.slug, "proj-1");
        assert_eq!(ctx.skill, "jj");
        assert_eq!(ctx.start_command, "jj-task start proj-1");
    }
}
