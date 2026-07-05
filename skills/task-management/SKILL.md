---
name: track-task-management
description: Manages development tasks with integrated JJ workspaces, WebUI, and link management. Use when creating tasks, adding TODOs, working through task lists, managing references, or when the user mentions 'track' CLI, task management, or workspaces.
license: MIT
compatibility: Requires track CLI installed
metadata:
  author: track
  version: 1.3.0
  tags: [task-management, jj, workspaces, todo, productivity, webui, markdown, npx-skills]
---

# Track Task Management

Lightweight CLI tool for managing development tasks with integrated JJ workspace support.

## Install this skill

**Recommended** — [Skills CLI](https://github.com/vercel-labs/skills) from the track repo root:

```bash
npx skills add ./skills/task-management -g -a cursor -a claude-code -a codex -y
```

| Agent | Flag | Skill path after install |
|-------|------|--------------------------|
| Cursor | `-a cursor` | `.agents/skills/` or `~/.cursor/skills/` |
| Claude Code | `-a claude-code` | `.claude/skills/` or `~/.claude/skills/` |
| Codex | `-a codex` | `.agents/skills/` or `~/.codex/skills/` |

Details: [../INSTALL.md](../INSTALL.md) · Discover skills: [skills.sh](https://skills.sh)

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

# TODO with isolated workspace
track todo add "Implement theme switcher" --worktree
```

### Working in a Workspace

```bash
track sync
cd "$(track todo workspace 1)"

# Verify bookmark and describe changes
jj status
jj bookmark list -r @
jj describe -m "Implement theme switcher"
```

### Checking Status (JSON-first for agents)

```bash
# Human-readable
track status

# Machine-readable — prefer this for agents
track status --json
```

Read these fields every turn:

| Field | Meaning |
|-------|---------|
| `workflow.phase` | `setup` · `sync_required` · `execute` · `task_complete` · `archived` |
| `workflow.next_action.command` | Suggested next command |
| `todos_agent[].is_next` | Current TODO to work on |
| `todos_agent[].allowed_actions` | Allowed ops (`complete`, `cancel`, … — **no reopen**) |
| `guardrails.must_sync_before_code_changes` | Must run `track sync` before editing code |

WebUI equivalent: `GET /api/status` (same agent fields when a task is active).

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
| `track todo add "<text>" --worktree` | Add TODO with workspace |
| `track todo workspace <index>` | Show or recreate TODO workspace |
| `track todo done <index>` | Complete TODO (JJ merge if workspace exists) |
| `track todo update <index> cancelled` | Cancel a pending TODO |
| `track todo next <index>` | Move TODO to front (make it next) |
| `track link add <url>` | Add reference link |
| `track link add <url> --title "<title>"` | Add link with title |
| `track repo add [path]` | Register repository |
| `track repo add --base <bookmark>` | Register with base bookmark |
| `track scrap add "<note>"` | Record progress note |
| `track sync` | Create bookmarks and workspaces |
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
Link tasks to external tickets (Jira, GitHub, GitLab). Ticket IDs are automatically used in bookmark names.

```bash
track new "Feature" --ticket PROJ-123
# Creates bookmark: task/PROJ-123
```

### JJ Workspaces
Automatically create isolated working directories for each TODO, enabling parallel development.

```bash
track todo add "Refactor auth" --worktree
track sync  # Creates workspace at: /repo/task/PROJ-123-todo-1
cd "$(track todo workspace 1)"
# ... work, jj describe ...
track todo done 1  # Automatically rebases and cleans up
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

1. **Read JSON context**: `track status --json` — follow `workflow.next_action`
2. **Sync first** (MANDATORY before code): `track sync`
3. **Verify bookmark**: `jj status` + `jj bookmark list -r @` (task bookmark, not main)
4. **Navigate to workspace**: `track todo workspace <index>` when `todos_agent[].workspace.lifecycle` is `ready`
5. **Implement** and test
6. **Describe changes**: `jj describe -m "..."`
7. **Record progress**: `track scrap add "..."`
8. **Complete**: `track todo done <index>` (never `todo update … done`)
9. **Repeat** — re-run `track status --json` for the next action

### Important Notes

- **Use `track status --json`** at the start of every turn; do not guess the next step
- **ALWAYS run `track sync` before making code changes**
- **Complete TODOs with `track todo done`** — not `todo update` (JJ merge required)
- **Reopening TODOs is forbidden** — add a new TODO instead
- **ALWAYS verify task bookmark** (`jj status` + `jj bookmark list -r @`)
- TODO, Link, and Repository indices are task-scoped (not global)
- Describe all changes before `track todo done`
- Use scraps to document decisions and findings (supports Markdown)
- Ticket IDs appear in bookmark names when linked
- Use `track webui` for visual interface with real-time updates
- Template feature available for recurring workflows

## Additional Resources

- **Install skill**: `npx skills add ./skills/task-management -g -a cursor -a claude-code -a codex -y`
- **Quick reference**: Run `track llm-help` for comprehensive guide
- **Install guide**: See [../INSTALL.md](../INSTALL.md)
