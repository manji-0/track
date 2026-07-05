# Task Execution — Detailed Guide

Complete workflow for working through TODOs and completing tasks.

**Skill:** [track-task-execute](../SKILL.md) · **Setup:** [track-task-setup](../../track-task-setup/SKILL.md) · **Archive:** [track-advanced](../../track-advanced/SKILL.md)

## When to Use

- `workflow.phase` is `sync_required` or `execute`
- Ready to implement changes
- Working through a task's TODO list

## Prerequisites

- Task created with TODOs
- Repos registered (if using workspaces)

## Step-by-Step Workflow

### Step 1: Read JSON Context

```bash
track status --json
```

Follow `workflow.next_action.command` — do not guess the next step.

Key fields:

| Field | Action |
|-------|--------|
| `workflow.phase` | `sync_required` → sync; `execute` → work TODO |
| `todos_agent[].is_next` | Current TODO |
| `todos_agent[].allowed_actions` | Only listed actions (no reopen) |
| `guardrails.must_sync_before_code_changes` | Run `track sync` before edits |

---

### Step 2: Sync and Verify Bookmark

```bash
track sync
jj status
jj bookmark list -r @
```

Stop if bookmark is `main` / `master` / `develop`.

---

### Step 3: Navigate to Workspace

```bash
cd "$(track todo workspace <index>)"
jj status
```

---

### Step 4: Implement and Test

```bash
jj describe -m "<summary>"
# run project tests / linters
```

---

### Step 5: Record Progress

```bash
track scrap add "<note>"
```

---

### Step 6: Complete TODO

```bash
track todo done <index>
```

**Never** `track todo update <index> done` — that skips JJ merge.

**Never** reopen done/cancelled TODOs — add a new TODO instead.

---

### Step 7: Repeat

Re-run `track status --json` until `workflow.phase` is `task_complete`.

---

## Complete Example

```bash
track status --json
track sync
cd "$(track todo workspace 2)"
# ... implement, test ...
jj describe -m "Implement Google OAuth flow"
track scrap add "Completed Google OAuth. Tests passing."
track todo done 2
track status --json
```

---

## Task Completion

When all TODOs are done:

1. `track status --json` — confirm `task_complete`
2. `track scrap list` — review notes
3. `jj git push --bookmark task/PROJ-123`
4. Switch to **track-advanced** for archive/handoff

---

## Troubleshooting

| Problem | Fix |
|---------|-----|
| `todo done` fails (dirty workspace) | `jj describe`, retry |
| Workspace missing | `track sync` |
| Wrong bookmark | `track sync`, verify with `jj status` |

## Quick Reference

| Command | Purpose |
|---------|---------|
| `track status --json` | Machine-readable context |
| `track sync` | Bookmarks + workspaces |
| `track scrap add` | Record progress |
| `track todo done <index>` | Complete + JJ merge |
| `jj describe -m "..."` | Describe changes |
