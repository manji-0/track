# Track

**A lightweight CLI tool for managing development tasks with integrated Git worktree support.**

Track helps developers organize their work by managing tasks, TODOs, and notes in a context-aware way. It seamlessly integrates with issue trackers (Jira, GitHub, GitLab) and automatically manages Git worktrees for parallel development.

Perfect for developers who want to:
- 📋 Keep track of multiple tasks and their progress
- 🌳 Work on multiple features simultaneously without branch switching
- 🎫 Link work directly to tickets in your issue tracker
- 📝 Document decisions and findings as you work
- ⚡ Stay focused with a simple, intuitive CLI


## Features

### 🎯 Context-based Task Management
Manage TODOs, notes, and repositories without specifying IDs each time. Simply switch to a task and all operations apply to it automatically.

```bash
track new "Feature X"        # Create and switch to task
track todo add "Step 1"      # Automatically added to current task
track scrap add "Note..."    # Automatically linked to current task
```

### 🎫 Ticket Integration
Seamlessly integrate with Jira, GitHub Issues, and GitLab Issues. Ticket IDs are automatically used in branch names for easy correlation.

```bash
track new "Fix bug" --ticket PROJ-123
track sync  # Creates branch: task/PROJ-123
track switch t:PROJ-123  # Reference by ticket ID
```

### 🌳 Git Worktree Management
Automatically create and manage isolated working directories for each TODO, enabling parallel development without branch switching.

```bash
track todo add "Refactor auth" --worktree
track sync  # Creates: /repo/task/PROJ-123-todo-1
# Work in isolation, then merge automatically with:
track todo done 1
```

### ⚡ Simple & Fast
- **Intuitive CLI**: Natural command structure that's easy to remember
- **Single binary**: No dependencies, just download and run
- **Fast execution**: Built with Rust for maximum performance


## Installation

```bash
# Build
cargo build --release

# Install (optional)
cargo install --path .
```

## Quick Start

```bash
# 1. Create a new task with ticket integration
track new "Implement User Authentication" \
  --description "Add JWT-based authentication system with login/logout endpoints" \
  --ticket AUTH-456 \
  --ticket-url https://jira.example.com/browse/AUTH-456

# 2. Add TODOs (use --worktree to schedule worktree creation)
track todo add "Design database schema for users table"
track todo add "Implement JWT token generation and validation" --worktree
track todo add "Create login endpoint"

# 3. Add reference links
track link add https://jwt.io/introduction "JWT Documentation"
track link add https://github.com/example/auth-spec "Auth Specification"

# 4. Record work notes as you progress
track scrap add "Decided to use bcrypt for password hashing"
track scrap add "JWT expiry set to 24 hours for security"

# 5. Mark TODOs as complete
track todo done 1

# 6. View current task status
track status
```

### Sample Output

Running `track status` displays a comprehensive overview of your current task:

```
# Task #12: Implement User Authentication

**Created:** 2026-01-01 17:20:43
**Ticket:** [AUTH-456](https://jira.example.com/browse/AUTH-456)

## Description

Add JWT-based authentication system with login/logout endpoints

## TODOs

- [x] **[1]** Design database schema for users table
- [ ] **[2]** Implement JWT token generation and validation
- [ ] **[3]** Create login endpoint

## Links

- [JWT Documentation](https://jwt.io/introduction)
- [Auth Specification](https://github.com/example/auth-spec)

## Recent Scraps

### [17:20]

JWT expiry set to 24 hours for security

### [17:21]

Decided to use bcrypt for password hashing
```

### Workflow with Git Worktrees

```bash
# Register repository and sync (creates task branches and worktrees)
track repo add /path/to/repo
track sync

# This creates:
# - Branch: task/AUTH-456 (base task branch)
# - Worktree: /path/to/repo/task/AUTH-456-todo-2 (for TODO #2)

# Navigate to worktree and work on TODO
cd /path/to/repo/task/AUTH-456-todo-2
# ... make changes, commit ...

# Complete TODO (automatically merges and cleans up worktree)
track todo done 2
```


## Command Reference

### Task Management

| Command | Description |
|---------|-------------|
| `track new <name>` | Create a new task and set it as active |
| `track list [--all]` | Display task list |
| `track switch <task_id>` | Switch tasks |
| `track status [id]` | Display detailed information about the current (or specified) task |
| `track desc [description]` | View or set task description |
| `track ticket <ticket_id> <url>` | Link a ticket to the task |
| `track archive [task_id]` | Archive a task (defaults to current task) |

**Example: Task List**

```bash
$ track list
```

```
+---+----+----------+---------------------------+--------+---------------------+
|   | ID | Ticket   | Name                      | Status | Created             |
+---+----+----------+---------------------------+--------+---------------------+
| * | 14 | -        | Add dark mode             | active | 2026-01-01 17:21:39 |
|   | 12 | AUTH-456 | Implement Authentication  | active | 2026-01-01 17:20:43 |
|   | 10 | PROJ-123 | API Implementation        | active | 2026-01-01 15:14:54 |
+---+----+----------+---------------------------+--------+---------------------+
```

The `*` marker indicates the current active task. Use `--all` to include archived tasks.


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
| `track repo add --base <branch>` | Register repository with custom base branch |
| `track repo list` | Display registered repositories |
| `track repo remove <id>` | Remove a repository registration |

### Sync

| Command | Description |
|---------|-------------|
| `track sync` | Sync repositories and setup task branches |

**Note**: The `track sync` command creates task branches in all registered repositories and sets up worktrees for TODOs that have `--worktree` flag.

## Ticket Reference

You can reference tasks by ticket ID instead of task ID:

```bash
# Switch task by ticket ID
track switch t:PROJ-123

# Archive by ticket ID
track archive t:PROJ-123
```

## Branch Naming Convention

For tasks with registered tickets, the ticket ID is automatically used in branch names:

```bash
# When ticket PROJ-123 is registered and sync is run:
track sync
# → Creates branch: task/PROJ-123 (base task branch)

# When TODO #1 has --worktree flag:
track todo add "Implement login" --worktree
track sync
# → Creates branch: PROJ-123-todo-1 (TODO work branch)
```

## Usage Examples

### Example 1: Bug Fix Workflow

```bash
# 1. Create task for bug fix
track new "Fix authentication timeout" \
  --ticket BUG-456 \
  --ticket-url https://github.com/myorg/myrepo/issues/456

# 2. Add investigation notes
track scrap add "Issue occurs after 30 minutes of inactivity"
track scrap add "Likely related to JWT expiration handling"

# 3. Add TODO with worktree for isolated work
track todo add "Fix token refresh logic" --worktree

# 4. Setup worktree
track repo add .
track sync

# 5. Work in isolation
cd task/BUG-456-todo-1
# ... make changes, test, commit ...

# 6. Complete and merge
track todo done 1  # Automatically merges and cleans up

# 7. Archive when done
track archive t:BUG-456
```

### Example 2: Feature Development with Multiple TODOs

```bash
# 1. Create feature task
track new "Add user profile page" --ticket FEAT-789

# 2. Break down into TODOs
track todo add "Design profile UI mockup"
track todo add "Implement backend API" --worktree
track todo add "Create frontend components" --worktree
track todo add "Write tests"

# 3. Add reference materials
track link add https://figma.com/design/profile "UI Design"
track link add https://api-docs.example.com "API Spec"

# 4. Work through TODOs
track todo done 1  # Complete design
track sync         # Create worktrees for #2 and #3

# Work on backend
cd task/FEAT-789-todo-2
# ... implement API ...
track scrap add "Using PostgreSQL for user data storage"
track todo done 2

# Work on frontend
cd ../task/FEAT-789-todo-3
# ... build components ...
track todo done 3

# 5. Check progress
track status
```

### Example 3: Managing Multiple Tasks

```bash
# Create multiple tasks
track new "Refactor authentication module" --ticket TECH-101
track new "Update documentation" --ticket DOC-202
track new "Performance optimization" --ticket PERF-303

# View all tasks
track list

# Switch between tasks
track switch t:TECH-101
track todo add "Extract auth logic to separate service"
track scrap add "Current code is in src/auth/legacy.rs"

track switch t:DOC-202
track todo add "Update API documentation"
track link add https://swagger.io "Swagger Editor"

# Check current task
track status
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

## For LLM Agents

Track provides **Agent Skills** following the [official Agent Skills specification](https://github.com/anthropics/skills) for LLM agents (Claude Code, Cline, Cursor, etc.).

### Quick Start for Agents

```bash
# Always start with status
track status

# Reference the main skill
skills/track-task-management/SKILL.md
```

Skills use progressive disclosure:
- **SKILL.md**: Quick start and overview (~1000 tokens)
- **references/**: Detailed guides loaded only when needed

### Main Skill: track-task-management

**Purpose**: Manages development tasks with integrated Git worktrees.

**Use when**: Creating tasks, adding TODOs, working through task lists, or managing development workflows.

**Quick commands:**
```bash
track new "<task>"              # Create task
track todo add "<item>"         # Add TODO  
track todo done <index>         # Complete TODO
track scrap add "<note>"        # Record progress
track status                    # Check state
```

### Detailed References

The main skill references detailed guides for specific workflows:

| Reference | When to Use |
|-----------|-------------|
| [creating-tasks.md](skills/track-task-management/references/creating-tasks.md) | Setting up new tasks |
| [executing-tasks.md](skills/track-task-management/references/executing-tasks.md) | Working through TODOs |
| [advanced-workflows.md](skills/track-task-management/references/advanced-workflows.md) | Multi-repo, parallel work |

### LLM Help Command

For quick CLI reference:

```bash
track llm-help
```

Outputs comprehensive guide with commands, ticket integration, and worktree details.

### Installation

**No setup required** - Skills auto-detected in workspace.

For agent-specific configuration, see [skills/INSTALL.md](skills/INSTALL.md).

Full skill documentation: [skills/README.md](skills/README.md)

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
