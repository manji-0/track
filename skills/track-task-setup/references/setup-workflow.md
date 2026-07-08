# Task Setup — Detailed Guide

Complete workflow for creating and setting up development tasks.

**Skill:** [track-task-setup](../SKILL.md) · **Next:** [track-task-execute](../../track-task-execute/SKILL.md)

## When to Use

- Starting a new feature or bug fix
- Breaking down a large project into actionable items
- `workflow.phase` is `setup`

## Prerequisites

- `track` CLI installed and initialized
- [agent-skill-jj](https://github.com/manji-0/agent-skill-jj) with `jj-task` on PATH

## Step-by-Step Workflow

### Step 1: Create the Task

**Basic creation:**
```bash
track new "<task_name>"
```

**With ticket (recommended):**
```bash
track new "<task_name>" --ticket <TICKET_ID> --ticket-url <URL>
track alias set <slug>    # optional: overrides jj.slug when ticket ID is awkward
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

Register every repo that will participate in the task (multi-repo tasks repeat this).

---

### Step 4: Initialize jj-task (once per repo)

From the **main workspace** (repo root):

```bash
jj git init --colocate    # if needed
jj-task repo init
```

---

### Step 5: Add TODOs

```bash
track todo add "<description>"
track todo add "Compare providers" --no-workspace   # research / planning only
```

One **jj-task** workspace covers all code TODOs for the task. Use `--no-workspace` when the TODO does not need a jj workspace (research, docs-only planning).

---

### Step 6: Review Setup

```bash
track status --json
```

Verify task, repos, `todos_agent`, `jj.slug`, and `workflow.checklist`.

---

## Common Patterns

### Simple Task (docs / planning)

```bash
track new "Update documentation"
track desc "Update README and add API examples"
track todo add "Update README with new features" --no-workspace
track todo add "Add API usage examples" --no-workspace
```

### Feature with Code Changes

```bash
track new "Add payment integration" --ticket PROJ-789
track desc "Integrate Stripe payment processing"
track repo add
jj-task repo init
track todo add "Set up Stripe SDK"
track todo add "Create payment models"
track todo add "Add integration tests"
```

---

## Expected Outcome

- Task created and current
- Description documented
- Repository(ies) registered
- Actionable TODO list
- `workflow.phase` moves toward `sync_required` or `execute`

## Next Step

Switch to **track-task-execute** — `jj-task start <jj.slug>` — see [execution-workflow.md](../../track-task-execute/references/execution-workflow.md).

## Quick Reference

| Command | Purpose |
|---------|---------|
| `track new "<name>"` | Create task |
| `track desc "<text>"` | Add description |
| `track repo add [path]` | Register repository |
| `track todo add "<text>"` | Add code TODO |
| `track todo add "<text>" --no-workspace` | Add research/planning TODO |
| `track status --json` | Verify setup state |
| `jj-task repo init` | Register repo with jj-task (once per repo) |
