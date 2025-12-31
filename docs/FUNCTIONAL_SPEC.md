# WorkTracker CLI Functional Specification

This document defines the specific functional specifications for the WorkTracker CLI tool.

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

# Export by Ticket ID
track export t:PROJ-123

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

### 1.4. Branch Naming Convention

For tasks with registered Ticket IDs, the worktree branch name automatically uses the Ticket ID.

**Naming Patterns**:
| Condition | Branch Name |
|---|---|
| Ticket exists + Branch name omitted | `task/<ticket_id>` (e.g., `task/PROJ-123`) |
| Ticket exists + Branch name specified | `<ticket_id>/<branch>` (e.g., `PROJ-123/feat-auth`) |
| No Ticket + Branch name omitted | `task-<task_id>-<timestamp>` |
| No Ticket + Branch name specified | Use specified branch name as is |

**Behavior in `worktree add`**:
```bash
# When ticket PROJ-123 is registered
track worktree add /path/to/repo
# -> Branch: task/PROJ-123

track worktree add /path/to/repo feat-auth
# -> Branch: PROJ-123/feat-auth
```

---

### 1.5. `track list` - Display Task List

**Overview**: Displays a list of registered tasks.

**Input**:
| Flag | Description |
|---|---|
| `--all` / `-a` | Show all tasks including archived ones |
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

### 1.7. `track info` - Display Current Task Details

**Overview**: Displays all information related to the current task.

**Process Flow**:
1. Get current `task_id` from `app_state`.
2. Retrieve and format the following related data:
   - Task basic info (Name, Ticket, Created At)
   - TODO list (Grouped by status)
   - Link list
   - Scrap list (Last 5 entries)
   - Related Worktree list

**Output Example**:
```
=== Task #1: API Implementation ===
Ticket: PROJ-123 (https://jira.example.com/browse/PROJ-123)
Created: 2025-01-01 10:00:00

Description:
  Implement RESTful API with JWT authentication and user management.
  This includes endpoint design, database schema, and integration tests.

[ TODOs ]
  [ ] Endpoint design
  [x] Schema definition

[ Links ]
  - Figma Design: https://figma.com/...

[ Recent Scraps ]
  [10:30] Completed DB design.
  
[ Worktrees ]
  #1 /home/user/api-worktrees/task/PROJ-123 (task/PROJ-123)
      └─ PR: https://github.com/.../pull/123
```

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

**Overview**: Marks a TODO as done and handles associated worktree cleanup.

**Input**:
| Argument | Type | Required | Description |
|---|---|---|---|
| `index` | Integer | ✓ | Task-scoped TODO index |

**Process Flow**:
1. Get current Task ID.
2. Resolve task-scoped index to internal TODO ID.
3. If TODO has associated worktree:
   - Merge worktree branch to base branch.
   - Remove worktree.
4. Update TODO status to 'done'.

**Output**:
```
Merged and removed worktree for TODO #<index> (branch: <branch>).
Marked TODO #<index> as done.
```

---

### 2.5. `track todo delete <index>` - Delete TODO

**Overview**: Deletes a specific TODO using its task-scoped index.

**Input**:
| Argument/Flag | Type | Required | Description |
|---|---|---|---|
| `index` | Integer | ✓ | Task-scoped TODO index to delete |
| `--force` / `-f` | Flag | | Skip confirmation prompt |

**Process Flow**:
1. Get current Task ID.
2. Resolve task-scoped index to internal TODO ID.
3. If `--force` is not specified, display a confirmation prompt.
4. Execute deletion only if user enters `y` or `yes`.

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

## 5. Worktree Integration Functions

Leverages Git worktree to manage independent working directories for each task.
Automates creation and deletion of worktrees linked to the task lifecycle.

### 5.1. `track worktree add <repo_path> [branch]` - Create & Register Worktree

**Overview**: Creates a new worktree in the specified repository and links it to the current task.

**Input**:
| Argument | Type | Required | Description |
|---|---|---|---|
| `repo_path` | Path | ✓ | Path to base Git repository |
| `branch` | String | | Branch name to create (Default: `task-<task_id>-<timestamp>`) |

**Process Flow**:
1. Validate that `repo_path` is a Git repository.
2. Determine branch name (specified or auto-generated).
3. Determine directory path for worktree.
   - Default: `<repo_path>/../<repo_name>-worktrees/<branch>`
4. Create git worktree.
   - Command: `git -C <repo_path> worktree add -b <branch> <worktree_path>`
5. INSERT record into `git_items` table.
   - `path`: Worktree path
   - `branch`: Branch name
   - `base_repo`: Base repository path
   - `status`: `'active'`

**Output**:
```
Created worktree: <worktree_path>
Branch: <branch>
Linked to task #<task_id>
```

**Error Cases**:
| Condition | Error Message |
|---|---|
| Not a repository | `Error: <repo_path> is not a Git repository` |
| Branch exists | `Error: Branch '<branch>' already exists` |
| Worktree creation failed | `Error: Failed to create worktree: <detail>` |

---

### 5.2. `track worktree list` - Display Worktree List

**Overview**: Displays a list of worktrees linked to the current task.

**Output Example**:
```
  ID | Path                              | Branch      | Status | Links
-----+-----------------------------------+-------------+--------+------------------
   1 | /home/user/api-worktrees/feat-x  | feat-x      | active | PR: #123
   2 | /home/user/web-worktrees/task-1  | task-1      | active | Issue: #45
```

---

### 5.3. `track worktree link <worktree_id> <url>` - Link URL to Worktree

**Overview**: Links an Issue/PR URL to a registered worktree.

**Input**:
| Argument | Type | Required | Description |
|---|---|---|---|
| `worktree_id` | Integer | ✓ | Worktree ID |
| `url` | String | ✓ | URL to link |

**Automatic URL Type Detection**:
| Pattern | Detection Result |
|---|---|
| `/pull/` or `/merge_requests/` | `PR` |
| `/issues/` | `Issue` |
| `/discussions/` | `Discussion` |
| Other | `Link` |

**Output**:
```
Added <kind> link to worktree #<worktree_id>: <url>
```

---

### 5.4. `track worktree remove <worktree_id>` - Remove Worktree

**Overview**: Unregisters the worktree and deletes the worktree from disk.

**Input**:
| Argument/Flag | Type | Required | Description |
|---|---|---|---|
| `worktree_id` | Integer | ✓ | Worktree ID to remove |
| `--force` / `-f` | Flag | | Skip confirmation prompt |
| `--keep-files` | Flag | | Keep files on disk (unregister only) |

**Process Flow**:
1. Validate that the worktree with the specified ID exists.
2. If `--force` is not specified, display confirmation prompt.
3. If user approves:
   - If `--keep-files` is absent: Execute `git worktree remove <path>`.
   - Delete record from DB (Cascade delete related `repo_links`).

**Confirmation Prompt**:
```
Remove worktree #<id>: "<path>" (branch: <branch>)?
This will delete the worktree directory. [y/N]: 
```

**Output**:
```
Removed worktree #<worktree_id>: <path>
```

---

### 5.5. Task Lifecycle Integration

Automatically manages relevant worktrees according to task state changes.

#### `track archive <task_id>` - On Task Archive

**Process Flow**:
1. Check for uncommitted changes in all unrelated worktrees.
   - If changes exist, display warning and ask for confirmation.
2. For all related worktrees:
   - Execute `git worktree remove <path>`.
   - Delete record from DB.
3. Update task `status` to `'archived'`.
4. If `app_state`'s `current_task_id` matches the task, clear it.

**Output**:
```
Archived task #<task_id>: <name>
  └─ Removed worktree #1: /path/to/worktree
  └─ Removed worktree #2: /path/to/worktree2
```

**Warning (If uncommitted changes exist)**:
```
WARNING: Worktree #<id> has uncommitted changes:
  M  src/main.rs
  ?? new_file.txt

Archive and remove worktrees anyway? [y/N]: 
```

---

#### `track cleanup [--dry-run]` - Clean Legacy/Orphaned Worktrees

**Overview**: Deletes `archived` worktrees from disk (if they exist) and cleans up DB records. Primarily used for migrating old tasks or cleaning up failed removals.

**Input**:
| Flag | Description |
|---|---|
| `--dry-run` | Only display what would be deleted (no actual deletion) |
| `--force` / `-f` | Skip confirmation prompt |

**Process Flow**:
1. Collect worktrees with `status = 'archived'` from all tasks.
2. For each worktree:
   - Execute `git worktree remove <path>` (if path exists).
   - Delete record from DB.

**Output (dry-run)**:
```
Would remove:
  Task #1 (API Implementation):
    └─ /home/user/api-worktrees/feat-auth
  Task #3 (Bug Fix):
    └─ /home/user/web-worktrees/fix-123

Total: 2 worktrees
```

**Output (execution)**:
```
Removed 2 archived worktrees.
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
// Use anyhow::Result to attach context
db.execute(...)
    .context("Failed to insert task")?;
```

---

## 7. Export Functionality (LLM Integration)

Exports full task information in a structured format.
Intended for use by LLM agents to generate task reports or for task handovers.

### 7.1. `track export [task_id]` - Export Task Info

**Overview**: Outputs full information for the specified task (Default: current task) in a structured format.

**Input**:
| Argument/Flag | Type | Required | Description |
|---|---|---|---|
| `task_id` | Integer | | Task ID to export (Default: current task) |
| `--format` / `-f` | String | | Output format: `markdown` (default), `json`, `yaml` |
| `--output` / `-o` | Path | | Output to file (Default: stdout) |
| `--template` / `-t` | Path | | Custom template file |

---

### 7.2. Output Formats

#### Markdown Format (Default)

Structured Markdown easy for LLMs to process directly.

```markdown
# Task Report: API Implementation

## Metadata
- **Task ID**: 1
- **Status**: active
- **Created**: 2025-01-01 10:00:00
- **Last Activity**: 2025-01-05 15:30:00

## Summary
<!-- Placeholder for LLM generated summary -->

## TODOs

### Pending
- [ ] Endpoint design
- [ ] Implement authentication logic

### Completed
- [x] Schema definition
- [x] DB connection setup

## Scraps (Work Log)

### 2025-01-01 10:30
DB design completed. see DESIGN.md for table structure.

### 2025-01-02 14:15
Started API implementation. Starting with authentication.

## Links
- [Figma Design](https://figma.com/file/...)
- [API Spec](https://docs.example.com/...)

## Worktrees

### #1: /home/user/api-worktrees/feat-auth
- **Branch**: feat-auth
- **Status**: active
- **Related**:
  - PR: https://github.com/org/repo/pull/123
  - Issue: https://github.com/org/repo/issues/45
```

---

#### JSON Format

Structured data suitable for programmatic or LLM API processing.

```json
{
  "task": {
    "id": 1,
    "name": "API Implementation",
    "status": "active",
    "created_at": "2025-01-01T10:00:00Z",
    "last_activity": "2025-01-05T15:30:00Z"
  },
  "todos": [
    {"id": 1, "content": "Endpoint design", "status": "pending"},
    {"id": 2, "content": "Schema definition", "status": "done"}
  ],
  "scraps": [
    {"timestamp": "2025-01-01T10:30:00Z", "content": "DB design completed..."}
  ],
  "links": [
    {"id": 1, "title": "Figma Design", "url": "https://..."}
  ],
  "worktrees": [
    {
      "id": 1,
      "path": "/home/user/api-worktrees/feat-auth",
      "branch": "feat-auth",
      "status": "active",
      "repo_links": [
        {"kind": "PR", "url": "https://github.com/.../pull/123"}
      ]
    }
  ]
}
```

---

### 7.3. LLM Prompt Template

With the `--template` option, you can use a custom template that includes instructions for the LLM.

**Template Example** (`report_template.md`):
```markdown
You are a Project Manager. Based on the following task information,
please create a progress report in English.

---

{{task_export}}

---

## Output Requirements
1. Summary of completed work (bullet points)
2. Remaining work and estimated time
3. Blockers or concerns
4. Next action items
```

**Process Flow**:
1. Load template file.
2. Replace `{{task_export}}` with actual task information.
3. Output result.

**Output**:
```
You are a Project Manager. Based on the following task information,
please create a progress report in English.

---

# Task Report: API Implementation
... (Actual task info) ...

---

## Output Requirements
...
```

---

### 7.4. Pipeline Integration Examples

```bash
# Request LLM to generate task report
track export | llm "Summarize the progress of this task"

# Pass to script in JSON format
track export --format json | jq '.todos[] | select(.status == "pending")'

# Save to file
track export --output ~/reports/task-1-report.md

# Generate report with custom template
track export --template ~/.config/track/report_template.md | llm
```

---

## 8. Repository Management and Worktree Sync

### 8.1. `track repo add [path]` - Register Repository

**Overview**: Registers a Git repository to the current task.

**Input**:
| Argument | Type | Required | Description |
|---|---|---|---| 
| `path` | Path | | Repository path (Default: current directory `.`) |

**Process Flow**:
1. Validate that a task is currently active.
2. Resolve path to absolute path.
3. Validate that path is a Git repository (check for `.git` directory).
4. Check if repository is already registered for this task.
5. INSERT record into `task_repos` table.

**Output**:
```
Registered repository: /absolute/path/to/repo
```

**Error Cases**:
| Condition | Error Message |
|---|---|
| No active task | `Error: No active task. Run 'track new' or 'track switch' first.` |
| Not a Git repository | `Error: <path> is not a Git repository` |
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

### 8.4. `track worktree sync` - Sync Repositories

**Overview**: Synchronizes all registered repositories with the task branch.

**Process Flow**:
1. Get current task.
2. Determine task branch name:
   - If `ticket_id` exists: `task/<ticket_id>` (e.g., `task/PROJ-123`)
   - Otherwise: `task/task-<task_id>` (e.g., `task/task-5`)
3. For each registered repository:
   - Check if repository path exists.
   - Check if task branch exists.
   - If branch doesn't exist:
     - Get current branch as base.
     - Create task branch from current HEAD.
   - Checkout task branch.
   - Display sync status.

**Output Example**:
```
Syncing task branch: task/PROJ-123

Repository: /home/user/projects/api
  ✓ Branch task/PROJ-123 created from main
  ✓ Checked out task/PROJ-123

Repository: /home/user/projects/frontend
  ✓ Branch task/PROJ-123 already exists
  ✓ Checked out task/PROJ-123

Sync complete.
```

**Error Cases**:
| Condition | Error Message |
|---|---|
| No active task | `Error: No active task` |
| No repositories registered | `Error: No repositories registered for this task` |
| Repository path doesn't exist | `Warning: Repository <path> not found, skipping` |
| Dirty working directory | `Error: Repository <path> has uncommitted changes` |

---

### 8.5. `track todo add <text> --worktree` - Add TODO with Worktree

**Overview**: Adds a TODO and automatically creates worktrees in all registered repositories.

**Input**:
| Argument/Flag | Type | Required | Description |
|---|---|---|---| 
| `text` | String | ✓ | TODO content |
| `--worktree` / `-w` | Flag | | Create worktrees for this TODO |

**Process Flow** (when `--worktree` is specified):
1. Create TODO record.
2. Get all registered repositories for the current task.
3. For each repository:
   - Determine worktree branch name:
     - If `ticket_id` exists: `<ticket_id>/todo-<todo_id>` (e.g., `PROJ-123/todo-15`)
     - Otherwise: `task-<task_id>/todo-<todo_id>` (e.g., `task-5/todo-15`)
   - Determine worktree path: `<repo_path>/../<repo_name>-worktrees/<branch_name>`
   - Create worktree: `git worktree add -b <branch> <path>`
   - INSERT `git_items` record with `todo_id` set.

**Output Example**:
```
Added TODO #15: Implement login endpoint

Created worktrees:
  /home/user/projects/api-worktrees/PROJ-123/todo-15 (PROJ-123/todo-15)
  /home/user/projects/frontend-worktrees/PROJ-123/todo-15 (PROJ-123/todo-15)
```

**Error Cases**:
| Condition | Error Message |
|---|---|
| No repositories registered | `Warning: No repositories registered, worktree creation skipped` |
| Branch already exists | `Error: Branch <branch> already exists in <repo>` |
| Worktree creation fails | `Error: Failed to create worktree: <detail>` |

---

### 8.6. `track todo done <id>` - Complete TODO with Worktree Cleanup

**Overview**: Completes a TODO and automatically merges and removes associated worktrees.

**Input**:
| Argument | Type | Required | Description |
|---|---|---|---| 
| `id` | Integer | ✓ | TODO ID |

**Process Flow**:
1. Validate TODO exists and belongs to current task.
2. Find all `git_items` where `todo_id = <id>`.
3. For each worktree:
   - Check for uncommitted changes: `git -C <path> status --porcelain`
   - If changes exist, display warning and prompt for confirmation.
   - Get task branch name (from task ticket or ID).
   - Checkout task branch in base repository.
   - Merge worktree branch: `git merge --no-ff <worktree_branch>`
   - If merge succeeds:
     - Remove worktree: `git worktree remove <path>`
     - Delete `git_items` record.
   - If merge fails:
     - Display error and abort.
4. Update TODO status to `'done'`.

**Output Example**:
```
Completing TODO #15: Implement login endpoint

Worktree: /home/user/projects/api-worktrees/PROJ-123/todo-15
  ✓ No uncommitted changes
  ✓ Merged PROJ-123/todo-15 into task/PROJ-123
  ✓ Removed worktree

Worktree: /home/user/projects/frontend-worktrees/PROJ-123/todo-15
  ✓ No uncommitted changes
  ✓ Merged PROJ-123/todo-15 into task/PROJ-123
  ✓ Removed worktree

TODO #15 marked as done.
```

**Warning Example** (uncommitted changes):
```
WARNING: Worktree has uncommitted changes:
  M  src/auth.rs
  ?? new_file.txt

Continue anyway? [y/N]: 
```

**Error Cases**:
| Condition | Error Message |
|---|---|
| TODO not found | `Error: TODO #<id> not found` |
| Merge conflict | `Error: Merge conflict in <repo>. Please resolve manually.` |
| Worktree removal fails | `Error: Failed to remove worktree: <detail>` |

---

### 8.7. Database Schema: `task_repos`

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

### 8.8. Database Schema: `tasks`

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
