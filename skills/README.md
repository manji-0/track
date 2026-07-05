# Track CLI Skills for LLM Agents

Official [Agent Skills](https://github.com/anthropics/skills) for the `track` CLI — installable via **[Skills CLI](https://github.com/vercel-labs/skills)** (`npx skills`), Cursor, Claude Code, and Codex.

## Quick install

```bash
# From track repo root — project scope (team-shared)
npx skills add ./skills/task-management -y

# Global + specific agents
npx skills add ./skills/task-management \
  -g -a cursor -a claude-code -a codex -y
```

See **[INSTALL.md](INSTALL.md)** for GitHub installs, agent paths, verification, and troubleshooting.

## Skill: track-task-management

| | |
|---|---|
| **Path** | `skills/task-management/SKILL.md` |
| **Name** | `track-task-management` |
| **Use when** | Creating tasks, TODOs, JJ workspaces, or user mentions `track` |

### What it covers

- Task / TODO management (task-scoped indices)
- JJ bookmark and workspace workflows
- Ticket integration (Jira, GitHub, GitLab)
- Markdown scraps, links, templates, aliases
- **JSON-first agent workflow** via `track status --json` and `/api/status`

### Agent loop (summary)

1. `track status --json` — read `workflow.phase` and `next_action`
2. `track sync` — before any code changes
3. `jj status` — verify task bookmark
4. Execute TODO → `track scrap add` → `track todo done <index>`
5. Repeat until `workflow.phase` is `task_complete`

Full detail: `SKILL.md` and `references/executing-tasks.md`.

## Directory layout

```
skills/
├── README.md                 # This file
├── INSTALL.md                # npx skills + Cursor / Claude / Codex setup
└── task-management/
    ├── SKILL.md              # Main skill (~500 lines max, YAML frontmatter)
    └── references/
        ├── creating-tasks.md
        ├── executing-tasks.md
        └── advanced-workflows.md
```

## Supported agents

| Agent | Install flag | Typical path |
|-------|--------------|--------------|
| Cursor | `-a cursor` | `.agents/skills/` or `~/.cursor/skills/` |
| Claude Code | `-a claude-code` | `.claude/skills/` or `~/.claude/skills/` |
| Codex | `-a codex` | `.agents/skills/` or `~/.codex/skills/` |

Browse more agents: [skills.sh](https://skills.sh) · `npx skills find task`

## Without installing

Agents with repo access can read `skills/task-management/SKILL.md` directly. Installing via `npx skills` is recommended when working outside the track repository.

## Resources

- **Install guide**: [INSTALL.md](INSTALL.md)
- **CLI reference**: `track llm-help`
- **Integration overview**: [docs/LLM_INTEGRATION.md](../docs/LLM_INTEGRATION.md)
- **Agent Skills spec**: https://github.com/anthropics/skills

## License

MIT (same as track)
