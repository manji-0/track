# LLM Integration

Track provides **Agent Skills** following the [official Agent Skills specification](https://github.com/anthropics/skills), installable via **[Skills CLI](https://github.com/vercel-labs/skills)** (`npx skills`).

## Quick Start for Agents

```bash
# Install skill (from track repo root)
npx skills add ./skills/task-management -g -a cursor -a claude-code -a codex -y

# Read machine-readable context FIRST
track status --json

# Mandatory before code changes
track sync
jj status
jj bookmark list -r @
```

## Install the skill

| Method | Command |
|--------|---------|
| **Project** (team-shared) | `npx skills add ./skills/task-management -y` |
| **Global** (all repos) | `npx skills add ./skills/task-management -g -y` |
| **Specific agents** | `npx skills add ./skills/task-management -a cursor -a claude-code -a codex -y` |
| **From GitHub** | `npx skills add manji-0/track --skill track-task-management -g -y` |

Agent install paths:

| Agent | `--agent` | Global path |
|-------|-----------|-------------|
| Cursor | `cursor` | `~/.cursor/skills/` |
| Claude Code | `claude-code` | `~/.claude/skills/` |
| Codex | `codex` | `~/.codex/skills/` |

Full guide: [skills/INSTALL.md](../skills/INSTALL.md)

## Main skill: track-task-management

**Path:** `skills/task-management/SKILL.md`

**Use when:** Creating tasks, adding TODOs, working through task lists, or managing development workflows.

## JSON-first workflow

Agents should call `track status --json` (or `GET /api/status` in WebUI) every turn:

```json
{
  "workflow": {
    "phase": "sync_required",
    "next_action": { "command": "track sync", "reason": "..." }
  },
  "todos_agent": [{ "todo_id": 1, "is_next": true, "allowed_actions": ["complete", "cancel"] }],
  "guardrails": { "must_sync_before_code_changes": true, "reopen_forbidden": true }
}
```

| Phase | Action |
|-------|--------|
| `setup` | Register repos / add TODOs |
| `sync_required` | Run `track sync` |
| `execute` | Work on `is_next` TODO |
| `task_complete` | Consider `track archive` |

## Detailed references

| Reference | When to Use |
|-----------|-------------|
| [creating-tasks.md](../skills/task-management/references/creating-tasks.md) | Setting up new tasks |
| [executing-tasks.md](../skills/task-management/references/executing-tasks.md) | Working through TODOs |
| [advanced-workflows.md](../skills/task-management/references/advanced-workflows.md) | Multi-repo, parallel work |

## LLM Help Command

```bash
track llm-help
```

Outputs the full agent workflow guide including JSON fields and `npx skills` install hints.

## Discover more skills

```bash
npx skills find task
npx skills list
```

Browse: [skills.sh](https://skills.sh)

## Resources

- [skills/README.md](../skills/README.md) — skill overview
- [skills/INSTALL.md](../skills/INSTALL.md) — install for Cursor / Claude Code / Codex
- [LLM_HELP_DESIGN.md](LLM_HELP_DESIGN.md) — `llm-help` command design
