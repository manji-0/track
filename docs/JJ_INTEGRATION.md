# JJ Integration Strategy

Track and [agent-skill-jj](https://github.com/manji-0/agent-skill-jj) form a **two-layer agent stack**:

| Layer | Tool / skill | Responsibility |
|-------|--------------|----------------|
| **Task** | `track` + track skills | WHAT to work on — tasks, TODOs, scraps, tickets, JSON workflow |
| **JJ / PR** | `$jj` + `jj-task` | HOW to commit — workspaces, squash, two-phase PR, prek, push |

Install both:

```bash
npx skills add manji-0/track \
  -s track -s track-task-setup -s track-task-execute -s track-advanced -g -y

npx skills add manji-0/agent-skill-jj -s jj -g -y

ln -s "$(pwd)/../agent-skill-jj/skills/jj/scripts/jj-task.sh" ~/.local/bin/jj-task
```

## Agent loop (combined)

```
track status --json     →  workflow.phase + jj.slug + next_action
        ↓
jj-task start <slug>    →  workspace at .worktrees/<slug>/  (once per task)
        ↓
cd "$(jj-task path <slug>)"  →  implement in task workspace (not main)
        ↓
$jj skill              →  prek, jj squash/commit, two-phase PR, push
        ↓
track scrap add       →  record decisions in track DB
        ↓
track todo done N     →  mark TODO complete (status only)
        ↓
repeat until task_complete
        ↓
$jj skill + jj-task done <slug> + track archive
```

## Division of responsibility

### Track owns

- Task / TODO lifecycle (`track new`, `track todo add/done`)
- Scraps, links, tickets, aliases
- `track status --json` / `GET /api/status` — `workflow`, `jj`, `todos_agent`, `guardrails`
- WebUI
- `track archive` (task-level cleanup)

### agent-skill-jj (`$jj`) owns

- **Main workspace = sync only** — no feature edits at repo root
- **Task workspace** — `.worktrees/<slug>/` via `jj-task start`
- **Global map** — `~/.config/jj/task-workspaces.json`
- **Commits** — Conventional Commits, `jj squash` (draft), `jj commit` (in review)
- **prek** before `jj commit` when hook config exists
- **PR phases** — draft vs in-review, force-push rules
- **Push** — `jj bookmark move`, `jj git push`, `gh pr`

### jj-task slug

Track derives `jj.slug` from the current task:

1. `track alias` if set
2. else `ticket_id` (sanitized, e.g. `PROJ-123` → `proj-123`)
3. else `task-{id}`

Set an alias when the ticket ID is not a good workspace slug:

```bash
track alias set fix-oauth-refresh
```

## JSON fields (`track status --json`)

```json
{
  "workflow": {
    "phase": "sync_required",
    "next_action": {
      "command": "jj-task start proj-123",
      "reason": "Start a jj-task workspace..."
    }
  },
  "jj": {
    "slug": "proj-123",
    "skill": "jj",
    "workspace_registered": false,
    "workspace_path": "/repo/.worktrees/proj-123",
    "start_command": "jj-task start proj-123",
    "path_command": "jj-task path proj-123",
    "repo_init_command": "jj-task repo init"
  },
  "guardrails": {
    "must_use_jj_skill": true,
    "jj_skill_name": "jj",
    "reopen_forbidden": true,
    "complete_requires_jj_merge": false
  }
}
```

| Phase | Track action | JJ action |
|-------|--------------|-----------|
| `setup` | `track repo add` | `jj-task repo init` (once) |
| `sync_required` | — | `jj-task start <slug>` |
| `execute` | `track scrap add`, `track todo done` | `$jj` for all jj commands |
| `task_complete` | `track archive` | `jj-task done <slug>`, PR merge via `$jj` |

## Legacy: per-TODO `--worktree`

`track todo add --worktree` still creates track-managed workspaces (older model).  
**New tasks should not use `--worktree`.** Use one jj-task workspace per track task and sequential TODOs.

When `--worktree` is used, `guardrails.complete_requires_jj_merge` is `true` and `track sync` / `track todo done` handle JJ merge.

## What changed from track-only JJ docs

| Old (track-only) | New (jj-first) |
|------------------|----------------|
| `track sync` before coding | `jj-task start <slug>` |
| Work in repo root after sync | Work in `.worktrees/<slug>/` only |
| `jj describe` before `todo done` | `$jj` skill: squash/commit per PR phase |
| `task/PROJ-123` bookmark at root | `<slug>` bookmark in task workspace |
| Per-TODO workspaces | One workspace per track task |

## References

- [agent-skill-jj](https://github.com/manji-0/agent-skill-jj) — `$jj` skill, `jj-task` script
- [skills/INSTALL.md](../skills/INSTALL.md) — install both skill packs
- [LLM_INTEGRATION.md](LLM_INTEGRATION.md) — agent overview
