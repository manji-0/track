use crate::cli::handlers::CommandCtx;
use crate::utils::Result;

pub fn handle_llm_help(_ctx: &CommandCtx) -> Result<()> {
    println!(
        r#"# Track CLI Help for LLM Agents

## ⚠️ MANDATORY: Two-Layer Stack

**Track** = WHAT to work on (tasks, TODOs, scraps).  
**[agent-skill-jj](https://github.com/manji-0/agent-skill-jj) (`$jj`)** = HOW to commit and open PRs.

**BEFORE making ANY code changes:**

1. Run `track status --json` — read `jj.slug` and `workflow.next_action`
2. Run `jj-task start <jj.slug>` (or `cd "$(jj-task path <jj.slug>)"` if already started)
3. Work **only** in the jj-task workspace (`.worktrees/<slug>/`) — never edit at repo root
4. Use **`$jj` skill** for all jj squash/commit/push/PR operations

See `docs/JJ_INTEGRATION.md` for the full strategy.

---

## LLM Agent Quick Start (MANDATORY STEPS)

### Step 1: Read track JSON context
```bash
track status --json
```

| Field | Use |
|-------|-----|
| `workflow.phase` | setup · sync_required · execute · task_complete · archived |
| `workflow.next_action.command` | Next command to run |
| `jj.slug` | jj-task workspace name |
| `jj.start_command` | e.g. `jj-task start proj-123` |
| `jj.path_command` | e.g. `cd "$(jj-task path proj-123)"` |
| `todos_agent[].is_next` | Current TODO |
| `guardrails.must_use_jj_skill` | Load `$jj` for jj/PR work |

Mutating commands accept `--json` and return the **same snapshot** plus `ok` and `mutation` (`kind`, `id`). Prefer `track todo add --json` / `track todo done <n> --json` / `track scrap add --json` so you do not need a second `status` call. `track list --json` lists tasks (not the current-task snapshot).

WebUI: `GET /api/status` — same fields.

### Step 2: Start jj-task workspace (when sync_required)
From repo root (main workspace):
```bash
jj-task repo init    # once per repo
jj-task start <jj.slug>
cd "$(jj-task path <jj.slug>)"
```

### Step 3: Implement + commit via $jj skill
- Run tests/linters in the jj-task workspace
- Follow **`$jj` skill** for prek, jj squash/commit, two-phase PR, push
- Do **not** use bare `jj describe` instead of `$jj` commit rules

### Step 4: Record progress in track
```bash
track scrap add --json "<note>"
```

### Step 5: Complete TODO in track
```bash
track todo done --json <index>
```

### Step 6: Repeat until task_complete, then archive
```bash
# $jj skill: merge PR
jj-task done <jj.slug>
track archive
```

**Install skills:**
```bash
npx skills add ./skills -s track -s track-task-execute -g -y
npx skills add manji-0/agent-skill-jj -s jj -g -y
```
See `skills/INSTALL.md` and `docs/JJ_INTEGRATION.md`.

---

## Overview

`track` is a personal work-context manager (current task, TODOs, scraps) for humans and coding agents.
JJ workspaces and commits are **`$jj` / jj-task**, not `track sync`. This guide is the agent workflow.

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

5. **Add TODOs**: `track todo add "<content>"` or `track todo add "<content>" --no-workspace`
   - Default TODOs expect a jj-task workspace. Use `--no-workspace` for research/planning items.
   - Legacy `--worktree` is deprecated — use one jj-task workspace per task.

6. **Add Links**: `track link add <url> [--title "<title>"]`
   - Add reference links (documentation, PRs, issues, etc.)

### Phase 2: Task Execution (LLM or Human)

7. **Start workspace** (when `workflow.phase` is `sync_required`):
   - JJ mode: `jj-task repo init` (once), then `jj-task start <jj.slug>`
   - Git mode: `track sync`
   - Follow `workflow.next_action` and `workflow.checklist` from JSON.

8. **Work in task workspace**: `cd "$(jj-task path <jj.slug>)"` — not repo root.

9. **Check Current State**: `track status --json` (prefer JSON; use `track status` for humans)
   - Follow `workflow.next_action` — do not guess the next step
   - WebUI `/api/status` exposes the same agent fields

10. **Execute TODOs**:
    - Implement changes; use **`$jj` skill** for all jj commit/PR operations.
    - Run tests to verify.
    - Use `track scrap add "<note>"` to record findings, decisions, or progress.

11. **Complete TODO**: `track todo done <index>`
    - Marks the TODO as done in track DB (not a substitute for `$jj` commits).

12. **Repeat** until all TODOs are complete.

## Key Commands Reference

| Command | Description |
|---------|-------------|
| `track sync [--legacy]` | Git: create worktree. JJ: legacy per-TODO only (else use jj-task) |
| `track migrate legacy-worktrees [--dry-run] [--force]` | Clear legacy flags; remove legacy worktree DB/jj workspaces |
| `track status` | Show current task, TODOs, workspaces, links |
| `track status --json` | **Preferred for agents** — task + workflow + todos_agent + guardrails |
| `track status --all` | Show all scraps instead of recent |
| `track new "<name>" [--json]` | Create new task |
| `track new "<name>" --ticket <id> --ticket-url <url>` | Create task with ticket |
| `track new "<name>" --template <ref>` | Create task from template (copies TODOs) |
| `track list` | List all tasks |
| `track list --json` | List tasks as JSON (`current_task_id` + `tasks[].is_current`) |
| `track desc [text]` | View or set task description |
| `track ticket <ticket_id> <url>` | Link ticket to current task |
| `track switch <id> [--json]` | Switch to another task |
| `track switch t:<ticket_id>` | Switch by ticket reference |
| `track switch a:<alias>` | Switch by alias |
| `track archive [task_ref] [--json]` | Archive after `jj-task done`. Blockers prompt on a TTY; non-TTY fails (do not `--force` unless skipping checks) |
| `track archive --force` | Skip jj-task/dirty checks (humans on TTY can confirm instead) |
| `track alias set <alias>` | Set alias for current task |
| `track alias set <alias> --force` | Overwrite existing alias on another task |
| `track alias remove` | Remove alias from current task |
| `track repo add [path] [--json]` | Register repository (default: current dir) |
| `track repo add --base <bookmark>` | Register with custom base bookmark |
| `track repo list` | List registered repositories |
| `track repo remove <index>` | Remove repository by task-scoped index |
| `track todo add "<text>" [--no-workspace] [--json]` | Add TODO (`--json` returns status snapshot + mutation) |
| `track todo list` | List TODOs |
| `track todo workspace <index>` | Show or recreate TODO workspace |
| `track todo done <index> [--json]` | Complete TODO |
| `track todo next <index> [--json]` | Move a TODO to the front |
| `track todo update <index> cancelled [--json]` | Cancel a pending TODO (use `todo done` to complete) |
| `track todo delete <index> --force [--json]` | Delete TODO (agents: always `--force`; non-TTY cannot confirm) |
| `track link add <url>` | Add reference link |
| `track link add <url> --title "<title>"` | Add link with custom title |
| `track link list` | List all links |
| `track link delete <index>` | Delete link by task-scoped index |
| `track scrap add "<note>" [--json]` | Record work note |
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

### jj-task slug
Track derives `jj.slug` (the jj-task workspace name) from the current task:

- **Alias** if set (`track alias set fix-oauth`)
- else **ticket id** sanitized (`PROJ-123` → `proj-123`)
- else `task-{{id}}`

Do not treat `task/PROJ-123` bookmarks from legacy `track sync` as the current workspace. Use `jj.slug` from JSON.

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

- **JJ mode**: use `jj-task start <slug>` and work in `.worktrees/<slug>/` — not repo root.
- **Git mode**: run `track sync` before coding in the task worktree.
- TODO, Link, and Repository indices are **task-scoped**, not global.
- `track archive` requires `jj-task done <slug>` when the jj-task map shows an active workspace.
- **Agents never wait on confirmation.** `todo delete` and `archive` prompt only on a TTY. Non-TTY stdin fails with `Confirmation required` — pass `--force` (delete) or finish `jj-task done` first (archive).
- Use `track archive --force` only to skip jj-task/dirty checks (interactive prompt without flag on a TTY).
- `track sync` in JJ mode is for **legacy** per-TODO `--worktree` tasks only (or `--legacy` flag).
- Run `track migrate legacy-worktrees` to move old tasks to jj-task (removes legacy worktree records).
- Use `track scrap add` to document decisions and findings during work.
- Scraps support Markdown formatting and are sanitized for WebUI rendering.

## Legacy (existing tasks only)

Per-TODO `--worktree` was removed from the CLI. Existing DB rows with `worktree_requested` still use `track sync` and `track todo workspace`.

### Archive Process
1. Verifies jj-task workspace is merged (`jj-task done`) — or prompts / `--force`.
2. Checks track-managed workspaces for uncommitted changes (TTY prompt; non-TTY error).
3. Removes track-managed workspaces and marks the task archived.
"#
    );
    Ok(())
}
