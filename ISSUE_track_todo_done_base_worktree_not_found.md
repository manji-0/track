# GitHub Issue: track todo done fails with 'Base worktree not found' error

## Title
`track todo done` fails with "Base worktree not found" error

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
6. Try to mark TODO as done: `track todo done 1`

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

## Root Cause (Hypothesis)

The `track todo done` command appears to be looking for a "base worktree" entry in the database (`git_items` table with `is_base = 1`). However, when using `track sync`, the main repository directory (which is checked out to the task branch) is not being registered as a base worktree in the database.

The command needs to either:
1. Properly register the main repository as a base worktree during `track sync`
2. Or fall back to using the current directory/repository when base worktree is not found in the database but the current directory is on the task branch

## Additional Notes

- This issue was encountered during Task #18 implementation
- Commit: 06e1260
- The manual workaround (`track todo update`) was used successfully
