//! Command-line interface definitions for the track CLI.
//!
//! This module defines the CLI structure using `clap`, including all commands,
//! subcommands, and their arguments. The actual command handling logic is in the
//! [`handler`] module.

pub mod handler;

use clap::{Parser, Subcommand};

/// Main CLI structure for the track application.
#[derive(Parser)]
#[command(name = "track")]
#[command(about = "WorkTracker CLI - Manage your development tasks and context", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create a new task and switch to it
    New {
        /// Task name
        name: String,

        /// Task description
        #[arg(short, long)]
        description: Option<String>,

        /// Ticket ID (e.g., PROJ-123, owner/repo/456)
        #[arg(short, long)]
        ticket: Option<String>,

        /// Ticket URL
        #[arg(long)]
        ticket_url: Option<String>,
    },

    /// List tasks
    List {
        /// Include archived tasks
        #[arg(short, long)]
        all: bool,
    },

    /// Switch to a different task
    Switch {
        /// Task ID or ticket reference (e.g., 1 or t:PROJ-123)
        task_ref: String,
    },

    /// Show detailed information about the current task
    Status {
        /// Output in JSON format
        #[arg(short, long)]
        json: bool,
    },

    /// View or set task description
    Desc {
        /// Description text (if omitted, displays current description)
        description: Option<String>,

        /// Target task ID (defaults to current task)
        #[arg(short, long)]
        task: Option<i64>,
    },

    /// Link a ticket to a task
    Ticket {
        /// Ticket ID
        ticket_id: String,

        /// Ticket URL
        url: String,

        /// Target task ID (defaults to current task)
        #[arg(long)]
        task: Option<i64>,
    },

    /// Archive a task
    Archive {
        /// Task ID or ticket reference
        task_ref: String,
    },

    /// TODO management
    #[command(subcommand)]
    Todo(TodoCommands),

    /// Link management
    #[command(subcommand)]
    Link(LinkCommands),

    /// Scrap (work notes) management
    #[command(subcommand)]
    Scrap(ScrapCommands),

    /// Sync repositories and setup task branches
    Sync,

    /// Repository management
    #[command(subcommand)]
    Repo(RepoCommands),

    /// Show help optimized for LLM agents
    LlmHelp,
}

#[derive(Subcommand)]
pub enum TodoCommands {
    /// Add a new TODO
    Add {
        /// TODO content
        text: String,

        /// Create worktrees for this TODO
        #[arg(short, long)]
        worktree: bool,
    },

    /// List TODOs
    List,

    /// Update TODO status
    Update {
        /// TODO ID
        id: i64,

        /// New status (pending, done, cancelled)
        status: String,
    },

    /// Complete a TODO (merges worktree if exists)
    Done {
        /// TODO ID
        id: i64,
    },

    /// Delete a TODO
    Delete {
        /// TODO ID
        id: i64,

        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },
}

#[derive(Subcommand)]
pub enum LinkCommands {
    /// Add a new link
    Add {
        /// URL
        url: String,

        /// Link title (defaults to URL)
        title: Option<String>,
    },

    /// List links
    List,
}

#[derive(Subcommand)]
pub enum ScrapCommands {
    /// Add a new scrap (work note)
    Add {
        /// Scrap content
        content: String,
    },

    /// List scraps
    List,
}

#[derive(Subcommand)]
pub enum RepoCommands {
    /// Add a repository to the current task
    Add {
        /// Repository path (defaults to current directory)
        path: Option<String>,
    },

    /// List repositories
    List,

    /// Remove a repository
    Remove {
        /// Repository ID
        id: i64,
    },
}
