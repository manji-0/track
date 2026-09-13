---
name: track-task-execute
description: Execute track TODOs in the track-owned git/jj workspace. Read track status --json, follow hint/next_action, implement in .worktrees/<slug>, track scrap/done for task state. Use when workflow.phase is sync_required or execute.
license: MIT
compatibility: Requires track CLI
metadata:
  author: track
  version: 4.0.0
  tags: [track, execute, agent, todo]
---

# Track — Task Execution (Agents)

**Track** owns TODO state and the coding workspace. Commit with git or jj **from that workspace**.

## Agent loop (every turn)

```
track status --json
        ↓
follow hint.next_command / workflow.next_action
  (track sync, or cd "<workspace_path>")
        ↓
implement + test in .worktrees/<slug>/
        ↓
commit/push from the workspace (branch/bookmark track/<slug>)
        ↓
track scrap add --json "..."
        ↓
track todo done --json <index>
        ↓
repeat until task_complete
```

## Step 1 — Read JSON

```bash
track status --json
```

| Field | Action |
|-------|--------|
| `hint.next_command` | Run this |
| `workflow.next_action` | Same as hint when present |
| `workflow.checklist` | Ordered steps with `done` flags |
| `git.workspace_path` / `jj.workspace_path` | `.worktrees/<slug>` |
| `todos_agent[].is_next` | Current TODO |

Do **not** look up jj-task maps or run `git worktree` / `jj workspace` yourself.

## Step 2 — Ensure workspace (sync_required)

```bash
track sync
# then the hint becomes: cd "<repo>/.worktrees/<slug>"
```

Workspace path: `.worktrees/<slug>/`. PR head: `track/<slug>`.

## Step 3 — Implement

- Work only inside the task workspace — **not** repo root
- Run project tests/linters
- Git: commit in the worktree, `git push -u origin track/<slug>`
- JJ: commit in the workspace, `jj git push --named track/<slug>`

## Step 4 — Record in track

```bash
track scrap add --json "Chose bcrypt; tests at 95%"
```

With `aggressive-mode on`, scraps are also git notes on the task revision.

## Step 5 — Complete TODO (track DB)

```bash
track todo done --json <index>
```

Marks TODO done in track. The JSON response includes `workflow.next_action` for the next TODO.

## Step 6 — Repeat

If the mutation JSON already includes `hint` / `workflow.next_action`, follow it. Otherwise re-run `track status --json`.

## Error recovery

| Problem | Fix |
|---------|-----|
| Missing workspace | `track sync` |
| Wrong directory | `cd` the path from `hint` / JSON |
| TODO state | `track status --json` |

Full walkthrough: [references/execution-workflow.md](references/execution-workflow.md)
