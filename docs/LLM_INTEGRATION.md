# LLM Integration

Track provides **Agent Skills** following the [official Agent Skills specification](https://github.com/anthropics/skills), split by use case and installable via **[Skills CLI](https://github.com/vercel-labs/skills)** (`npx skills`).

## Quick Start for Agents

```bash
# Install skills (from track repo root)
npx skills add ./skills \
  -s track -s track-task-setup -s track-task-execute -s track-advanced \
  -g -a cursor -a claude-code -a codex -y

# Read machine-readable context FIRST
track status --json

# Mandatory before code changes
track sync
jj status
jj bookmark list -r @
```

## Install skills

| Method | Command |
|--------|---------|
| **All skills (recommended)** | `npx skills add ./skills -s track -s track-task-setup -s track-task-execute -s track-advanced -g -y` |
| **Router + execute only** | `npx skills add ./skills -s track -s track-task-execute -a cursor -y` |
| **From GitHub** | `npx skills add manji-0/track -s track -s track-task-execute -g -y` |

Agent install paths:

| Agent | `--agent` | Global path |
|-------|-----------|-------------|
| Cursor | `cursor` | `~/.cursor/skills/` |
| Claude Code | `claude-code` | `~/.claude/skills/` |
| Codex | `codex` | `~/.codex/skills/` |

Full guide: [skills/INSTALL.md](../skills/INSTALL.md)

## Plugin manifests

Track ships Claude Code and Codex plugin metadata (same pattern as [kamae-rs](https://github.com/manji-0/kamae-rs)):

| File | Agent |
|------|-------|
| `.claude-plugin/plugin.json` | Claude Code |
| `.codex-plugin/plugin.json` | Codex |
| `.agents/plugins/marketplace.json` | Cursor / Agents |
| `skills/*/agents/openai.yaml` | Per-skill Codex hints |

Validate: `python3 scripts/validate_package.py`

## Skill catalog

| Skill | Path | Use when |
|-------|------|----------|
| **track** | `skills/track/SKILL.md` | Router — read `workflow.phase`, pick specialized skill |
| **track-task-setup** | `skills/track-task-setup/SKILL.md` | Create task, repos, TODOs (`setup`) |
| **track-task-execute** | `skills/track-task-execute/SKILL.md` | Agent coding loop (`sync_required`, `execute`) |
| **track-advanced** | `skills/track-advanced/SKILL.md` | Multi-repo, parallel, hotfix, archive |

Legacy **`track-task-management`** (`skills/task-management/`) is deprecated.

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

| Phase | Skill | Action |
|-------|-------|--------|
| `setup` | track-task-setup | Register repos / add TODOs |
| `sync_required` | track-task-execute | Run `track sync` |
| `execute` | track-task-execute | Work on `is_next` TODO |
| `task_complete` | track-advanced | Archive / push / handoff |

## Detailed references

| Reference | Skill |
|-----------|-------|
| [setup-workflow.md](../skills/track-task-setup/references/setup-workflow.md) | track-task-setup |
| [execution-workflow.md](../skills/track-task-execute/references/execution-workflow.md) | track-task-execute |
| [advanced-patterns.md](../skills/track-advanced/references/advanced-patterns.md) | track-advanced |

## LLM Help Command

```bash
track llm-help
```

Outputs the full agent workflow guide including JSON fields and `npx skills` install hints.

## Discover more skills

```bash
npx skills find track
npx skills list
```

Browse: [skills.sh](https://skills.sh)

## Resources

- [skills/README.md](../skills/README.md) — skill catalog and routing
- [skills/INSTALL.md](../skills/INSTALL.md) — install for Cursor / Claude Code / Codex
- [LLM_HELP_DESIGN.md](LLM_HELP_DESIGN.md) — `llm-help` command design
