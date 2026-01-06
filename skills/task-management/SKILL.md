---
name: track-task-management
description: Manages development tasks with integrated Git worktrees, WebUI, and link management. Use when creating tasks, adding TODOs, working through task lists, managing references, or when the user mentions 'track' CLI, task management, or worktrees.
license: MIT
compatibility: Requires track CLI installed
metadata:
  author: track
  version: 1.1.0
  tags: [task-management, git, worktrees, todo, productivity, webui, markdown]
---

# Track Task Management

Lightweight CLI tool for managing development tasks with integrated Git worktree support.

## Quick Start

### Creating a Task

```bash
# Basic task
track new "Implement dark mode"

# With ticket integration
track new "Fix login bug" --ticket PROJ-123 --ticket-url https://jira.example.com/browse/PROJ-123
```

### Adding TODOs

```bash
# Simple TODO
track todo add "Create dark mode CSS variables"

# TODO with isolated worktree
track todo add "Implement theme switcher" --worktree
```

### Checking Status

```bash
track status
```

## Essential Commands

| Command | Purpose |
|---------|---------|
| `track status [id]` | View current (or specified) task and TODOs |
| `track status --all` | Show all scraps instead of recent |
| `track status --json` | Output in JSON format |
| `track new "<name>"` | Create new task |
| `track new "<name>" --template <ref>` | Create task from template |
| `track list` | List all tasks |
| `track switch <id>` | Switch to another task |
| `track switch t:<ticket>` | Switch by ticket reference |
| `track switch a:<alias>` | Switch by alias |
| `track todo add "<text>"` | Add TODO |
| `track todo add "<text>" --worktree` | Add TODO with worktree |
| `track todo done <index>` | Complete TODO |
| `track todo next <index>` | Move TODO to front (make it next) |
| `track link add <url>` | Add reference link |
| `track link add <url> --title "<title>"` | Add link with title |
| `track repo add [path]` | Register repository |
| `track repo add --base <branch>` | Register with base branch |
| `track scrap add "<note>"` | Record progress note |
| `track sync` | Create branches and worktrees |
| `track config set-calendar <id>` | Set Google Calendar ID |
| `track config show` | Show current configuration |
| `track webui` | Start web-based UI |
| `track llm-help` | Show comprehensive guide |

## Workflows

For detailed step-by-step workflows, see:

- **[Creating Tasks](references/creating-tasks.md)** - Complete guide for task creation and TODO setup
- **[Executing Tasks](references/executing-tasks.md)** - Working through TODOs to completion
- **[Advanced Workflows](references/advanced-workflows.md)** - Multi-repo tasks, parallel work

## Key Concepts

### Ticket Integration
Link tasks to external tickets (Jira, GitHub, GitLab). Ticket IDs are automatically used in branch names.

```bash
track new "Feature" --ticket PROJ-123
# Creates branch: task/PROJ-123
```

### Git Worktrees
Automatically create isolated working directories for each TODO, enabling parallel development.

```bash
track todo add "Refactor auth" --worktree
track sync  # Creates worktree at: /repo/task/PROJ-123-todo-1
cd /repo/task/PROJ-123-todo-1
# ... work, commit ...
track todo done 1  # Automatically merges and cleans up
```

### Task-Scoped Indices
TODO, Link, and Repository indices (1, 2, 3...) are scoped to each task, not global.
Each task maintains its own independent numbering for better organization.

### Link Management
Add reference links to tasks for documentation, PRs, issues, or any relevant URLs.

```bash
track link add https://docs.example.com/api --title "API Documentation"
track link list
```

### Template Feature
Create new tasks based on existing ones, copying all TODOs.

```bash
track new "Sprint 2" --template t:PROJ-100
# All TODOs copied with status reset to 'pending'
```

### Alias Support
Set memorable aliases for tasks to make switching easier.

```bash
track alias set feature-x
track switch a:feature-x
```

### Progress Notes (Scraps)
Record findings, decisions, and progress as you work. Supports Markdown formatting.

```bash
track scrap add "Decided to use bcrypt for password hashing"
track scrap add "## Performance Results\n- Query time: 50ms\n- Memory: 128MB"
```

### Web UI
Visual interface with real-time updates, Markdown rendering, and inline editing.

```bash
track webui  # Access at http://localhost:3000
```

## For LLM Agents

### Standard Workflow Pattern

1. **Sync first** (MANDATORY): `track sync`
2. **Verify branch**: `git branch --show-current` (must be task branch, not main/master)
3. **Check context**: `track status`
4. **Identify next action**: Look at pending TODOs
5. **Navigate to worktree** (if applicable)
6. **Implement changes** and test
7. **Commit changes**: `git commit -m "..."`
8. **Record progress**: `track scrap add "..."`
9. **Complete**: `track todo done <index>`
10. **Repeat** for next TODO

### Important Notes

- **ALWAYS run `track sync` before making code changes**
- **ALWAYS verify you are on task branch, not main/master/develop**
- Always run `track status` first to understand current state
- TODO, Link, and Repository indices are task-scoped (not global)
- Commit all changes before `track todo done`
- Use scraps to document decisions and findings (supports Markdown)
- Ticket IDs in branch names when linked
- Use `track webui` for visual interface with real-time updates
- Template feature available for recurring workflows

## Additional Resources

- **Quick reference**: Run `track llm-help` for comprehensive guide
- **All commands**: Run `track --help`
- **Installation guide**: See [../INSTALL.md](../INSTALL.md)
