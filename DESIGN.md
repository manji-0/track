# WorkTracker CLI Design Specification

## 1. Overview
WorkTracker is a CLI tool for recording and managing developer work logs based on "context" (current work state).
Users can set the current task (WorkTracker) they are working on, allowing them to manage TODOs, record notes, and manage related repositories without specifying IDs each time.

## 2. Technology Stack (Rust)
Rust is adopted to achieve fast operation with a single binary and robust error handling.

| Category | Crate | Purpose |
| :--- | :--- | :--- |
| CLI argument parsing | clap (v4.4+) | Automatic generation of subcommands, flags, and help messages |
| DB operations | rusqlite (bundled) | SQLite connection. Uses bundled feature to reduce system dependencies |
| Path management | directories | XDG Base Directory compliant (~/.local/share/...) path resolution |
| Error handling | anyhow, thiserror | Simplification of error propagation and context addition |
| Date/time | chrono | Timestamp management for work logs |
| Display formatting | prettytable-rs | Table formatting for list display |

## 3. Database Design (SQLite)
Data is stored in `$HOME/.local/share/track/track.db`.

### 3.1. Schema Definition

#### app_state
Holds the current state of the application.
- `key`: TEXT PK (e.g., 'current_task_id')
- `value`: TEXT

#### tasks (Work Context)
- `id`: INTEGER PK
- `name`: TEXT (task name)
- `status`: TEXT (e.g., 'active', 'archived')
- `created_at`: DATETIME

#### todos (TODOs within a task)
- `id`: INTEGER PK
- `task_id`: INTEGER (FK -> tasks.id)
- `content`: TEXT
- `status`: TEXT (e.g., 'pending', 'done')
- `created_at`: DATETIME

#### links (Generic related URLs)
- `id`: INTEGER PK
- `task_id`: INTEGER (FK -> tasks.id)
- `url`: TEXT
- `title`: TEXT
- `created_at`: DATETIME

#### logs (Work records/Scraps)
- `id`: INTEGER PK
- `task_id`: INTEGER (FK -> tasks.id)
- `content`: TEXT
- `created_at`: DATETIME

#### git_items (Related repositories/Worktrees)
- `id`: INTEGER PK
- `task_id`: INTEGER (FK -> tasks.id)
- `path`: TEXT (absolute path of repository)
- `branch`: TEXT (branch name at registration)
- `description`: TEXT (optional)

#### repo_links (Issues/PRs related to repositories)
- `id`: INTEGER PK
- `git_item_id`: INTEGER (FK -> git_items.id)
- `url`: TEXT
- `kind`: TEXT (auto-detected: 'PR', 'Issue', 'Discussion', 'Link')
- `created_at`: DATETIME

## 4. Command Interface Design
Commands are executed with `track` as a prefix.

### 4.1. Context Management (Global Operations)

| Command | Arguments | Behavior |
| :--- | :--- | :--- |
| `track new` | `<name>` | Creates a new task and automatically switches to that task. |
| `track list` | `--all` | Displays a list of recent tasks. Shows `*` for the current task. |
| `track switch` | `<task_id>` | Switches the working task. |
| `track info` | | Displays all information (TODO, Log, Repo, Link) for the current task. |

### 4.2. Task Item Operations
These are executed on the currently switched task.

| Category | Command | Arguments | Behavior |
| :--- | :--- | :--- | :--- |
| **TODO** | `track todo add` | `<text>` | Adds a TODO. |
| | `track todo list` | | Displays TODO list. |
| | `track todo update` | `<id> <status>` | Updates status (e.g., done). |
| | `track todo delete` | `<id>` | Deletes a TODO. |
| **Link** | `track link add` | `<url> [title]` | Adds a reference URL. |
| | `track link list` | | Displays link list. |
| **Log** | `track log add` | `<content>` | Adds a work log (Scrap). |
| | `track log list` | | Displays logs in chronological order. |

### 4.3. Git Repository Integration (repo)

| Command | Arguments | Behavior |
| :--- | :--- | :--- |
| `track repo add` | `[path]` | Registers the specified path (current directory if omitted) as a Git item. Internally calls `git rev-parse` to automatically save the branch name. |
| `track repo list` | | Displays registered repositories and their associated Issues/PRs. |
| `track repo link` | `<repo_id> <url>` | Links a URL to the specified repository item. Automatically detects PR, Issue, etc. from URL pattern. |
| `track repo delete` | `<repo_id>` | Unregisters a repository. |

## 5. Logic Details

### Automatic Context Switch
When `track new` is executed, after INSERT to the database, the `current_task_id` in the `app_state` table is immediately updated.
Subsequent `add` commands retrieve the ID from `app_state` and use it as a foreign key.

### Git Information Retrieval
When `track repo add` is executed, git commands are executed as subprocesses using Rust's `std::process::Command`.
Retrieval command: `git -C <path> rev-parse --abbrev-ref HEAD`
This eliminates dependency on Git libraries (git2), optimizing build time and binary size.

### URL Type Inference
When a URL is passed to `track repo link`, the `kind` is determined by the following string matching:
- `/pull/` or `/merge_requests/` -> `PR`
- `/issues/` -> `Issue`
- `/discussions/` -> `Discussion`
- Others -> `Link`