# LLM Help Command Design

## Overview
The `llm-help` subcommand is designed to assist LLM Agents in understanding how to interact with the `track` CLI and the expected workflow for completing tasks.

## Goals
- Provide a clear, concise guide for LLM Agents.
- Document the standard development workflow using `track`.
- Describe key commands and their purposes.

## Command Specification
- **Command:** `track llm-help`
- **Output:** Markdown formatted text to stdout.

## Content Structure

### 1. Introduction
Briefly explain that `track` is a CLI for managing development tasks, TODOs, and git worktrees.

### 2. Complete Task Workflow

The workflow is divided into two phases: **Task Setup** (typically done by a human) and **Task Execution** (performed by LLM or human).

#### Phase 1: Task Setup (Human)
1. **`track new <name>`** - Create a new task.
2. **`track desc "<description>"`** - Add a detailed description of what needs to be done.
3. **`track repo add [path]`** - Register working repositories for this task.
4. **`track todo add "<content>" [--worktree]`** - Add TODOs. Use `--worktree` to schedule worktree creation.

#### Phase 2: Task Execution (LLM or Human)
5. **`track sync`** - Create task branch and worktrees on all registered repos.
6. **Execute TODOs** - Work on the TODOs. Use `track scrap add "<note>"` to record findings.
7. **`track todo done <index>`** - Mark TODO as complete. Worktrees are automatically merged to the base branch.
8. **Repeat until all TODOs are complete.**

### 3. Key Commands

| Command | Description |
|---------|-------------|
| `track status` | Show current task context (task, TODOs, worktrees) |
| `track status --json` | Output task context in JSON format |
| `track new <name>` | Create a new task |
| `track new <name> --ticket <id> --ticket-url <url>` | Create task with ticket |
| `track desc [text]` | View or set task description |
| `track ticket <ticket_id> <url>` | Link ticket to current task |
| `track switch <id>` | Switch to another task |
| `track switch t:<ticket_id>` | Switch by ticket reference |
| `track repo add [path]` | Register repository (default: current directory) |
| `track repo add --base <branch>` | Register repository with custom base branch |
| `track repo list` | List registered repositories |
| `track todo add "<text>"` | Add a new TODO |
| `track todo add "<text>" --worktree` | Add TODO with scheduled worktree |
| `track todo list` | List all TODOs |
| `track todo done <index>` | Complete a TODO (merges worktree if exists) |
| `track sync` | Sync task branch and create pending worktrees |
| `track scrap add "<note>"` | Add a work note/finding |
| `track scrap list` | List all scraps |

### 4. Ticket Integration

#### Linking Tickets
Tasks can be linked to external tickets (Jira, GitHub Issues, GitLab Issues):
- **During creation**: `track new "Fix bug" --ticket PROJ-123 --ticket-url <url>`
- **After creation**: `track ticket PROJ-123 <url>`

#### Ticket References
Once linked, tasks can be referenced by ticket ID:
- `track switch t:PROJ-123` - Switch to task by ticket ID
- `track archive t:PROJ-123` - Archive by ticket ID

#### Automatic Branch Naming
When a ticket is linked, `track sync` uses the ticket ID in branch names:
- **With ticket**: `task/PROJ-123` (and `task/PROJ-123-todo-1` for TODO worktrees)
- **Without ticket**: `task/task-42` (and `task/task-42-todo-1` for TODO worktrees)



### 5. For LLM Agents

When starting work on a task, follow this pattern:

1. **Read Context**: Run `track status` to understand the current task and pending TODOs.
2. **Check Worktrees**: If worktrees exist, navigate to the appropriate worktree path.
3. **Execute TODO**: Implement the required changes and run tests.
4. **Record Progress**: Use `track scrap add` to document findings, decisions, and progress.
5. **Complete TODO**: Run `track todo done <index>` when finished.
6. **Repeat**: Continue with the next pending TODO.

### 6. Important Notes

- **Task-Scoped TODOs**: TODO indices (1, 2, 3...) are scoped to each task, not global.
- **Worktree Lifecycle**: When `track todo done` is called, associated worktrees are automatically merged into the task base branch and removed.
- **Scraps**: Use scraps to record intermediate findings, questions, or decisions during work. These are timestamped and preserved with the task.
- **Repository Registration**: Always register repositories with `track repo add` before running `track sync`.
- **Ticket Integration**: Ticket IDs are used in branch names when linked (e.g., `task/PROJ-123`).

### 7. Implementation Details
- The command is implemented as a subcommand in `src/cli/mod.rs` and handled by `handle_llm_help()` in `src/cli/handler.rs`.
### 8. Detailed Specifications

#### Worktree Location
Worktrees are created as subdirectories within the registered repository root.
- **Path Structure**: `<repo_root>/<branch_name>`
- **Example**: If repo is `/src/app` and branch is `PROJ-123-todo-1`, worktree is at `/src/app/PROJ-123-todo-1`.

#### TODO Completion Behavior (`track todo done <id>`)
When you run `track todo done`, the following atomic steps occur:
1. **Uncommitted Changes Check**: Scans the TODO's worktree. If changes exist, operation aborts (you must commit or stash first).
2. **Retrieve Base Worktree**: Locates the main task worktree (checked out to the task branch).
3. **Merge**: Merges the TODO worktree's branch **into** the Base worktree's branch (Task Branch).
4. **Cleanup**: 
   - Deletes the TODO worktree directory.
   - Deletes the worktree record from the database.
5. **Update Status**: Marks the TODO as `done` in the database.
