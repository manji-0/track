# Installing Track Skills for AI Agents

Track ships an [Agent Skills](https://github.com/anthropics/skills)-compatible skill at `skills/task-management/`. This guide covers the **recommended `npx skills` install**, agent-specific paths, and manual fallbacks.

## Requirements

- **track CLI** built and on your `PATH`
- **Node.js 18+** (for `npx skills` only)
- **jj** (for workspace workflows)

---

## Recommended: `npx skills` (Skills CLI)

The [Skills CLI](https://github.com/vercel-labs/skills) (`npx skills`) is the package manager for the open agent skills ecosystem. It detects installed agents and links skills into the correct directories.

### Quick install (from this repo)

Run at the **track repository root**:

```bash
# Project scope — committed with the repo, shared with the team
npx skills add ./skills/task-management -y

# Global scope — available in every project
npx skills add ./skills/task-management -g -y

# Target specific agents (recommended)
npx skills add ./skills/task-management \
  -a cursor -a claude-code -a codex -y
```

### Install from GitHub

```bash
npx skills add manji-0/track \
  --skill track-task-management \
  -g -a cursor -a claude-code -a codex -y
```

Use your fork or branch URL if needed:

```bash
npx skills add https://github.com/you/track/tree/your-branch/skills/task-management -y
```

### Useful commands

| Command | Purpose |
|---------|---------|
| `npx skills list` | List installed skills |
| `npx skills find track` | Search skills.sh / registry |
| `npx skills update track-task-management` | Update one skill |
| `npx skills update -y` | Update all installed skills |
| `npx skills remove track-task-management` | Uninstall |
| `npx skills use ./skills/task-management \| claude` | One-shot prompt without installing |

### Install options

| Flag | Description |
|------|-------------|
| `-g, --global` | Install to user home (all projects) |
| `-a, --agent <name>` | Target agent: `cursor`, `claude-code`, `codex`, … |
| `-y, --yes` | Non-interactive (CI-friendly) |
| `--copy` | Copy files instead of symlinking |
| `-l, --list` | List skills in a repo without installing |

Docs: [skills.sh](https://skills.sh) · [Skills CLI installation](https://vercel-labs-skills.mintlify.app/installation)

---

## Agent paths (where skills land)

When you use `npx skills`, files are linked into agent-specific directories:

| Agent | `--agent` flag | Project path | Global path |
|-------|----------------|--------------|-------------|
| **Cursor** | `cursor` | `.agents/skills/` | `~/.cursor/skills/` |
| **Claude Code** | `claude-code` | `.claude/skills/` | `~/.claude/skills/` |
| **OpenAI Codex** | `codex` | `.agents/skills/` | `~/.codex/skills/` |
| Cline / OpenCode / Copilot | `cline`, `opencode`, `github-copilot` | `.agents/skills/` | varies |

Track's skill name (from `SKILL.md` frontmatter) is **`track-task-management`**.

### Cursor

**Via Skills CLI (recommended):**

```bash
npx skills add ./skills/task-management -a cursor -y
```

Skills appear under `.agents/skills/` (project) or `~/.cursor/skills/` (global). Cursor loads them automatically when relevant.

**Manual:** copy or symlink `skills/task-management/` into `.agents/skills/track-task-management/` in your project.

**In chat:** mention the skill explicitly, e.g. *"Follow the track-task-management skill and run `track status --json` first."*

### Claude Code

**Via Skills CLI:**

```bash
npx skills add ./skills/task-management -a claude-code -g -y
```

Install location: `~/.claude/skills/track-task-management/` (global) or `.claude/skills/` (project).

Claude Code discovers `SKILL.md` automatically. For one-off use:

```bash
track llm-help
```

### OpenAI Codex

**Via Skills CLI:**

```bash
npx skills add ./skills/task-management -a codex -g -y
```

Install location: `~/.codex/skills/` (global) or `.agents/skills/` (project).

Codex reads skills from these paths when executing tasks in a repo.

---

## No install needed (repo checkout)

If your agent already has the track repo open, it can read skills directly:

```
skills/task-management/SKILL.md
skills/task-management/references/executing-tasks.md
```

This works in Cursor Cloud Agents, Claude Code, and Codex when the workspace includes track. For daily use across unrelated repos, prefer `npx skills add -g`.

---

## Verify installation

```bash
# Skills CLI
npx skills list | rg track-task-management

# track CLI
track llm-help
track status --json | jq '.workflow.phase, .workflow.next_action'
```

Ask your agent:

> Read the track-task-management skill, run `track status --json`, and tell me the workflow phase and next action.

Expected: agent reports `workflow.phase` (e.g. `setup`, `sync_required`, `execute`) and follows `next_action.command`.

---

## Agent workflow (JSON-first)

After installing the skill, agents should prefer structured status:

```bash
track status --json
```

Key fields:

| Field | Use |
|-------|-----|
| `workflow.phase` | Current lifecycle stage |
| `workflow.next_action` | Suggested command and reason |
| `todos_agent[].is_next` | Which TODO to work on |
| `todos_agent[].allowed_actions` | Valid operations (no reopen) |
| `guardrails` | Must-follow rules (`must_sync_before_code_changes`, etc.) |

Same shape is available from the WebUI at `GET /api/status` when a task is active.

---

## Troubleshooting

### Skill not picked up

1. Confirm install: `npx skills list`
2. Check agent path exists (see table above)
3. Restart the agent / IDE
4. Try `--copy` instead of symlink: `npx skills add ./skills/task-management --copy -y`

### Wrong agent directory

Pass `-a` explicitly:

```bash
npx skills remove track-task-management -y
npx skills add ./skills/task-management -a cursor -a claude-code -y
```

### Private GitHub repo

```bash
export GITHUB_TOKEN=ghp_...
npx skills add your-org/track --skill track-task-management -y
```

---

## Manual / legacy setups

<details>
<summary>Aider, Copilot Chat, ChatGPT (expand)</summary>

### Aider

```yaml
# .aider.conf.yml
read:
  - skills/task-management/SKILL.md
```

### GitHub Copilot Chat

```
@workspace skills/task-management/SKILL.md — help me execute the current track task
```

### ChatGPT

Paste `skills/task-management/SKILL.md` or upload the file at session start.

</details>

---

## Updating

```bash
# Pull latest track
jj git pull   # or git pull

# Update linked skill
npx skills update track-task-management -y
```

---

## Related docs

- [skills/README.md](README.md) — skill overview
- [docs/LLM_INTEGRATION.md](../docs/LLM_INTEGRATION.md) — agent integration guide
- [track llm-help](../src/cli/handlers/llm_help.rs) — full CLI workflow (`track llm-help`)
