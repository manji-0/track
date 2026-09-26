# Command reference

Scannable CLI tables. Command-level behavior and error cases live in [FUNCTIONAL_SPEC.md](FUNCTIONAL_SPEC.md). Copy-paste workflows: [USAGE_EXAMPLES.md](USAGE_EXAMPLES.md). VCS, workspaces, and git notes: [JJ_INTEGRATION.md](JJ_INTEGRATION.md).

Human output is tables/prose. `--json` / `-j` is for agents. Mutating commands that accept `--json` return the same snapshot as `track status --json` plus `ok` and `mutation`.

## Task management

| Command | Description |
|---------|-------------|
| `track new <name> [--json]` | Create a new task and set it as active |
| `track new <name> --template <task_ref>` | Create task from template (copies TODOs) |
| `track list [--all] [--json]` | Display task list |
| `track switch <task_id> [--json]` | Switch tasks |
| `track switch today` | Switch to today's task (auto-creates if needed) |
| `track status [id]` | Display task information |
| `track status --json` | Output in JSON format |
| `track status --all` | Show all scraps |
| `track desc [description]` | View or set task description |
| `track ticket <ticket_id> <url>` | Link a ticket to the task |
| `track alias set <alias>` | Set an alias for the current task |
| `track alias set <alias> --force` | Overwrite existing alias on another task |
| `track alias remove` | Remove alias from the current task |
| `track archive [task_id] [--json]` | Archive a task |

Task refs resolve as numeric id, `t:<ticket>`, `a:<alias>`, or `today`.

## Configuration

| Command | Description |
|---------|-------------|
| `track config show` | Show current configuration |
| `track config set vcs-mode git\|jj` | Git worktrees (default) or colocated jj workspaces |
| `track config set aggressive-mode on\|off` | Per-task empty revision + published work record as git notes |
| `track import [path] [--json]` | Restore a task from git notes on the current branch |
| `track notes push [--remote]` | Disclose `refs/notes/track` |
| `track notes fetch [--remote]` | Receive `refs/notes/track` |
| `track config set-calendar <calendar-id>` | Set Google Calendar ID for today task |

## TODO management

| Command | Description |
|---------|-------------|
| `track todo add <text> [--no-workspace] [--json]` | Add a TODO (`--json` returns the status snapshot) |
| `track todo list` | Display TODO list |
| `track todo update <index> <status> [--json]` | Update TODO status |
| `track todo done <index> [--json]` | Complete a TODO |
| `track todo workspace <index> [--recreate --force --all]` | Show or recreate workspaces for a TODO |
| `track todo next <index> [--json]` | Move a TODO to the front (make it the next todo to work on) |
| `track todo delete <index>` | Delete a TODO |
| `track todo delete <index> --force [--json]` | Delete without confirmation |

`--no-workspace` marks research/planning items. Off-TTY deletes require `--force`. Reopening a done/cancelled TODO is forbidden — add a follow-up TODO.

## Link management

| Command | Description |
|---------|-------------|
| `track link add <url> [title]` | Add a reference URL |
| `track link list` | Display link list |
| `track link delete <index>` | Delete a link |

## Scrap (work notes) management

| Command | Description |
|---------|-------------|
| `track scrap add <content> [--share] [--json]` | Add a work note (`--share` = git notes on the matching TODO commit) |
| `track scrap list` | Display all scraps |
| `track scrap share <id> [--json]` | Include a scrap in git notes on that TODO's commit |
| `track scrap unshare <id> [--json]` | Keep a scrap local-only |

Scraps attach to the oldest pending TODO at insert time. Local is the default; `--share` / `share` / `unshare` select what is published with aggressive mode.

## Repository management

| Command | Description |
|---------|-------------|
| `track repo add [path] [--json]` | Register a repository to the current task |
| `track repo add --base <bookmark>` | Register repository with custom base bookmark |
| `track repo list` | Display registered repositories |
| `track repo remove <id>` | Remove a repository registration |

## Sync

| Command | Description |
|---------|-------------|
| `track sync` | Create/refresh the task workspace (`.worktrees/<slug>` on `track/<slug>`). New workspaces fetch `origin` and start from the remote base branch when it exists, so a dirty base checkout does not block them ([details](JJ_INTEGRATION.md#workspace-base)) |
| `track sync --legacy` | Also rebuild old per-TODO jj worktrees |
| `track migrate legacy-worktrees [--dry-run] [--force]` | Clear legacy per-TODO worktree flags |

## Web UI

| Command | Description |
|---------|-------------|
| `track webui` | Start web-based user interface on port 3000 |
| `track webui --port 8080` | Start on custom port |
| `track webui --open` | Start and open browser automatically |

See [WEBUI.md](WEBUI.md).

## Shell completion

| Command | Description |
|---------|-------------|
| `track completion bash` | Generate bash completion script |
| `track completion zsh` | Generate zsh completion script |
| `track completion fish` | Generate fish completion script |
| `track completion powershell` | Generate PowerShell completion script |

**Quick install (dynamic — recommended):**

```bash
# Bash (dynamic)
mkdir -p ~/.local/share/bash-completion/completions
track completion bash --dynamic > ~/.local/share/bash-completion/completions/track
source ~/.local/share/bash-completion/completions/track

# Zsh (dynamic)
mkdir -p ~/.zsh/completions
track completion zsh --dynamic > ~/.zsh/completions/_track
# Add to ~/.zshrc: fpath=(~/.zsh/completions $fpath)
# Then: exec zsh

# Fish (static only)
mkdir -p ~/.config/fish/completions
track completion fish > ~/.config/fish/completions/track.fish
```

**What you get with dynamic completions:**

```bash
track switch <TAB>              # Shows your actual task IDs and names
track todo done <TAB>            # Shows pending TODO IDs with content
track todo update 6 <TAB>        # Shows status: pending, done, cancelled
track link delete <TAB>          # Shows link IDs with titles
track repo remove <TAB>          # Shows repository IDs with paths
track new --template <TAB>       # Shows task IDs for templates
```

Install details and troubleshooting: [completions/README.md](../completions/README.md).

## Other

| Command | Description |
|---------|-------------|
| `track llm-help` | Agent-oriented help (workspace + commit/notes loop) |

## Additional features

For copy-paste workflows, see [USAGE_EXAMPLES.md](USAGE_EXAMPLES.md):

- **Task aliases**: human-readable names (`track alias set`, `track switch a:…`)
- **Task templates**: `track new "…" --template <ref>` copies TODOs
- **Ticket reference**: `t:PROJ-123` (Jira / GitHub / GitLab IDs are labels, not the source of truth)
- **Workspace slug**: alias, else ticket id, else `task-{id}` — [JJ_INTEGRATION.md](JJ_INTEGRATION.md)
- **VCS modes**: `git` (default) or `jj`; `track sync` / `track repo add` create `.worktrees/<slug>`
