# LLM Integration

Track is a **personal work-context manager**. It also **owns the coding workspace**. Follow `track status --json` (`hint` / `workflow.next_action`) rather than jj-task or ad-hoc `git worktree` commands. Mutations that accept `--json` return the same snapshot as `track status --json` plus `ok` / `mutation`.

## Quick Start for Agents

```bash
# Optional thin skills (they tell you to follow hint / next_action)
npx skills add manji-0/track \
  -s track -s track-task-setup -s track-task-execute -s track-advanced -g -y

# Read context FIRST
track status --json

# Create workspace when sync_required / after repo add
track sync
# then: cd "<git.workspace_path or jj.workspace_path>"

# Human commands also print a stderr footer:
#   hint: vcs=git aggressive=off | workspace ready at ...
#   next: cd "..."
```

Full strategy (workspaces, git vs jj, commit + notes): **[JJ_INTEGRATION.md](JJ_INTEGRATION.md)**

## Install skills

Skills are optional. Prefer `track llm-help` and the JSON `hint` object.

| Method | Command |
|--------|---------|
| **Track skills** | `npx skills add manji-0/track -s track -s track-task-execute -g -y` |

Full guide: [skills/INSTALL.md](../skills/INSTALL.md)

## Skill catalog

| Skill | Source | Use when |
|-------|--------|----------|
| **track** | track repo | Router — read `workflow.phase` and `hint` |
| **track-task-setup** | track repo | Create task, repos, TODOs |
| **track-task-execute** | track repo | `track sync` + TODO loop |
| **track-advanced** | track repo | Archive, handoff |

## JSON-first workflow

```json
{
  "vcs_mode": "git",
  "aggressive": false,
  "workflow": { "phase": "sync_required", "next_action": { "command": "track sync" } },
  "hint": {
    "post_state": "workspace missing at /repo/.worktrees/proj-123 — run track sync",
    "next_command": "track sync"
  },
  "git": {
    "slug": "proj-123",
    "branch": "track/proj-123",
    "sync_command": "track sync"
  },
  "guardrails": {
    "must_use_jj_skill": false,
    "reopen_forbidden": true
  }
}
```

| Phase | Track |
|-------|-------|
| `setup` | `track repo add`, `track todo add` |
| `sync_required` | `track sync` |
| `execute` | `cd` workspace, scrap, `todo done` |
| `task_complete` | push `track/<slug>`, `track archive` |

## Plugin manifests

Track plugin metadata: `.claude-plugin/`, `.codex-plugin/`, `.agents/plugins/`.  
Validate: `python3 scripts/validate_package.py`

## Resources

- [JJ_INTEGRATION.md](JJ_INTEGRATION.md) — git / jj workspaces, commit + notes
- [skills/README.md](../skills/README.md) — skill catalog
- [skills/INSTALL.md](../skills/INSTALL.md) — install guide
- `track llm-help` — CLI reference
