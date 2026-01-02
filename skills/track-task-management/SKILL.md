---
name: track-task-management
description: Manages development tasks with integrated Git worktrees. Use when creating tasks, adding TODOs, working through task lists, or when the user mentions 'track' CLI, task management, or worktrees.
license: MIT
compatibility: Requires track CLI installed
metadata:
  author: track
  version: 1.0.0
  tags: [task-management, git, worktrees, todo, productivity]
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
| `track status` | View current task and TODOs |
| `track new "<name>"` | Create new task |
| `track todo add "<text>"` | Add TODO |
| `track todo done <index>` | Complete TODO |
| `track scrap add "<note>"` | Record progress note |
| `track sync` | Create branches and worktrees |

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

### Task-Scoped TODOs
TODO indices (1, 2, 3...) are scoped to each task, not global.

### Progress Notes (Scraps)
Record findings, decisions, and progress as you work.

```bash
track scrap add "Decided to use bcrypt for password hashing"
```

## For LLM Agents

### Standard Workflow Pattern

1. **Check context**: `track status`
2. **Identify next action**: Look at pending TODOs
3. **Navigate to worktree** (if applicable)
4. **Implement changes** and test
5. **Commit changes**: `git commit -m "..."`
6. **Record progress**: `track scrap add "..."`
7. **Complete**: `track todo done <index>`
8. **Repeat** for next TODO

### Important Notes

- Always run `track status` first to understand current state
- TODO indices are task-scoped (not global)
- Commit all changes before `track todo done`
- Use scraps to document decisions and findings
- Ticket IDs in branch names when linked

## Additional Resources

- **Quick reference**: Run `track llm-help` for comprehensive guide
- **All commands**: Run `track --help`
- **Installation guide**: See [../INSTALL.md](../INSTALL.md)
