# Installing Track Skills for AI Agents

Track ships four [Agent Skills](https://github.com/anthropics/skills) under `skills/`, split by use case. This guide covers the **recommended `npx skills` install**, agent-specific paths, and manual fallbacks.

## Requirements

- **track CLI** built and on your `PATH`
- **Node.js 18+** (for `npx skills` only)
- **jj** (for workspace workflows)

---

## Recommended: install all track skills

Run at the **track repository root**:

```bash
npx skills add ./skills \
  -s track -s track-task-setup -s track-task-execute -s track-advanced \
  -g -a cursor -a claude-code -a codex -y
```

### Install subset

| Need | Command |
|------|---------|
| Router + execution (typical agent) | `npx skills add ./skills -s track -s track-task-execute -a cursor -y` |
| Setup / planning only | `npx skills add ./skills -s track-task-setup -y` |
| Advanced patterns only | `npx skills add ./skills -s track-advanced -y` |

### Project vs global

```bash
# Project scope — committed with the repo, shared with the team
npx skills add ./skills -s track -s track-task-execute -y

# Global scope — available in every project
npx skills add ./skills -s track -s track-task-execute -g -y
```

### Install from GitHub

```bash
npx skills add manji-0/track \
  -s track -s track-task-setup -s track-task-execute -s track-advanced \
  -g -a cursor -a claude-code -a codex -y
```

---

## Skill names

| Skill | Name (frontmatter) | When to load |
|-------|-------------------|--------------|
| Router | `track` | Any track mention; routes by `workflow.phase` |
| Setup | `track-task-setup` | Create task, repos, TODOs |
| Execute | `track-task-execute` | Sync, implement, `todo done` |
| Advanced | `track-advanced` | Multi-repo, parallel, archive |

Legacy monolithic skill **`track-task-management`** is deprecated — see `skills/task-management/SKILL.md`.

---

## Useful commands

| Command | Purpose |
|---------|---------|
| `npx skills list` | List installed skills |
| `npx skills find track` | Search skills.sh / registry |
| `npx skills update track -y` | Update one skill |
| `npx skills update -y` | Update all installed skills |
| `npx skills remove track-task-execute -y` | Uninstall one skill |
| `npx skills use ./skills/track-task-execute \| claude` | One-shot prompt without installing |

### Install options

| Flag | Description |
|------|-------------|
| `-s, --skill <name>` | Install specific skill(s) from a repo |
| `-g, --global` | Install to user home (all projects) |
| `-a, --agent <name>` | Target agent: `cursor`, `claude-code`, `codex`, … |
| `-y, --yes` | Non-interactive (CI-friendly) |
| `--copy` | Copy files instead of symlinking |
| `-l, --list` | List skills in a repo without installing |

Docs: [skills.sh](https://skills.sh) · [Skills CLI installation](https://vercel-labs-skills.mintlify.app/installation)

---

## Agent paths (where skills land)

| Agent | `--agent` flag | Project path | Global path |
|-------|----------------|--------------|-------------|
| **Cursor** | `cursor` | `.agents/skills/` | `~/.cursor/skills/` |
| **Claude Code** | `claude-code` | `.claude/skills/` | `~/.claude/skills/` |
| **OpenAI Codex** | `codex` | `.agents/skills/` | `~/.codex/skills/` |

### Cursor

```bash
npx skills add ./skills -s track -s track-task-execute -a cursor -y
```

**In chat:** *"Follow track-task-execute and run `track status --json` first."*

### Claude Code

```bash
npx skills add ./skills -s track -s track-task-execute -a claude-code -g -y
```

### OpenAI Codex

```bash
npx skills add ./skills -s track -s track-task-execute -a codex -g -y
```

---

## No install needed (repo checkout)

If your agent already has the track repo open:

```
skills/track/SKILL.md
skills/track-task-execute/SKILL.md
skills/track-task-execute/references/execution-workflow.md
```

For daily use across unrelated repos, prefer `npx skills add -g`.

---

## Verify installation

```bash
npx skills list | rg 'track|track-task'

track llm-help
track status --json | jq '.workflow.phase, .workflow.next_action'
```

Ask your agent:

> Read the track-task-execute skill, run `track status --json`, and tell me the workflow phase and next action.

---

## Agent workflow (JSON-first)

```bash
track status --json
```

| Field | Use |
|-------|-----|
| `workflow.phase` | Route to setup / execute / advanced skill |
| `workflow.next_action` | Suggested command and reason |
| `todos_agent[].is_next` | Which TODO to work on |
| `todos_agent[].allowed_actions` | Valid operations (no reopen) |
| `guardrails` | Must-follow rules |

Same shape from WebUI: `GET /api/status`.

---

## Troubleshooting

### Skill not picked up

1. `npx skills list`
2. Check agent path exists (see table above)
3. Restart the agent / IDE
4. Try `--copy`: `npx skills add ./skills -s track-task-execute --copy -y`

### Migrating from track-task-management

```bash
npx skills remove track-task-management -y
npx skills add ./skills \
  -s track -s track-task-setup -s track-task-execute -s track-advanced \
  -a cursor -a claude-code -y
```

### Private GitHub repo

```bash
export GITHUB_TOKEN=ghp_...
npx skills add your-org/track -s track -s track-task-execute -y
```

---

## Manual / legacy setups

<details>
<summary>Aider, Copilot Chat, ChatGPT (expand)</summary>

### Aider

```yaml
read:
  - skills/track/SKILL.md
  - skills/track-task-execute/SKILL.md
```

### GitHub Copilot Chat

```
@workspace skills/track-task-execute/SKILL.md — continue the current track task
```

</details>

---

## Updating

```bash
jj git pull   # or git pull
npx skills update track track-task-setup track-task-execute track-advanced -y
```

---

## Related docs

- [skills/README.md](README.md) — skill catalog and routing
- [docs/LLM_INTEGRATION.md](../docs/LLM_INTEGRATION.md) — agent integration guide
- `track llm-help` — full CLI workflow
