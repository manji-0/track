# Section Rev Number Refactoring Plan

## Overview

Refactor the WebUI section update mechanism to use revision (rev) numbers instead of complex snapshot comparisons. Each section (todos, scraps, links, repos, worktrees, task) will have a global rev number that increments on any data change.

## Current Problems

1. **Inefficient polling**: Multiple COUNT queries executed every second in `state.rs`
2. **Complex detection logic**: String concatenation for `todo_status_hash`
3. **Inaccurate detection**: Content updates (e.g., TODO content edit) not detected if count/hash unchanged
4. **Scattered update logic**: Change detection logic duplicated between polling and route handlers

## Design

### Rev Keys in app_state

| Key | Increment Trigger |
|-----|-------------------|
| `rev:task` | Task metadata update (description, ticket, alias) |
| `rev:todos` | TODO add/delete/update (status, content) |
| `rev:scraps` | Scrap add/delete |
| `rev:links` | Link add/delete |
| `rev:repos` | Repository add/delete |
| `rev:worktrees` | Worktree add/delete/update |

### WebUI Update Triggers

| Section | Trigger Revs |
|---------|--------------|
| Header | `rev:task` |
| Description | `rev:task` |
| Ticket | `rev:task` |
| Links | `rev:links` |
| Repos | `rev:repos` |
| TODOs | `rev:todos` OR `rev:worktrees` |
| Scraps | `rev:scraps` |
| Worktrees | `rev:worktrees` |

### Key Design Decisions

1. **Global rev numbers**: Single integer per section, not per-task
2. **Increment on write**: Services increment rev when modifying data
3. **All revs in single query**: Change detection fetches all revs with one SELECT
4. **Task switch detection**: Use `current_task_id` change as before (triggers full reload)

## Implementation Steps

### Step 1: Add rev increment/get functions to Database

Add to `src/db/mod.rs`:

```rust
/// Increment a section revision number and return the new value
pub fn increment_rev(&self, section: &str) -> Result<i64> {
    let key = format!("rev:{}", section);
    let current: i64 = self.get_app_state(&key)?
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let new_rev = current + 1;
    self.set_app_state(&key, &new_rev.to_string())?;
    Ok(new_rev)
}

/// Get current revision number for a section
pub fn get_rev(&self, section: &str) -> Result<i64> {
    let key = format!("rev:{}", section);
    Ok(self.get_app_state(&key)?
        .and_then(|s| s.parse().ok())
        .unwrap_or(0))
}

/// Get all section revisions at once
pub fn get_all_revs(&self) -> Result<SectionRevs> {
    // Single query implementation
}
```

### Step 2: Define SectionRevs struct

Add to `src/db/mod.rs` or a new module:

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct SectionRevs {
    pub task: i64,
    pub todos: i64,
    pub scraps: i64,
    pub links: i64,
    pub repos: i64,
    pub worktrees: i64,
}
```

### Step 3: Update Services to increment revs

Update each service to call `db.increment_rev()` after data modifications:

| Service | Method | Rev to Increment |
|---------|--------|------------------|
| `TaskService` | `update_description`, `set_ticket`, `set_alias` | `task` |
| `TodoService` | `add_todo`, `update_todo_status`, `delete_todo`, `update_todo_content` | `todos` |
| `ScrapService` | `add_scrap`, `delete_scrap` | `scraps` |
| `LinkService` | `add_link`, `delete_link` | `links` |
| `RepoService` | `add_repo`, `delete_repo` | `repos` |
| `WorktreeService` | `add_worktree`, `delete_worktree`, `update_worktree` | `worktrees` |

### Step 4: Simplify state.rs change detection

Replace `DbSnapshot` with `SectionRevs` based detection:

```rust
struct ChangeState {
    current_task_id: Option<i64>,
    revs: SectionRevs,
}

async fn start_change_detection(&self) {
    loop {
        let current = self.get_change_state().await;
        
        if let Some(ref prev) = *last {
            if current.current_task_id != prev.current_task_id {
                // Task switched - full reload
                self.broadcast_all();
            } else {
                // Check individual rev changes
                if current.revs.task != prev.revs.task {
                    self.broadcast(SseEvent::Header);
                    self.broadcast(SseEvent::Description);
                    self.broadcast(SseEvent::Ticket);
                }
                if current.revs.todos != prev.revs.todos 
                   || current.revs.worktrees != prev.revs.worktrees {
                    self.broadcast(SseEvent::Todos);
                }
                // ... etc
            }
        }
        *last = Some(current);
    }
}
```

### Step 5: Update WebUI route handlers (Optional optimization)

Route handlers currently broadcast SSE events directly. With rev-based detection, we could:
- **Option A**: Keep route broadcasts (immediate update) + polling as backup
- **Option B**: Remove route broadcasts, rely solely on polling

Recommend **Option A** for best UX (immediate feedback).

### Step 6: Add tests

- Unit tests for `increment_rev`, `get_rev`, `get_all_revs`
- Integration test: verify rev increments on service operations
- Integration test: verify SSE events triggered on rev changes

## File Changes Summary

| File | Changes |
|------|---------|
| `src/db/mod.rs` | Add `SectionRevs`, `increment_rev()`, `get_rev()`, `get_all_revs()` |
| `src/services/task_service.rs` | Add `increment_rev("task")` calls |
| `src/services/todo_service.rs` | Add `increment_rev("todos")` calls |
| `src/services/scrap_service.rs` | Add `increment_rev("scraps")` calls |
| `src/services/link_service.rs` | Add `increment_rev("links")` calls |
| `src/services/repo_service.rs` | Add `increment_rev("repos")` calls |
| `src/services/worktree_service.rs` | Add `increment_rev("worktrees")` calls |
| `src/webui/state.rs` | Replace `DbSnapshot` with `SectionRevs` based detection |

## Migration

No database schema migration needed - uses existing `app_state` table.

## Risks and Considerations

1. **Rev overflow**: i64 max is ~9.2×10¹⁸, not a practical concern
2. **Concurrent writes**: SQLite serializes writes, no race condition
3. **Rev not task-scoped**: On task switch, all sections update anyway, so acceptable
4. **CLI changes**: CLI operations increment rev, WebUI detects via polling

## Implementation Status

**Completed: 2026-01-04**

All steps implemented:
- ✅ Step 1: Added `increment_rev()`, `get_rev()`, `get_all_revs()` to Database
- ✅ Step 2: Defined `SectionRevs` struct with Clone, Debug, PartialEq, Default
- ✅ Step 3: Updated all services to increment revs on data modifications
- ✅ Step 4: Refactored `state.rs` to use `ChangeState` with `SectionRevs`
- ✅ Step 5: Kept Option A (route broadcasts + polling backup)
- ✅ Step 6: Added unit tests for rev functions

Additional changes:
- Moved `delete_link` from routes.rs direct SQL to LinkService
- CLI `track link delete` command now uses LinkService

