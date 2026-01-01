# TODO Index Design: Task-Scoped Numbering

## Overview

Change TODO indexing from global database IDs to task-scoped sequential numbers starting from 1.

## Current Implementation

### Database Schema
```sql
CREATE TABLE todos (
    id INTEGER PRIMARY KEY AUTOINCREMENT,  -- Global ID
    task_id INTEGER NOT NULL,
    content TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    created_at TEXT NOT NULL,
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
);
```

### Current Behavior
- TODO IDs are globally unique across all tasks (e.g., 1, 2, 3, 4...)
- When displaying TODOs for Task #1: might show IDs 1, 5, 8
- When displaying TODOs for Task #2: might show IDs 2, 3, 4, 6, 7

## Proposed Changes

### New Schema
Add a `task_index` column to maintain task-scoped sequential numbering:

```sql
ALTER TABLE todos ADD COLUMN task_index INTEGER;
CREATE UNIQUE INDEX idx_todos_task_index ON todos(task_id, task_index);
```

### New Behavior
- Keep global `id` for internal references (foreign keys, etc.)
- Add `task_index` for user-facing display (1, 2, 3... within each task)
- When displaying TODOs for Task #1: show indices 1, 2, 3
- When displaying TODOs for Task #2: show indices 1, 2, 3, 4, 5

### Example
```
Task #1 TODOs:
  [1] Design schema           (internal id: 28)
  [2] Update documents        (internal id: 29)
  [3] Implement functions     (internal id: 30)

Task #2 TODOs:
  [1] Write tests             (internal id: 34)
  [2] Update README           (internal id: 35)
```

## Implementation Plan

### Phase 1: Schema Migration
1. Add `task_index` column to `todos` table
2. Populate `task_index` for existing TODOs
3. Add unique constraint on (task_id, task_index)
4. Update migration tests

### Phase 2: Model Updates
1. Add `task_index` field to `Todo` model
2. Update all SQL queries to include `task_index`

### Phase 3: Service Layer
1. Update `TodoService::add_todo()` to calculate and assign `task_index`
2. Add `TodoService::get_todo_by_index(task_id, index)` method
3. Update `TodoService::list_todos()` to order by `task_index`
4. Update service tests

### Phase 4: CLI Updates
1. Update `track todo list` to display `task_index` instead of `id`
2. Update `track status` to display `task_index` for TODOs
3. Add support for referencing TODOs by index in commands:
   - `track todo update <index> <status>` (instead of global ID)
   - `track todo done <index>` (instead of global ID)
   - `track todo delete <index>` (instead of global ID)
4. Update CLI help text and examples

### Phase 5: Worktree Integration
1. Update worktree commands to use task_index
2. Ensure `track todo add --worktree` uses task_index for branch naming
3. Update worktree display to show task_index

## Migration Strategy

### Backward Compatibility
- Keep global `id` for internal database operations
- Use `task_index` only for user-facing operations
- Existing foreign key references (e.g., git_items.todo_id) continue to use global `id`

### Data Migration
```sql
-- Assign task_index to existing TODOs based on creation order
WITH numbered_todos AS (
  SELECT id, task_id, 
         ROW_NUMBER() OVER (PARTITION BY task_id ORDER BY created_at) as idx
  FROM todos
)
UPDATE todos 
SET task_index = (
  SELECT idx FROM numbered_todos WHERE numbered_todos.id = todos.id
);
```

## Command Interface Changes

### Before
```bash
track todo update 28 done    # Uses global ID
track todo delete 29 --force # Uses global ID
```

### After
```bash
track todo update 1 done     # Uses task-scoped index
track todo delete 2 --force  # Uses task-scoped index
```

### Ambiguity Resolution
Since we're changing from global ID to task-scoped index, commands now require current task context:
- All TODO commands operate on the current task
- To operate on a different task's TODOs, switch to that task first

## Error Handling

- If `task_index` is out of range, show clear error: "TODO #5 not found in current task"
- If no current task, show: "No active task. Use 'track switch <task>' first"
- Handle concurrent TODO creation (race condition on task_index calculation)

## Testing Requirements

1. **Migration Tests**
   - Verify task_index is correctly assigned to existing TODOs
   - Verify unique constraint works
   - Test migration with empty database

2. **Service Tests**
   - Test task_index assignment for new TODOs
   - Test get_todo_by_index()
   - Test task_index uniqueness within task
   - Test task_index independence across tasks

3. **CLI Tests**
   - Verify display shows task_index
   - Verify commands accept task_index
   - Test error messages

## Future Enhancements

- Support global TODO search across all tasks
- Add `track todo list --all` to show TODOs from all tasks with task context
- Consider adding TODO reordering capability
