---
name: track-task-setup
description: Set up a track task before implementation — create task, description, register repos, add TODOs and links, optional initial sync. Use when workflow.phase is setup, or the user asks to create, plan, or scaffold a task.
license: MIT
compatibility: Requires track CLI installed
metadata:
  author: track
  version: 2.0.0
  tags: [track, setup, planning, todo, repo]
---

# Track — Task Setup

Use this skill when **designing work**, not when implementing it. Typical actor: human or agent preparing a handoff.

## When to use

- `workflow.phase` is **`setup`** (no repos registered, or task empty)
- User asks: "create a track task", "break this into TODOs", "register the repo"
- Before **`track-task-execute`** — setup must be complete first

## Outcome checklist

When setup is done, `track status --json` should show:

- Task with name (and ideally description + ticket)
- At least one registered repo (if code changes expected)
- Actionable TODO list
- `workflow.phase` moves toward `sync_required` or `execute`

## Workflow (ordered)

### 1. Create task

```bash
track new "<name>"
track new "<name>" --ticket PROJ-123 --ticket-url https://...
track new "<name>" --template t:PROJ-100   # copy TODOs from template
```

### 2. Describe scope

```bash
track desc "What to build, constraints, acceptance criteria"
```

### 3. Register repositories

```bash
track repo add              # current directory
track repo add /path/to/app
track repo add --base develop
```

Required before `track sync` if TODOs use `--worktree`.

### 4. Add TODOs

```bash
track todo add "Implement API endpoint"
track todo add "Refactor auth module" --worktree   # isolated JJ workspace
```

Use `--worktree` when:

- Changes need isolation before merge to task bookmark
- Parallel work on independent TODOs is planned

Skip `--worktree` for docs-only or research TODOs.

### 5. Add links (optional)

```bash
track link add https://github.com/org/repo/pull/42 --title "Design PR"
```

### 6. Review

```bash
track status --json
```

Confirm `todos_agent` list and that repos appear.

### 7. Hand off to execution (optional)

```bash
track sync   # creates task bookmark + workspaces; run before agent codes
```

## Common patterns

**Simple (no workspaces):**

```bash
track new "Update docs"
track todo add "Update README"
track todo add "Add examples"
```

**Feature with workspaces:**

```bash
track new "OAuth login" --ticket PROJ-123
track desc "Google + GitHub providers"
track repo add
track todo add "OAuth client setup" --worktree
track todo add "Callback handlers" --worktree
track todo add "Integration tests"
```

## Task-scoped indices

TODO / link / repo numbers (`#1`, `#2`) reset per task — not global.

## Next step

When setup is complete → switch to **track-task-execute** skill.

Detailed walkthrough: [references/setup-workflow.md](references/setup-workflow.md)

## Commands

| Command | Purpose |
|---------|---------|
| `track new` | Create and switch to task |
| `track desc` | Set description |
| `track repo add` | Register JJ repo |
| `track todo add` | Add TODO |
| `track link add` | Add reference URL |
| `track status --json` | Verify setup state |
