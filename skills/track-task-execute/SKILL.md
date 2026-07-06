---
name: track-task-execute
description: Execute track TODOs with jj-task workspaces and agent-skill-jj ($jj) for commits. Read track status --json, start jj-task workspace, implement, use $jj for jj operations, track scrap/done for task state. Use when workflow.phase is sync_required or execute.
license: MIT
compatibility: Requires track CLI, jj-task, and agent-skill-jj ($jj skill)
metadata:
  author: track
  version: 3.0.0
  tags: [track, execute, agent, jj-task, todo]
---

# Track — Task Execution (Agents)

**Track** = TODO state. **`$jj`** = all jj commit/PR operations.

## Prerequisites

- [agent-skill-jj](https://github.com/manji-0/agent-skill-jj) `$jj` skill installed
- `jj-task` on PATH

## Agent loop (every turn)

```
track status --json
        ↓
jj-task start <jj.slug>   (if sync_required)
        ↓
cd "$(jj-task path <jj.slug>)"
        ↓
implement + test
        ↓
$jj skill → prek, jj squash/commit, push (per PR phase)
        ↓
track scrap add "..."
        ↓
track todo done <index>
        ↓
repeat until task_complete
```

## Step 1 — Read JSON

```bash
track status --json
```

| Field | Action |
|-------|--------|
| `jj.slug` | jj-task workspace name |
| `jj.start_command` | Run when `sync_required` |
| `jj.path_command` | cd target when `execute` |
| `workflow.next_action` | Always follow this |
| `guardrails.must_use_jj_skill` | Load `$jj` for jj commands |

## Step 2 — Start workspace (sync_required)

From **main workspace** (repo root):

```bash
jj-task repo init          # once per repo
jj-task start <jj.slug>
cd "$(jj-task path <jj.slug>)"
```

Workspace path: `.worktrees/<slug>/` (agent-skill-jj convention).

## Step 3 — Implement

- Work only inside the jj-task workspace — **not** repo root
- Run project tests/linters
- For **all** jj operations, follow **`$jj` skill**:
  - Draft phase: `jj squash`, force push OK
  - In review: `jj commit`, append only
  - prek before commit when hook config exists
  - Conventional Commits format

Do **not** use bare `jj describe` as a substitute for `$jj` commit rules.

## Step 4 — Record in track

```bash
track scrap add "Chose bcrypt; tests at 95%"
```

## Step 5 — Complete TODO (track DB)

```bash
track todo done <index>
```

Marks TODO done in track. JJ history stays in the jj-task workspace via `$jj`.

## Step 6 — Repeat

Re-run `track status --json` for the next TODO.

## Legacy `--worktree`

If a TODO used `--worktree`, `guardrails.complete_requires_jj_merge` is true — use `track sync` and `track todo workspace` instead of jj-task. Prefer jj-task for new work.

## Error recovery

| Problem | Fix |
|---------|-----|
| Unknown slug | `jj-task start <jj.slug>` |
| Wrong directory | `cd "$(jj-task path <jj.slug>)"` |
| Commit/PR questions | Load **`$jj`** skill |
| TODO state | `track status --json` |

Full walkthrough: [references/execution-workflow.md](references/execution-workflow.md)
