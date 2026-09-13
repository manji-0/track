# Track CLI Functional Specification

Command-level behavior for Track, a personal work-context manager (not a repo issue tracker). Product framing and the two-layer JJ stack live in [DESIGN.md](../DESIGN.md) and [JJ_INTEGRATION.md](JJ_INTEGRATION.md).

Mutating commands that accept `--json` / `-j` return the same snapshot as `track status --json` plus `ok` and `mutation`. `track list --json` is a task inventory.

---

## 1. Task Management Functions

### 1.1. `track new <name>` - Create New Task

**Overview**: Creates a new work context (task) and sets it as the active task.

**Input**:
| Argument/Flag | Type | Required | Description |
|---|---|---|---|
| `name` | String | ✓ | Task name (arbitrary length, cannot be empty) |
| `--description` / `-d` | String | | Task description (detailed context about the task) |
| `--ticket` / `-t` | String | | Ticket ID (see format below) |
| `--ticket-url` | URL | | Ticket URL |
| `--json` / `-j` | Flag | | Print the status snapshot after create |

**Ticket ID Format**:
| Platform | Format | Example |
|---|---|---|
| Jira | `<PROJECT>-<NUMBER>` | `PROJ-123` |
| GitHub Issue | `<owner>/<repo>/<number>` | `myorg/api/456` |
| GitLab Issue | `<group>/<project>/<number>` | `mygroup/app/789` |

**Process Flow**:
1. Validate that `name` is not empty.
2. If `--ticket` is specified:
   - Validate Ticket ID format (matches one of the above).
   - If `--ticket-url` is unspecified, register as empty.
3. INSERT a new record into the `tasks` table.
   - `status`: `'active'`
   - `description`: Description (if specified)
   - `ticket_id`: Ticket ID (if specified)
   - `ticket_url`: Ticket URL (if specified)
   - `created_at`: Current time (UTC)
4. Update `current_task_id` in the `app_state` table to the new task ID.
5. Output success message.

**Output**:
```
Created task #<id>: <name>
Ticket: <ticket_id> (<ticket_url>)
Switched to task #<id>
```

**Error Cases**:
| Condition | Error Message |
|---|---|
| Name is empty | `Error: Task name cannot be empty` |
| Duplicate Ticket ID | `Error: Ticket '<ticket_id>' is already linked to task #<existing_id>` |
| DB write failure | `Error: Failed to create task: <detail>` |

---

### 1.2. Task Reference by Ticket ID

All commands requiring a Task ID support reference by Ticket ID.

**Notation**:
- Numeric: Task ID (e.g., `1`, `42`)
- `t:<ticket_id>`: Reference by Ticket (e.g., `t:PROJ-123`, `t:myorg/api/456`)

**Usage Examples**:
```bash
# Switch by Task ID
track switch 1

# Switch by Ticket ID
track switch t:PROJ-123

# Switch by GitHub Issue format ticket
track switch t:myorg/api/456



# Archive by Ticket ID
track archive t:PROJ-123
```

**Resolution Flow**:
1. If argument is numeric: Use as Task ID directly.
2. If starts with `t:`: Search `tasks.ticket_id` for the corresponding task.
3. If no matching task found: Error.

---

### 1.3. `track ticket <ticket_id> <url>` - Register Ticket to Existing Task

**Overview**: Adds or updates ticket information for the current task (or specified task).

**Input**:
| Argument/Flag | Type | Required | Description |
|---|---|---|---|
| `ticket_id` | String | ✓ | Ticket ID |
| `url` | URL | ✓ | Ticket URL |
| `--task` | Integer | | Target Task ID (Default: Current task) |

**Output**:
```
Linked ticket <ticket_id> to task #<task_id>
URL: <url>
```

---

### 1.4. Workspace slug

Workspaces are named by `git.slug` / `jj.slug` from `track status --json`. Directory: `.worktrees/<slug>/`. Git branch / jj bookmark: `track/<slug>` (GitHub PR head).

| Source (first match) | Example |
|---|---|
| Task alias | `fix-oauth-refresh` |
| Sanitized ticket id | `PROJ-123` → `proj-123` |
| Fallback | `task-{id}` |

Per-TODO `--worktree` is removed; migrate with `track migrate legacy-worktrees`.

---

### 1.5. `track list` - Display Task List

**Overview**: Displays a list of registered tasks.

**Input**:
| Flag | Description |
|---|---|
| `--all` / `-a` | Show all tasks including archived ones |
| `--json` / `-j` | Task inventory JSON (`current_task_id`, `tasks[].is_current`) |
| (default) | Show only tasks with `status = 'active'` |

**Process Flow**:
1. Get `current_task_id` from `app_state`.
2. Retrieve records from `tasks` table (filtered by flag).
3. Output in table format (mark current task with `*`).

**Output Example**:
```
  ID | Ticket     | Name              | Status   | Created
-----+------------+-------------------+----------+---------------------
*  1 | PROJ-123   | API Implementation| active   | 2025-01-01 10:00:00
   2 | -          | Bug Fix           | active   | 2025-01-02 14:30:00
```

---

### 1.6. `track switch <task_id>` - Switch Task

**Overview**: Switches the active working task.

**Input**:
| Argument | Type | Required | Description |
|---|---|---|---|
| `task_id` | Integer | ✓ | Target Task ID |

**Process Flow**:
1. Validate that the task with the specified ID exists.
2. Validate that the task `status` is `'active'`.
3. Update `current_task_id` in `app_state`.

**Output**:
```
Switched to task #<id>: <name>
```

**Error Cases**:
| Condition | Error Message |
|---|---|
| Task does not exist | `Error: Task #<id> not found` |
| Archived | `Error: Task #<id> is archived` |

---

### 1.7. `track status` - Display Current Task Details

**Overview**: Displays all information related to the current task.

**Input**:
| Flag | Description |
|---|---|
| `--json` / `-j` | Output in JSON format |

**Process Flow**:
1. Get current `task_id` from `app_state`.
2. Retrieve and format the following related data:
   - Task basic info (Name, Ticket, Created At)
   - TODO list (Grouped by status)
   - Link list
   - Scrap list (Last 5 entries)
   - Related Worktree list

**Output Example (Standard)**:
```
# Task #1: API Implementation

**Created:** 2025-01-01 10:00:00
**Ticket:** [PROJ-123](https://jira.example.com/browse/PROJ-123)

## Description

Implement RESTful API with JWT authentication and user management.
This includes endpoint design, database schema, and integration tests.

## TODOs

- [ ] **[1]** Endpoint design
- [x] **[2]** Schema definition

## Links

- [Figma Design](https://figma.com/...)

## Recent Scraps

- **[10:30]** Completed DB design.
  
## Workspaces

### Workspace #1

- **Path:** `/home/user/api-workspaces/task/PROJ-123`
- **Bookmark:** `task/PROJ-123`
- **Repository Links:**
  - PR: https://github.com/.../pull/123
```

**Output Example (JSON)**:
```json
{
  "task": {
    "id": 1,
    "name": "API Implementation",
    "description": "Implement RESTful API...",
    "status": "active",
    "ticket_id": "PROJ-123",
    "ticket_url": "https://jira.example.com/browse/PROJ-123",
    "created_at": "2025-01-01T10:00:00Z"
  },
  "todos": [
    {
      "id": 1,
      "task_index": 1,
      "content": "Endpoint design",
      "status": "pending",
      "created_at": "..."
    }
  ],
  "links": [...],
  "scraps": [...],
  "workspaces": [
    {
      "id": 1,
      "path": "...",
      "repo_links": [...]
    }
  ]
}
```

Mutating commands (`new`, `switch`, `archive`, `todo add/done/update/next/delete`, `scrap add`, `repo add`) also accept `--json` / `-j`. Success output is the same snapshot as `track status --json`, plus:

```json
{
  "ok": true,
  "mutation": { "kind": "todo_add", "id": 3 }
}
```

`track list --json` is a task inventory (`current_task_id` and `tasks[].is_current`), not a status snapshot.

---

### 1.8. `track desc [description]` - View or Set Task Description

**Overview**: Views or sets the description for the current task (or specified task).

**Input**:
| Argument/Flag | Type | Required | Description |
|---|---|---|---|
| `description` | String | | Description text (if omitted, displays current description) |
| `--task` / `-t` | Integer | | Target Task ID (Default: Current task) |

**Process Flow**:

**View Mode** (no description argument):
1. Get current Task ID (or use `--task` value).
2. Retrieve task description from database.
3. Display description or message if none set.

**Set Mode** (description provided):
1. Get current Task ID (or use `--task` value).
2. Validate task exists and is active.
3. UPDATE task description in database.
4. Display confirmation message.

**Output (View Mode)**:
```
=== Task #6: feat: add task description ===

Description:
  Add support for task descriptions to provide more context about tasks.
  This includes schema changes, CLI commands, and documentation updates.
```

**Output (View Mode - No Description)**:
```
=== Task #6: feat: add task description ===

No description set. Use 'track desc <text>' to add one.
```

**Output (Set Mode)**:
```
Updated description for task #6
```

**Error Cases**:
| Condition | Error Message |
|---|---|
| No active task | `Error: No active task. Run 'track new' or 'track switch' first.` |
| Task does not exist | `Error: Task #<id> not found` |
| Task is archived | `Error: Cannot modify archived task #<id>` |

---

## 2. TODO Management Functions

### Task-Scoped TODO Indexing

**Overview**: TODOs use task-scoped sequential indices (1, 2, 3...) for user-facing operations, while maintaining global IDs internally for database integrity.

**Key Concepts**:
- Each task has its own TODO numbering starting from 1
- TODO indices are sequential and unique within a task
- Commands accept task-scoped indices, not global IDs
- All TODO operations require an active task context

**Example**:
```
Task #1 TODOs:          Task #2 TODOs:
  [1] Design schema      [1] Write tests
  [2] Implement code     [2] Update docs
  [3] Add tests          [3] Review PR
```

### 2.1. `track todo add <text>` - Add TODO

**Overview**: Adds a TODO to the current task with the next available task-scoped index.

**Input**:
| Argument | Type | Required | Description |
|---|---|---|---|
| `text` | String | ✓ | TODO content |

**Process Flow**:
1. Get current Task ID (Error if not set).
2. Calculate next task_index for this task.
3. INSERT record into `todos` table.
   - `task_index`: Next sequential number within task
   - `status`: `'pending'`
   - `created_at`: Current time

**Output**:
```
Added TODO #<index>: <text>
```

**Error Cases**:
| Condition | Error Message |
|---|---|
| No task selected | `Error: No active task. Run 'track new' or 'track switch' first.` |

---

### 2.2. `track todo list` - Display TODO List

**Overview**: Displays the TODO list for the current task with task-scoped indices.

**Output Example**:
```
  ID | Status  | Content
-----+---------+---------------------------
   1 | pending | Design schema
   2 | done    | Implement code
   3 | pending | Add tests
```

**Note**: The ID column shows task-scoped indices (1, 2, 3...), not global database IDs.

---

### 2.3. `track todo update <index> <status>` - Update TODO Status

**Overview**: Updates the status of a specific TODO using its task-scoped index.

**Input**:
| Argument | Type | Required | Description |
|---|---|---|---|
| `index` | Integer | ✓ | Task-scoped TODO index (1, 2, 3...) |
| `status` | String | ✓ | New status |

**Valid Status Values**:
- `pending`: Incomplete
- `done`: Completed
- `cancelled`: Cancelled

**Process Flow**:
1. Get current Task ID.
2. Resolve task-scoped index to internal TODO ID.
3. Update TODO status.

**Output**:
```
Updated TODO #<index> status to '<status>'
```

**Error Cases**:
| Condition | Error Message |
|---|---|
| No active task | `Error: No active task. Run 'track new' or 'track switch' first.` |
| Index out of range | `Error: TODO #<index> not found in current task` |

---

### 2.4. `track todo done <index>` - Complete TODO

**Overview**: Marks a TODO as done and handles associated workspace cleanup.

**Input**:
| Argument | Type | Required | Description |
|---|---|---|---|
| `index` | Integer | ✓ | Task-scoped TODO index |

**Process Flow**:
1. Get current Task ID.
2. Resolve task-scoped index to internal TODO ID.
3. If TODO has associated workspace:
   - Merge the workspace bookmark to the base bookmark.
   - Remove workspace.
4. Update TODO status to 'done'.

**Output**:
```
Merged and removed workspace for TODO #<index> (bookmark: <bookmark>).
Marked TODO #<index> as done.
```

---

### 2.5. `track todo workspace <index>` - Manage TODO Workspaces

**Overview**: Shows or recreates the workspace for a TODO in the current repo, with an option to operate across all repos.

**Input**:
| Argument/Flag | Type | Required | Description |
|---|---|---|---|
| `index` | Integer | ✓ | Task-scoped TODO index |
| `--recreate` | Flag | | Recreate workspace from latest task bookmark |
| `--force` | Flag | | Allow recreation with uncommitted changes |
| `--all` | Flag | | Operate across all registered repos |

**Process Flow**:
1. Resolve task-scoped index to internal TODO ID.
2. If `--all` is not set, resolve the current repo from the working directory.
3. If a workspace exists, print its path (or all paths with `--all`).
4. If no workspace exists, create it from the TODO bookmark.
5. If `--recreate` is set:
   - Abort if uncommitted changes are present unless `--force` is set.
   - Recreate the workspace from the latest bookmark.

**Output**:
```
/path/to/repo/task/PROJ-123-todo-1
```

**Error Cases**:
| Condition | Error Message |
|---|---|
| Current directory not a registered repo | `Error: Current directory is not a registered repo for this task.` |
| No repositories registered | `Error: No repositories registered for this task.` |
| No worktree paths available | `Error: No worktree paths available for this TODO.` |
| Uncommitted changes without `--force` | `Error: Worktree <path> has uncommitted changes. Use --force to recreate.` |

---

### 2.6. `track todo delete <index>` - Delete TODO

**Overview**: Deletes a specific TODO using its task-scoped index.

**Input**:
| Argument/Flag | Type | Required | Description |
|---|---|---|---|
| `index` | Integer | ✓ | Task-scoped TODO index to delete |
| `--force` / `-f` | Flag | | Skip confirmation prompt |

**Process Flow**:
1. Get current Task ID.
2. Resolve task-scoped index to internal TODO ID.
3. If `--force` is not specified:
   - TTY: display a confirmation prompt.
   - Non-TTY: error (do not wait for stdin).
4. Execute deletion only if user enters `y` or `yes`, or if `--force` is set.

**Confirmation Prompt**:
```
Delete TODO #<index>: "<content>"? [y/N]: 
```

**Output**:
```
Deleted TODO #<index>
```

**On Cancel**:
```
Cancelled.
```

**Error Cases**:
| Condition | Error Message |
|---|---|
| No active task | `Error: No active task. Run 'track new' or 'track switch' first.` |
| Index out of range | `Error: TODO #<index> not found in current task` |
| Confirmation needed and stdin is not a TTY | `Error: Confirmation required (stdin is not a TTY). re-run with \`track todo delete <index> --force\`` |

---

## 3. Link Management Functions

### 3.1. `track link add <url> [title]` - Add Link

**Overview**: Adds a reference URL to the current task.

**Input**:
| Argument | Type | Required | Description |
|---|---|---|---|
| `url` | String | ✓ | URL (starts with http/https) |
| `title` | String | | Link title (defaults to URL if omitted) |

**Process Flow**:
1. Validate URL format (starts with http:// or https://).
2. INSERT record into `links` table.

**Output**:
```
Added link #<id>: <title>
```

---

### 3.2. `track link list` - Display Link List

**Output Example**:
```
  ID | Title                | URL
-----+----------------------+--------------------------------
   1 | Figma Design         | https://figma.com/file/...
   2 | API Spec             | https://docs.example.com/...
```

---

## 4. Scrap (Work Note) Management Functions

### 4.1. `track scrap add <content>` - Add Scrap

**Overview**: Adds a work note (Scrap). Records temporary thoughts or notes in chronological order.

**Input**:
| Argument | Type | Required | Description |
|---|---|---|---|
| `content` | String | ✓ | Note content |

**Output**:
```
Added scrap at <timestamp>
```

---

### 4.2. `track scrap list` - Display Scrap List

**Overview**: Displays Scraps in chronological order.

**Output Example**:
```
[2025-01-01 10:30:00]
  DB design completed. see DESIGN.md for table structure.

[2025-01-01 14:15:00]
  Started API implementation. Starting with authentication.
```

---

## 5. Workspace Integration Functions

Both VCS modes create a **track-owned** coding workspace at `.worktrees/<slug>/` on `track/<slug>` (`track repo add` / `track sync`). Switch backends with `track config set vcs-mode git|jj`. See [JJ_INTEGRATION.md](JJ_INTEGRATION.md).

The old `track workspace add/list/link/remove` commands are gone. Register repos with `track repo`; complete TODOs with `track todo done`.

### 5.1. Task Lifecycle Integration

Automatically manages relevant workspaces according to task state changes.

#### `track archive [task_id]` - On Task Archive

**Process Flow**:
1. Determine target task:
   - If `task_id` provided: Use that task.
   - If omitted: Use current active task (Error if no active task).
2. Check for uncommitted changes in all related workspaces.
   - TTY: display warning and ask for confirmation.
   - Non-TTY: error with a hint (`--force`). Do not wait for stdin.
3. For all related workspaces:
   - Remove the git worktree or jj workspace directory.
   - Keep branch/bookmark `track/<slug>` for GitHub PRs.
   - Delete leftover non-VCS directories best-effort.
4. Update task `status` to `'archived'`.
5. If `app_state`'s `current_task_id` matches the task, clear it.

**Output**:
```
Archived task #<task_id>: <name>
  └─ Removed workspace #1: /path/to/workspace
  └─ Removed workspace #2: /path/to/workspace2
```

**Warning (If uncommitted changes exist)**:
```
WARNING: Workspace #<id> has uncommitted changes:
  M  src/main.rs
  ?? new_file.txt

Archive and remove workspaces anyway? [y/N]: 
```

---

## 6. Common Specifications

### 6.1. Database Path

```
$HOME/.local/share/track/track.db
```

Complies with XDG Base Directory specification. Uses `directories` crate.

### 6.2. Timestamp Format

- Storage: ISO 8601 (UTC)
- Display: Local Time `YYYY-MM-DD HH:MM:SS`

### 6.3. Exit Codes

| Code | Meaning |
|---|---|
| `0` | Success |
| `1` | General Error |
| `2` | Argument Error |

### 6.4. Common Error Handling

```rust
fn insert_task(db: &Database) -> Result<(), TrackError> {
    db.execute(...)?;
    Ok(())
}
```

---



## 8. Repository Management and Worktree Sync

### 8.1. `track repo add [path]` - Register Repository

**Overview**: Registers a git or jj repository to the current task and creates the task workspace.

**Input**:
| Argument/Flag | Type | Required | Description |
|---|---|---|---| 
| `path` | Path | | Repository path (Default: current directory `.`) |
| `--base` / `-b` | String | | Base branch/bookmark (Default: current) |

**Process Flow**:
1. Validate that a task is currently active.
2. Resolve path to absolute path.
3. Validate that path is a git and/or jj repository (depends on `vcs-mode`).
4. Check if repository is already registered for this task.
5. Determine base revision:
   - If `--base` is specified, use that name.
   - Otherwise, use the current branch/bookmark.
6. INSERT record into `task_repos` and create `.worktrees/<slug>` on `track/<slug>`.

**Output**:
```
Registered repository: /absolute/path/to/repo
```

**Error Cases**:
| Condition | Error Message |
|---|---|
| No active task | `Error: No active task. Run 'track new' or 'track switch' first.` |
| Not a VCS repository | `Error: <path> is not a git/jj repository` |
| Already registered | `Error: Repository already registered for this task` |

---

### 8.2. `track repo list` - List Repositories

**Overview**: Lists all repositories registered to the current task.

**Output Example**:
```
  ID | Repository Path
-----+----------------------------------
   1 | /home/user/projects/api
   2 | /home/user/projects/frontend
```

---

### 8.3. `track repo remove <id>` - Remove Repository

**Overview**: Removes a repository registration from the current task.

**Input**:
| Argument | Type | Required | Description |
|---|---|---|---| 
| `id` | Integer | ✓ | Repository ID |

**Output**:
```
Removed repository #<id>
```

---

### 8.4. `track sync` - Sync Repositories

**Overview**: Creates or refreshes the task workspace for every registered repo. Both git and jj modes use `.worktrees/<slug>` on `track/<slug>`. `--legacy` rebuilds old per-TODO jj worktrees.

**Git / jj (current)**:
1. Get current task and registered repos.
2. Create `.worktrees/<slug>` on `track/<slug>` as needed (jj: colocate + workspace add).
3. When aggressive mode is on, ensure a per-task empty revision exists.

**Error Cases**:
| Condition | Error Message |
|---|---|
| No active task | `Error: No active task` |
| No repositories registered | `Error: No repositories registered for this task` |

---

### 8.5. `track todo add <text> [--no-workspace]` - Add TODO

**Overview**: Adds a TODO to the current task. Default TODOs expect a coding workspace. `--no-workspace` marks research/planning items. `--worktree` is removed and returns an error.

**Input**:
| Argument/Flag | Type | Required | Description |
|---|---|---|---|
| `text` | String | ✓ | TODO content |
| `--no-workspace` | Flag | | Do not require a coding workspace |
| `--json` / `-j` | Flag | | Status snapshot after insert |

**Process Flow**:
1. Reject `--worktree` (`WorktreeFlagRemoved`).
2. INSERT TODO with `requires_workspace = false` when `--no-workspace`.
3. Optionally print JSON snapshot.

**Output Example**:
```
Added TODO #15: Implement login endpoint
```

---

### 8.6. `track todo done <id>` - Complete TODO

**Overview**: Marks a TODO done in the track DB. This is **not** a git/jj commit — commit from the task workspace, then keep working there. Legacy per-TODO workspaces (`worktree_requested`) may still rebase and remove a worktree.

**Input**:
| Argument | Type | Required | Description |
|---|---|---|---|
| `id` | Integer | ✓ | Task-scoped TODO index |
| `--json` / `-j` | Flag | | Status snapshot after complete |

**Process Flow**:
1. Validate TODO exists and is pending.
2. If a legacy track-managed worktree exists for that TODO, rebase/cleanup as before.
3. Set TODO `status` to `done`.
4. Print JSON snapshot when requested (`workflow.next_action` for the next TODO).

**Output Example**:
```
Completing TODO #15: Implement login endpoint

Workspace: /home/user/projects/api-workspaces/PROJ-123/todo-15
  ✓ No uncommitted changes
  ✓ Merged PROJ-123/todo-15 into task/PROJ-123
  ✓ Removed workspace

Workspace: /home/user/projects/frontend-workspaces/PROJ-123/todo-15
  ✓ No uncommitted changes
  ✓ Merged PROJ-123/todo-15 into task/PROJ-123
  ✓ Removed workspace

TODO #15 marked as done.
```

**Warning Example** (uncommitted changes):
```
WARNING: Workspace has uncommitted changes:
  M  src/auth.rs
  ?? new_file.txt

Continue anyway? [y/N]: 
```

**Error Cases**:
| Condition | Error Message |
|---|---|
| TODO not found | `Error: TODO #<id> not found` |
| Merge conflict | `Error: Merge conflict in <repo>. Please resolve manually.` |
| Workspace removal fails | `Error: Failed to remove workspace: <detail>` |

---

### 8.7. `track todo workspace <id>` - Manage TODO Workspaces

**Overview**: Shows or recreates the workspace for a TODO. By default it operates on the current repo; `--all` targets every registered repo.

**Input**:
| Argument/Flag | Type | Required | Description |
|---|---|---|---|
| `id` | Integer | ✓ | TODO ID |
| `--recreate` | Flag | | Recreate workspaces from latest task bookmark |
| `--force` | Flag | | Allow recreation with uncommitted changes |
| `--all` | Flag | | Operate across all registered repos |

**Process Flow**:
1. Resolve task-scoped index to internal TODO ID.
2. If `--all` is not set, resolve the current repo from the working directory.
3. If a workspace exists, print its path (or all paths with `--all`).
4. If no workspace exists, create it from the TODO bookmark.
5. If `--recreate` is set:
   - Abort if uncommitted changes are present unless `--force` is set.
   - Recreate the workspace from the latest bookmark.

**Output Example**:
```
/path/to/repo/task/PROJ-123-todo-1
```

**Error Cases**:
| Condition | Error Message |
|---|---|
| Current directory not a registered repo | `Error: Current directory is not a registered repo for this task.` |
| No repositories registered | `Error: No repositories registered for this task.` |
| No worktree paths available | `Error: No worktree paths available for this TODO.` |
| Uncommitted changes without `--force` | `Error: Worktree <path> has uncommitted changes. Use --force to recreate.` |

---

### 8.8. Database Schema: `task_repos`

```sql
CREATE TABLE task_repos (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id INTEGER NOT NULL,
    repo_path TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
    UNIQUE(task_id, repo_path)
);

CREATE INDEX idx_task_repos_task_id ON task_repos(task_id);
```

---

### 8.9. Database Schema: `tasks`

The `tasks` table schema with the description field:

```sql
CREATE TABLE tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL DEFAULT 'active',
    ticket_id TEXT UNIQUE,
    ticket_url TEXT,
    created_at TEXT NOT NULL
);
```

**Migration**: For existing databases, the `description` column is added via:
```sql
ALTER TABLE tasks ADD COLUMN description TEXT;
```

---

## 9. Future Support (Not Implemented)

The following are not currently implemented but are under consideration for the future:

- `track search <query>`: Full text search
- `track import`: Import external data
- `track server`: MCP Server integration for direct manipulation by LLM agents
