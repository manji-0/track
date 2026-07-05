---
name: track-task-execute
description: Execute track TODOs as a coding agent — read JSON status, sync JJ bookmarks, work in workspaces, describe changes, record scraps, complete with track todo done. Use when workflow.phase is sync_required or execute, or the user asks to continue, implement, or finish TODOs.
license: MIT
compatibility: Requires track CLI and jj on PATH
metadata:
  author: track
  version: 2.0.0
  tags: [track, execute, agent, jj, sync, todo]
---

# Track — Task Execution (Agents)

**Primary skill for AI agents writing code.** Run this loop every turn.

## When to use

- `workflow.phase` is **`sync_required`** or **`execute`**
- User asks to implement, continue, or complete track TODOs
- After **track-task-setup** is done

## Agent loop (every turn)

```
track status --json  →  follow workflow.next_action
        ↓
track sync (if required)  →  jj verify bookmark
        ↓
track todo workspace N  →  implement + test
        ↓
jj describe  →  track scrap add  →  track todo done N
        ↓
repeat until workflow.phase == task_complete
```

## Step 1 — Read JSON context

```bash
track status --json
```

| Field | Action |
|-------|--------|
| `workflow.next_action.command` | Run this next |
| `workflow.phase` | `sync_required` → sync first; `execute` → work TODO |
| `todos_agent[].is_next` | Current TODO |
| `todos_agent[].allowed_actions` | Only use listed actions |
| `todos_agent[].workspace.lifecycle` | `requested` → sync; `ready` → cd workspace |
| `guardrails` | Never skip sync or use `todo update done` |

WebUI: `GET /api/status` — same fields.

## Step 2 — Sync (mandatory before edits)

```bash
track sync
jj status
jj bookmark list -r @
```

Stop if bookmark is `main` / `master` / `develop` — re-run sync.

## Step 3 — Enter workspace

```bash
cd "$(track todo workspace <index>)"
jj status
```

If no workspace: work in registered repo root after sync moved to task bookmark.

## Step 4 — Implement and test

- Match TODO scope; run project tests/linters
- Describe incrementally:

```bash
jj describe -m "Implement X for TODO #N"
```

## Step 5 — Record decisions

```bash
track scrap add "Chose bcrypt; tests pass at 95% coverage"
```

## Step 6 — Complete TODO

```bash
track todo done <index>
```

**Never** `track todo update <index> done` — that skips JJ merge.

**Never** reopen done/cancelled TODOs — add a new TODO instead.

## Phase reference

| Phase | Your action |
|-------|-------------|
| `setup` | Switch to **track-task-setup** |
| `sync_required` | `track sync` |
| `execute` | Work `is_next` TODO |
| `task_complete` | **track-advanced** (archive/push) |

## Error recovery

| Problem | Fix |
|---------|-----|
| `todo done` fails (dirty workspace) | `jj describe`, retry |
| Workspace missing | `track sync` |
| Wrong bookmark | `track sync`, verify with `jj status` |
| Need to cancel TODO | `track todo update N cancelled` (pending only) |

## Commands

| Command | Purpose |
|---------|---------|
| `track status --json` | Machine-readable context |
| `track sync` | Bookmarks + workspaces |
| `track todo workspace <n>` | Workspace path |
| `track scrap add` | Progress / decisions |
| `track todo done <n>` | Complete + JJ merge |
| `track todo next <n>` | Reorder pending TODOs |

Full walkthrough: [references/execution-workflow.md](references/execution-workflow.md)
