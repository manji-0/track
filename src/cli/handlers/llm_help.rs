use crate::cli::handlers::CommandCtx;
use crate::utils::Result;

pub fn handle_llm_help(_ctx: &CommandCtx) -> Result<()> {
    println!(
        r#"# Track CLI Help for LLM Agents

## ⚠️ MANDATORY: Read This First

**BEFORE making ANY code changes, you MUST:**

1. Run `track sync` to create/move to the task bookmark
2. Verify you are on the correct bookmark (NOT main/master/develop)
3. ONLY THEN begin coding

**FAILURE TO FOLLOW THIS WORKFLOW WILL RESULT IN:**
- Changes on the wrong bookmark (main/master)
- Merge conflicts that are difficult to resolve
- Loss of work isolation between tasks
- Broken change history

---

## LLM Agent Quick Start (MANDATORY STEPS)

When you start working on a task, follow these steps **IN ORDER**:

### Step 1: Sync (REQUIRED - DO THIS FIRST)
```bash
track sync
```
This creates the task bookmark and moves the workspace. **Do NOT skip this step.**

### Step 2: Verify Bookmark
```bash
jj status
jj bookmark list -r @
```
Confirm the output shows `task/<ticket-id>` or `task/task-<id>`.
**If you see main/master/develop, STOP and run `track sync` again.**

### Step 3: Check Status (JSON-first for agents)
```bash
track status --json
```
Parse `workflow.phase`, `workflow.next_action`, and `todos_agent` before doing anything else.
Human-readable fallback: `track status`

**Agent JSON fields:**
| Field | Use |
|-------|-----|
| `workflow.phase` | setup · sync_required · execute · task_complete · archived |
| `workflow.next_action.command` | Next command to run |
| `todos_agent[].is_next` | Current TODO |
| `todos_agent[].allowed_actions` | Valid operations (reopen forbidden) |
| `guardrails` | must_sync_before_code_changes, complete_requires_jj_merge |

WebUI: `GET /api/status` returns the same agent fields when a task is active.

**Install track skills (split by use case):**
```bash
npx skills add ./skills \
  -s track -s track-task-setup -s track-task-execute -s track-advanced \
  -g -a cursor -a claude-code -a codex -y
```
| Skill | When |
|-------|------|
| `track` | Router — read workflow.phase |
| `track-task-setup` | Create task, repos, TODOs |
| `track-task-execute` | Sync, implement, todo done |
| `track-advanced` | Multi-repo, archive, hotfix |
See `skills/INSTALL.md` for agent paths and troubleshooting.

### Step 4: Navigate to Workspace
```bash
# Use track todo workspace to find the workspace path
cd "$(track todo workspace <index>)"
```

### Step 5: Execute Work
- Implement the required changes.
- Run tests and checks.
- Use `jj describe` to record the change summary.

### Step 6: Record Progress
```bash
track scrap add "<note>"
```

### Step 7: Complete TODO
```bash
track todo done <index>
```

### Step 8: Repeat
Continue with the next pending TODO until all are complete.

---

## Overview

`track` is a CLI tool for managing development tasks, TODOs, and JJ workspaces.
This guide explains the standard workflow for completing tasks.

## Complete Task Workflow

### Phase 1: Task Setup (typically done by human)

1. **Create Task**: `track new "<task_name>"`
   - Creates a new task and switches to it.
   - Optionally link a ticket: `track new "<name>" --ticket PROJ-123 --ticket-url <url>`
   - Use template: `track new "<name>" --template <task_ref>` to copy TODOs from existing task

2. **Add Description**: `track desc "<description>"`
   - Provides detailed context about what needs to be done.

3. **Link Ticket** (if not done during creation): `track ticket <ticket_id> <url>`
   - Associates a Jira/GitHub/GitLab ticket with the task.
   - Enables ticket-based references and automatic bookmark naming.

4. **Register Repositories**: `track repo add [path]`
   - Register working repositories (default: current directory).
   - Optionally specify base bookmark: `track repo add --base <bookmark>`
   - Run this for each repository involved in the task.

5. **Add TODOs**: `track todo add "<content>" [--worktree]`
   - Add actionable items. Use `--worktree` flag to schedule workspace creation.

6. **Add Links**: `track link add <url> [--title "<title>"]`
   - Add reference links (documentation, PRs, issues, etc.)

### Phase 2: Task Execution (LLM or Human)

7. **Sync Repositories**: `track sync` **(MANDATORY FIRST STEP)**
   - Creates task bookmarks on all registered repos.
   - Moves workspaces to the task bookmark.
   - Creates workspaces for TODOs that requested them.
   - Sync aborts if the repo has pending changes.
   - **You MUST run this before making any code changes.**

8. **Verify Bookmark**: `jj status`
   - **STOP if you are not on the task bookmark. Run `track sync` again.**

9. **Check Current State**: `track status --json` (prefer JSON; use `track status` for humans)
   - Follow `workflow.next_action` — do not guess the next step
   - WebUI `/api/status` exposes the same agent fields

10. **Execute TODOs**:
    - Run `track todo workspace <index>` to get the workspace path.
    - Implement the required changes.
    - Run tests to verify.
    - Use `track scrap add "<note>"` to record findings, decisions, or progress.

11. **Complete TODO**: `track todo done <index>`
    - Marks the TODO as done.
    - If workspace exists: rebases the TODO bookmark onto the task bookmark, moves the task bookmark, and removes the workspace.

12. **Repeat** until all TODOs are complete.

## Key Commands Reference

| Command | Description |
|---------|-------------|
| `track sync` | **MANDATORY FIRST STEP** - Sync bookmarks and create workspaces |
| `track status` | Show current task, TODOs, workspaces, links |
| `track status --json` | **Preferred for agents** — task + workflow + todos_agent + guardrails |
| `track status --all` | Show all scraps instead of recent |
| `track new "<name>"` | Create new task |
| `track new "<name>" --ticket <id> --ticket-url <url>` | Create task with ticket |
| `track new "<name>" --template <ref>` | Create task from template (copies TODOs) |
| `track list` | List all tasks |
| `track desc [text]` | View or set task description |
| `track ticket <ticket_id> <url>` | Link ticket to current task |
| `track switch <id>` | Switch to another task |
| `track switch t:<ticket_id>` | Switch by ticket reference |
| `track switch a:<alias>` | Switch by alias |
| `track archive [task_ref]` | Archive task (removes workspaces) |
| `track alias set <alias>` | Set alias for current task |
| `track alias set <alias> --force` | Overwrite existing alias on another task |
| `track alias remove` | Remove alias from current task |
| `track repo add [path]` | Register repository (default: current dir) |
| `track repo add --base <bookmark>` | Register with custom base bookmark |
| `track repo list` | List registered repositories |
| `track repo remove <index>` | Remove repository by task-scoped index |
| `track todo add "<text>"` | Add TODO |
| `track todo add "<text>" --worktree` | Add TODO with workspace |
| `track todo list` | List TODOs |
| `track todo workspace <index>` | Show or recreate TODO workspace |
| `track todo done <index>` | Complete TODO (rebases workspace if exists) |
| `track todo update <index> cancelled` | Cancel a pending TODO (use `todo done` to complete) |
| `track todo delete <index>` | Delete TODO |
| `track link add <url>` | Add reference link |
| `track link add <url> --title "<title>"` | Add link with custom title |
| `track link list` | List all links |
| `track link delete <index>` | Delete link by task-scoped index |
| `track scrap add "<note>"` | Record work note |
| `track scrap list` | List all scraps |
| `track webui` | Start web-based UI (default: http://localhost:3000) |
| `track llm-help` | Show this help message |

## Task-Scoped Indices

**Important**: TODO, Link, and Repository indices are **task-scoped**, not global.
- Each task has its own numbering starting from 1
- When you switch tasks, indices reset to that task's scope
- This prevents confusion when working on multiple tasks

Example:
```bash
# Task 1 has TODOs: 1, 2, 3
track switch 1
track todo list  # Shows: 1, 2, 3

# Task 2 has TODOs: 1, 2 (different from Task 1's TODOs)
track switch 2
track todo list  # Shows: 1, 2
```

## Ticket Integration

### Linking Tickets
Tasks can be linked to external tickets (Jira, GitHub Issues, GitLab Issues):

**During task creation:**
```bash
track new "Fix login bug" --ticket PROJ-123 --ticket-url https://jira.example.com/browse/PROJ-123
```

**After task creation:**
```bash
track ticket PROJ-123 https://jira.example.com/browse/PROJ-123
```

### Ticket References
Once a ticket is linked, you can reference tasks by ticket ID:

```bash
# Switch to task by ticket ID
track switch t:PROJ-123

# Archive task by ticket ID
track archive t:PROJ-123

# View status by ticket ID
track status t:PROJ-123
```

### Automatic Bookmark Naming
When a ticket is linked, `track sync` automatically uses the ticket ID in bookmark names:

- **With ticket**: `task/PROJ-123` (and `task/PROJ-123-todo-1` for TODO workspaces)
- **Without ticket**: `task/task-42` (and `task/task-42-todo-1` for TODO workspaces)

This makes it easy to correlate bookmarks with tickets in your issue tracker.

## Template Feature

Create new tasks based on existing ones:

```bash
# Create task from template (copies all TODOs)
track new "Sprint 2 Feature" --template t:PROJ-100

# TODOs are copied with status reset to 'pending'
# Useful for recurring workflows or similar tasks
```

## Web UI

Launch the web-based interface for visual task management:

```bash
track webui
```

Features:
- Real-time task status updates via Server-Sent Events (SSE)
- Visual TODO management with drag-and-drop
- Markdown rendering for scraps
- Inline editing of descriptions
- Link management
- Responsive design with dark mode

Access at: http://localhost:3000

## Important Notes

- **ALWAYS run `track sync` before making code changes.**
- **ALWAYS verify you are on the task bookmark, not main/master/develop.**
- TODO, Link, and Repository indices are **task-scoped**, not global.
- Use `track todo workspace <index>` to find the workspace path.
- `track sync` aborts if the repo has uncommitted changes.
- `track todo done` automatically rebases and removes associated workspaces.
- Always register repos with `track repo add` before running `track sync`.
- Use `track scrap add` to document decisions and findings during work.
- Ticket IDs are used in bookmark names when linked (e.g., `task/PROJ-123`).
- Scraps support Markdown formatting and are sanitized for WebUI rendering.

## Detailed Specifications

### Workspace Location
Workspaces are created as subdirectories inside the registered repository:
- **Path**: `<repo_root>/<bookmark_name>` (slashes are replaced with `_`)
- **Example**: `/src/my-app/task_PROJ-123-todo-1`

### TODO Completion Process
Executing `track todo done <index>` performs the following:
1. **Checks** for uncommitted changes in the TODO workspace (must be clean).
2. **Merges** the TODO bookmark into the Task Base bookmark (in the base workspace).
3. **Removes** the TODO workspace directory and DB record.
4. **Updates** TODO status to 'done'.

### Archive Process
Executing `track archive` performs the following:
1. **Checks** for uncommitted changes in all workspaces.
2. **Prompts** for confirmation if dirty workspaces are found.
3. **Removes** all workspaces associated with the task.
4. **Marks** the task as archived.
"#
    );
    Ok(())
}
