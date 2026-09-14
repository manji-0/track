<div align="center">
  <img src="static/track.svg" width="128" height="128" alt="Track Logo" />
  <h1>Track</h1>
</div>

A personal work-context manager for humans and coding agents: one implicit current task in an XDG SQLite database (`$HOME/.local/share/track/track.db`). Track holds **what** you are doing (tasks, TODOs, scraps) and **owns the coding workspace** (git worktree or colocated jj workspace). It is not a repo issue tracker or a multi-agent planner.

## Features

- **Implicit current task**: `track new` / `track switch`, then `todo` / `scrap` / `link` without repeating IDs
- **Agent JSON**: `track status --json` and write commands with `--json` share `workflow`, `hint`, `git`/`jj`, `todos_agent`, `guardrails`
- **Tickets as labels**: optional Jira / GitHub / GitLab IDs on a personal task — not the source of truth
- **Track-owned workspaces**: `.worktrees/<slug>` on `track/<slug>` (git branch or jj bookmark). `track config set vcs-mode git|jj`
- **Commit + notes**: optional aggressive mode — one commit per TODO and a published work record as git notes (`refs/notes/track`)
- **Hints**: every command ends with post-state + next action (stderr; also `hint` in `--json`)
- **Web UI**: browser view with SSE, including [today task](docs/TODAY_TASK.md)

## Installation

```bash
# From crates.io (CLI binary is `track`)
cargo install task-track

# From a checkout
cargo install --path .
```

## Quick start

```bash
track new "Implement User Authentication" \
  --ticket AUTH-456 \
  --ticket-url https://jira.example.com/browse/AUTH-456

track repo add .
track todo add "Design database schema"
track todo add "Compare auth providers" --no-workspace   # research — no coding workspace

# The command footer / `hint.next_command` tells you where to work:
#   next: cd "/path/to/repo/.worktrees/auth-456"

track scrap add "Using bcrypt for password hashing"
track todo done 1
track status --json
```

Default VCS mode is **git**. Switch with `track config set vcs-mode jj`. Optional `track config set aggressive-mode on` aligns commits with TODOs and publishes notes — see [VCS integration](docs/JJ_INTEGRATION.md#commit--notes-strategy).

## Web UI

```bash
track webui          # http://localhost:3000
track webui --open
```

![Track Web UI](docs/images/webui-overview.png)

Layout, command bar, and screenshots: [docs/WEBUI.md](docs/WEBUI.md).

## Documentation

| Doc | Use |
|-----|-----|
| [docs/README.md](docs/README.md) | Index |
| [DESIGN.md](DESIGN.md) | Product design |
| [docs/JJ_INTEGRATION.md](docs/JJ_INTEGRATION.md) | VCS: git / jj workspaces, **commit + notes** |
| [docs/CLI.md](docs/CLI.md) | Command reference |
| [docs/WEBUI.md](docs/WEBUI.md) | Browser UI |
| [docs/USAGE_EXAMPLES.md](docs/USAGE_EXAMPLES.md) | Copy-paste workflows |
| [docs/TODAY_TASK.md](docs/TODAY_TASK.md) | `track switch today` |
| [docs/LLM_INTEGRATION.md](docs/LLM_INTEGRATION.md) | Agent skills and `track llm-help` |
| [docs/FUNCTIONAL_SPEC.md](docs/FUNCTIONAL_SPEC.md) | Command-level spec |
| [completions/README.md](completions/README.md) | Shell completions |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Build, test, WebUI development |
| [PROJECT_STRUCTURE.md](PROJECT_STRUCTURE.md) | Crate layout |
| [CHANGELOG.md](CHANGELOG.md) | Release notes (source of truth for GitHub Releases) |

## License

MIT License
