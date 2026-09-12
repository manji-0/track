use crate::models::{
    AgentGuardrails, GitAgentContext, JjAgentContext, RepoRegistration, Task, TaskRepo, Todo,
    TodoAgentAction, TodoAgentView, TodoStatus, VcsMode, WorkflowContext, WorkspaceAgentView,
    WorkspaceFacts, Worktree, build_workflow_context, jj_map_phase_is_complete, jj_slug,
    oldest_pending_todo, workspace_lifecycle,
};
use crate::services::{WorktreeService, git_worktree, jj_task};
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

fn repo_paths(repos: &[TaskRepo]) -> Vec<String> {
    repos.iter().map(|repo| repo.repo_path.clone()).collect()
}

/// Observes filesystem / jj-task map state so workflow functions stay pure.
pub fn observe_workspace(vcs_mode: VcsMode, task: &Task, repos: &[TaskRepo]) -> WorkspaceFacts {
    let slug = jj_slug(task);
    let paths = repo_paths(repos);

    match vcs_mode {
        VcsMode::Jj => {
            let statuses = jj_task::repos_workspace_status(&slug, &paths);
            let registered_repo_count = statuses.iter().filter(|status| status.registered).count();
            let coding_workspace_ready = !paths.is_empty() && registered_repo_count == paths.len();
            let workspace_path = if coding_workspace_ready {
                statuses
                    .iter()
                    .find_map(|status| status.workspace_path.clone())
            } else {
                paths
                    .first()
                    .map(|first_repo| jj_task::expected_workspace_path(first_repo, &slug))
            };
            let task_phase = jj_task::task_phase(&slug, &paths);

            WorkspaceFacts {
                coding_workspace_ready,
                slug_registered: jj_task::slug_registered(&slug, &paths),
                jj_repos_initialized: !repos.is_empty()
                    && repos
                        .iter()
                        .all(|repo| jj_task::repo_initialized(&repo.repo_path)),
                registered_repo_count,
                total_repo_count: paths.len(),
                task_phase_completed: jj_map_phase_is_complete(task_phase.as_deref()),
                workspace_path,
                repo_registrations: statuses
                    .iter()
                    .map(|status| RepoRegistration {
                        repo_path: status.repo_path.clone(),
                        registered: status.registered,
                    })
                    .collect(),
            }
        }
        VcsMode::Git => {
            let workspace_path = repos
                .first()
                .map(|repo| git_worktree::git_worktree_path(&repo.repo_path, &slug));
            let coding_workspace_ready = workspace_path
                .as_deref()
                .is_some_and(git_worktree::git_worktree_exists);

            WorkspaceFacts {
                coding_workspace_ready,
                slug_registered: coding_workspace_ready,
                jj_repos_initialized: false,
                registered_repo_count: usize::from(coding_workspace_ready),
                total_repo_count: repos.len(),
                task_phase_completed: false,
                workspace_path,
                repo_registrations: Vec::new(),
            }
        }
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

/// Builds agent-oriented JSON extensions for status endpoints.
pub fn build_agent_extensions(
    vcs_mode: VcsMode,
    task: &Task,
    todos: &[Todo],
    worktrees: &[Worktree],
    repos: &[TaskRepo],
    worktree_service: &WorktreeService<'_>,
) -> AgentStatusExtensions {
    let facts = observe_workspace(vcs_mode, task, repos);
    let workflow = build_workflow_context(vcs_mode, task, todos, worktrees, repos, &facts);
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
                allowed_actions: TodoAgentAction::allowed_for(todo),
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
    use crate::models::{TaskId, TaskStatus, TicketId, WorkflowPhase};
    use crate::services::{TaskService, TodoService};
    use chrono::Utc;

    fn sample_task() -> Task {
        Task {
            id: TaskId::from_i64(1),
            name: "Task".to_string(),
            description: None,
            status: TaskStatus::Active,
            ticket_id: Some(TicketId::from_stored("PROJ-1".to_string())),
            ticket_url: None,
            alias: None,
            is_today_task: false,
            created_at: Utc::now(),
        }
    }

    fn sample_repo() -> TaskRepo {
        TaskRepo {
            id: crate::models::TaskRepoId::from_i64(1),
            task_id: TaskId::from_i64(1),
            task_index: crate::models::RepoIndex::from_i64(1),
            repo_path: "/repo".to_string(),
            base_branch: None,
            base_commit_hash: None,
            created_at: Utc::now(),
        }
    }

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
        assert_eq!(
            extensions.todos_agent[0].allowed_actions,
            vec![
                TodoAgentAction::MakeNext,
                TodoAgentAction::Complete,
                TodoAgentAction::Cancel,
                TodoAgentAction::Delete,
            ]
        );
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

    #[test]
    fn build_jj_context_includes_slug_and_commands() {
        let task = sample_task();
        let ctx = build_jj_context(&task, &[]);
        assert_eq!(ctx.slug.as_str(), "proj-1");
        assert_eq!(ctx.skill, "jj");
        assert_eq!(ctx.start_command, "jj-task start proj-1");
    }

    #[test]
    fn build_git_context_includes_branch_and_sync() {
        let task = sample_task();
        let repos = vec![sample_repo()];
        let ctx = build_git_context(&task, &repos);
        assert_eq!(ctx.slug.as_str(), "proj-1");
        assert_eq!(ctx.branch, "track/proj-1");
        assert_eq!(ctx.sync_command, "track sync");
        assert!(!ctx.workspace_ready);
    }
}
