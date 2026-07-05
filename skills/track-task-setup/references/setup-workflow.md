# Task Setup — Detailed Guide

Complete workflow for creating and setting up development tasks.

**Skill:** [track-task-setup](../SKILL.md) · **Next:** [track-task-execute](../../track-task-execute/SKILL.md)

## When to Use

- Starting a new feature or bug fix
- Breaking down a large project into actionable items
- `workflow.phase` is `setup`

## Prerequisites

- `track` CLI installed and initialized

## Step-by-Step Workflow

### Step 1: Create the Task

**Basic creation:**
```bash
track new "<task_name>"
```

**With ticket (recommended):**
```bash
track new "<task_name>" --ticket <TICKET_ID> --ticket-url <URL>
```

**Examples:**
```bash
track new "Add user authentication"

track new "Fix login bug" --ticket PROJ-123 \
  --ticket-url https://jira.company.com/browse/PROJ-123

track new "Dark mode" --ticket GH-456 \
  --ticket-url https://github.com/org/repo/issues/456
```

---

### Step 2: Add Description

```bash
track desc "<detailed_description>"
```

---

### Step 3: Register Repository

```bash
track repo add [path]
track repo add --base develop
```

Required before `track sync` when TODOs use `--worktree`.

---

### Step 4: Add TODOs

```bash
track todo add "<description>"
track todo add "<description>" --worktree
```

Use `--worktree` for isolated or parallel work.

---

### Step 5: Review Setup

```bash
track status --json
```

Verify task, repos, and `todos_agent` list.

---

### Step 6: Sync (optional before handoff)

```bash
track sync
```

Creates task bookmarks and workspaces. Can defer until execution starts.

---

## Common Patterns

### Simple Task (No Workspaces)

```bash
track new "Update documentation"
track desc "Update README and add API examples"
track todo add "Update README with new features"
track todo add "Add API usage examples"
```

### Complex Feature with Workspaces

```bash
track new "Add payment integration" --ticket PROJ-789
track desc "Integrate Stripe payment processing"
track repo add
track todo add "Set up Stripe SDK" --worktree
track todo add "Create payment models" --worktree
track sync
```

---

## Expected Outcome

- Task created and current
- Description documented
- Repository(ies) registered
- Actionable TODO list
- `workflow.phase` moves toward `sync_required` or `execute`

## Next Step

Switch to **track-task-execute** — see [execution-workflow.md](../../track-task-execute/references/execution-workflow.md).

## Quick Reference

| Command | Purpose |
|---------|---------|
| `track new "<name>"` | Create task |
| `track desc "<text>"` | Add description |
| `track repo add [path]` | Register repository |
| `track todo add "<text>" --worktree` | Add TODO with workspace |
| `track status --json` | Verify setup state |
| `track sync` | Create bookmarks/workspaces |
