//! Business logic services for the track CLI application.
//!
//! This module contains services that implement the core business logic for managing
//! tasks, TODOs, repositories, worktrees, links, and scraps. Each service encapsulates
//! operations related to its domain and interacts with the database layer.

pub mod link_service;
pub mod repo_service;
pub mod task_service;
pub mod todo_service;
pub mod worktree_service;

pub use link_service::{LinkService, ScrapService};
pub use repo_service::RepoService;
pub use task_service::TaskService;
pub use todo_service::TodoService;
pub use worktree_service::WorktreeService;
