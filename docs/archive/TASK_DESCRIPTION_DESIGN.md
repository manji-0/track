# Task Description Feature Design

## Overview

Add support for task descriptions to provide more context and details about tasks beyond just the task name.

## Schema Changes

### Database Migration

Add `description` column to the `tasks` table:

```sql
ALTER TABLE tasks ADD COLUMN description TEXT;
```

**Updated `tasks` table schema:**

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

## Command Interface

### 1. Add description when creating a task

```bash
track new <name> --description <text>
track new <name> -d <text>
```

**Example:**
```bash
track new "Implement authentication" --description "Add JWT-based authentication to the API with refresh token support"
```

### 2. View or set description for current task

```bash
# View current task description
track desc

# Set/update description for current task
track desc <text>

# Set description for specific task
track desc <text> --task <task_id>
```

**Examples:**
```bash
# View description
track desc

# Set description
track desc "This task implements user authentication using JWT tokens"

# Update description for specific task
track desc "Updated description" --task 5
```

### 3. Display description in task info

The `track status` command should display the task description:

```
# Task #6: feat: add task description

**Created:** 2025-12-31 23:35:37

## Description

Add support for task descriptions to provide more context about tasks.
This includes schema changes, CLI commands, and documentation updates.

## TODOs

- [ ] **[1]** Design schema and command interface
  ...
```

### 4. Display description in task list (optional)

For `track list`, show a truncated description (first 50 chars):

```
  ID | Ticket     | Name                  | Description           | Status
-----+------------+-----------------------+-----------------------+--------
*  6 | -          | feat: add task desc   | Add support for ta... | active
```

## Implementation Plan

### Phase 1: Database Migration
1. Add migration logic in `db/mod.rs` to add `description` column
2. Update `Task` model in `models/mod.rs` to include `description` field
3. Add tests for migration

### Phase 2: Service Layer
1. Update `TaskService::create_task()` to accept optional description
2. Add `TaskService::set_description()` method
3. Add `TaskService::get_description()` method
4. Update tests

### Phase 3: CLI Commands
1. Add `--description` flag to `track new` command
2. Implement `track desc` subcommand
3. Update `track status` to display description
4. Optionally update `track list` to show truncated description

### Phase 4: Documentation
1. Update `FUNCTIONAL_SPEC.md` with description feature
2. Update `README.md` with usage examples
3. Add migration notes if needed

## Error Handling

- If description is too long (>1000 chars), warn or truncate
- If no current task when running `track desc`, show appropriate error
- Handle NULL descriptions gracefully in display

## Future Enhancements

- Support multiline descriptions (read from stdin or file)
- Markdown rendering for descriptions
- Search tasks by description content
