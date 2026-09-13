//! # Track — personal work-context manager
//!
//! CLI + Web UI for one implicit current task (TODOs, scraps, links) in XDG SQLite.
//! Track owns the coding workspace (git worktree or colocated jj workspace at
//! `.worktrees/<slug>` on `track/<slug>`). Switch backends with `track config set vcs-mode`.
//!
//! ```bash
//! track new "Implement feature X"
//! track repo add .
//! track todo add "Write tests"
//! track status --json
//! ```
//!
//! Modules: [`cli`], [`db`], [`models`], [`services`], [`use_cases`], [`utils`], [`webui`].

// Re-export modules for testing and external use
pub mod cli;
pub mod db;
pub mod models;
pub mod ports;
pub mod services;
pub mod use_cases;
pub mod utils;
pub mod webui;
