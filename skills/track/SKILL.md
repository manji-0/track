---
name: track
description: Entry point for the track CLI and JJ workspace task management. Use when the user mentions track, todos, workspaces, or task context. Reads workflow.phase and routes to track-task-setup, track-task-execute, or track-advanced.
license: MIT
compatibility: Requires track CLI and jj on PATH
metadata:
  author: track
  version: 2.0.0
  tags: [track, router, task-management, jj]
---

# Track — Skill Router

Lightweight index for the track CLI. **Load a specialized skill** instead of guessing commands.

## Install all track skills

From the track repository root:

```bash
npx skills add ./skills \
  -s track -s track-task-setup -s track-task-execute -s track-advanced \
  -g -a cursor -a claude-code -a codex -y
```

See [../INSTALL.md](../INSTALL.md) for agent paths and troubleshooting.

## Which skill to use?

Run first (every session):

```bash
track status --json
```

Route by `workflow.phase` or user intent:

| Phase / intent | Skill | When |
|----------------|-------|------|
| `setup` | **track-task-setup** | No repos, new task, planning TODOs |
| `sync_required` | **track-task-execute** | Run `track sync` before coding |
| `execute` | **track-task-execute** | Implement TODOs, `todo done` |
| `task_complete` | **track-advanced** | Archive, handoff, push |
| Multi-repo / parallel / hotfix | **track-advanced** | Cross-repo or special patterns |
| User says "create task" | **track-task-setup** | Greenfield setup |
| User says "continue / implement" | **track-task-execute** | Active development |

## Universal guardrails

These apply to **every** track skill:

1. **`track status --json` first** — follow `workflow.next_action`
2. **`track sync` before code changes** — when `guardrails.must_sync_before_code_changes`
3. **Verify JJ bookmark** — `jj status` + `jj bookmark list -r @` (not main/master)
4. **Complete with `track todo done`** — never `todo update … done` (JJ merge)
5. **No reopen** — done/cancelled TODOs stay terminal; add a new TODO instead

## Minimal command map

| Goal | Command |
|------|---------|
| Read context | `track status --json` |
| Sync bookmarks/workspaces | `track sync` |
| Workspace path | `track todo workspace <index>` |
| Record note | `track scrap add "..."` |
| Finish TODO | `track todo done <index>` |
| Full CLI guide | `track llm-help` |

## Skill catalog

| Skill | Responsibility |
|-------|----------------|
| [track-task-setup](../track-task-setup/SKILL.md) | Create task, repos, TODOs, links |
| [track-task-execute](../track-task-execute/SKILL.md) | Agent execution loop (sync → implement → done) |
| [track-advanced](../track-advanced/SKILL.md) | Multi-repo, parallel, hotfix, archive, templates |

## WebUI

```bash
track webui   # http://localhost:3000
```

`GET /api/status` returns the same `workflow` / `todos_agent` / `guardrails` fields as `track status --json`.
