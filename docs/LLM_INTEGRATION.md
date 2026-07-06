# LLM Integration

Track provides **Agent Skills** for task/TODO management. **[agent-skill-jj](https://github.com/manji-0/agent-skill-jj)** provides the **`$jj` skill** for JJ commits and PR workflows. Use both together.

## Quick Start for Agents

```bash
# Track — WHAT to work on
npx skills add manji-0/track \
  -s track -s track-task-setup -s track-task-execute -s track-advanced -g -y

# JJ / PR — HOW to commit
npx skills add manji-0/agent-skill-jj -s jj -g -y

# Read context FIRST
track status --json

# Start workspace (when sync_required)
jj-task start <jj.slug>
cd "$(jj-task path <jj.slug>)"

# Commits / PR via $jj skill — not bare jj describe
```

Full strategy: **[JJ_INTEGRATION.md](JJ_INTEGRATION.md)**

## Install skills

| Method | Command |
|--------|---------|
| **Track skills** | `npx skills add manji-0/track -s track -s track-task-execute -g -y` |
| **JJ skill (required)** | `npx skills add manji-0/agent-skill-jj -s jj -g -y` |
| **jj-task script** | `ln -s .../agent-skill-jj/skills/jj/scripts/jj-task.sh ~/.local/bin/jj-task` |

Full guide: [skills/INSTALL.md](../skills/INSTALL.md)

## Skill catalog

| Skill | Source | Use when |
|-------|--------|----------|
| **track** | track repo | Router — read `workflow.phase` |
| **track-task-setup** | track repo | Create task, repos, TODOs |
| **track-task-execute** | track repo | jj-task workspace + TODO loop |
| **track-advanced** | track repo | Archive, handoff |
| **`jj`** | agent-skill-jj | Commits, squash, PR, prek, push |

## JSON-first workflow

```json
{
  "workflow": { "phase": "sync_required", "next_action": { "command": "jj-task start proj-123" } },
  "jj": {
    "slug": "proj-123",
    "skill": "jj",
    "start_command": "jj-task start proj-123",
    "path_command": "jj-task path proj-123"
  },
  "guardrails": {
    "must_use_jj_skill": true,
    "jj_skill_name": "jj",
    "reopen_forbidden": true,
    "complete_requires_jj_merge": false
  }
}
```

| Phase | Track | JJ (`$jj` + jj-task) |
|-------|-------|----------------------|
| `setup` | `track repo add` | `jj-task repo init` |
| `sync_required` | — | `jj-task start <slug>` |
| `execute` | scrap, `todo done` | squash/commit/push |
| `task_complete` | `track archive` | `jj-task done <slug>` |

## Plugin manifests

Track plugin metadata: `.claude-plugin/`, `.codex-plugin/`, `.agents/plugins/`.  
Validate: `python3 scripts/validate_package.py`

## Resources

- [JJ_INTEGRATION.md](JJ_INTEGRATION.md) — two-layer strategy
- [skills/README.md](../skills/README.md) — skill catalog
- [skills/INSTALL.md](../skills/INSTALL.md) — install guide
- `track llm-help` — CLI reference
