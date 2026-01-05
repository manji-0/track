# LLM Help Command Design

## Overview
The `llm-help` subcommand is designed to assist LLM Agents in understanding how to interact with the `track` CLI and the expected workflow for completing tasks.

## Goals
- Provide a clear, concise guide for LLM Agents.
- Document the standard development workflow using `track`.
- Describe key commands and their purposes.
- **ENFORCE the correct workflow to prevent common mistakes (e.g., working on base branch).**

## Command Specification
- **Command:** `track llm-help`
- **Output:** Markdown formatted text to stdout.

## Content Structure

### 0. MANDATORY Section (Top of Output)

The output starts with a prominent warning section that emphasizes:
1. **`track sync` must be run before any code changes**
2. **Branch verification is required**
3. **Consequences of not following the workflow** (commits on wrong branch, merge conflicts, etc.)

This section is placed at the very top to ensure LLM agents see it first.

### 1. LLM Agent Quick Start (MANDATORY STEPS)

**This is the primary section for LLM Agents.** It provides step-by-step instructions with explicit ordering:

1. **Step 1: Sync (REQUIRED)** - `track sync` - Creates and checks out task branch
2. **Step 2: Verify Branch** - `git branch --show-current` - Confirms correct branch
3. **Step 3: Check Status** - `track status` - Understands current state
4. **Step 4: Navigate to Worktree** - If applicable
5. **Step 5: Execute Work** - Implement changes
6. **Step 6: Record Progress** - `track scrap add`
7. **Step 7: Complete TODO** - `track todo done <index>`
8. **Step 8: Repeat** - Continue with next TODO

### 2. Introduction
Briefly explain that `track` is a CLI for managing development tasks, TODOs, and git worktrees.

### 3. Complete Task Workflow

The workflow is divided into two phases: **Task Setup** (typically done by a human) and **Task Execution** (performed by LLM or human).

#### Phase 1: Task Setup (Human)
1. **`track new <name>`** - Create a new task.
2. **`track desc "<description>"`** - Add a detailed description of what needs to be done.
3. **`track repo add [path]`** - Register working repositories for this task.
4. **`track todo add "<content>" [--worktree]`** - Add TODOs. Use `--worktree` to schedule worktree creation.

#### Phase 2: Task Execution (LLM or Human)
5. **`track sync`** - **(MANDATORY FIRST STEP)** Create task branch and worktrees on all registered repos.
6. **Verify Branch** - `git branch --show-current` - Confirm you are on task branch.
7. **Execute TODOs** - Work on the TODOs. Use `track scrap add "<note>"` to record findings.
8. **`track todo done <index>`** - Mark TODO as complete. Worktrees are automatically merged to the base branch.
9. **Repeat until all TODOs are complete.**

### 4. Key Commands

| Command | Description |
|---------|-------------|
| `track sync` | **MANDATORY FIRST STEP** - Sync branches and create worktrees |
| `track status` | Show current task context (task, TODOs, worktrees, links) |
| `track status --json` | Output task context in JSON format |
| `track status --all` | Show all scraps instead of recent |
| `track new <name>` | Create a new task |
| `track new <name> --ticket <id> --ticket-url <url>` | Create task with ticket |
| `track new <name> --template <ref>` | Create task from template (copies TODOs) |
| `track list` | List all tasks |
| `track desc [text]` | View or set task description |
| `track ticket <ticket_id> <url>` | Link ticket to current task |
| `track switch <id>` | Switch to another task |
| `track switch t:<ticket_id>` | Switch by ticket reference |
| `track switch a:<alias>` | Switch by alias |
| `track archive [task_ref]` | Archive task (removes worktrees) |
| `track alias set <alias>` | Set alias for current task |
| `track alias set <alias> --force` | Overwrite existing alias on another task |
| `track alias remove` | Remove alias from current task |
| `track repo add [path]` | Register repository (default: current directory) |
| `track repo add --base <branch>` | Register repository with custom base branch |
| `track repo list` | List registered repositories |
| `track repo remove <index>` | Remove repository by task-scoped index |
| `track todo add "<text>"` | Add a new TODO |
| `track todo add "<text>" --worktree` | Add TODO with scheduled worktree |
| `track todo list` | List all TODOs |
| `track todo done <index>` | Complete a TODO (merges worktree if exists) |
| `track todo update <index> <status>` | Update TODO status |
| `track todo delete <index>` | Delete TODO |
| `track link add <url>` | Add reference link |
| `track link add <url> --title "<title>"` | Add link with custom title |
| `track link list` | List all links |
| `track link delete <index>` | Delete link by task-scoped index |
| `track scrap add "<note>"` | Add a work note/finding |
| `track scrap list` | List all scraps |
| `track webui` | Start web-based UI (default: http://localhost:3000) |
| `track llm-help` | Show help optimized for LLM agents |

### 5. Ticket Integration

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

### 6. Important Notes

- **ALWAYS run `track sync` before making code changes.**
- **ALWAYS verify you are on the task branch, not main/master/develop.**
- **Task-Scoped Indices**: TODO, Link, and Repository indices are scoped to each task, not global.
- **Worktree Lifecycle**: When `track todo done` is called, associated worktrees are automatically merged into the task base branch and removed.
- **Scraps**: Use scraps to record intermediate findings, questions, or decisions during work. These are timestamped and preserved with the task. Scraps support Markdown formatting.
- **Repository Registration**: Always register repositories with `track repo add` before running `track sync`.
- **Ticket Integration**: Ticket IDs are used in branch names when linked (e.g., `task/PROJ-123`).
- **Template Feature**: Use `--template` when creating tasks to copy TODOs from existing tasks.
- **Alias Support**: Set aliases for tasks to make switching easier (e.g., `track alias set feature-x`).
- **Web UI**: Launch `track webui` for a visual interface with real-time updates, Markdown rendering, and inline editing.

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
