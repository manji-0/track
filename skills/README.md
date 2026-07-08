# Track CLI Skills for LLM Agents

Official [Agent Skills](https://github.com/anthropics/skills) for the `track` CLI — split by **use case** and installable via **[Skills CLI](https://github.com/vercel-labs/skills)** (`npx skills`), Cursor, Claude Code, and Codex.

## Quick install (all skills)

From the track repository root:

```bash
npx skills add ./skills \
  -s track -s track-task-setup -s track-task-execute -s track-advanced \
  -g -a cursor -a claude-code -a codex -y
```

Install only what you need:

```bash
# Router + execution (most agents)
npx skills add ./skills -s track -s track-task-execute -a cursor -y

# Planning / scaffolding only
npx skills add ./skills -s track-task-setup -y
```

See **[INSTALL.md](INSTALL.md)** for GitHub installs, agent paths, verification, and troubleshooting.

## Plugin manifests (Claude Code / Codex)

Like [kamae-rs](https://github.com/manji-0/kamae-rs), track ships plugin metadata at the repository root:

| Path | Purpose |
|------|---------|
| `.claude-plugin/plugin.json` | Claude Code plugin package |
| `.claude-plugin/marketplace.json` | Claude marketplace entry |
| `.codex-plugin/plugin.json` | Codex plugin + interface metadata |
| `.agents/plugins/marketplace.json` | Agents/Cursor marketplace entry |

Each skill also has `agents/openai.yaml` for Codex-style interface hints.

Validate before publishing:

```bash
python3 scripts/validate_package.py
```

## Skill catalog

| Skill | Use when | Path |
|-------|----------|------|
| **track** | User mentions track; read `workflow.phase` and route | `skills/track/SKILL.md` |
| **track-task-setup** | Create task, repos, TODOs, links (`setup`) | `skills/track-task-setup/SKILL.md` |
| **track-task-execute** | Agent coding loop (`sync_required`, `execute`) | `skills/track-task-execute/SKILL.md` |
| **track-advanced** | Multi-repo, hotfix, archive | `skills/track-advanced/SKILL.md` |

### Routing by workflow phase

Run `track status --json` first, then pick a skill:

| `workflow.phase` | Skill |
|------------------|-------|
| `setup` | track-task-setup |
| `sync_required` / `execute` | track-task-execute |
| `task_complete` | track-advanced |
| Multi-repo / hotfix | track-advanced |

The **track** skill is a lightweight router — it maps phase and intent to the specialized skills above.

## Directory layout

```
skills/
├── README.md
├── INSTALL.md
├── track/                      # Router / index
│   ├── SKILL.md
│   └── agents/openai.yaml
├── track-task-setup/           # Planning & scaffolding
│   ├── SKILL.md
│   ├── agents/openai.yaml
│   └── references/setup-workflow.md
├── track-task-execute/         # Agent execution loop
│   ├── SKILL.md
│   ├── agents/openai.yaml
│   └── references/execution-workflow.md
├── track-advanced/             # Multi-repo, archive, patterns
│   ├── SKILL.md
│   ├── agents/openai.yaml
│   └── references/advanced-patterns.md
└── task-management/            # Deprecated — use split skills above
    └── SKILL.md
```

## Supported agents

| Agent | Install flag | Typical path |
|-------|--------------|--------------|
| Cursor | `-a cursor` | `.agents/skills/` or `~/.cursor/skills/` |
| Claude Code | `-a claude-code` | `.claude/skills/` or `~/.claude/skills/` |
| Codex | `-a codex` | `.agents/skills/` or `~/.codex/skills/` |

Browse more agents: [skills.sh](https://skills.sh) · `npx skills find track`

## Without installing

Agents with repo access can read skills directly:

```
skills/track/SKILL.md
skills/track-task-execute/SKILL.md
```

Installing via `npx skills` is recommended when working outside the track repository.

## Resources

- **Install guide**: [INSTALL.md](INSTALL.md)
- **CLI reference**: `track llm-help`
- **Integration overview**: [docs/LLM_INTEGRATION.md](../docs/LLM_INTEGRATION.md)
- **Agent Skills spec**: https://github.com/anthropics/skills

## License

MIT (same as track)
