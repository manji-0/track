# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `track archive --force` to skip jj-task and dirty-workspace checks
- `track migrate legacy-worktrees [--dry-run]` to clear legacy worktree flags
- `requires_workspace` on TODOs and `--no-workspace` flag for research/planning items
- `workflow.checklist` in `track status --json` and WebUI
- `jj.repos` per-repo workspace status and `jj.task_phase` from jj-task map
- WebUI workflow panel (phase, next action, checklist)

### Changed
- JJ mode: `track sync` rejected unless legacy `--worktree` TODOs are pending (`--legacy` to force)
- `--worktree` removed from CLI (returns error; use jj-task per task)
- Archive validates jj-task phase before archiving
- Workflow sync uses `all_repos_registered` for multi-repo tasks
- `task_complete` suggests `$jj` skill when jj-task phase is not `done`
- README, skills, and legacy docs aligned with JJ_INTEGRATION strategy

## [0.6.1] - 2026-07-07

### Changed
- crates.io package renamed from `jj-track` to `task-track` (`track` name is taken); CLI binary remains `track`

## [0.6.0] - 2026-07-07

### Added
- `vcs-mode` config (`track config set vcs-mode jj|git`) to switch between JJ and plain git worktrees
- `VcsMode` persisted in app state; git mode creates worktrees at `.worktrees/<slug>` on `track/<slug>` branches
- `docs/JJ_INTEGRATION.md` — combined track + agent-skill-jj strategy

### Changed
- **JJ strategy realignment**: track assumes [agent-skill-jj](https://github.com/manji-0/agent-skill-jj) for commits/PR; jj-task for workspaces
- Agent JSON adds `jj` / `git` context by mode; guardrails use `must_use_jj_skill` in JJ mode
- `workflow.next_action` suggests `jj-task start <slug>` instead of `track sync` for new tasks
- Skills v3.0: two-layer track + `$jj` documentation

## [0.5.0] - 2026-07-06

### Added
- `TodoAction` and `ApplyTodoActionUseCase` for unified CLI/WebUI todo operations
- `TaskStatus` / `TodoStatus` modules with transition rules and SQL constants
- Agent JSON fields in `track status --json` and `GET /api/status` (`workflow`, `todos_agent`, `guardrails`)
- `SyncTaskUseCase`, `ArchiveTaskUseCase`, and expanded use-case layer
- WebUI route tests and HTTP status mapping (400/404)
- DB CHECK constraints on `tasks.status` and `todos.status`
- Skills split by use case: `track`, `track-task-setup`, `track-task-execute`, `track-advanced`
- Skill plugin manifests for Claude Code, Codex, and Agents (kamae-rs-style)
- `scripts/validate_package.py` for skill/plugin smoke tests

### Changed
- CLI handlers split into focused modules; shared row mapping extracted
- Domain modeling and transaction boundaries strengthened across use cases
- Release profile: LTO, single codegen unit, strip symbols, panic=abort (~38% smaller binary)
- Trim tokio features to only those required by webui (was `full`)
- Skills install docs updated for `npx skills` multi-skill install

### Fixed
- Reopening completed or cancelled TODOs is forbidden (add a new TODO instead)
- WebUI no longer offers "Mark as Pending" for terminal TODO states

## [0.4.2] - 2026-01-28

### Changed
- LLM integration docs and skill workflows now start with `track sync` and JJ bookmark verification

### Fixed
- TODO worktree completion handles NULL ticket IDs without failing
- CI installs `jj` so JJ-dependent tests run reliably

## [0.4.1] - 2026-01-20

### Changed
- `track sync` is JJ-only and focuses on bookmark/workspace setup for the current task
- LLM help and skill guides now include JJ bookmark verification steps (`jj status`, `jj bookmark list -r @`)
- TODO completion guidance now reflects rebase + bookmark move behavior in JJ workflows
- Package metadata updated for JJ workspace terminology

### Fixed
- JJ workspace commands run from the correct working directory

## [0.4.0] - 2026-01-20

### Added
- `track todo workspace` for showing/recreating TODO workspaces (`--recreate`, `--force`, `--all`)
- JJ-first workflow guidance across CLI help and docs

### Changed
- `track sync` now aborts on dirty repos (excluding workspace directories)
- Workspaces paths sanitize bookmark names (slashes replaced with `_`)
- Markdown rendering is sanitized and templates auto-escape to mitigate XSS

### Fixed
- Todo reordering uses collision-safe temporary indices
- Timestamp parsing errors now surface as database conversion failures

## [0.3.6] - 2026-01-06

### Changed
- **Web UI Todo Status Display**: The oldest pending todo now displays as "in progress" with a distinctive blue color
  - Helps identify which task is currently being worked on
  - Regular pending todos show in a subdued gray color
  - Visual distinction makes it easier to focus on the active task

## [0.3.5] - 2026-01-06

### Added
- **Todo Prioritization**: New `track todo next <id>` command to move a todo to the front of the work queue
  - CLI: `track todo next <id>` moves a pending todo to become the next todo to work on
  - Web UI: "⬆️ Make Next" button in todo menu for easy reordering
  - Only pending todos can be moved; completed/cancelled todos are excluded
  
- **Todo-Scrap Linking**: Automatic linking between todos and scraps
  - Scraps are automatically linked to the active (oldest pending) todo when created
  - Database: Added `active_todo_id` column to scraps table with automatic migration
  - Web UI: 📝 button on todos to jump to related scraps
  - Smooth scrolling and highlighting animation for linked scraps
  - Button only appears on todos that have associated scraps
  
- **Web UI Enhancements**:
  - Focus mode for concentrated work on current task
  - Real-time updates via Server-Sent Events (SSE)
  - Improved visual feedback with animations
  - Conditional UI elements based on data availability
  - Hidden scrollbars in todo and scrap cards for cleaner appearance
  - Automatic scroll positioning to oldest pending task in focus mode

### Changed
- Database schema updated with migration support for existing databases
- Web UI todo list now includes scrap count information
- Improved error handling for todo reordering operations

### Fixed
- UNIQUE constraint handling in todo reordering with two-phase update strategy
- Trailing whitespace in source files (cargo fmt compliance)

## [0.3.4] - 2026-01-05

### Added
- Focus mode and overview mode toggle in Web UI
- Consistent toolbar layout with view mode, connection status, and theme toggle
- Improved overscroll background handling

### Changed
- Renamed UI elements to reflect "Focus Mode" and "Overview Mode" paradigm
- Consolidated control elements into single horizontal toolbar

## [0.3.3] - 2026-01-05

### Added
- Automatic URL linkification in todo and scrap content
- Markdown links open in new tab with `target="_blank"`
- Proper handling of trailing punctuation in URLs

### Fixed
- Link rendering in Web UI with proper HTML escaping

## [0.3.2] - 2026-01-05

### Added
- Task alias management with `--force` option to overwrite existing aliases
- Improved alias validation and error messages

## [0.3.1] - 2026-01-05

### Added
- Web UI with real-time updates
- Server-Sent Events (SSE) for live synchronization
- Modern browser-based interface with HTMX

## [0.3.0] - 2026-01-04

### Added
- Task templates: Create new tasks from existing task templates
- Task aliases: Assign human-readable aliases to tasks
- Ticket reference: Reference tasks by ticket ID (e.g., `t:PROJ-123`)
- Repository management with base branch tracking
- Git worktree automation

### Changed
- Improved CLI help and documentation
- Enhanced task switching and reference resolution

## [0.2.0] - 2026-01-03

### Added
- TODO management with worktree support
- Link management for reference URLs
- Scrap (work notes) management
- Task archiving

## [0.1.0] - 2026-01-02

### Added
- Initial release
- Basic task management
- SQLite database with XDG compliance
- CLI interface with clap
