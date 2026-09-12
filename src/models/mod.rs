//! Data models for the track CLI application.
//!
//! This module defines the core data structures used throughout the application,
//! including tasks, TODOs, links, scraps, and JJ-related items.

mod alias;
mod entities;
mod http_url;
mod ids;
mod jj;
mod markdown;
mod status;
mod ticket;
mod todo_action;
mod todo_add_options;
mod vcs_mode;
mod workflow;

pub use alias::TaskAlias;
pub use entities::{Link, RepoLink, Scrap, Task, TaskRepo, Todo, Worktree};
pub use http_url::HttpUrl;
pub use ids::{
    LinkId, LinkIndex, RepoIndex, RepoLinkId, ScrapId, ScrapIndex, TaskId, TaskRepoId, TodoId,
    TodoIndex, WorktreeId,
};
pub use jj::{JjSlug, jj_slug, sanitize_jj_slug};
pub use status::{TaskStatus, TodoStatus};
pub use ticket::TicketId;
pub use todo_action::{TodoAction, TodoAgentAction};
pub use todo_add_options::TodoAddOptions;
pub use vcs_mode::VcsMode;
pub use workflow::{
    AgentGuardrails, GitAgentContext, JjAgentContext, NextAction, NextActionKind, RepoRegistration,
    RepoWorkspaceStatus, TodoAgentView, WorkflowContext, WorkflowPhase, WorkflowStep,
    WorkspaceAgentView, WorkspaceFacts, WorkspaceLifecycle, build_next_action,
    build_workflow_checklist, build_workflow_context, compute_workflow_phase,
    jj_map_phase_is_complete, legacy_worktree_pending, legacy_worktree_sync_needed,
    oldest_pending_todo, workspace_lifecycle,
};
