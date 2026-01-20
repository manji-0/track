# Workspace Sync Feature Design

## Overview

The workspace sync feature enables automatic management of JJ workspaces linked to tasks and TODOs. This design introduces repository management at the task level and integrates workspace lifecycle with TODO completion.

## Key Features

### 1. Repository Management (`track repo`)

Tasks can have multiple associated repositories. Each repository is registered with its absolute path.

**Commands:**
- `track repo add [path]` - Register a repository to the current task (defaults to current directory)
- `track repo add --base <bookmark>` - Register repository with custom base bookmark
- `track repo list` - List all repositories registered to the current task
- `track repo remove <id>` - Remove a repository registration

### 2. Workspace Sync (`track sync`)

Synchronizes the task's registered repositories with the task bookmark.

**Behavior:**
1. For each registered repository in the current task:
   - Ensure the task bookmark exists (create if it doesn't)
   - Move the workspace to the task bookmark
   - Task bookmark name is determined by:
     - If ticket is registered: `task/<ticket_id>` (e.g., `task/PROJ-123`)
     - If no ticket: `task/task-<task_id>` (e.g., `task/task-5`)
2. Set up any necessary workspaces for active TODOs

**Command:**
```bash
track sync
```

### 3. TODO-Workspace Integration

#### Adding TODOs with Workspace

`track todo add <text> --worktree`

**Behavior:**
1. Create the TODO with `worktree_requested` flag set to true
2. Helper message prompts user to run `track sync`
3. Workspace is **not** created immediately

To create the workspaces, run:
```bash
track sync
```
This will:
1. Iterate through pending TODOs
2. For each registered repository:
   - Create workspace at: `<repo_path>/<bookmark_name>`
   - Create workspace record (git_item) linked to the TODO

#### Managing TODO Workspaces

`track todo workspace <index> [--recreate --force --all]`

**Behavior:**
1. Resolve task-scoped index to internal TODO ID.
2. If `--all` is not set, resolve the current repo from the working directory.
3. If a workspace exists, print its path (or all paths with `--all`).
4. If no workspace exists, create it from the TODO bookmark.
5. If `--recreate` is set:
   - Abort if uncommitted changes are present unless `--force` is set.
   - Recreate the workspace from the latest bookmark.

#### Completing TODOs

`track todo done <id>`

**Behavior:**
1. Find all workspaces associated with the TODO
2. For each workspace:
   - Check for uncommitted changes with `jj status` (warn if found)
   - Merge or rebase the workspace bookmark into the task bookmark
   - Remove workspace directory
   - Delete workspace record (git_item)
3. Update TODO status to 'done'


### 4. Task Lifecycle Integration

#### Archiving Tasks

`track archive <task_id>`

**Behavior:**
1. Check for uncommitted changes in all workspaces associated with the task (both TODO workspaces and any others).
2. If changes exist, warn and require confirmation.
3. For each workspace:
   - Forget the workspace (`jj workspace forget <name>`) and remove the directory
   - Delete the `git_items` record
4. Update task status to 'archived'

## Database Schema Changes

### New Table: `task_repos`

```sql
CREATE TABLE task_repos (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id INTEGER NOT NULL,
    repo_path TEXT NOT NULL,
    base_bookmark TEXT,            -- Optional base bookmark for workspace merges
    base_commit_hash TEXT,         -- Commit hash when repository was registered
    created_at TEXT NOT NULL,
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
    UNIQUE(task_id, repo_path)
);

CREATE INDEX idx_task_repos_task_id ON task_repos(task_id);
```

### Existing Tables

No changes needed to existing tables. The `git_items` table already has:
- `todo_id` - Links a workspace to a TODO
- `is_base` - Distinguishes base workspaces from TODO workspaces

## Bookmark Naming Convention

### Task Bookmark (Base Bookmark)

| Condition | Bookmark Name | Example |
|-----------|---------------|---------|
| Ticket registered | `task/<ticket_id>` | `task/PROJ-123` |
| No ticket | `task/task-<task_id>` | `task/task-5` |

### TODO Workspace Bookmark

| Condition | Bookmark Name | Example |
|-----------|---------------|---------|
| Ticket registered | `<ticket_id>-todo-<task_index>` | `PROJ-123-todo-1` |
| No ticket | `task-<task_id>-todo-<task_index>` | `task-5-todo-1` |

## Workflow Example

```bash
# 1. Create a task with ticket
track new "Implement authentication" --ticket PROJ-123

# 2. Register repositories
cd /path/to/api-repo
track repo add .

cd /path/to/frontend-repo
track repo add .

# 3. Sync repositories (creates task bookmarks)
track sync
# → Creates bookmark task/PROJ-123 in both repos

# 4. Add TODO with workspace
track todo add "Add login endpoint" --worktree
# → Creates TODO #1 (task-scoped index)
# → Schedules workspace creation

# 5. Create the workspaces
track sync
# → Creates workspace at: api-repo/PROJ-123-todo-1
# → Creates workspace at: frontend-repo/PROJ-123-todo-1

# 6. Work in the workspace
cd "$(track todo workspace 1)"
# ... make changes, jj describe ...

# 7. Complete the TODO
track todo done 1
# → Merges PROJ-123-todo-1 into task/PROJ-123
# → Removes workspace directories
# → Marks TODO as done
```

## Implementation Plan

### Phase 1: Repository Management
1. Create `task_repos` table migration
2. Implement `RepoService` with add/list/remove operations
3. Add `track repo` CLI commands

### Phase 2: Worktree Sync
1. Implement task bookmark determination logic
2. Implement `workspace sync` command
3. Add tests for sync functionality

### Phase 3: TODO-Worktree Integration
1. Add `--worktree` flag to `todo add`
2. Implement automatic workspace creation
3. Enhance `todo done` to handle workspace merging
4. Add comprehensive tests

## Error Handling

### Repository Registration
- Error if path is not a JJ repository (missing `.jj` directory)
- Error if path is already registered for the task
- Error if no current task is set

### Workspace Sync
- Skip repositories that don't exist
- Error if repository has uncommitted changes (excluding workspace directories)
- Error if bookmark creation fails

### TODO Completion
- Warn if workspace has uncommitted changes
- Prompt for confirmation before merging
- Error if merge conflicts occur
- Rollback on failure

## Future Enhancements

- `track workspace sync --force` - Force sync even with dirty state
- `track repo sync` - Pull latest changes from remote
- `track todo add --no-worktree` - Explicitly skip workspace creation
- Automatic PR creation on TODO completion
