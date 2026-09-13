---
name: track-advanced
description: Advanced track workflows — archive, handoff, multi-repo tasks, hotfixes. Track owns git/jj workspaces. Use when workflow.phase is task_complete or for cross-repo patterns.
license: MIT
compatibility: Requires track CLI
metadata:
  author: track
  version: 4.0.0
  tags: [track, advanced, multi-repo, archive]
---

# Track — Advanced Workflows

Track handles **task state and workspaces**. Push/merge `track/<slug>` from the workspace, then archive.

## Task completion

```bash
track status --json                    # confirm task_complete
# push track/<slug> and merge the PR
track archive                          # removes workspaces; keeps branch/bookmark
```

Do **not** pass `track archive --force` unless the user explicitly wants to skip dirty-workspace checks. Agents cannot confirm prompts: non-TTY stdin fails instead of hanging.

## Multi-repository task

```bash
track new "Cross-service feature" --ticket PROJ-789
track repo add /path/to/frontend
track repo add /path/to/backend
track todo add "API endpoint"
track todo add "Client sync"
```

`track repo add` / `track sync` creates `.worktrees/<slug>` in **each** repo (same slug).

## Hotfix

```bash
track new "Fix auth bug" --ticket BUG-999
track alias set fix-auth-bug
track todo add "Fix refresh logic"
# follow hint → cd .worktrees/fix-auth-bug
track todo done 1
```

## Task switching

```bash
track switch t:PROJ-123
track status --json    # new slug; track ensures workspace when repos exist
```

## Team handoff

```bash
# push track/<slug>, open PR
track status --json    # share slug + TODOs
track scrap list
```

## Research / non-code TODOs

```bash
track todo add "Compare providers" --no-workspace
track scrap add "Finding: ..."
track todo done 1
```

Patterns: [references/advanced-patterns.md](references/advanced-patterns.md)
