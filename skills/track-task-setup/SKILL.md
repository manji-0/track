---
name: track-task-setup
description: Set up a track task before implementation — create task, description, register repos, add TODOs and links. Assumes agent-skill-jj for jj-task repo init and workspace creation. Use when workflow.phase is setup.
license: MIT
compatibility: Requires track CLI and jj-task (agent-skill-jj)
metadata:
  author: track
  version: 3.0.0
  tags: [track, setup, planning, todo, repo]
---

# Track — Task Setup

Prepare **what** to build. JJ workspaces are created later via **jj-task** (see **track-task-execute** and **`$jj`**).

## When to use

- `workflow.phase` is **`setup`**
- User asks to create, plan, or scaffold a track task

## Outcome checklist

- Task with name (ticket/alias recommended)
- Registered repo(s)
- TODO list (no `--worktree` — use one jj-task workspace per task)
- `track alias set <slug>` when ticket ID is not a good jj-task slug

## Workflow

### 1. Create task

```bash
track new "Implement OAuth" --ticket PROJ-123 --ticket-url https://...
track alias set oauth-login    # optional: overrides jj.slug
```

### 2. Describe scope

```bash
track desc "Acceptance criteria, constraints, links"
```

### 3. Register repository

```bash
track repo add              # current directory (main workspace)
```

### 4. Initialize jj-task (once per repo)

From the **main workspace** (repo root):

```bash
jj git init --colocate    # if needed
jj-task repo init
```

### 5. Add TODOs

```bash
track todo add "Implement token refresh"
track todo add "Add integration tests"
```

Do **not** use `--worktree` for new tasks. One **jj-task** workspace covers all TODOs sequentially.

### 6. Review

```bash
track status --json
```

Check `jj.slug` and `workflow.phase`.

### 7. Hand off

Switch to **track-task-execute** → `jj-task start <jj.slug>`.

## jj.slug derivation

| Priority | Source | Example slug |
|----------|--------|--------------|
| 1 | alias | `oauth-login` |
| 2 | ticket_id | `proj-123` |
| 3 | task id | `task-42` |

## Next step

**track-task-execute** — [setup-workflow.md](references/setup-workflow.md)
