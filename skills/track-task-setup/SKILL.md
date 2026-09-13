---
name: track-task-setup
description: Set up a track task before implementation — create task, description, register repos, add TODOs and links. Track creates the git/jj workspace on repo add. Use when workflow.phase is setup.
license: MIT
compatibility: Requires track CLI
metadata:
  author: track
  version: 4.0.0
  tags: [track, setup, planning, todo, repo]
---

# Track — Task Setup

Prepare **what** to build. `track repo add` also creates the coding workspace.

## When to use

- `workflow.phase` is **`setup`**
- User asks to create, plan, or scaffold a track task

## Outcome checklist

- Task with name (ticket/alias recommended)
- Registered repo(s) — workspace at `.worktrees/<slug>/`
- TODO list (`--no-workspace` for research TODOs)
- `track alias set <slug>` when ticket ID is not a good workspace slug

## Workflow

### 1. Create task

```bash
track new "Implement OAuth" --ticket PROJ-123 --ticket-url https://...
track alias set oauth-login    # optional: overrides slug
```

### 2. Describe scope

```bash
track desc "Acceptance criteria, constraints, links"
```

### 3. Register repository (creates workspace)

```bash
track repo add              # current directory
```

Follow the stderr `next:` line or JSON `hint.next_command`.

### 4. Add TODOs

```bash
track todo add --json "Implement token refresh"
track todo add --json "Compare providers" --no-workspace
track todo add --json "Add integration tests"
```

One workspace covers all code TODOs sequentially.

### 5. Review

```bash
track status --json
```

Check `hint`, `vcs_mode`, and `workflow.phase`.

### 6. Hand off

Switch to **track-task-execute** → follow `hint.next_command`.

## Slug derivation

| Priority | Source | Example slug |
|----------|--------|--------------|
| 1 | alias | `oauth-login` |
| 2 | ticket_id | `proj-123` |
| 3 | task id | `task-42` |

PR head is always `track/<slug>`.

## Next step

**track-task-execute** — [setup-workflow.md](references/setup-workflow.md)
