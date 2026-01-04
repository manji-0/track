# Task-Scoped ID Refactoring for Links and Repos

## Summary

Changed the ID numbering system for `links` and `task_repos` from global (database auto-increment) to task-scoped, similar to how `todos` and `scraps` work.

## Changes Made

### 1. Database Schema
- Added `task_index` column to `links` table
- Added `task_index` column to `task_repos` table
- Created unique indices on `(task_id, task_index)` for both tables
- Added migration logic to populate existing records with sequential task-scoped indices

### 2. Models (`src/models/mod.rs`)
- **Link**: Added `task_index` field, serialized as `link_id`
- **TaskRepo**: Added `task_index` field, serialized as `repo_id`
- Both models now skip serializing the global `id` and `task_id` fields

### 3. Services
- **LinkService** (`src/services/link_service.rs`):
  - `add_link`: Auto-assigns next available task_index
  - `get_link`: Retrieves task_index from database
  - `list_links`: Orders by task_index instead of created_at
  
- **RepoService** (`src/services/repo_service.rs`):
  - `add_repo`: Auto-assigns next available task_index
  - `list_repos`: Retrieves task_index and orders by it

### 4. CLI (`src/cli/handler.rs`)
- **Link commands**:
  - `link add`: Displays task_index when adding
  - `link list`: Shows task_index in ID column
  - `link delete <index>`: Uses task_index to find and delete link
  
- **Repo commands**:
  - `repo list`: Shows task_index in ID column
  - `repo remove <id>`: Uses task_index to find and delete repo

### 5. WebUI (`src/webui/routes.rs`)
- **delete_link**: Changed from 1-based position index to task_index lookup
- Path parameter changed from `usize` to `i64`

### 6. Templates
- **partials/links.html**: Changed delete button to use `link.link_id` instead of `loop.index`

## Benefits

1. **Consistency**: Links and repos now use the same ID scheme as todos and scraps
2. **Task-scoped**: IDs are sequential within each task (1, 2, 3...) instead of global
3. **User-friendly**: Easier to reference items by their task-scoped number
4. **Stable references**: Task-scoped IDs remain stable even if items from other tasks are deleted

## Migration

The migration is automatic and runs when the database is opened:
- Existing links and repos are assigned task_index values based on their creation order
- No data loss occurs
- The global `id` field is retained internally but not exposed to users

## Testing

All existing tests pass without modification, confirming backward compatibility of the internal implementation.
