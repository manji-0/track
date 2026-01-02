# GitHub Issue: track todo done fails with 'Base worktree not found' error

## Title
`track todo done` fails with "Base worktree not found" error

## Status
✅ **RESOLVED**

## Description

When trying to complete a TODO using `track todo done <id>`, the command fails with the error:
```
Error: Base worktree not found. Please init a base worktree first.
```

This occurs even when the repository is properly set up with git worktrees and the main directory is on the correct task branch.

## Steps to Reproduce

1. Create a new task with `track new "task name"`
2. Add a repository with `track repo add .`
3. Add a TODO with worktree request: `track todo add "TODO content" --worktree`
4. Run `track sync` to create worktrees
5. Complete work in the TODO worktree and commit changes
6. Try to mark TODO as done: `track todo down 1`

## Expected Behavior

The command should:
1. Merge the TODO worktree branch into the task branch
2. Remove the worktree
3. Mark the TODO as done

## Actual Behavior

The command fails with:
```
Error: Base worktree not found. Please init a base worktree first.
```

## Environment

- Current branch: `task/task-18` (the task's base branch)
- Git worktrees exist as shown by `git worktree list`:
  ```
  /home/manji0/src/track                         06e1260 [task/task-18]
  /home/manji0/src/track/task-18-todo-1          2695090 [task-18-todo-1]
  ```
- Database shows TODO with worktree associated

## Workaround

Use `track todo update <id> done` instead of `track todo done <id>` to manually mark the TODO as done. 

Note: This workaround does NOT perform the merge and worktree cleanup that `track todo done` is supposed to do.

## Root Cause

The `track todo done` command was looking for a "base worktree" entry in the database (`git_items` table with `is_base = 1`). However, when using `track sync`, the main repository directory (which is checked out to the task branch) was not being registered as a base worktree in the database.

The `complete_worktree_for_todo()` method in `WorktreeService` was requiring a base worktree to exist in the database before it could merge TODO branches.

## Resolution

Modified `WorktreeService::complete_worktree_for_todo()` to fall back to using the TODO worktree's `base_repo` field when no base worktree is found in the database. This allows the merge to happen in the main repository directory even when it's not formally registered as a git worktree.

### Changes Made

1. **Updated `src/services/worktree_service.rs`**:
   - Modified `complete_worktree_for_todo()` to check for a base worktree in the database
   - If not found, fall back to using the TODO worktree's `base_repo` path
   - This allows merging into the main repository directory without requiring a formal base worktree entry

2. **Added Test Coverage**:
   - Created `tests/worktree_no_base_test.rs` with integration test `test_complete_worktree_without_base_worktree`
   - This test verifies that `track todo done` works correctly when no base worktree exists in the database

### Testing

All 116 tests pass, including the new integration test that specifically verifies this scenario.

### Commit

Commit hash: f9e61fd
Commit message: "fix: resolve 'Base worktree not found' error in track todo done"

## Additional Notes

- This issue was encountered during Task #18 implementation
- The fix maintains backward compatibility with existing workflows that do register base worktrees
- The solution is cleaner than the alternative of registering the main repository as a base worktree during `track sync`, as it avoids confusion about what constitutes a "worktree" (the main repo is not technically a git worktree)
