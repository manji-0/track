# JJ Workflow Design and Implementation Plan

## Overview

This document defines the JJ-centered workflow for Track, replacing Git branch/worktree assumptions with JJ bookmarks and workspaces. It also outlines the design and implementation work required to support JJ end-to-end.

## Goals

- Use JJ bookmarks instead of Git branches for task and TODO naming.
- Use JJ workspaces instead of Git worktrees for isolated work.
- Treat each TODO as an independent JJ change.
- Allow TODO ordering to change without Track-level dependencies.
- Keep user-facing workflow simple and predictable.

## Non-Goals

- Support mixed Git and JJ workflows at the same time.
- Implement custom VCS abstractions beyond the Track CLI needs.
- Add explicit dependency tracking between TODOs.

## Terminology Mapping

| Git Term | JJ Term | Track Usage |
| --- | --- | --- |
| branch | bookmark | task/todo identifiers |
| worktree | workspace | isolated working directory |
| checkout | workspace move | move base workspace to bookmark |
| commit | change | user workflow terminology |

## Naming Conventions

- Task bookmark:
  - With ticket: `task/<ticket_id>`
  - Without ticket: `task/task-<task_id>`
- TODO bookmark:
  - With ticket: `<ticket_id>-todo-<task_index>`
  - Without ticket: `task-<task_id>-todo-<task_index>`
- Workspace path:
  - `<repo_root>/<bookmark_name>`

## Change Model

- Each TODO is tracked as an independent JJ change.
- A TODO change lives at a TODO bookmark and is edited in a TODO workspace.
- The task bookmark is the integration point that collects completed TODO changes.
- Track does not assume any fixed order between TODO changes.
- If ordering becomes necessary, users (or LLMs) can rebase changes to a new order.

## JJ-Centered Workflow

### Task Setup

1. `track new <name>` creates a task (optionally with ticket).
2. `track repo add [path]` registers a JJ repository.
3. `track todo add "<text>" --worktree` schedules a workspace for a TODO.

### Sync

`track sync` performs:

- Ensure the task bookmark exists in each repo.
- Move the base workspace to the task bookmark.
- Create workspaces for pending TODOs (one per repo and TODO).

### Work

- Work inside the TODO workspace.
- Use `jj status` for change detection.
- Use `jj describe` or other JJ commands for change tracking.

### Completion

- `track todo done <index>` rebases the TODO bookmark onto the task bookmark.
- The TODO workspace is forgotten after a successful rebase.

### Workspace Management

`track todo workspace <index>` creates or shows TODO workspaces for the current repo.

- If a workspace exists for the current repo, the command prints its path.
- If a workspace does not exist for the current repo, the command creates it.
- Use `--recreate` to remove and recreate workspaces from the latest task bookmark.
- The default behavior aborts recreation if uncommitted changes are present.
- Use `--force` with `--recreate` to override the safety check.
- Use `--all` to operate across all registered repos and print each path.

### Archive

- `track archive` checks each workspace for uncommitted changes using `jj status`.
- If clean (or forced), it forgets workspaces and archives the task.

## Repository Validation

When registering a repo with `track repo add`:

- Validate the path contains a `.jj` directory.
- Capture the base bookmark (current bookmark unless `--base` is provided).

## Data Model Updates

Proposed changes to existing fields only. No new tables required.

- `task_repos.base_branch` -> `task_repos.base_bookmark`
- `git_items` remains as the workspace record table
- `git_items.path` remains the workspace directory

## Command Behavior Updates

- Replace Git CLI calls with JJ equivalents:
  - `git status --porcelain` -> `jj status`
  - `git worktree remove` -> `jj workspace forget`
  - `git branch` queries -> `jj bookmark list` and workspace metadata
- Replace language in help output and error messages:
  - branch -> bookmark
  - worktree -> workspace
- Add `track todo workspace` for manual workspace recreation.

## Implementation Plan

### Phase 1: JJ Repository Support

1. Update repo validation to detect `.jj`.
2. Store base bookmark name in `task_repos`.
3. Update CLI messaging to mention JJ repositories.

### Phase 2: Bookmark and Workspace Operations

1. `track sync` creates task bookmarks when missing.
2. `track sync` creates JJ workspaces for TODOs.
3. `track todo done` rebases TODO bookmarks onto the task bookmark.
4. `track todo workspace` removes and recreates TODO workspaces.
5. `track archive` forgets JJ workspaces and updates records.

### Phase 3: UX and Documentation

1. Update help output to use JJ terms.
2. Update usage examples and specs to show JJ commands.
3. Add JJ-specific troubleshooting guidance if needed.

## Testing Plan

- Unit tests for repo validation and bookmark naming.
- Integration tests that initialize temporary JJ repositories.
- End-to-end test covering:
  - task creation
  - sync
  - workspace creation
  - workspace recreation
  - todo completion
  - archive cleanup

## Open Decisions

- Behavior when the task bookmark workspace has uncommitted changes.
- Rebase conflict resolution workflow for `track todo done`.
