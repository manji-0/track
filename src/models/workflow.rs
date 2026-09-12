use crate::models::jj::{jj_slug, JjSlug};
use crate::models::{
    Task, TaskRepo, TaskStatus, Todo, TodoAgentAction, TodoStatus, VcsMode, Worktree,
};
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

/// Suggested next step kind for an agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NextActionKind {
    RunCommand,
    ExecuteTodo,
    UseJjSkill,
    WaitHuman,
}

/// Suggested next step for an agent.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct NextAction {
    pub kind: NextActionKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    pub reason: String,
}

/// A single item in the setup/sync checklist (agents and WebUI).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct WorkflowStep {
    pub id: &'static str,
    pub label: String,
    pub done: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
}

/// Derived workflow context (not persisted).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct WorkflowContext {
    pub phase: WorkflowPhase,
    pub next_action: NextAction,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub checklist: Vec<WorkflowStep>,
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
    pub allowed_actions: Vec<TodoAgentAction>,
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

/// Per-repository workspace registration for agent JSON.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RepoWorkspaceStatus {
    pub repo_path: String,
    pub registered: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
}

/// jj-task / agent-skill-jj context for the current task.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct JjAgentContext {
    pub slug: JjSlug,
    pub skill: &'static str,
    pub workspace_registered: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_phase: Option<String>,
    pub repos: Vec<RepoWorkspaceStatus>,
    pub start_command: String,
    pub path_command: String,
    pub repo_init_command: &'static str,
}

/// Git worktree context for the current task.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct GitAgentContext {
    pub slug: JjSlug,
    pub branch: String,
    pub workspace_ready: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_path: Option<String>,
    pub sync_command: String,
}

/// Guardrails exposed to agents via JSON status output.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AgentGuardrails {
    /// Commits, squash, push, and PR workflow belong to the `$jj` skill (jj mode only).
    pub must_use_jj_skill: bool,
    pub jj_skill_name: &'static str,
    pub reopen_forbidden: bool,
    /// True only for legacy per-TODO `--worktree` items managed by track.
    pub complete_requires_jj_merge: bool,
}

impl AgentGuardrails {
    pub fn for_mode(vcs_mode: VcsMode, complete_requires_jj_merge: bool) -> Self {
        match vcs_mode {
            VcsMode::Jj => Self {
                must_use_jj_skill: true,
                jj_skill_name: "jj",
                reopen_forbidden: true,
                complete_requires_jj_merge,
            },
            VcsMode::Git => Self {
                must_use_jj_skill: false,
                jj_skill_name: "jj",
                reopen_forbidden: true,
                complete_requires_jj_merge: false,
            },
        }
    }
}

impl Default for AgentGuardrails {
    fn default() -> Self {
        Self::for_mode(VcsMode::Jj, false)
    }
}

/// Observed VCS workspace state. Built at the service boundary (filesystem / jj-task map);
/// workflow functions stay pure given these facts.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorkspaceFacts {
    pub coding_workspace_ready: bool,
    pub slug_registered: bool,
    pub jj_repos_initialized: bool,
    pub registered_repo_count: usize,
    pub total_repo_count: usize,
    pub task_phase_completed: bool,
    pub workspace_path: Option<String>,
    pub repo_registrations: Vec<RepoRegistration>,
}

impl WorkspaceFacts {
    /// Facts for registered repos whose coding workspace has not been created yet.
    pub fn missing_workspace(repo_count: usize) -> Self {
        Self {
            total_repo_count: repo_count,
            ..Self::default()
        }
    }
}

/// One repo's jj-task / git registration, already observed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoRegistration {
    pub repo_path: String,
    pub registered: bool,
}

/// True when a jj-task map phase means the workspace is finished (`merged` or legacy `done`).
pub fn jj_map_phase_is_complete(phase: Option<&str>) -> bool {
    matches!(phase, Some("merged") | Some("done"))
}

fn pending_needs_workspace(todos: &[Todo]) -> bool {
    todos
        .iter()
        .any(|todo| todo.status == TodoStatus::Pending && todo.requires_workspace)
}

/// Repo registration is only a setup gate when pending work actually needs a workspace.
fn repo_registration_required(todos: &[Todo], repos: &[TaskRepo]) -> bool {
    repos.is_empty() && pending_needs_workspace(todos)
}

/// True when pending TODOs still use the legacy per-TODO `--worktree` model.
pub fn legacy_worktree_pending(todos: &[Todo]) -> bool {
    todos
        .iter()
        .any(|todo| todo.status == TodoStatus::Pending && todo.worktree_requested)
}

/// True when legacy TODOs need `track sync` to create missing workspaces.
pub fn legacy_worktree_sync_needed(todos: &[Todo], worktrees: &[Worktree]) -> bool {
    todos.iter().any(|todo| {
        todo.status == TodoStatus::Pending
            && todo.worktree_requested
            && !worktrees.iter().any(|wt| wt.todo_id == Some(todo.id))
    })
}

fn coding_workspace_missing(todos: &[Todo], repos: &[TaskRepo], facts: &WorkspaceFacts) -> bool {
    !repos.is_empty() && pending_needs_workspace(todos) && !facts.coding_workspace_ready
}

/// Computes the workflow phase from current task state and observed workspace facts.
///
/// An empty repository list is Setup only when the task has no TODOs yet, or
/// when pending TODOs require a workspace. Notes and `/plan` work skip repo
/// registration and go straight to Execute.
pub fn compute_workflow_phase(
    vcs_mode: VcsMode,
    task: &Task,
    todos: &[Todo],
    worktrees: &[Worktree],
    repos: &[TaskRepo],
    facts: &WorkspaceFacts,
) -> WorkflowPhase {
    if task.status == TaskStatus::Archived {
        return WorkflowPhase::Archived;
    }

    if todos.is_empty() && repos.is_empty() {
        return WorkflowPhase::Setup;
    }

    if repo_registration_required(todos, repos) {
        return WorkflowPhase::Setup;
    }

    let sync_needed = match vcs_mode {
        VcsMode::Jj => {
            legacy_worktree_sync_needed(todos, worktrees)
                || coding_workspace_missing(todos, repos, facts)
        }
        VcsMode::Git => coding_workspace_missing(todos, repos, facts),
    };

    if sync_needed {
        return WorkflowPhase::SyncRequired;
    }

    if todos.iter().any(|todo| todo.status == TodoStatus::Pending) {
        return WorkflowPhase::Execute;
    }

    WorkflowPhase::TaskComplete
}

/// Builds the full workflow context including checklist steps.
pub fn build_workflow_context(
    vcs_mode: VcsMode,
    task: &Task,
    todos: &[Todo],
    worktrees: &[Worktree],
    repos: &[TaskRepo],
    facts: &WorkspaceFacts,
) -> WorkflowContext {
    let phase = compute_workflow_phase(vcs_mode, task, todos, worktrees, repos, facts);
    WorkflowContext {
        next_action: build_next_action(vcs_mode, phase, task, todos, worktrees, facts),
        checklist: build_workflow_checklist(vcs_mode, phase, task, todos, repos, facts),
        phase,
    }
}

/// Builds a progress checklist for setup and sync phases.
pub fn build_workflow_checklist(
    vcs_mode: VcsMode,
    phase: WorkflowPhase,
    task: &Task,
    todos: &[Todo],
    repos: &[TaskRepo],
    facts: &WorkspaceFacts,
) -> Vec<WorkflowStep> {
    let slug = jj_slug(task);

    match phase {
        WorkflowPhase::Setup => {
            let mut steps = Vec::new();
            if pending_needs_workspace(todos) {
                steps.push(WorkflowStep {
                    id: "repo",
                    label: "Register at least one repository".to_string(),
                    done: !repos.is_empty(),
                    command: Some("track repo add".to_string()),
                });
                if vcs_mode == VcsMode::Jj && !repos.is_empty() {
                    steps.push(WorkflowStep {
                        id: "jj_repo_init",
                        label: "Initialize jj-task in each repo (once)".to_string(),
                        done: facts.jj_repos_initialized,
                        command: Some("jj-task repo init".to_string()),
                    });
                }
            }
            if todos.is_empty() {
                steps.push(WorkflowStep {
                    id: "todos",
                    label: "Add TODOs for this task".to_string(),
                    done: false,
                    command: Some("track todo add \"...\"".to_string()),
                });
            }
            steps
        }
        WorkflowPhase::SyncRequired => {
            let mut steps = Vec::new();
            if vcs_mode == VcsMode::Jj && !repos.is_empty() {
                for status in &facts.repo_registrations {
                    steps.push(WorkflowStep {
                        id: "jj_task_start",
                        label: format!("jj-task start in {}", status.repo_path),
                        done: status.registered,
                        command: Some(format!("jj-task start {slug}")),
                    });
                }
            } else if vcs_mode == VcsMode::Git {
                steps.push(WorkflowStep {
                    id: "git_sync",
                    label: "Create git worktree for this task".to_string(),
                    done: facts.coding_workspace_ready,
                    command: Some("track sync".to_string()),
                });
            }
            steps
        }
        _ => Vec::new(),
    }
}

/// Builds the suggested next action for the current workflow phase.
pub fn build_next_action(
    vcs_mode: VcsMode,
    phase: WorkflowPhase,
    task: &Task,
    todos: &[Todo],
    worktrees: &[Worktree],
    facts: &WorkspaceFacts,
) -> NextAction {
    let slug = jj_slug(task);

    match phase {
        WorkflowPhase::Setup => {
            if todos.is_empty() {
                NextAction {
                    kind: NextActionKind::RunCommand,
                    command: Some("track todo add \"...\"".to_string()),
                    reason: "Add TODOs to start work. Use --no-workspace (or /plan in the WebUI) when a repository is not needed.".to_string(),
                }
            } else {
                NextAction {
                    kind: NextActionKind::RunCommand,
                    command: Some("track repo add [path]".to_string()),
                    reason: match vcs_mode {
                        VcsMode::Jj => {
                            "Register at least one repository, then run jj-task repo init from the main workspace".to_string()
                        }
                        VcsMode::Git => {
                            "Register at least one git repository, then run track sync to create a worktree".to_string()
                        }
                    },
                }
            }
        }
        WorkflowPhase::SyncRequired => match vcs_mode {
            VcsMode::Jj => {
                if legacy_worktree_sync_needed(todos, worktrees) {
                    NextAction {
                        kind: NextActionKind::RunCommand,
                        command: Some("track sync".to_string()),
                        reason: "Legacy per-TODO --worktree workspaces are pending (prefer jj-task for new tasks)".to_string(),
                    }
                } else {
                    let missing = facts
                        .total_repo_count
                        .saturating_sub(facts.registered_repo_count);
                    let reason = if missing > 1 {
                        format!(
                            "Start jj-task workspace in each repo ({}/{} ready). Run jj-task repo init once from each main workspace if needed.",
                            facts.registered_repo_count,
                            facts.total_repo_count
                        )
                    } else {
                        format!(
                            "Start a jj-task workspace at .worktrees/{slug}. Run jj-task repo init once from the main repo if needed. Load the $jj skill for commits and PR."
                        )
                    };
                    NextAction {
                        kind: NextActionKind::RunCommand,
                        command: Some(format!("jj-task start {slug}")),
                        reason,
                    }
                }
            }
            VcsMode::Git => NextAction {
                kind: NextActionKind::RunCommand,
                command: Some("track sync".to_string()),
                reason: format!("Create git worktree at .worktrees/{slug} on branch track/{slug}"),
            },
        },
        WorkflowPhase::Execute => {
            let next_todo = oldest_pending_todo(todos);
            if let Some(todo) = next_todo {
                match vcs_mode {
                    VcsMode::Jj => {
                        if todo.requires_workspace {
                            if facts.coding_workspace_ready {
                                return NextAction {
                                    kind: NextActionKind::RunCommand,
                                    command: Some(format!("cd \"$(jj-task path {slug})\"")),
                                    reason: format!(
                                        "Work on TODO #{} in jj-task workspace. Use $jj skill for jj commit/squash/push — not jj describe alone.",
                                        todo.task_index
                                    ),
                                };
                            }

                            return NextAction {
                                kind: NextActionKind::RunCommand,
                                command: Some(format!("jj-task start {slug}")),
                                reason: format!(
                                    "Workspace required for TODO #{} — run jj-task start first",
                                    todo.task_index
                                ),
                            };
                        }

                        if facts.slug_registered {
                            return NextAction {
                                kind: NextActionKind::RunCommand,
                                command: Some(format!("cd \"$(jj-task path {slug})\"")),
                                reason: format!(
                                    "Optional: work in jj-task workspace for TODO #{}",
                                    todo.task_index
                                ),
                            };
                        }

                        let has_workspace = worktrees.iter().any(|wt| wt.todo_id == Some(todo.id));
                        if todo.worktree_requested && has_workspace {
                            return NextAction {
                                kind: NextActionKind::RunCommand,
                                command: Some(format!("track todo workspace {}", todo.task_index)),
                                reason: format!(
                                    "Legacy TODO workspace for #{}: {}",
                                    todo.task_index, todo.content
                                ),
                            };
                        }
                    }
                    VcsMode::Git => {
                        if todo.requires_workspace {
                            if facts.coding_workspace_ready {
                                if let Some(worktree_path) = facts.workspace_path.as_deref() {
                                    return NextAction {
                                        kind: NextActionKind::RunCommand,
                                        command: Some(format!("cd \"{worktree_path}\"")),
                                        reason: format!(
                                            "Work on TODO #{} in git worktree. Commit and push with standard git commands.",
                                            todo.task_index
                                        ),
                                    };
                                }
                            }
                            return NextAction {
                                kind: NextActionKind::RunCommand,
                                command: Some("track sync".to_string()),
                                reason: format!(
                                    "Git worktree required for TODO #{} — run track sync first",
                                    todo.task_index
                                ),
                            };
                        }
                        if let Some(worktree_path) = facts.workspace_path.as_deref() {
                            if facts.coding_workspace_ready {
                                return NextAction {
                                    kind: NextActionKind::RunCommand,
                                    command: Some(format!("cd \"{worktree_path}\"")),
                                    reason: format!(
                                        "Work on TODO #{} in git worktree. Commit and push with standard git commands.",
                                        todo.task_index
                                    ),
                                };
                            }
                        }
                    }
                }

                NextAction {
                    kind: NextActionKind::ExecuteTodo,
                    command: Some(format!("track todo done {}", todo.task_index)),
                    reason: format!("Continue with TODO #{}: {}", todo.task_index, todo.content),
                }
            } else {
                NextAction {
                    kind: NextActionKind::WaitHuman,
                    command: None,
                    reason: "No pending TODOs found".to_string(),
                }
            }
        }
        WorkflowPhase::TaskComplete => match vcs_mode {
            VcsMode::Jj => {
                let had_workspace_todos = todos.iter().any(|t| t.requires_workspace);
                if had_workspace_todos
                    && facts.coding_workspace_ready
                    && !facts.task_phase_completed
                {
                    NextAction {
                        kind: NextActionKind::UseJjSkill,
                        command: None,
                        reason: format!(
                            "All TODOs done — use $jj skill to push/merge PR, then `jj-task done {slug}` and `track archive`"
                        ),
                    }
                } else if facts.task_phase_completed {
                    NextAction {
                        kind: NextActionKind::RunCommand,
                        command: Some("track archive".to_string()),
                        reason: "jj-task workspace is merged — archive the track task".to_string(),
                    }
                } else {
                    NextAction {
                        kind: NextActionKind::RunCommand,
                        command: Some(format!("jj-task done {slug}; track archive")),
                        reason: "All TODOs done — use $jj skill to push/merge PR, then jj-task done and track archive".to_string(),
                    }
                }
            }
            VcsMode::Git => NextAction {
                kind: NextActionKind::RunCommand,
                command: Some("track archive".to_string()),
                reason: "All TODOs done — push/merge your PR with git, then archive the task"
                    .to_string(),
            },
        },
        WorkflowPhase::Archived => NextAction {
            kind: NextActionKind::WaitHuman,
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
            requires_workspace: true,
            created_at: Utc::now(),
            completed_at: None,
        }
    }

    fn sample_research_todo(index: i64) -> Todo {
        Todo {
            id: index,
            task_id: 1,
            task_index: index,
            content: format!("Research {}", index),
            status: TodoStatus::Pending,
            worktree_requested: false,
            requires_workspace: false,
            created_at: Utc::now(),
            completed_at: None,
        }
    }

    fn sample_repo() -> TaskRepo {
        TaskRepo {
            id: 1,
            task_id: 1,
            task_index: 1,
            repo_path: "/repo".to_string(),
            base_branch: None,
            base_commit_hash: None,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn workflow_phase_detects_legacy_sync_required() {
        let task = sample_task(TaskStatus::Active);
        let todos = vec![sample_todo(1, true)];
        let repos = vec![sample_repo()];

        assert_eq!(
            compute_workflow_phase(
                VcsMode::Jj,
                &task,
                &todos,
                &[],
                &repos,
                &WorkspaceFacts::missing_workspace(1)
            ),
            WorkflowPhase::SyncRequired
        );
    }

    #[test]
    fn workflow_phase_jj_needs_workspace_without_map() {
        let task = sample_task(TaskStatus::Active);
        let todos = vec![sample_todo(1, false)];
        let repos = vec![sample_repo()];

        assert_eq!(
            compute_workflow_phase(
                VcsMode::Jj,
                &task,
                &todos,
                &[],
                &repos,
                &WorkspaceFacts::missing_workspace(1)
            ),
            WorkflowPhase::SyncRequired
        );
    }

    #[test]
    fn workflow_phase_git_needs_sync_without_worktree() {
        let task = sample_task(TaskStatus::Active);
        let todos = vec![sample_todo(1, false)];
        let repos = vec![sample_repo()];

        assert_eq!(
            compute_workflow_phase(
                VcsMode::Git,
                &task,
                &todos,
                &[],
                &repos,
                &WorkspaceFacts::missing_workspace(1)
            ),
            WorkflowPhase::SyncRequired
        );
    }

    #[test]
    fn sync_required_action_prefers_jj_task_start() {
        let task = sample_task(TaskStatus::Active);
        let todos = vec![sample_todo(1, false)];
        let facts = WorkspaceFacts::missing_workspace(1);

        let action = build_next_action(
            VcsMode::Jj,
            WorkflowPhase::SyncRequired,
            &task,
            &todos,
            &[],
            &facts,
        );
        assert_eq!(action.command.as_deref(), Some("jj-task start proj-1"));
    }

    #[test]
    fn sync_required_action_uses_track_sync_in_git_mode() {
        let task = sample_task(TaskStatus::Active);
        let todos = vec![sample_todo(1, false)];
        let facts = WorkspaceFacts::missing_workspace(1);

        let action = build_next_action(
            VcsMode::Git,
            WorkflowPhase::SyncRequired,
            &task,
            &todos,
            &[],
            &facts,
        );
        assert_eq!(action.command.as_deref(), Some("track sync"));
    }

    #[test]
    fn plan_todos_without_repos_skip_setup() {
        let task = sample_task(TaskStatus::Active);
        let todos = vec![sample_research_todo(1)];
        let facts = WorkspaceFacts::default();

        assert_eq!(
            compute_workflow_phase(VcsMode::Jj, &task, &todos, &[], &[], &facts),
            WorkflowPhase::Execute
        );

        let action = build_next_action(
            VcsMode::Jj,
            WorkflowPhase::Execute,
            &task,
            &todos,
            &[],
            &facts,
        );
        assert_eq!(action.kind, NextActionKind::ExecuteTodo);
        assert_eq!(action.command.as_deref(), Some("track todo done 1"));

        let checklist = build_workflow_checklist(
            VcsMode::Jj,
            WorkflowPhase::Execute,
            &task,
            &todos,
            &[],
            &facts,
        );
        assert!(checklist.is_empty());
    }

    #[test]
    fn empty_task_without_repos_asks_for_todos_not_a_repo() {
        let task = sample_task(TaskStatus::Active);
        let facts = WorkspaceFacts::default();

        assert_eq!(
            compute_workflow_phase(VcsMode::Jj, &task, &[], &[], &[], &facts),
            WorkflowPhase::Setup
        );

        let action = build_next_action(VcsMode::Jj, WorkflowPhase::Setup, &task, &[], &[], &facts);
        assert_eq!(action.command.as_deref(), Some("track todo add \"...\""));

        let checklist =
            build_workflow_checklist(VcsMode::Jj, WorkflowPhase::Setup, &task, &[], &[], &facts);
        assert!(checklist.iter().all(|step| step.id != "repo"));
        assert!(checklist.iter().any(|step| step.id == "todos"));
    }

    #[test]
    fn workspace_todos_without_repos_still_require_repo_setup() {
        let task = sample_task(TaskStatus::Active);
        let todos = vec![sample_todo(1, false)];
        let facts = WorkspaceFacts::default();

        assert_eq!(
            compute_workflow_phase(VcsMode::Jj, &task, &todos, &[], &[], &facts),
            WorkflowPhase::Setup
        );

        let action = build_next_action(
            VcsMode::Jj,
            WorkflowPhase::Setup,
            &task,
            &todos,
            &[],
            &facts,
        );
        assert_eq!(action.command.as_deref(), Some("track repo add [path]"));

        let checklist = build_workflow_checklist(
            VcsMode::Jj,
            WorkflowPhase::Setup,
            &task,
            &todos,
            &[],
            &facts,
        );
        assert!(checklist.iter().any(|step| step.id == "repo"));
    }

    #[test]
    fn research_todo_skips_sync_required_without_workspace() {
        let task = sample_task(TaskStatus::Active);
        let todos = vec![sample_research_todo(1)];
        let repos = vec![sample_repo()];

        assert_eq!(
            compute_workflow_phase(
                VcsMode::Jj,
                &task,
                &todos,
                &[],
                &repos,
                &WorkspaceFacts::missing_workspace(1)
            ),
            WorkflowPhase::Execute
        );
    }

    #[test]
    fn guardrails_disable_jj_skill_in_git_mode() {
        let guardrails = AgentGuardrails::for_mode(VcsMode::Git, false);
        assert!(!guardrails.must_use_jj_skill);
        assert!(guardrails.reopen_forbidden);
    }

    #[test]
    fn research_todo_without_workspace_skips_sync_in_jj_mode() {
        let task = sample_task(TaskStatus::Active);
        let todos = vec![sample_research_todo(1)];
        let repos = vec![sample_repo()];

        assert_eq!(
            compute_workflow_phase(
                VcsMode::Jj,
                &task,
                &todos,
                &[],
                &repos,
                &WorkspaceFacts::missing_workspace(1)
            ),
            WorkflowPhase::Execute
        );
    }

    #[test]
    fn execute_action_for_no_workspace_todo_suggests_done() {
        let task = sample_task(TaskStatus::Active);
        let todos = vec![sample_research_todo(1)];
        let facts = WorkspaceFacts::missing_workspace(1);

        let action = build_next_action(
            VcsMode::Jj,
            WorkflowPhase::Execute,
            &task,
            &todos,
            &[],
            &facts,
        );
        assert_eq!(action.kind, NextActionKind::ExecuteTodo);
        assert_eq!(action.command.as_deref(), Some("track todo done 1"));
    }

    #[test]
    fn ready_workspace_facts_skip_sync_for_coding_todos() {
        let task = sample_task(TaskStatus::Active);
        let todos = vec![sample_todo(1, false)];
        let repos = vec![sample_repo()];
        let facts = WorkspaceFacts {
            coding_workspace_ready: true,
            slug_registered: true,
            jj_repos_initialized: true,
            registered_repo_count: 1,
            total_repo_count: 1,
            task_phase_completed: false,
            workspace_path: Some("/repo/.worktrees/proj-1".to_string()),
            repo_registrations: vec![RepoRegistration {
                repo_path: "/repo".to_string(),
                registered: true,
            }],
        };

        assert_eq!(
            compute_workflow_phase(VcsMode::Jj, &task, &todos, &[], &repos, &facts),
            WorkflowPhase::Execute
        );

        let action = build_next_action(
            VcsMode::Jj,
            WorkflowPhase::Execute,
            &task,
            &todos,
            &[],
            &facts,
        );
        assert_eq!(
            action.command.as_deref(),
            Some("cd \"$(jj-task path proj-1)\"")
        );
    }

    #[test]
    fn jj_map_phase_is_complete_accepts_merged_and_legacy_done() {
        assert!(jj_map_phase_is_complete(Some("merged")));
        assert!(jj_map_phase_is_complete(Some("done")));
        assert!(!jj_map_phase_is_complete(Some("draft")));
        assert!(!jj_map_phase_is_complete(None));
    }

    #[test]
    fn next_action_kind_serializes_as_snake_case() {
        let action = NextAction {
            kind: NextActionKind::UseJjSkill,
            command: None,
            reason: "test".to_string(),
        };
        let json = serde_json::to_value(action).unwrap();
        assert_eq!(json["kind"], "use_jj_skill");
    }
}
