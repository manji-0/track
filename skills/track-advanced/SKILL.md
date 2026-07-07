---
name: track-advanced
description: Advanced track workflows with agent-skill-jj — archive, handoff, multi-repo tasks, hotfixes. Assumes jj-task and $jj skill for all JJ/PR operations. Use when workflow.phase is task_complete or for cross-repo patterns.
license: MIT
compatibility: Requires track CLI, jj-task, and agent-skill-jj ($jj skill)
metadata:
  author: track
  version: 3.0.0
  tags: [track, advanced, multi-repo, archive]
---

# Track — Advanced Workflows

Track handles **task state**; **`$jj`** handles **PR merge and push**.

## Task completion

```bash
track status --json                    # confirm task_complete
# $jj skill: merge PR, final push
jj-task done <jj.slug>                 # mark workspace merged in jj map
track archive                          # archive track task
```

## Multi-repository task

```bash
track new "Cross-service feature" --ticket PROJ-789
track repo add /path/to/frontend
track repo add /path/to/backend
track todo add "API endpoint"
track todo add "Client sync"
```

Each repo: `jj-task repo init` once, then `jj-task start <slug>` per repo (same slug, separate maps).

## Hotfix

```bash
track new "Fix auth bug" --ticket BUG-999
track alias set fix-auth-bug
track todo add "Fix refresh logic"
# track-task-execute: jj-task start fix-auth-bug
# $jj: squash, draft PR, push
track todo done 1
```

## Task switching

```bash
track switch t:PROJ-123
track status --json    # new jj.slug
cd "$(jj-task path <jj.slug>)"
```

## Team handoff

```bash
# $jj skill: push bookmark, open PR
track status --json    # share jj.slug + TODOs
track scrap list
```

## Research / non-code TODOs

```bash
track todo add "Compare providers" --no-workspace
track scrap add "Finding: ..."
track todo done 1
```

## Deprecated patterns

- **`track todo add --worktree`** — legacy; use jj-task instead
- **`track sync` + repo root editing** — replaced by jj-task workspace
- **`jj describe` only** — use **`$jj`** for commit/PR rules

Patterns: [references/advanced-patterns.md](references/advanced-patterns.md)
