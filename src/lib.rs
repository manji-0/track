//! # Track - Git Worktree-Based Task Management CLI
//!
//! `track` is a command-line tool for managing development tasks and TODOs using Git worktrees.
//! It helps developers organize their work by creating isolated Git worktrees for each task,
//! managing TODOs, tracking progress, and maintaining context through notes (scraps) and links.
//!
//! ## Features
//!
//! - **Task Management**: Create, list, switch between, and archive development tasks
//! - **TODO Tracking**: Add, update, and complete TODOs with optional Git worktree creation
//! - **Git Worktree Integration**: Automatically create and manage Git worktrees for isolated work
//! - **Repository Linking**: Associate multiple Git repositories with tasks
//! - **Context Preservation**: Keep work notes (scraps) and relevant links with each task
//! - **Sync Operations**: Synchronize repositories and set up task branches across worktrees
//!
//! ## Quick Start
//!
//! ```bash
//! # Create a new task
//! track new "Implement feature X" --description "Add new feature"
//!
//! # Add a TODO with automatic worktree creation
//! track todo add "Write tests" --worktree
//!
//! # Sync repositories and create worktrees
//! track sync
//!
//! # View current task status
//! track status
//!
//! # Complete a TODO (automatically merges worktree)
//! track todo done 1
//! ```
//!
//! ## Modules
//!
//! - [`cli`]: Command-line interface definitions and handlers
//! - [`db`]: Database initialization and management
//! - [`models`]: Data models for tasks, TODOs, links, and scraps
//! - [`services`]: Business logic for task, TODO, repository, and worktree operations
//! - [`utils`]: Utility functions and error types

// Re-export modules for testing and external use
pub mod cli;
pub mod db;
pub mod models;
pub mod services;
pub mod utils;
