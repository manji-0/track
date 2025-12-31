# WorkTracker CLI - Project Structure

## Overview

WorkTracker is a Rust-based CLI tool for managing development tasks and context. This document describes the project structure and architecture.

## Directory Structure

```
track/
├── Cargo.toml              # Project dependencies and metadata
├── DESIGN.md               # Design specification
├── docs/
│   └── FUNCTIONAL_SPEC.md  # Functional specification
└── src/
    ├── main.rs             # Application entry point
    ├── cli/                # CLI command definitions and handlers
    │   ├── mod.rs          # Command definitions using clap
    │   └── handler.rs      # Command execution logic
    ├── db/                 # Database layer
    │   └── mod.rs          # SQLite connection and schema
    ├── models/             # Data models
    │   └── mod.rs          # Task, Todo, Link, Scrap, GitItem, RepoLink
    ├── services/           # Business logic layer
    │   ├── mod.rs          # Service exports
    │   ├── task_service.rs     # Task CRUD operations
    │   ├── todo_service.rs     # TODO management
    │   ├── link_service.rs     # Link and Scrap management
    │   └── worktree_service.rs # Git worktree operations
    └── utils/              # Utilities
        ├── mod.rs          # Utility exports
        └── error.rs        # Custom error types
```

## Architecture

### Layered Architecture

```
┌─────────────────────────────────────┐
│         CLI Layer (clap)            │
│  Command parsing & user interaction │
└─────────────────┬───────────────────┘
                  │
┌─────────────────▼───────────────────┐
│       Command Handlers              │
│  Business logic orchestration       │
└─────────────────┬───────────────────┘
                  │
┌─────────────────▼───────────────────┐
│      Service Layer                  │
│  TaskService, TodoService, etc.     │
└─────────────────┬───────────────────┘
                  │
┌─────────────────▼───────────────────┐
│      Database Layer                 │
│  SQLite connection & queries        │
└─────────────────────────────────────┘
```

### Key Components

#### 1. CLI Layer (`src/cli/`)

- **mod.rs**: Defines command structure using `clap` derive macros
  - Main commands: `new`, `list`, `switch`, `info`, `ticket`, `archive`, `export`
  - Subcommands: `todo`, `link`, `scrap`, `worktree`

- **handler.rs**: Implements command execution logic
  - `CommandHandler`: Orchestrates service calls
  - User interaction (confirmations, table output)
  - Error handling and formatting

#### 2. Database Layer (`src/db/`)

- **mod.rs**: Database connection and schema management
  - XDG Base Directory compliant path (`~/.local/share/track/track.db`)
  - Schema initialization with foreign key constraints
  - App state management (current task tracking)

**Schema**:
- `app_state`: Application state (current task ID)
- `tasks`: Task metadata with ticket information
- `todos`: Task TODO items
- `links`: Reference URLs
- `scraps`: Work notes/logs
- `git_items`: Git worktree information
- `repo_links`: Repository-related URLs (PR, Issue, etc.)

#### 3. Service Layer (`src/services/`)

Each service encapsulates business logic for a specific domain:

- **TaskService**: Task lifecycle management
  - Create, read, update, archive tasks
  - Ticket validation and linking
  - Task reference resolution (ID or ticket-based)

- **TodoService**: TODO management
  - CRUD operations for TODOs
  - Status validation and updates

- **LinkService & ScrapService**: Reference and note management
  - URL validation for links
  - Chronological scrap storage

- **WorktreeService**: Git worktree integration
  - Worktree creation with automatic branch naming
  - Ticket-based branch naming (e.g., `task/PROJ-123`)
  - Repository link management with automatic kind detection

#### 4. Models (`src/models/`)

Data structures representing database entities:
- `Task`, `Todo`, `Link`, `Scrap`, `GitItem`, `RepoLink`
- Status enums: `TaskStatus`, `TodoStatus`

#### 5. Utilities (`src/utils/`)

- **error.rs**: Custom error types using `thiserror`
  - Comprehensive error variants for all failure cases
  - Automatic conversion from standard library errors

## Dependencies

| Crate | Purpose |
|-------|---------|
| `clap` (v4.4+) | CLI argument parsing with derive macros |
| `rusqlite` (bundled) | SQLite database operations |
| `directories` (v5.0) | XDG Base Directory path resolution |
| `anyhow` (v1.0) | Error context propagation |
| `thiserror` (v1.0) | Custom error type derivation |
| `chrono` (v0.4) | Date/time handling |
| `prettytable-rs` (v0.10) | Table formatting for output |

## Data Flow Example

### Creating a New Task

```
User: track new "API Implementation" --ticket PROJ-123

1. CLI Layer (mod.rs)
   └─> Parses command into Commands::New

2. Command Handler (handler.rs)
   └─> handle_new() called

3. Service Layer (task_service.rs)
   └─> TaskService::create_task()
       ├─> Validates ticket format
       ├─> Checks for duplicate tickets
       └─> Creates task in database

4. Database Layer (db/mod.rs)
   └─> INSERT into tasks table
   └─> UPDATE app_state (current_task_id)

5. Response
   └─> "Created task #1: API Implementation"
       "Ticket: PROJ-123"
       "Switched to task #1"
```

## Building and Running

```bash
# Build the project
cargo build

# Run with help
cargo run -- --help

# Create a new task
cargo run -- new "My Task" --ticket PROJ-123

# List tasks
cargo run -- list

# Add a TODO
cargo run -- todo add "Implement feature X"
```

## Testing

```bash
# Run tests (when implemented)
cargo test

# Run with verbose output
cargo test -- --nocapture
```

## Future Enhancements

- Export functionality (Markdown, JSON, YAML)
- Template support for LLM integration
- Full-text search
- Data import/export
- MCP Server integration
