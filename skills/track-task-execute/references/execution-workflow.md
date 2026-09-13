# Task Execution — Detailed Guide

Complete workflow for working through TODOs and completing tasks.

**Skill:** [track-task-execute](../SKILL.md) · **Setup:** [track-task-setup](../../track-task-setup/SKILL.md) · **Archive:** [track-advanced](../../track-advanced/SKILL.md)

## When to Use

- `workflow.phase` is `sync_required` or `execute`
- Ready to implement changes
- Working through a task's TODO list

## Prerequisites

- Task created with TODOs
- Repos registered (`track repo add`)

## Step-by-Step Workflow

### Step 1: Read JSON Context

```bash
track status --json
```

Follow `hint.next_command` and `workflow.checklist` — do not guess the next step.

Key fields:

| Field | Action |
|-------|--------|
| `workflow.phase` | `sync_required` → `track sync`; `execute` → work TODO |
| `hint.next_command` | Exact next command |
| `workflow.checklist` | Ordered setup/sync steps with `done` flags |
| `git.slug` / `jj.slug` | Workspace name |
| `todos_agent[].is_next` | Current TODO |
| `todos_agent[].allowed_actions` | Only listed actions (no reopen) |

---

### Step 2: Ensure Workspace (sync_required)

```bash
track sync
cd "<repo>/.worktrees/<slug>"
```

Multi-repo tasks: `track sync` creates a workspace in each registered repo (same slug).

Workspace path: `.worktrees/<slug>/`. PR head: `track/<slug>`.

---

### Step 3: Implement and Test

- Work only inside the task workspace — **not** repo root
- Run project tests / linters
- Git: commit in the worktree; `git push -u origin track/<slug>`
- JJ: commit in the workspace; `jj git push --named track/<slug>`

---

### Step 4: Record Progress

```bash
track scrap add --json "<note>"
```

---

### Step 5: Complete TODO

```bash
track todo done --json <index>
```

Marks TODO done in track DB. The JSON response includes `workflow.next_action` for the next TODO.

**Never** reopen done/cancelled TODOs — add a new TODO instead.

---

### Step 6: Repeat

If the mutation JSON already includes `hint` / `workflow.next_action`, follow it. Otherwise re-run `track status --json` until `workflow.phase` is `task_complete`.

---

## Complete Example

```bash
track status --json
track sync
cd /repo/.worktrees/proj-123
# ... implement, test, commit ...
track scrap add --json "Completed OAuth flow. Tests passing."
track todo done --json 2
```

---

## Task Completion

When all TODOs are done:

1. `track status --json` — confirm `task_complete`
2. `track scrap list` — review notes
3. Switch to **track-advanced** for push/merge and `track archive`

---

## Troubleshooting

| Problem | Fix |
|---------|-----|
| Missing workspace | `track sync` |
| Wrong directory | `cd` the path from `hint` / JSON |
| TODO state | `track status --json` |
| Archive blocked (dirty) | Commit or discard, then archive. `--force` only if the user asks. |

## Quick Reference

| Command | Purpose |
|---------|---------|
| `track status --json` | Machine-readable context + hint |
| `track sync` | Create/open task workspace |
| `track scrap add` | Record progress |
| `track todo done <index>` | Mark TODO done in track DB |
