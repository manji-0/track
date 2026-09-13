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

## Track-owned workspaces

<!-- constrained-by ./docs/JJ_INTEGRATION.md#Division of responsibility -->

| Layer | Tool | Responsibility |
|-------|------|----------------|
| **WHAT** | `track` | Current task, TODOs, scraps, workflow JSON |
| **WHERE** | `track` | Git worktree or colocated jj workspace at `.worktrees/<slug>` on `track/<slug>` |

`git` is the default VCS mode (`track config set vcs-mode git|jj`). Track creates the coding workspace on `track repo add` / `track sync`. Agents read `hint` / `workflow.next_action` from `track status --json` (or the stderr footer) instead of running `git worktree` / `jj workspace` / jj-task by hand.

Aggressive mode (`track config set aggressive-mode on`) adds a per-task empty marker revision. Commit history on `track/<slug>` matches done TODOs (one commit per `track todo done`, same order). Git notes (`refs/notes/track`) on the marker hold task identity (name, description, ticket, links). Shared scraps live as notes on **that TODO's commit**. Follow-up review work is a new TODO → new commit → new notes; published SHAs are never rewritten (`git notes add -f` only on unpublished commits). Turning aggressive on after work exists backfills the marker **under** unpublished commits. Unique commits already on origin (`origin/track/<slug>` or jj `track/<slug>@origin`) are not rewritten; the local colocated `{bookmark}@git` export is not origin. Scraps default to local; `track scrap add --share` / `track scrap share` selects what is published. `track notes fetch` + `track import` restore the work record from the PR branch without the author's SQLite file.

Commits and GitHub PRs still use git or jj from the task workspace; the PR head is always `track/<slug>`.

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
- `vcs-mode` (`git` \| `jj`; default `git` on new DBs)
- `aggressive_mode` (`on` \| `off`; default `off`)
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

Task-scoped URL list and chronological notes. Scraps may set `active_todo_id` to the pending TODO at insert time. Each scrap is `local` (default) or `shared`; only shared scraps are exported in git notes.

### task_repos

Registered working copies for the current task (`repo_path`, `base_branch`, `base_commit_hash`, `task_index`). This is what `track repo add` writes.

### worktrees, repo_links, task_revisions

Track-owned workspace rows. Both VCS modes use `<repo>/.worktrees/<slug>/`. Git notes / empty task commits live in `task_revisions` when aggressive mode is on. Legacy per-TODO `worktree_requested` rows still exist until `track migrate legacy-worktrees`.

## Command surface

Prefix: `track`. Human tables/prose are the default. `--json` / `-j` is for agents.

### Context

| Command | Behavior |
| :--- | :--- |
| `track new <name>` | Create task, switch to it. Optional `--ticket`, `--template`, `--json`. |
| `track list [--all]` | Task inventory. `--json` → `current_task_id` + `tasks[].is_current` (not a status snapshot). |
| `track switch <ref>` | Switch by id, `t:<ticket>`, `a:<alias>`, or `today`. |
| `track status [--json]` | Current-task snapshot. JSON includes `workflow`, `hint`, `git`/`jj`, `todos_agent`, `guardrails`. |
| `track archive` | Archive after the PR is done. Removes workspaces; keeps `track/<slug>` for PRs. Prompts only on a TTY; `--force` skips dirty checks. |

### Items (current task)

| Command | Behavior |
| :--- | :--- |
| `track todo add/list/done/update/next/delete` | TODOs. Delete requires `--force` off-TTY. |
| `track scrap add/list/share/unshare` | Work notes. `--share` marks a scrap for git notes on the matching TODO commit. |
| `track import` | Restore a task from `refs/notes/track` (marker identity + per-TODO commits). |
| `track notes push/fetch` | Disclose / receive notes (`refs/notes/track`). |
| `track link add/list/delete` | Reference URLs. |
| `track repo add/list/remove` | Register repos. Track creates the task workspace (git or jj). |

Mutations that accept `--json` (`new`, `switch`, `archive`, `todo add/done/update/next/delete`, `scrap add/share/unshare`, `repo add`, `import`) print the **same shape as `track status --json`** plus `ok` and `mutation` (`kind`, `id`). They do not invent per-command schemas.

### Other

`desc`, `ticket`, `alias`, `config` (`vcs-mode`, `aggressive-mode`), `sync`, `import`, `notes`, `migrate legacy-worktrees`, `webui`, `llm-help`, `completion`.

## Agent contract

<!-- constrained-by ./docs/LLM_INTEGRATION.md -->
<!-- dagayn: implemented-by src/use_cases/get_task_info.rs::GetTaskInfoUseCase.to_cli_json -->

Coding agents should:

1. Read `track status --json` (or a mutation `--json` response).
2. Follow `hint.next_command` / `workflow.next_action` / `workflow.checklist`.
3. Work only in `.worktrees/<slug>/` (never repo root).
4. Use track for TODO/scrap/archive; commit and push from the task workspace (`git` or `jj`).

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
