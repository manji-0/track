//! Command-line interface definitions for the track CLI.
//!
//! This module defines the CLI structure using `clap`, including all commands,
//! subcommands, and their arguments. The actual command handling logic is in the
//! [`handler`] module.

pub mod handler;
pub mod handlers;

use clap::{Parser, Subcommand, ValueEnum};

use crate::models::TodoStatus;

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
#[command(about = "Personal work-context manager for tasks, TODOs, and scraps", long_about = None)]
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

        /// Output the status snapshot after creating the task
        #[arg(short, long)]
        json: bool,
    },

    /// List tasks
    List {
        /// Include archived tasks
        #[arg(short, long)]
        all: bool,

        /// Output tasks as JSON
        #[arg(short, long)]
        json: bool,
    },

    /// Switch to a different task
    Switch {
        /// Task ID or ticket reference (e.g., 1 or t:PROJ-123)
        task_ref: String,

        /// Output the status snapshot after switching
        #[arg(short, long)]
        json: bool,
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

        /// Skip dirty-workspace checks (required when stdin is not a TTY)
        #[arg(short, long)]
        force: bool,

        /// Output the status snapshot after archiving
        #[arg(short, long)]
        json: bool,
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

    /// Restore a track task from git notes on the current branch
    Import {
        /// Repository path (defaults to current directory)
        path: Option<String>,

        /// Output the status snapshot after importing
        #[arg(short, long)]
        json: bool,
    },

    /// Push or fetch `refs/notes/track` for task replay
    #[command(subcommand)]
    Notes(NotesCommands),

    /// Sync repositories and setup task branches
    Sync {
        /// JJ mode: also run legacy bookmark / per-TODO workspace sync
        #[arg(long)]
        legacy: bool,
    },

    /// Migrate data between workflow models
    #[command(subcommand)]
    Migrate(MigrateCommands),

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

        /// Generate dynamic completion script (with real-time data)
        #[arg(short, long)]
        dynamic: bool,
    },

    /// Output completion candidates (hidden, for shell completion scripts)
    #[command(hide = true)]
    #[command(name = "_complete")]
    Complete {
        /// Type of completion data to output
        #[arg(value_enum)]
        completion_type: CompletionType,
    },

    /// Configuration management
    #[command(subcommand)]
    Config(ConfigCommands),

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
pub enum MigrateCommands {
    /// Clear legacy per-TODO worktree flags (switch to one workspace per task)
    LegacyWorktrees {
        /// Task ID or ticket reference (defaults to all tasks)
        task_ref: Option<String>,

        /// Show what would change without writing
        #[arg(long)]
        dry_run: bool,

        /// Remove legacy workspaces even when jj reports uncommitted changes
        #[arg(short, long)]
        force: bool,
    },
}

#[derive(Subcommand)]
pub enum TodoCommands {
    /// Add a new TODO
    Add {
        /// TODO content
        text: String,

        /// [DEPRECATED] Legacy per-TODO worktree — use one workspace per task (`track sync`)
        #[arg(short, long, hide = true)]
        worktree: bool,

        /// Research/planning TODO that does not need a git/jj workspace
        #[arg(long, conflicts_with = "worktree")]
        no_workspace: bool,

        /// Output the status snapshot after adding
        #[arg(short, long)]
        json: bool,
    },

    /// List TODOs
    List,

    /// Update TODO status
    Update {
        /// TODO ID
        id: i64,

        /// New status (`cancelled`; use `todo done` to complete). Reopen to pending is not allowed.
        status: TodoStatus,

        /// Output the status snapshot after updating
        #[arg(short, long)]
        json: bool,
    },

    /// Complete a TODO (merges worktree if exists)
    Done {
        /// TODO ID
        id: i64,

        /// Output the status snapshot after completing
        #[arg(short, long)]
        json: bool,
    },

    /// Create or show worktrees for a TODO in the current repo
    Workspace {
        /// TODO ID
        id: i64,

        /// Recreate worktrees from the latest bookmark
        #[arg(long)]
        recreate: bool,

        /// Force recreation even if uncommitted changes exist
        #[arg(short, long)]
        force: bool,

        /// Operate on all registered repos for the task
        #[arg(long)]
        all: bool,
    },

    /// Delete a TODO
    Delete {
        /// TODO ID
        id: i64,

        /// Skip confirmation (required when stdin is not a TTY)
        #[arg(short, long)]
        force: bool,

        /// Output the status snapshot after deleting
        #[arg(short, long)]
        json: bool,
    },

    /// Move a TODO to the front (make it the next todo to work on)
    Next {
        /// TODO ID
        id: i64,

        /// Output the status snapshot after reordering
        #[arg(short, long)]
        json: bool,
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

        /// Include this scrap in the published git-notes snapshot
        #[arg(long)]
        share: bool,

        /// Output the status snapshot after adding
        #[arg(short, long)]
        json: bool,
    },

    /// List scraps
    List,

    /// Mark a scrap as shared (included in git notes)
    Share {
        /// Scrap ID
        id: i64,

        /// Output the status snapshot after sharing
        #[arg(short, long)]
        json: bool,
    },

    /// Mark a scrap as local-only (excluded from git notes)
    Unshare {
        /// Scrap ID
        id: i64,

        /// Output the status snapshot after unsharing
        #[arg(short, long)]
        json: bool,
    },
}

#[derive(Subcommand)]
pub enum NotesCommands {
    /// Push `refs/notes/track` so others can replay the task
    Push {
        /// Git remote (default: origin)
        #[arg(long, default_value = "origin")]
        remote: String,
    },

    /// Fetch `refs/notes/track` from a remote
    Fetch {
        /// Git remote (default: origin)
        #[arg(long, default_value = "origin")]
        remote: String,
    },
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

        /// Output the status snapshot after registering
        #[arg(short, long)]
        json: bool,
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

        /// Force overwrite if alias already exists on another task
        #[arg(short, long)]
        force: bool,
    },

    /// Remove the alias from the current task
    Remove {
        /// Target task ID (defaults to current task)
        #[arg(short, long)]
        task: Option<i64>,
    },
}

#[derive(Subcommand)]
pub enum ConfigCommands {
    /// Set a configuration value (e.g. vcs-mode git|jj, aggressive-mode on|off)
    Set {
        /// Configuration key (e.g. vcs-mode, aggressive-mode)
        key: String,

        /// Configuration value
        value: String,
    },

    /// Set Google Calendar ID for today task
    SetCalendar {
        /// Google Calendar ID
        calendar_id: String,
    },

    /// Show current configuration
    Show,
}
