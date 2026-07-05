---
name: track-advanced
description: Advanced track workflows — multi-repository tasks, parallel JJ workspaces, hotfixes, templates, task switching, archive, and long-running tasks. Use when work spans multiple repos, needs parallel terminals, or workflow.phase is task_complete.
license: MIT
compatibility: Requires track CLI and jj on PATH
metadata:
  author: track
  version: 2.0.0
  tags: [track, advanced, multi-repo, parallel, archive]
---

# Track — Advanced Workflows

Use when **basic setup + execute** is not enough.

## When to use

- Multiple repositories in one task
- Parallel TODO workspaces (separate terminals)
- Hotfix / urgent single-TODO flow
- `workflow.phase` is **`task_complete`** → archive or push
- Task switching, templates, ticket-based refs
- Research tasks (TODOs without workspaces)

## Multi-repository task

```bash
track new "Cross-service feature" --ticket PROJ-789
track repo add /path/to/frontend
track repo add /path/to/backend
track todo add "API endpoint" --worktree
track todo add "Client sync" --worktree
track sync
```

Work each TODO via **track-task-execute** — `track sync` touches all repos.

## Parallel workspaces

```bash
# Terminal A
cd "$(track todo workspace 2)"
# ... work ...
track todo done 2

# Terminal B (simultaneously)
cd "$(track todo workspace 3)"
track todo done 3
```

Each workspace has an independent bookmark under the same task bookmark.

## Hotfix pattern

```bash
track new "Fix auth bug" --ticket BUG-999
track desc "Token refresh fails after 30min"
track todo add "Fix refresh logic" --worktree
track sync
cd "$(track todo workspace 1)"
# fix, test, describe, done
track todo done 1
jj git push --bookmark task/BUG-999
```

## Task switching

```bash
track switch t:PROJ-123
track switch a:my-alias
track switch 5
track list
```

Always run `track status --json` after switch.

## Templates

```bash
track new "Sprint 2" --template t:PROJ-100
# TODOs copied as pending
```

## Archive completed task

```bash
track status --json   # confirm task_complete
track archive         # removes workspaces; marks archived
track archive t:PROJ-123
```

Dirty workspaces block archive — commit or discard first.

## Ticket references

```bash
track switch t:PROJ-123
track status t:PROJ-123
track archive t:PROJ-123
```

Bookmarks: `task/<ticket>` and `task/<ticket>-todo-<n>`.

## Research / non-code TODOs

```bash
track new "Evaluate auth providers"
track todo add "Compare Okta vs Auth0"    # no --worktree
track scrap add "Finding: ..."
track todo done 1
```

## Team handoff

```bash
jj git push --bookmark task/PROJ-123
# teammate:
track switch t:PROJ-123
track status --json
track scrap list
```

## Patterns reference

Detailed examples: [references/advanced-patterns.md](references/advanced-patterns.md)

## Skill routing

| Situation | Skill |
|-----------|-------|
| New task, no repos | track-task-setup |
| Implementing code | track-task-execute |
| Multi-repo / archive | track-advanced (this) |
