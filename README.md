# Track

A CLI tool for recording and managing developer work logs based on "context" (current work state).

## Features

- **Context-based task management**: Manage TODOs, notes, and repositories without specifying IDs each time by setting the current task
- **Ticket integration**: Integration with Jira, GitHub Issues, and GitLab Issues
- **Git Worktree management**: Automatically manage independent working directories for each task
- **Simple CLI**: Intuitive command structure
- **Fast**: Single binary implementation in Rust

## Installation

```bash
# Build
cargo build --release

# Install (optional)
cargo install --path .
```

## Quick Start

```bash
# Create a new task
track new "API Implementation" \
  --description "Implement RESTful API with JWT authentication" \
  --ticket PROJ-123 \
  --ticket-url https://jira.example.com/browse/PROJ-123

# List tasks
track list

# Add TODOs
track todo add "Design endpoints"
track todo add "Implement authentication"

# Add links
track link add https://figma.com/design/... "Figma Design Document"

# Add work notes
track scrap add "Completed DB design. See DESIGN.md for table structure"

# Create a worktree
track worktree add /path/to/repo

# Display current task information
track info
```

## Command Reference

### Task Management

| Command | Description |
|---------|-------------|
| `track new <name>` | Create a new task and set it as active |
| `track list [--all]` | Display task list |
| `track switch <task_id>` | Switch tasks |
| `track info` | Display detailed information about the current task |
| `track desc [description]` | View or set task description |
| `track ticket <ticket_id> <url>` | Link a ticket to the task |
| `track archive <task_id>` | Archive a task |

### TODO Management

TODOs are numbered sequentially within each task (1, 2, 3...). All TODO commands operate on the current task.

| Command | Description |
|---------|-------------|
| `track todo add <text> [--worktree]` | Add a TODO (optionally create worktrees) |
| `track todo list` | Display TODO list with task-scoped indices |
| `track todo update <index> <status>` | Update TODO status (index: 1, 2, 3...) |
| `track todo done <index>` | Complete a TODO (merges and removes worktrees) |
| `track todo delete <index>` | Delete a TODO |

### Link Management

| Command | Description |
|---------|-------------|
| `track link add <url> [title]` | Add a reference URL |
| `track link list` | Display link list |

### Scrap (Work Notes) Management

| Command | Description |
|---------|-------------|
| `track scrap add <content>` | Add a work note |
| `track scrap list` | Display note list |

### Repository Management

| Command | Description |
|---------|-------------|
| `track repo add [path]` | Register a repository to the current task |
| `track repo list` | Display registered repositories |
| `track repo remove <id>` | Remove a repository registration |

### Worktree Management

| Command | Description |
|---------|-------------|
| `track worktree sync` | Sync repositories and setup task branches |
| `track worktree add <repo_path> [branch]` | Create a worktree |
| `track worktree list` | Display worktree list |
| `track worktree link <worktree_id> <url>` | Link a URL to a worktree |
| `track worktree remove <worktree_id>` | Remove a worktree |

## Ticket Reference

You can reference tasks by ticket ID instead of task ID:

```bash
# Switch task by ticket ID
track switch t:PROJ-123

# Archive by ticket ID
track archive t:PROJ-123
```

## Branch Naming Convention

For tasks with registered tickets, the ticket ID is automatically used when creating worktrees:

```bash
# When ticket PROJ-123 is registered
track worktree add /path/to/repo
# → Branch: task/PROJ-123

track worktree add /path/to/repo feat-auth
# → Branch: PROJ-123/feat-auth
```

## Database

Data is stored at the following path:

```
$HOME/.local/share/track/track.db
```

Complies with the XDG Base Directory specification.

## Technology Stack

- **Language**: Rust (Edition 2021)
- **CLI**: clap v4.4+
- **Database**: SQLite (rusqlite with bundled feature)
- **Error handling**: anyhow, thiserror
- **Date/time**: chrono
- **Display**: prettytable-rs

## Project Structure

See [PROJECT_STRUCTURE.md](PROJECT_STRUCTURE.md) for details.

## Documentation

- [DESIGN.md](DESIGN.md) - Design specification
- [docs/FUNCTIONAL_SPEC.md](docs/FUNCTIONAL_SPEC.md) - Functional specification
- [PROJECT_STRUCTURE.md](PROJECT_STRUCTURE.md) - Project structure

## License

MIT License

## Development

```bash
# Development build
cargo build

# Run tests
cargo test

# Release build
cargo build --release
```
