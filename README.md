<div align="center">
  <img src="static/track.svg" width="128" height="128" alt="Track Logo" />
  <h1>Track</h1>
</div>

A lightweight CLI tool for managing development tasks with integrated Git worktree support.

## Features

- **Context-based Task Management**: Switch to a task and all operations apply automatically
- **Ticket Integration**: Seamlessly integrate with Jira, GitHub Issues, and GitLab Issues
- **Git Worktree Management**: Automatically manage isolated working directories for parallel development
- **Web UI**: Modern browser-based interface with real-time updates


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
track new "Implement User Authentication" \
  --ticket AUTH-456 \
  --ticket-url https://jira.example.com/browse/AUTH-456

# Add TODOs
track todo add "Design database schema"
track todo add "Implement JWT token generation" --worktree

# Add reference links
track link add https://jwt.io/introduction "JWT Documentation"

# Record work notes
track scrap add "Using bcrypt for password hashing"

# Mark TODO as complete
track todo done 1

# View current task status
track status
```


## Command Reference

### Task Management

| Command | Description |
|---------|-------------|
| `track new <name>` | Create a new task and set it as active |
| `track new <name> --template <task_ref>` | Create task from template (copies TODOs) |
| `track list [--all]` | Display task list |
| `track switch <task_id>` | Switch tasks |
| `track status [id]` | Display task information |
| `track desc [description]` | View or set task description |
| `track ticket <ticket_id> <url>` | Link a ticket to the task |
| `track alias set <alias>` | Set an alias for the current task |
| `track alias remove` | Remove alias from the current task |
| `track archive [task_id]` | Archive a task |

### TODO Management

| Command | Description |
|---------|-------------|
| `track todo add <text> [--worktree]` | Add a TODO (optionally create worktrees) |
| `track todo list` | Display TODO list |
| `track todo update <index> <status>` | Update TODO status |
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
| `track repo add --base <branch>` | Register repository with custom base branch |
| `track repo list` | Display registered repositories |
| `track repo remove <id>` | Remove a repository registration |

### Sync

| Command | Description |
|---------|-------------|
| `track sync` | Sync repositories and setup task branches |

### Web UI

<img width="2520" height="2001" alt="de5e316bf86187756a01c867ddb199df" src="https://github.com/user-attachments/assets/49c5ce74-2eac-4448-87d7-1eadb4214743" />

| Command | Description |
|---------|-------------|
| `track webui` | Start web-based user interface on port 3000 |
| `track webui --port 8080` | Start on custom port |
| `track webui --open` | Start and open browser automatically |

The Web UI provides a modern, browser-based interface with real-time updates via Server-Sent Events (SSE).

## Additional Features

For detailed information on the following features, see [docs/USAGE_EXAMPLES.md](docs/USAGE_EXAMPLES.md):

- **Task Aliases**: Assign human-readable aliases to tasks
- **Task Templates**: Create new tasks from existing task templates
- **Ticket Reference**: Reference tasks by ticket ID
- **Branch Naming Convention**: Automatic branch naming based on ticket IDs
- **Git Worktree Workflows**: Detailed workflows for parallel development

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
- **Web UI**: Axum, MiniJinja, HTMX, SSE

## Project Structure

See [PROJECT_STRUCTURE.md](PROJECT_STRUCTURE.md) for details.

## Documentation

- [DESIGN.md](DESIGN.md) - Design specification
- [docs/FUNCTIONAL_SPEC.md](docs/FUNCTIONAL_SPEC.md) - Functional specification
- [PROJECT_STRUCTURE.md](PROJECT_STRUCTURE.md) - Project structure
- [docs/USAGE_EXAMPLES.md](docs/USAGE_EXAMPLES.md) - Detailed usage examples
- [docs/LLM_INTEGRATION.md](docs/LLM_INTEGRATION.md) - LLM agent integration guide

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
