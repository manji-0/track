# Track Design Specification

<!-- constrained-by ./docs/JJ_INTEGRATION.md -->
<!-- constrained-by ./docs/LLM_HELP_DESIGN.md -->

## Overview

Track is a **personal work-context manager**. It stores one person's current work — tasks, TODOs, scraps, links, tickets — in an XDG SQLite database at `$HOME/.local/share/track/track.db`. After `track new` or `track switch`, every command applies to `current_task_id`. You do not pass a task ID on each `todo add`.

Track is **not**:

- a repository issue tracker (GitHub Issues, Jira, Beads)
- a PRD planner that expands specs into a graph (Task Master)
- a multi-agent claim / orchestration layer (Gas Town)

Tickets and URLs are optional labels on a personal task. The source of truth for "what am I doing" is the local DB, shared by a human and coding agents.

## Two-layer stack

<!-- constrained-by ./docs/JJ_INTEGRATION.md#Division of responsibility -->

| Layer | Tool | Responsibility |
|-------|------|----------------|
| **WHAT** | `track` + track skills | Current task, TODOs, scraps, workflow JSON |
| **HOW** | `$jj` skill + `jj-task` | Workspaces, squash/commit, PR, push |

In JJ mode (default), track does **not** create the coding workspace. Agents read `jj.slug` from `track status --json` and run `jj-task start <slug>`. Commits follow the `$jj` skill, not `track todo done`.

Git mode (`track config set vcs-mode git`) still uses `track sync` for `.worktrees/<slug>` on `track/<slug>` branches.

## Technology stack

| Category | Crate | Purpose |
| :--- | :--- | :--- |
| CLI | clap 4.6 | Subcommands and `--json` / `-j` |
| DB | rusqlite (bundled) | SQLite at the XDG data dir |
| Paths | directories | XDG Base Directory |
| Errors | thiserror | `TrackError` |
| Time | chrono | Timestamps |
| Tables | prettytable-rs | Human list output |
| JSON | serde / serde_json | Agent snapshots |
| Web | axum, minijinja, HTMX, SSE | Browser UI |

The CLI binary is `track` (`task-track` on crates.io).

## Implicit current task

`app_state.current_task_id` is the session. `track new` inserts a task and writes that key. `track switch` only updates the key (or `track switch today` for the daily task). Item commands (`todo`, `scrap`, `link`, `repo`) fail with `NoActiveTask` when the key is missing.

User-facing TODO / link / repo IDs are **task-scoped** (`task_index` starting at 1 per task), not global row IDs.

## Database

Path: `$HOME/.local/share/track/track.db`. `CREATE TABLE` in `src/db/mod.rs` plus columns added in `src/db/migrate.rs`.

### app_state

- `current_task_id`
- `vcs-mode` (`jj` \| `git`)
- `calendar_id` (WebUI today-task calendar)
- section revision counters for SSE

### tasks

- `id`, `name`, `description`, `status` (`active` \| `archived`)
- `ticket_id`, `ticket_url`, `alias` (unique)
- `is_today_task`
- `created_at`

### todos

- `id`, `task_id`, `task_index`, `content`
- `status` (`pending` \| `done` \| `cancelled`)
- `requires_workspace` (false = research; `--no-workspace`)
- `worktree_requested` (legacy per-TODO flag; CLI `--worktree` is removed)
- `created_at`, `completed_at`

### links, scraps

Task-scoped URL list and chronological notes. Scraps may set `active_todo_id` to the pending TODO at insert time.

### task_repos

Registered working copies for the current task (`repo_path`, `base_branch`, `base_commit_hash`, `task_index`). This is what `track repo add` writes.

### worktrees, repo_links

Legacy / git-mode workspace rows. JJ-mode coding happens in jj-task's `.worktrees/<slug>/` and `~/.config/jj/task-workspaces.json`, not as the primary store.

## Command surface

Prefix: `track`. Human tables/prose are the default. `--json` / `-j` is for agents.

### Context

| Command | Behavior |
| :--- | :--- |
| `track new <name>` | Create task, switch to it. Optional `--ticket`, `--template`, `--json`. |
| `track list [--all]` | Task inventory. `--json` → `current_task_id` + `tasks[].is_current` (not a status snapshot). |
| `track switch <ref>` | Switch by id, `t:<ticket>`, `a:<alias>`, or `today`. |
| `track status [--json]` | Current-task snapshot. JSON includes `workflow`, `jj`, `todos_agent`, `guardrails`. |
| `track archive` | Archive after `jj-task done` (JJ). Prompts only on a TTY; `--force` skips checks. |

### Items (current task)

| Command | Behavior |
| :--- | :--- |
| `track todo add/list/done/update/next/delete` | TODOs. Delete requires `--force` off-TTY. |
| `track scrap add/list` | Work notes (not `track log`). |
| `track link add/list/delete` | Reference URLs. |
| `track repo add/list/remove` | Register repos. JJ: `jj` subprocess for base bookmark. |

Mutations that accept `--json` (`new`, `switch`, `archive`, `todo add/done/update/next/delete`, `scrap add`, `repo add`) print the **same shape as `track status --json`** plus `ok` and `mutation` (`kind`, `id`). They do not invent per-command schemas.

### Other

`desc`, `ticket`, `alias`, `config`, `sync` (git / legacy JJ `--worktree`), `migrate legacy-worktrees`, `webui`, `llm-help`, `completion`.

## Agent contract

<!-- constrained-by ./docs/LLM_INTEGRATION.md -->
<!-- dagayn: implemented-by src/use_cases/get_task_info.rs::GetTaskInfoUseCase.to_cli_json -->

Coding agents should:

1. Read `track status --json` (or a mutation `--json` response).
2. Follow `workflow.next_action` / `workflow.checklist`.
3. Start `jj-task` from `jj.slug` when `phase` is `sync_required`.
4. Use `$jj` for commits; use track only for TODO/scrap/archive.

MCP is intentionally absent: Cursor / Claude Code / Codex already have a shell and skills. A future MCP would wrap this same snapshot, not a second schema.

## Web UI

`track webui` — Axum + HTMX + SSE. `GET /api/status` returns the same agent fields as CLI JSON. HTML partials are for humans, not a second agent API.

Today-task and calendar behavior: [docs/TODAY_TASK.md](docs/TODAY_TASK.md).

## Related documents

- [docs/README.md](docs/README.md) — index
- [docs/JJ_INTEGRATION.md](docs/JJ_INTEGRATION.md) — two-layer runtime strategy
- [docs/LLM_HELP_DESIGN.md](docs/LLM_HELP_DESIGN.md) — `track llm-help` content contract
- [docs/LLM_INTEGRATION.md](docs/LLM_INTEGRATION.md) — skills install
- [docs/FUNCTIONAL_SPEC.md](docs/FUNCTIONAL_SPEC.md) — command-level spec
- [docs/TODAY_TASK.md](docs/TODAY_TASK.md) — today task
- [PROJECT_STRUCTURE.md](PROJECT_STRUCTURE.md) — crate layout
