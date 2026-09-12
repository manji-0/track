<div align="center">
  <img src="static/track.svg" width="128" height="128" alt="Track Logo" />
  <h1>Track</h1>
</div>

A personal work-context manager for humans and coding agents: one implicit current task in an XDG SQLite database (`$HOME/.local/share/track/track.db`). Track holds **what** you are doing (tasks, TODOs, scraps). [agent-skill-jj](https://github.com/manji-0/agent-skill-jj) (`$jj` + `jj-task`) holds **how** you commit. It is not a repo issue tracker or a multi-agent planner.

## Features

- **Implicit current task**: `track new` / `track switch` then `todo` / `scrap` / `link` without repeating IDs
- **Agent JSON**: `track status --json` and write commands with `--json` share `workflow`, `jj`, `todos_agent`, `guardrails`
- **Tickets as labels**: optional Jira / GitHub / GitLab IDs on a personal task — not the source of truth
- **Two-layer JJ**: `jj-task start <slug>` for workspaces; `$jj` for commits (see [docs/JJ_INTEGRATION.md](docs/JJ_INTEGRATION.md))
- **Web UI**: browser view with SSE, including [today task](docs/TODAY_TASK.md)


## Installation

```bash
# From crates.io (CLI binary is `track`)
cargo install task-track

# From a checkout
cargo install --path .
```

## Quick Start

```bash
# Create a new task
track new "Implement User Authentication" \
  --ticket AUTH-456 \
  --ticket-url https://jira.example.com/browse/AUTH-456

# Register repo and add TODOs
track repo add .
track todo add "Design database schema"
track todo add "Compare auth providers" --no-workspace   # research — no jj-task workspace

# Start jj-task workspace (from repo root; see docs/JJ_INTEGRATION.md)
jj-task repo init    # once per repo
jj-task start auth-456
cd "$(jj-task path auth-456)"

# Record work notes
track scrap add "Using bcrypt for password hashing"

# Mark TODO complete (track DB only; use $jj skill for jj commits)
track todo done 1

# Agent-oriented status
track status --json
```


## Command Reference

### Task Management

| Command | Description |
|---------|-------------|
| `track new <name> [--json]` | Create a new task and set it as active |
| `track new <name> --template <task_ref>` | Create task from template (copies TODOs) |
| `track list [--all] [--json]` | Display task list |
| `track switch <task_id> [--json]` | Switch tasks |
| `track switch today` | Switch to today's task (auto-creates if needed) |
| `track status [id]` | Display task information |
| `track status --json` | Output in JSON format |
| `track status --all` | Show all scraps |
| `track desc [description]` | View or set task description |
| `track ticket <ticket_id> <url>` | Link a ticket to the task |
| `track alias set <alias>` | Set an alias for the current task |
| `track alias set <alias> --force` | Overwrite existing alias on another task |
| `track alias remove` | Remove alias from the current task |
| `track archive [task_id] [--json]` | Archive a task |

### Configuration

| Command | Description |
|---------|-------------|
| `track config set-calendar <calendar-id>` | Set Google Calendar ID for today task |
| `track config show` | Show current configuration |

### TODO Management

| Command | Description |
|---------|-------------|
| `track todo add <text> [--no-workspace] [--json]` | Add a TODO (`--json` returns the status snapshot) |
| `track todo list` | Display TODO list |
| `track todo update <index> <status> [--json]` | Update TODO status |
| `track todo done <index> [--json]` | Complete a TODO |
| `track todo workspace <index> [--recreate --force --all]` | Show or recreate workspaces for a TODO |
| `track todo next <index> [--json]` | Move a TODO to the front (make it the next todo to work on) |
| `track todo delete <index>` | Delete a TODO |
| `track todo delete <index> --force [--json]` | Delete without confirmation |

### Link Management

| Command | Description |
|---------|-------------|
| `track link add <url> [title]` | Add a reference URL |
| `track link list` | Display link list |
| `track link delete <index>` | Delete a link |

### Scrap (Work Notes) Management

| Command | Description |
|---------|-------------|
| `track scrap add <content> [--json]` | Add a work note |
| `track scrap list` | Display note list |

### Repository Management

| Command | Description |
|---------|-------------|
| `track repo add [path] [--json]` | Register a repository to the current task |
| `track repo add --base <bookmark>` | Register repository with custom base bookmark |
| `track repo list` | Display registered repositories |
| `track repo remove <id>` | Remove a repository registration |

### Sync

| Command | Description |
|---------|-------------|
| `track sync` | Git mode: create the task worktree. JJ mode: legacy per-TODO `--worktree` only (`jj-task start` is the default) |

### Web UI

<img width="2520" height="2001" alt="de5e316bf86187756a01c867ddb199df" src="https://github.com/user-attachments/assets/49c5ce74-2eac-4448-87d7-1eadb4214743" />

| Command | Description |
|---------|-------------|
| `track webui` | Start web-based user interface on port 3000 |
| `track webui --port 8080` | Start on custom port |
| `track webui --open` | Start and open browser automatically |

The Web UI provides a modern, browser-based interface with real-time updates via Server-Sent Events (SSE).

**Key Features:**

- **Today Task**: Special task type that automatically inherits incomplete TODOs from the previous day. Access with `track switch today`.
- **Calendar Integration**: Display your Google Calendar in the today task view. Configure with `track config set-calendar <calendar-id>`.
- **Todo-Scrap Linking**: Click the 📝 button on any todo to jump to related scraps. Scraps are automatically linked to the active todo when created.
- **Todo Reordering**: Use the "⬆️ Make Next" option in the todo menu to move a todo to the front of your work queue.
- **Real-time Updates**: All changes are instantly reflected across all connected browsers.
- **Focus Mode**: Toggle between overview and focus modes to concentrate on the current task.
- **Dark/Light Theme**: Automatic theme switching with calendar color adaptation.
- **Safe Markdown Rendering**: Markdown is sanitized and raw HTML is stripped; links open safely in a new tab.

### Shell Completion

`track` provides shell completion scripts for bash, zsh, fish, and powershell.

| Command | Description |
|---------|-------------|
| `track completion bash` | Generate bash completion script |
| `track completion zsh` | Generate zsh completion script |
| `track completion fish` | Generate fish completion script |
| `track completion powershell` | Generate PowerShell completion script |

**Quick Install (Dynamic - Recommended):**

```bash
# Bash (dynamic)
mkdir -p ~/.local/share/bash-completion/completions
track completion bash --dynamic > ~/.local/share/bash-completion/completions/track
source ~/.local/share/bash-completion/completions/track

# Zsh (dynamic)
mkdir -p ~/.zsh/completions
track completion zsh --dynamic > ~/.zsh/completions/_track
# Add to ~/.zshrc: fpath=(~/.zsh/completions $fpath)
# Then: exec zsh

# Fish (static only)
mkdir -p ~/.config/fish/completions
track completion fish > ~/.config/fish/completions/track.fish
```

**What you get with dynamic completions:**

```bash
track switch <TAB>              # Shows your actual task IDs and names
track todo done <TAB>            # Shows pending TODO IDs with content
track todo update 6 <TAB>        # Shows status: pending, done, cancelled
track link delete <TAB>          # Shows link IDs with titles
track repo remove <TAB>          # Shows repository IDs with paths
track new --template <TAB>       # Shows task IDs for templates
```

For detailed installation instructions and troubleshooting, see [completions/README.md](completions/README.md).

## Additional Features

For detailed information on the following features, see [docs/USAGE_EXAMPLES.md](docs/USAGE_EXAMPLES.md):

- **Task Aliases**: Assign human-readable aliases to tasks
- **Task Templates**: Create new tasks from existing task templates
- **Ticket Reference**: Reference tasks by ticket ID
- **jj-task slug**: alias, else ticket id, else `task-{id}` — see [docs/JJ_INTEGRATION.md](docs/JJ_INTEGRATION.md)
- **JJ workflows**: two-layer track + `$jj` (not `track sync` before every edit)

## Database


Data is stored at the following path:

```
$HOME/.local/share/track/track.db
```

Complies with the XDG Base Directory specification.

## Technology Stack

- **Language**: Rust (Edition 2021)
- **CLI**: clap v4.6+
- **Database**: SQLite (rusqlite with bundled feature)
- **Error handling**: thiserror
- **Date/time**: chrono
- **Display**: prettytable-rs
- **Web UI**: Axum, MiniJinja, HTMX, SSE

## Project Structure

See [PROJECT_STRUCTURE.md](PROJECT_STRUCTURE.md) for details.

## Documentation

- [docs/README.md](docs/README.md) — documentation index
- [DESIGN.md](DESIGN.md) — product design (personal context, implicit current task)
- [docs/JJ_INTEGRATION.md](docs/JJ_INTEGRATION.md) — track + `$jj` / jj-task
- [docs/LLM_INTEGRATION.md](docs/LLM_INTEGRATION.md) — agent skills and `track llm-help`
- [docs/TODAY_TASK.md](docs/TODAY_TASK.md) — `track switch today`
- [docs/FUNCTIONAL_SPEC.md](docs/FUNCTIONAL_SPEC.md) — command-level spec
- [docs/USAGE_EXAMPLES.md](docs/USAGE_EXAMPLES.md) — copy-paste workflows
- [docs/TESTING.md](docs/TESTING.md) — test layout
- [PROJECT_STRUCTURE.md](PROJECT_STRUCTURE.md) — crate layout

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
