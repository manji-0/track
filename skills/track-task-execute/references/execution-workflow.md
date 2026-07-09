# Task Execution — Detailed Guide

Complete workflow for working through TODOs and completing tasks.

**Skill:** [track-task-execute](../SKILL.md) · **Setup:** [track-task-setup](../../track-task-setup/SKILL.md) · **Archive:** [track-advanced](../../track-advanced/SKILL.md)

## When to Use

- `workflow.phase` is `sync_required` or `execute`
- Ready to implement changes
- Working through a task's TODO list

## Prerequisites

- Task created with TODOs
- Repos registered
- [agent-skill-jj](https://github.com/manji-0/agent-skill-jj) `$jj` skill installed
- `jj-task` on PATH

## Step-by-Step Workflow

### Step 1: Read JSON Context

```bash
track status --json
```

Follow `workflow.next_action.command` and `workflow.checklist` — do not guess the next step.

Key fields:

| Field | Action |
|-------|--------|
| `workflow.phase` | `sync_required` → start workspace; `execute` → work TODO |
| `workflow.checklist` | Ordered setup/sync steps with `done` flags |
| `workflow.next_action` | Suggested command and reason |
| `jj.slug` | jj-task workspace name |
| `jj.start_command` | Run when `sync_required` |
| `jj.path_command` | cd target when `execute` |
| `todos_agent[].is_next` | Current TODO |
| `todos_agent[].allowed_actions` | Only listed actions (no reopen) |
| `guardrails.must_use_jj_skill` | Load `$jj` for all jj commands |

---

### Step 2: Start jj-task Workspace (sync_required)

From the **main workspace** (repo root):

```bash
jj-task repo init          # once per repo, if not done in setup
jj-task start <jj.slug>
cd "$(jj-task path <jj.slug>)"
```

Multi-repo tasks: repeat `jj-task start` per registered repo (same slug).

Workspace path: `.worktrees/<slug>/` (agent-skill-jj convention).

---

### Step 3: Implement and Test

- Work only inside the jj-task workspace — **not** repo root
- Run project tests / linters
- For **all** jj operations, follow the **`$jj` skill**:
  - Draft phase: `jj squash`, force push OK
  - In review: `jj commit`, append only
  - prek before commit when hook config exists
  - Conventional Commits format

Do **not** use bare `jj describe` as a substitute for `$jj` commit rules.

---

### Step 4: Record Progress

```bash
track scrap add "<note>"
```

---

### Step 5: Complete TODO

```bash
track todo done <index>
```

Marks TODO done in track DB. JJ history stays in the jj-task workspace via `$jj`.

**Never** reopen done/cancelled TODOs — add a new TODO instead.

---

### Step 6: Repeat

Re-run `track status --json` for the next TODO until `workflow.phase` is `task_complete`.

---

## Complete Example

```bash
track status --json
jj-task start proj-123
cd "$(jj-task path proj-123)"
# ... implement, test ...
# $jj skill: squash/commit per PR phase
track scrap add "Completed OAuth flow. Tests passing."
track todo done 2
track status --json
```

---

## Task Completion

When all TODOs are done:

1. `track status --json` — confirm `task_complete`
2. `track scrap list` — review notes
3. Switch to **track-advanced** for `$jj` merge/PR and `track archive`

---

## Troubleshooting

| Problem | Fix |
|---------|-----|
| Unknown slug | `jj-task start <jj.slug>` |
| Wrong directory | `cd "$(jj-task path <jj.slug>)"` |
| Commit/PR questions | Load **`$jj`** skill |
| TODO state | `track status --json` |
| jj-task phase not merged at archive | `$jj` skill to finish PR, `jj-task done`, or `track archive --force` |

## Quick Reference

| Command | Purpose |
|---------|---------|
| `track status --json` | Machine-readable context + checklist |
| `jj-task start <slug>` | Create/open task workspace |
| `jj-task path <slug>` | Print workspace path for `cd` |
| `track scrap add` | Record progress |
| `track todo done <index>` | Mark TODO done in track DB |
