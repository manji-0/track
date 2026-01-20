# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
