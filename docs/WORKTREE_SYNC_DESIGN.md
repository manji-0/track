# Worktree Sync Feature Design

## Overview

The worktree sync feature enables automatic management of Git worktrees linked to tasks and TODOs. This design introduces repository management at the task level and integrates worktree lifecycle with TODO completion.

## Key Features

### 1. Repository Management (`track repo`)

Tasks can have multiple associated repositories. Each repository is registered with its absolute path.

**Commands:**
- `track repo add [path]` - Register a repository to the current task (defaults to current directory)
- `track repo list` - List all repositories registered to the current task
- `track repo remove <id>` - Remove a repository registration

### 2. Worktree Sync (`track worktree sync`)

Synchronizes the task's registered repositories with the task branch.

**Behavior:**
1. For each registered repository in the current task:
   - Checkout the task branch (create if doesn't exist)
   - Task branch name is determined by:
     - If ticket is registered: `task/<ticket_id>` (e.g., `task/PROJ-123`)
     - If no ticket: `task/task-<task_id>` (e.g., `task/task-5`)
2. Set up any necessary worktrees for active TODOs

**Command:**
```bash
track worktree sync
```

### 3. TODO-Worktree Integration

#### Adding TODOs with Worktree

`track todo add <text> --worktree`

**Behavior:**
1. Create the TODO with `worktree_requested` flag set to true
2. Helper message prompts user to run `track sync`
3. Worktree is **not** created immediately

To create the worktrees, run:
```bash
track sync
```
This will:
1. Iterate through pending TODOs
2. For each registered repository:
   - Create worktree at: `<repo_path>/<branch_name>`
   - Create git_item record linked to the TODO

#### Completing TODOs

`track todo done <id>`

**Behavior:**
1. Find all worktrees associated with the TODO
2. For each worktree:
   - Check for uncommitted changes (warn if found)
   - Merge worktree branch into task branch
   - Remove worktree directory
   - Delete git_item record
3. Update TODO status to 'done'


### 4. Task Lifecycle Integration

#### Archiving Tasks

`track archive <task_id>`

**Behavior:**
1. Check for uncommitted changes in all worktrees associated with the task (both TODO worktrees and any others).
2. If changes exist, warn and require confirmation.
3. For each worktree:
   - Remove the worktree directory (`git worktree remove`)
   - Delete the `git_items` record
4. Update task status to 'archived'

## Database Schema Changes

### New Table: `task_repos`

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

### Existing Tables

No changes needed to existing tables. The `git_items` table already has:
- `todo_id` - Links worktree to a TODO
- `is_base` - Distinguishes base worktrees from TODO worktrees

## Branch Naming Convention

### Task Branch (Base Branch)

| Condition | Branch Name | Example |
|-----------|-------------|---------|
| Ticket registered | `task/<ticket_id>` | `task/PROJ-123` |
| No ticket | `task/task-<task_id>` | `task/task-5` |

### TODO Worktree Branch

| Condition | Branch Name | Example |
|-----------|-------------|---------|
| Ticket registered | `<ticket_id>/todo-<todo_id>` | `PROJ-123/todo-15` |
| No ticket | `task-<task_id>/todo-<todo_id>` | `task-5/todo-15` |

## Workflow Example

```bash
# 1. Create a task with ticket
track new "Implement authentication" --ticket PROJ-123

# 2. Register repositories
cd /path/to/api-repo
track repo add .

cd /path/to/frontend-repo
track repo add .

# 3. Sync repositories (creates task branches)
track worktree sync
# → Creates branch task/PROJ-123 in both repos

# 4. Add TODO with worktree
track todo add "Add login endpoint" --worktree
# → Creates TODO #15
# → Schedules worktree creation

# 5. Create the worktrees
track sync
# → Creates worktree at: api-repo/PROJ-123/todo-15
# → Creates worktree at: frontend-repo/PROJ-123/todo-15

# 6. Work in the worktree
cd /path/to/api-repo/PROJ-123/todo-15
# ... make changes, commit ...

# 7. Complete the TODO
track todo done 15
# → Merges PROJ-123/todo-15 into task/PROJ-123
# → Removes worktree directories
# → Marks TODO as done
```

## Implementation Plan

### Phase 1: Repository Management
1. Create `task_repos` table migration
2. Implement `RepoService` with add/list/remove operations
3. Add `track repo` CLI commands

### Phase 2: Worktree Sync
1. Implement task branch determination logic
2. Implement `worktree sync` command
3. Add tests for sync functionality

### Phase 3: TODO-Worktree Integration
1. Add `--worktree` flag to `todo add`
2. Implement automatic worktree creation
3. Enhance `todo done` to handle worktree merging
4. Add comprehensive tests

## Error Handling

### Repository Registration
- Error if path is not a Git repository
- Error if path is already registered for the task
- Error if no current task is set

### Worktree Sync
- Skip repositories that don't exist
- Warn if repository is in a dirty state
- Error if branch creation fails

### TODO Completion
- Warn if worktree has uncommitted changes
- Prompt for confirmation before merging
- Error if merge conflicts occur
- Rollback on failure

## Future Enhancements

- `track worktree sync --force` - Force sync even with dirty state
- `track repo sync` - Pull latest changes from remote
- `track todo add --no-worktree` - Explicitly skip worktree creation
- Automatic PR creation on TODO completion
