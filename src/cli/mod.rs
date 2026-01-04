//! Command-line interface definitions for the track CLI.
//!
//! This module defines the CLI structure using `clap`, including all commands,
//! subcommands, and their arguments. The actual command handling logic is in the
//! [`handler`] module.

pub mod handler;

use clap::{Parser, Subcommand, ValueEnum};

/// Types of completion data that can be output
#[derive(Debug, Clone, ValueEnum)]
pub enum CompletionType {
    /// Task IDs and names for 'track switch'
    Tasks,
    /// TODO IDs and content for current task
    Todos,
    /// Link IDs and URLs for current task
    Links,
    /// Repository IDs and paths for current task
    Repos,
}

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

        /// Template task reference (ID, ticket, or alias) to copy TODOs from
        #[arg(long)]
        template: Option<String>,
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
        /// Task ID or ticket reference (e.g., 1 or t:PROJ-123)
        id: Option<String>,

        /// Output in JSON format
        #[arg(short, long)]
        json: bool,

        /// Show all scraps
        #[arg(short, long)]
        all: bool,
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
        /// Task ID or ticket reference (defaults to current task)
        task_ref: Option<String>,
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

    /// Task alias management
    #[command(subcommand)]
    Alias(AliasCommands),

    /// Show help optimized for LLM agents
    LlmHelp,

    /// Generate shell completion script
    Completion {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },

    /// Output completion candidates (hidden, for shell completion scripts)
    #[command(hide = true)]
    #[command(name = "_complete")]
    Complete {
        /// Type of completion data to output
        #[arg(value_enum)]
        completion_type: CompletionType,
    },

    /// Start web-based user interface
    Webui {
        /// Port to listen on
        #[arg(short, long, default_value = "3000")]
        port: u16,

        /// Open browser automatically
        #[arg(short, long)]
        open: bool,
    },
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

    /// Delete a link
    Delete {
        /// Link index (1-based)
        index: usize,
    },
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

        /// Base branch to use (defaults to current branch)
        #[arg(short, long)]
        base: Option<String>,
    },

    /// List repositories
    List,

    /// Remove a repository
    Remove {
        /// Repository ID
        id: i64,
    },
}

#[derive(Subcommand)]
pub enum AliasCommands {
    /// Set an alias for the current task
    Set {
        /// Alias name
        alias: String,

        /// Target task ID (defaults to current task)
        #[arg(short, long)]
        task: Option<i64>,
    },

    /// Remove the alias from the current task
    Remove {
        /// Target task ID (defaults to current task)
        #[arg(short, long)]
        task: Option<i64>,
    },
}
