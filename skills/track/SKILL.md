---
name: track
description: Entry point for track CLI task management. Use when the user mentions track, todos, or task context. Assumes agent-skill-jj ($jj) for all JJ commits and PR work. Reads workflow.phase and routes to track-task-setup, track-task-execute, or track-advanced.
license: MIT
compatibility: Requires track CLI, jj, jj-task, and agent-skill-jj ($jj skill) on PATH
metadata:
  author: track
  version: 3.0.0
  tags: [track, router, task-management]
---

# Track — Skill Router

Track manages **what** to work on. **[agent-skill-jj](https://github.com/manji-0/agent-skill-jj) (`$jj`)** manages **how** to commit and open PRs.

## Prerequisites

```bash
# Track skills
npx skills add ./skills \
  -s track -s track-task-setup -s track-task-execute -s track-advanced \
  -g -a cursor -a claude-code -a codex -y

# JJ / PR skill (required)
npx skills add manji-0/agent-skill-jj -s jj -g -y

# jj-task helper
ln -s /path/to/agent-skill-jj/skills/jj/scripts/jj-task.sh ~/.local/bin/jj-task
```

See [../../docs/JJ_INTEGRATION.md](../../docs/JJ_INTEGRATION.md) and [../INSTALL.md](../INSTALL.md).

## Two-layer loop

```
track status --json  →  jj.slug + workflow.next_action
jj-task start <slug> →  cd "$(jj-task path <slug>)"
$jj skill            →  prek, squash/commit, PR, push
track scrap / todo done  →  track DB only
```

## Which skill to use?

| Phase / intent | Skill |
|----------------|-------|
| `setup` | **track-task-setup** |
| `sync_required` | **track-task-execute** → `jj-task start` |
| `execute` | **track-task-execute** → work TODO + `$jj` |
| `task_complete` | **track-advanced** → `$jj` + `track archive` |
| jj / PR / commit | **`$jj`** (agent-skill-jj) — not track |

## Universal guardrails

1. **`track status --json` first** — follow `workflow.next_action` and `jj.slug`
2. **Never feature-work in main workspace** — use `jj-task path <slug>`
3. **All jj commands via `$jj` skill** — squash, commit, push, PR phases
4. **`track todo done`** — marks TODO in track DB (not a substitute for `$jj`)
5. **No reopen** — done/cancelled TODOs stay terminal

## Skill catalog

| Skill | Responsibility |
|-------|----------------|
| [track-task-setup](../track-task-setup/SKILL.md) | Create task, repos, TODOs |
| [track-task-execute](../track-task-execute/SKILL.md) | jj-task workspace + TODO loop |
| [track-advanced](../track-advanced/SKILL.md) | Archive, handoff, multi-repo |
| **`jj`** (external) | Commits, PR, prek, push |
