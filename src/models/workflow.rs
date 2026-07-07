use crate::models::jj::jj_slug;
use crate::models::{Task, TaskRepo, TaskStatus, Todo, TodoStatus, VcsMode, Worktree};
use crate::services::{git_worktree, jj_task};
use jj_task::RepoWorkspaceStatus;
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
    pub slug: String,
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

fn repo_paths(repos: &[TaskRepo]) -> Vec<String> {
    repos.iter().map(|repo| repo.repo_path.clone()).collect()
}

fn pending_needs_workspace(todos: &[Todo]) -> bool {
    todos
        .iter()
        .any(|todo| todo.status == TodoStatus::Pending && todo.requires_workspace)
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

fn jj_workspace_needed(task: &Task, todos: &[Todo], repos: &[TaskRepo]) -> bool {
    if repos.is_empty() || !pending_needs_workspace(todos) {
        return false;
    }
    let slug = jj_slug(task);
    !jj_task::all_repos_registered(&slug, &repo_paths(repos))
}

fn git_workspace_needed(task: &Task, todos: &[Todo], repos: &[TaskRepo]) -> bool {
    if repos.is_empty() || !pending_needs_workspace(todos) {
        return false;
    }
    let slug = jj_slug(task);
    !repos.iter().any(|repo| {
        let path = git_worktree::git_worktree_path(&repo.repo_path, &slug);
        git_worktree::git_worktree_exists(&path)
    })
}

/// Computes the workflow phase from current task state.
pub fn compute_workflow_phase(
    vcs_mode: VcsMode,
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

    let sync_needed = match vcs_mode {
        VcsMode::Jj => {
            legacy_worktree_sync_needed(todos, worktrees) || jj_workspace_needed(task, todos, repos)
        }
        VcsMode::Git => git_workspace_needed(task, todos, repos),
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
) -> WorkflowContext {
    let phase = compute_workflow_phase(vcs_mode, task, todos, worktrees, repos);
    WorkflowContext {
        next_action: build_next_action(vcs_mode, phase, task, todos, worktrees, repos),
        checklist: build_workflow_checklist(vcs_mode, phase, task, todos, repos),
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
) -> Vec<WorkflowStep> {
    let slug = jj_slug(task);
    let paths = repo_paths(repos);

    match phase {
        WorkflowPhase::Setup => {
            let mut steps = vec![WorkflowStep {
                id: "repo",
                label: "Register at least one repository".to_string(),
                done: !repos.is_empty(),
                command: Some("track repo add".to_string()),
            }];
            if vcs_mode == VcsMode::Jj && !repos.is_empty() {
                let all_init = repos
                    .iter()
                    .all(|repo| jj_task::repo_initialized(&repo.repo_path));
                steps.push(WorkflowStep {
                    id: "jj_repo_init",
                    label: "Initialize jj-task in each repo (once)".to_string(),
                    done: all_init,
                    command: Some("jj-task repo init".to_string()),
                });
            }
            steps.push(WorkflowStep {
                id: "todos",
                label: "Add TODOs for this task".to_string(),
                done: !todos.is_empty(),
                command: Some("track todo add \"...\"".to_string()),
            });
            steps
        }
        WorkflowPhase::SyncRequired => {
            let mut steps = Vec::new();
            if vcs_mode == VcsMode::Jj && !repos.is_empty() {
                let statuses = jj_task::repos_workspace_status(&slug, &paths);
                for status in statuses {
                    let label = format!("jj-task start in {}", status.repo_path);
                    steps.push(WorkflowStep {
                        id: "jj_task_start",
                        label,
                        done: status.registered,
                        command: Some(format!("jj-task start {slug}")),
                    });
                }
            } else if vcs_mode == VcsMode::Git {
                steps.push(WorkflowStep {
                    id: "git_sync",
                    label: "Create git worktree for this task".to_string(),
                    done: false,
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
    repos: &[TaskRepo],
) -> NextAction {
    let slug = jj_slug(task);
    let paths = repo_paths(repos);

    match phase {
        WorkflowPhase::Setup => NextAction {
            kind: "run_command",
            command: Some("track repo add [path]".to_string()),
            reason: match vcs_mode {
                VcsMode::Jj => {
                    "Register at least one repository, then run jj-task repo init from the main workspace".to_string()
                }
                VcsMode::Git => {
                    "Register at least one git repository, then run track sync to create a worktree".to_string()
                }
            },
        },
        WorkflowPhase::SyncRequired => match vcs_mode {
            VcsMode::Jj => {
                if legacy_worktree_sync_needed(todos, worktrees) {
                    NextAction {
                        kind: "run_command",
                        command: Some("track sync".to_string()),
                        reason: "Legacy per-TODO --worktree workspaces are pending (prefer jj-task for new tasks)".to_string(),
                    }
                } else {
                    let missing = jj_task::unregistered_repo_paths(&slug, &paths);
                    let reason = if missing.len() > 1 {
                        format!(
                            "Start jj-task workspace in each repo ({}/{} ready). Run jj-task repo init once from each main workspace if needed.",
                            paths.len() - missing.len(),
                            paths.len()
                        )
                    } else {
                        format!(
                            "Start a jj-task workspace at .worktrees/{slug}. Run jj-task repo init once from the main repo if needed. Load the $jj skill for commits and PR."
                        )
                    };
                    NextAction {
                        kind: "run_command",
                        command: Some(format!("jj-task start {slug}")),
                        reason,
                    }
                }
            }
            VcsMode::Git => NextAction {
                kind: "run_command",
                command: Some("track sync".to_string()),
                reason: format!(
                    "Create git worktree at .worktrees/{slug} on branch track/{slug}"
                ),
            },
        },
        WorkflowPhase::Execute => {
            let next_todo = oldest_pending_todo(todos);
            if let Some(todo) = next_todo {
                match vcs_mode {
                    VcsMode::Jj => {
                        if todo.requires_workspace {
                            if jj_task::all_repos_registered(&slug, &paths) {
                                return NextAction {
                                    kind: "run_command",
                                    command: Some(format!("cd \"$(jj-task path {slug})\"")),
                                    reason: format!(
                                        "Work on TODO #{} in jj-task workspace. Use $jj skill for jj commit/squash/push — not jj describe alone.",
                                        todo.task_index
                                    ),
                                };
                            }

                            return NextAction {
                                kind: "run_command",
                                command: Some(format!("jj-task start {slug}")),
                                reason: format!(
                                    "Workspace required for TODO #{} — run jj-task start first",
                                    todo.task_index
                                ),
                            };
                        }

                        if jj_task::slug_registered(&slug, &paths) {
                            return NextAction {
                                kind: "run_command",
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
                                kind: "run_command",
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
                            if let Some(repo) = repos.first() {
                                let worktree_path =
                                    git_worktree::git_worktree_path(&repo.repo_path, &slug);
                                if git_worktree::git_worktree_exists(&worktree_path) {
                                    return NextAction {
                                        kind: "run_command",
                                        command: Some(format!("cd \"{worktree_path}\"")),
                                        reason: format!(
                                            "Work on TODO #{} in git worktree. Commit and push with standard git commands.",
                                            todo.task_index
                                        ),
                                    };
                                }
                            }
                            return NextAction {
                                kind: "run_command",
                                command: Some("track sync".to_string()),
                                reason: format!(
                                    "Git worktree required for TODO #{} — run track sync first",
                                    todo.task_index
                                ),
                            };
                        }
                        if let Some(repo) = repos.first() {
                            let worktree_path =
                                git_worktree::git_worktree_path(&repo.repo_path, &slug);
                            if git_worktree::git_worktree_exists(&worktree_path) {
                                return NextAction {
                                    kind: "run_command",
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
        WorkflowPhase::TaskComplete => match vcs_mode {
            VcsMode::Jj => {
                let had_workspace_todos = todos.iter().any(|t| t.requires_workspace);
                let registered = jj_task::all_repos_registered(&slug, &paths);
                let phase = jj_task::task_phase(&slug, &paths);
                if had_workspace_todos && registered && phase.as_deref() != Some("done") {
                    NextAction {
                        kind: "use_jj_skill",
                        command: None,
                        reason: format!(
                            "All TODOs done — use $jj skill to push/merge PR, then `jj-task done {slug}` and `track archive`"
                        ),
                    }
                } else {
                    NextAction {
                        kind: "run_command",
                        command: Some(format!("jj-task done {slug}; track archive")),
                        reason: "All TODOs done — use $jj skill to push/merge PR, then jj-task done and track archive".to_string(),
                    }
                }
            }
            VcsMode::Git => NextAction {
                kind: "run_command",
                command: Some("track archive".to_string()),
                reason: "All TODOs done — push/merge your PR with git, then archive the task".to_string(),
            },
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
    let repo_statuses = jj_task::repos_workspace_status(&slug, &paths);
    let workspace_registered = jj_task::all_repos_registered(&slug, &paths);
    let task_phase = jj_task::task_phase(&slug, &paths);
    let workspace_path = if workspace_registered {
        repo_statuses
            .iter()
            .find_map(|status| status.workspace_path.clone())
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
        task_phase,
        repos: repo_statuses,
        start_command: format!("jj-task start {slug}"),
        path_command: format!("jj-task path {slug}"),
        repo_init_command: "jj-task repo init",
    }
}

pub fn build_git_context(task: &Task, repos: &[TaskRepo]) -> GitAgentContext {
    let slug = jj_slug(task);
    let branch = git_worktree::git_branch_name(&slug);
    let workspace_path = repos
        .first()
        .map(|repo| git_worktree::git_worktree_path(&repo.repo_path, &slug));
    let workspace_ready = workspace_path
        .as_deref()
        .is_some_and(git_worktree::git_worktree_exists);

    GitAgentContext {
        slug,
        branch,
        workspace_ready,
        workspace_path,
        sync_command: "track sync".to_string(),
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
            compute_workflow_phase(VcsMode::Jj, &task, &todos, &[], &repos),
            WorkflowPhase::SyncRequired
        );
    }

    #[test]
    fn workflow_phase_jj_needs_workspace_without_map() {
        let task = sample_task(TaskStatus::Active);
        let todos = vec![sample_todo(1, false)];
        let repos = vec![sample_repo()];

        assert_eq!(
            compute_workflow_phase(VcsMode::Jj, &task, &todos, &[], &repos),
            WorkflowPhase::SyncRequired
        );
    }

    #[test]
    fn workflow_phase_git_needs_sync_without_worktree() {
        let task = sample_task(TaskStatus::Active);
        let todos = vec![sample_todo(1, false)];
        let repos = vec![sample_repo()];

        assert_eq!(
            compute_workflow_phase(VcsMode::Git, &task, &todos, &[], &repos),
            WorkflowPhase::SyncRequired
        );
    }

    #[test]
    fn sync_required_action_prefers_jj_task_start() {
        let task = sample_task(TaskStatus::Active);
        let todos = vec![sample_todo(1, false)];
        let repos = vec![sample_repo()];

        let action = build_next_action(
            VcsMode::Jj,
            WorkflowPhase::SyncRequired,
            &task,
            &todos,
            &[],
            &repos,
        );
        assert_eq!(action.command.as_deref(), Some("jj-task start proj-1"));
    }

    #[test]
    fn sync_required_action_uses_track_sync_in_git_mode() {
        let task = sample_task(TaskStatus::Active);
        let todos = vec![sample_todo(1, false)];
        let repos = vec![sample_repo()];

        let action = build_next_action(
            VcsMode::Git,
            WorkflowPhase::SyncRequired,
            &task,
            &todos,
            &[],
            &repos,
        );
        assert_eq!(action.command.as_deref(), Some("track sync"));
    }

    #[test]
    fn build_jj_context_includes_slug_and_commands() {
        let task = sample_task(TaskStatus::Active);
        let ctx = build_jj_context(&task, &[]);
        assert_eq!(ctx.slug, "proj-1");
        assert_eq!(ctx.skill, "jj");
        assert_eq!(ctx.start_command, "jj-task start proj-1");
    }

    #[test]
    fn build_git_context_includes_branch_and_sync() {
        let task = sample_task(TaskStatus::Active);
        let repos = vec![sample_repo()];
        let ctx = build_git_context(&task, &repos);
        assert_eq!(ctx.slug, "proj-1");
        assert_eq!(ctx.branch, "track/proj-1");
        assert_eq!(ctx.sync_command, "track sync");
        assert!(!ctx.workspace_ready);
    }

    #[test]
    fn research_todo_skips_sync_required_without_workspace() {
        let task = sample_task(TaskStatus::Active);
        let todos = vec![sample_research_todo(1)];
        let repos = vec![sample_repo()];

        assert_eq!(
            compute_workflow_phase(VcsMode::Jj, &task, &todos, &[], &repos),
            WorkflowPhase::Execute
        );
    }

    #[test]
    fn guardrails_disable_jj_skill_in_git_mode() {
        let guardrails = AgentGuardrails::for_mode(VcsMode::Git, false);
        assert!(!guardrails.must_use_jj_skill);
        assert!(guardrails.reopen_forbidden);
    }
}
