# Track - Project Structure

## Overview

Track is a Rust CLI and Web UI for managing development tasks with JJ workspace integration. Data is stored in SQLite under `~/.local/share/track/track.db`.

## Directory Structure

```
track/
├── Cargo.toml
├── src/
│   ├── main.rs              # Binary entry (delegates to lib)
│   ├── lib.rs               # Library exports
│   ├── cli/                 # Clap command tree and CommandHandler
│   │   ├── mod.rs
│   │   ├── handler.rs       # Dispatch only
│   │   └── handlers/        # task, todo, sync, repo, …
│   ├── db/                  # SQLite schema, migrations, transactions
│   │   ├── mod.rs
│   │   └── row_mapping.rs   # Shared row → domain parsing
│   ├── models/              # Task, Todo, status, workflow, TodoAction
│   │   ├── status.rs        # TaskStatus, TodoStatus + transitions
│   │   ├── todo_action.rs   # Intent-based TODO operations
│   │   └── workflow.rs      # WorkflowPhase, agent view types
│   ├── services/            # Domain services (SQL + business rules)
│   │   ├── task_service.rs
│   │   ├── todo_service.rs
│   │   ├── link_service.rs  # LinkService + ScrapService
│   │   ├── repo_service.rs
│   │   └── worktree_service.rs
│   ├── use_cases/           # Multi-step workflows / transaction boundaries
│   │   ├── complete_todo.rs
│   │   ├── create_today_task.rs
│   │   ├── archive_task.rs
│   │   └── sync_task.rs
│   ├── utils/               # TrackError, Result alias
│   └── webui/               # Axum server, routes, SSE, MiniJinja, error mapping
├── templates/               # HTMX HTML templates
├── static/                  # Static assets
└── tests/                   # Integration and CLI handler tests
```

## Architecture

```
CLI (clap) / WebUI (axum)
        ↓
CommandHandler / route handlers
        ↓
Use cases (optional) — multi-service / external-system workflows
        ↓
Services — TaskService, TodoService, …
        ↓
Database (rusqlite, with_transaction)
        ↓
Models (typed enums, task-scoped indices)
```

### Use cases

| Use case | Responsibility |
|----------|----------------|
| `CompleteTodoUseCase` | JJ workspace merge + mark TODO done (CLI & WebUI) |
| `CreateTodayTaskUseCase` | Atomic today-task creation with todo/scrap inheritance |
| `ArchiveTaskUseCase` | Workspace cleanup + task archive with dirty-workspace guard |
| `SyncTaskUseCase` | JJ bookmark sync + pending TODO workspace creation |
| `ApplyTodoActionUseCase` | Routes complete/cancel/make-next through correct paths |

### Agent JSON (`track status --json`)

Adds `workflow`, `todos_agent`, and `guardrails` alongside the existing payload for LLM agents.

### WebUI errors

`webui/error.rs` maps `TrackError` to HTTP status codes (400 validation, 404 not found, 500 default).

### Key patterns

- **Task-scoped IDs**: user-facing `#1`, `#2` per task via `task_index` columns
- **Transactions**: `Database::with_transaction` + `BEGIN IMMEDIATE` for index allocation and today-task creation
- **Status types**: `TaskStatus`, `TodoStatus` enums with explicit transition rules
- **Real-time WebUI**: section revision counters + SSE polling for CLI-originated changes

## CLI command tree (summary)

```
track new | list | switch | status | desc | ticket | archive | sync
track todo { add, list, update, done, workspace, delete, next }
track link { add, list, delete }
track scrap { add, list }
track repo { add, list, remove }
track alias { set, remove }
track config { set-calendar, show }
track completion | llm-help | webui
```

Worktree operations live under `track todo workspace`, `track sync`, and `track repo` — not a separate top-level command.

## Dependencies

See `Cargo.toml`. Core: `clap`, `rusqlite`, `axum`, `minijinja`, `chrono`, `thiserror`.

## Testing

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test
```

Service-layer unit tests live beside implementations; integration tests are in `tests/`. JJ-dependent tests require `jj` on PATH (installed in CI).
