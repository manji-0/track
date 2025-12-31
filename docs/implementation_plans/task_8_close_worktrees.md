# Implementation Plan - Task #8: Close worktrees when task is closed

## Goal
Implement functionality to automatically close (remove) Git worktrees associated with a task when the task is archived (closed).

## Design Changes

### Functional Specification (`docs/FUNCTIONAL_SPEC.md`)
- Update `track archive` command behavior.
- Instead of just marking worktrees as 'archived', it should:
  1. Check for uncommitted changes in all worktrees linked to the task.
  2. If changes exist, warn the user and abort (or require force).
  3. If no changes (or forced), execute `git worktree remove` for each worktree.
  4. Update the worktree status in DB to 'archived' (or delete? spec says `track cleanup` deletes. Keeping as 'archived' but removed from disk might be inconsistent with `git_items` table having a path. If path doesn't exist, is it valid? Maybe status 'archived' implies path is not relevant anymore or is historical).
     - *Correction*: `git_items` has a `path`. If we remove the directory, the path is invalid.
     - However, we might want to keep the record for history.
     - `FUNCTIONAL_SPEC` says `track cleanup` deletes the record.
     - If we remove the worktree immediately upon archive, we should probably update the record to reflect this, or just keep it as 'archived' (meaning 'historical record').
     - Let's stick to: Remove the physical worktree, set status to 'archived'. `track cleanup` can explicitly *prune* these records from DB if needed, or we just leave them.
     - Actually, `track cleanup` currently "Deletes `archived` worktrees from disk". If they are already deleted, it should just delete the DB record.

### Worktree Sync Design (`docs/WORKTREE_SYNC_DESIGN.md`)
- Add section on Task Archival/Closure.

## Implementation Steps

1. **Update `WorktreeService`**:
   - Add `get_task_worktrees(task_id: i64) -> Result<Vec<GitItem>>`
   - Add `remove_worktree(worktree_id: i64, force: bool) -> Result<()>` (Refactoring existing logic if any, or implementing new).
   - Ensure `remove_worktree` handles:
     - Check uncommitted changes (using `git status --porcelain`)
     - `git worktree remove`
     - DB update (status -> 'archived' or delete?) -> Let's set status 'archived' and maybe clear path? Or keep path for reference? Spec says `track cleanup` deletes record. Let's keep record as 'archived'.

2. **Update `TaskService`**:
   - In `archive_task`:
     - Retrieve associated worktrees.
     - For each worktree:
       - Check status.
       - Remove worktree using `WorktreeService`.
     - Proceed with archiving the task.

3. **CLI Updates**:
   - `track archive`:
     - Add logic to handle worktree removal flow (confirmation prompts etc, if not handled in service).
     - Ideally, keep logic in Service, but prompt in CLI?
     - Service should return "Worktrees having uncommitted changes" error, CLI handles prompt, then calls `archive_task(force: true)`.

4. **Tests**:
   - Test `archive_task` with clean worktrees -> Worktrees removed, task archived.
   - Test `archive_task` with dirty worktrees -> Error/Warning.
   - Test `archive_task` with force -> Worktrees removed despite changes.

## Refinement on "Closed" vs "Archived"
The task title allows for interpretation. Given `track` commands, `archive` is the closest to `close`.
We will assume "Task Closed" = "Task Archived".

