---
name: track
description: Entry point for track CLI task management. Use when the user mentions track, todos, or task context. Track owns git/jj workspaces. Read workflow.phase, hint, and next_action; route to track-task-setup, track-task-execute, or track-advanced.
license: MIT
compatibility: Requires track CLI on PATH (git and/or jj depending on vcs-mode)
metadata:
  author: track
  version: 4.0.0
  tags: [track, router, task-management]
---

# Track — Skill Router

Track manages **what** to work on and **where** the coding workspace lives. Follow command output (`hint` / `workflow.next_action`) instead of running `git worktree`, `jj workspace`, or jj-task by hand.

## Prerequisites

```bash
npx skills add ./skills \
  -s track -s track-task-setup -s track-task-execute -s track-advanced \
  -g -a cursor -a claude-code -a codex -y
```

See [../../docs/JJ_INTEGRATION.md](../../docs/JJ_INTEGRATION.md) and [../INSTALL.md](../INSTALL.md).

## Loop

```
track status --json  →  hint + workflow.next_action
track repo add / track sync  →  .worktrees/<slug>/ on track/<slug>
cd "<workspace_path>"  →  implement (not repo root)
track scrap / todo done  →  track DB (one commit + notes per TODO when aggressive)
```

Human commands print the same next step on stderr (`hint:` / `next:`). `--json` includes `hint`. `TRACK_HINTS=0` hides the footer.

## Which skill to use?

| Phase / intent | Skill |
|----------------|-------|
| `setup` | **track-task-setup** |
| `sync_required` | **track-task-execute** → `track sync` |
| `execute` | **track-task-execute** → work TODO in the workspace |
| `task_complete` | **track-advanced** → push `track/<slug>`, `track archive` |

## Universal guardrails

1. **`track status --json` first** — follow `hint`, `workflow.next_action`, `workflow.checklist`
2. **Never feature-work in the main checkout** — use `.worktrees/<slug>/`
3. **Do not invoke jj-task** — track creates workspaces
4. **`track todo done`** — marks TODO in track DB (not a substitute for git/jj commit)
5. **No reopen** — done/cancelled TODOs stay terminal
6. **Never wait on confirmation prompts** — stdin is not a TTY; confirmation fails
7. **`track todo delete N --force`** — always `--force` (do not prompt)
8. **`track archive` without `--force`** — if it errors, follow the hint. `--force` only when the user explicitly skips dirty checks
9. **Prefer `--json` on writes** — skip a second status call when you already have the snapshot

## Skill catalog

| Skill | Responsibility |
|-------|----------------|
| [track-task-setup](../track-task-setup/SKILL.md) | Create task, repos, TODOs |
| [track-task-execute](../track-task-execute/SKILL.md) | Workspace + TODO loop |
| [track-advanced](../track-advanced/SKILL.md) | Archive, handoff, multi-repo |
