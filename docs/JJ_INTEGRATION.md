# VCS Integration

Track owns the **coding workspace**. Git or jj is a switchable backend; agents and humans do not run `git worktree`, `jj workspace add`, or jj-task by hand.

| Layer | Tool | Responsibility |
|-------|------|----------------|
| **WHAT** | `track` | Tasks, TODOs, scraps, tickets, JSON workflow + hints |
| **WHERE** | `track` | `.worktrees/<slug>/` on branch/bookmark `track/<slug>` |
| **HOW** | git or jj (in that workspace) | Commits and GitHub PRs |

```bash
track config set vcs-mode git          # default
track config set vcs-mode jj           # colocated jj
track config set aggressive-mode on    # per-task revision + git notes
```

Existing databases that already had tasks and never set `vcs-mode` stay on **jj** so current jj users are not flipped silently. New databases default to **git**.

## Agent loop

```
track status --json     →  workflow.phase + hint + next_action
        ↓
track repo add . / track sync   →  workspace at .worktrees/<slug>/
        ↓
cd "<workspace_path>"   →  implement (not main / repo root)
        ↓
git commit / jj commit from that workspace
        ↓
track scrap add --json   →  local journal (`--share` = published snapshot)
track notes push         →  disclose refs/notes/track
track import             →  restore a task from notes on this branch
        ↓
track todo done N --json →  mark TODO complete; follow returned next_action
        ↓
repeat until task_complete
        ↓
push track/<slug>, open/merge PR, track archive
```

Human commands print the same next step on **stderr**:

```
hint: vcs=git aggressive=off | workspace ready at /repo/.worktrees/slug
next: cd "/repo/.worktrees/slug"
```

`--json` includes `hint`. Set `TRACK_HINTS=0` to hide the footer.

## Division of responsibility

### Track owns

- Task / TODO lifecycle (`track new`, `track todo add/done`)
- Scraps, links, tickets, aliases
- Workspace create/remove (`track repo add`, `track sync`, `track archive`, `track switch`)
- `track status --json` / mutation `--json` / `GET /api/status` — `workflow`, `hint`, `git`/`jj`, `todos_agent`, `guardrails`
- Aggressive mode: empty task revision + `refs/notes/track`
- WebUI

### Workspace base

A new task workspace starts from `origin` when possible. Track first runs a best-effort `git fetch origin` (git mode) or `jj git fetch --remote origin` (jj mode), then picks the base:

1. A registered base branch/bookmark (`track repo add --base`, or the bookmark on the current revision at `repo add`) uses its remote counterpart: `origin/<name>` in git mode, `<name>@origin` in jj mode.
2. With no registered name, the remote default branch wins (`origin/HEAD`, then `main`, then `master`).
3. When no such remote ref exists (offline, or no `origin` remote), track falls back to the local base: the registered name, the commit recorded at `repo add`, or `HEAD` / `@`.

Only the local fallback refuses a base checkout with uncommitted changes; a remote base cannot pick them up, so a dirty `main` checkout no longer blocks workspace creation. To start from unpushed local commits, register a base that has no `origin` counterpart (for example a commit id). The git task branch is created with `--no-track`, so it never inherits `origin/<base>` as its upstream.

### Git mode (`vcs-mode=git`)

- `git worktree add` at `<repo>/.worktrees/<slug>/` on branch `track/<slug>`
- Base selection as described in [Workspace base](#workspace-base)
- `.worktrees/` is excluded locally (`.git/info/exclude`), not via a committed `.gitignore`
- PR head: `git push -u origin track/<slug>` then `gh pr create`

### JJ mode (`vcs-mode=jj`)

- Ensures a colocated repo (`jj git init --colocate` when the path is git-only)
- `jj workspace add` at `<repo>/.worktrees/<slug>/`
- Bookmark `track/<slug>` is the GitHub PR head (`jj git push --named track/<slug>`)
- Git collocate stays enabled so `gh` and git remotes keep working

Track does **not** read `~/.config/jj/task-workspaces.json` or invoke `jj-task`.

### Workspace slug

Same in both modes (`git.slug` / `jj.slug`):

1. `track alias` if set
2. else `ticket_id` (sanitized, e.g. `PROJ-123` → `proj-123`)
3. else `task-{id}`

```bash
track alias set fix-oauth-refresh
```

### Aggressive mode

`track config set aggressive-mode on` is the **commit + notes** strategy. Off (default) still creates the workspace; you commit with git or jj yourself. On, Track aligns history with TODOs and publishes a replayable work record as git notes.

Inspired by [jjtask](https://github.com/Coobaha/jjtask) (per-task empty revisions + notes), implemented entirely inside track.

## Commit + notes strategy

The SQLite DB remains the source of truth. Notes failures warn on stderr; they do not roll back the task/TODO write. Set `TRACK_HINTS=0` to hide the human footer; `--json` still includes `hint`.

### Two layers on `track/<slug>`

```
main (or the repo's base)
 └── empty marker commit          git notes: track-task v3 (identity)
      ├── TODO 1 commit           git notes: track-todo (shared scraps)
      ├── TODO 2 commit           git notes: track-todo
      └── … follow-up TODOs only (no rewrite of published SHAs)
```

1. **Marker (identity).** Each `(task, repo)` gets **one empty marker revision**, stored in `task_revisions`. The SHA is insert-once.
   - **Git:** empty commit on `track/<slug>` at workspace birth (ancestor of later work; included in the PR).
   - **JJ:** empty change described `[track:<slug>] <name>`. The working copy is a **described wip child** so `jj git push` is never blocked by an empty description. Bookmark `track/<slug>` sits on the last TODO commit (`@-`).
2. **One commit per completed TODO**, in TODO order. `track todo done` folds unpublished WIP in the **task workspace only** (files dirty at the repo root stay on `main`) into that commit. Trailers in the message:
   - `Task-Todo: <index>`
   - `Task-Slug: <slug>`
3. **Git notes** (`refs/notes/track`) on those commits — not a second git history.

### What `todo done` folds

- **Git:** `git reset --soft` back to the last TODO commit (or the marker), then one `--allow-empty` commit.
- **JJ:** unpublished `jj new` experiments are folded into one described TODO (parity with git); then a fresh wip child.
- A **merge from `main`** is not folded away. The follow-up TODO **appends on top** of the merge.
- A **conflicted** workspace is refused. Resolve, then add a follow-up TODO (reopen is forbidden).
- Unique commits already on origin (`refs/remotes/origin/track/<slug>` or jj `track/<slug>@origin`) are not rewritten. The local colocated `{bookmark}@git` export is **not** origin.

### Notes payloads

| Object | Format | Contents |
|--------|--------|----------|
| Marker | `track-task` v3 | slug, name, description, ticket, links. Frozen once that marker SHA is published. TODOs/scraps are **not** on the marker in v3. |
| TODO commit | `track-todo` v1 | slug, todo index/content/status, **shared** scraps for that TODO |

`track scrap add` is **local** by default. Publish decisions with `track scrap add --share` or `track scrap share N`. `unshare` keeps a scrap in the DB only. Older whole-task blobs (`track-task` v2) still import.

`git notes add -f` is used only on **unpublished** commits. After a TODO commit is on origin, do not amend it — add a follow-up TODO + commit + notes (fast-forward).

### Disclose, fetch, import

```bash
track notes push [--remote origin]    # disclose refs/notes/track
track notes fetch [--remote origin]   # receive the ref
track import [path]                   # restore a local task from notes on this branch
```

- Push first projects the current task's notes, fetches the remote ref, then pushes. It **refuses** to rewrite a note blob that already exists on the remote for the same commit.
- Import walks the task tip (`HEAD` or jj `@`) for `Task-Todo` commits and uses **this line's marker**. After a sibling task has landed on `main`, import on this branch ignores TODO commits already on trunk.
- Git helpers use the colocated object store — they do not mistake the parent repo's `HEAD` (usually `main`) for the task tip from a jj workspace.
- Others do not need the author's `track.db`. Fetch notes, check out the PR branch, `track import`.

### Turning aggressive on or off later

- On after a workspace exists: `track sync` backfills a marker **under this workspace's unpublished commits** (new empty child of trunk, then rebase this line only — other task workspaces are not reparented). Published unique SHAs stay put.
- Off stops writing notes; existing marker and notes stay.

### Agent loop with notes

```
track config set aggressive-mode on
track repo add . / track sync     # marker at workspace birth
cd "<workspace>"                  # implement
track scrap add --share "…"       # optional published decision
track todo done N --json          # fold WIP → one TODO commit + track-todo notes
track notes push                  # disclose refs/notes/track with the PR
# reviewer / other machine:
track notes fetch && track import
```

Without aggressive mode, skip notes: commit with git or jj in the workspace, then `track todo done` only updates the DB.

## Implementation reference

Workflow phase and `next_action` are pure functions in
[`src/models/workflow.rs`](../src/models/workflow.rs), given observed `WorkspaceFacts`.
Filesystem observation lives in [`src/services/agent_context.rs`](../src/services/agent_context.rs).
Workspace create/remove: [`src/services/task_workspace.rs`](../src/services/task_workspace.rs).
Git notes / task revisions: [`src/services/git_notes.rs`](../src/services/git_notes.rs), [`src/services/task_revision.rs`](../src/services/task_revision.rs).

## JSON fields (`track status --json`)

```json
{
  "vcs_mode": "git",
  "aggressive": false,
  "workflow": {
    "phase": "sync_required",
    "next_action": { "command": "track sync", "reason": "Create git worktree at .worktrees/proj-123 on branch/bookmark track/proj-123" },
    "checklist": [
      { "id": "sync", "label": "Create git worktree for this task", "done": false, "command": "track sync" }
    ]
  },
  "hint": {
    "vcs_mode": "git",
    "aggressive": false,
    "post_state": "workspace missing at /repo/.worktrees/proj-123 — run track sync",
    "next_command": "track sync",
    "next_reason": "Create git worktree at .worktrees/proj-123 on branch/bookmark track/proj-123"
  },
  "git": {
    "slug": "proj-123",
    "branch": "track/proj-123",
    "workspace_ready": false,
    "workspace_path": "/repo/.worktrees/proj-123",
    "sync_command": "track sync"
  },
  "guardrails": {
    "must_use_jj_skill": false,
    "jj_skill_name": "jj",
    "reopen_forbidden": true,
    "complete_requires_jj_merge": false
  }
}
```

In jj mode the `git` object is omitted and `jj` is present (`start_command` is `track sync`, `path_command` is `cd "<path>"`). `must_use_jj_skill` is always `false`.

| Phase | Track action |
|-------|----------------|
| `setup` | `track repo add`, `track todo add` |
| `sync_required` | `track sync` (creates the workspace) |
| `execute` | `cd` into the workspace, `track scrap add`, `track todo done` |
| `task_complete` | push `track/<slug>`, merge PR, `track archive` |

Research TODOs: `track todo add "..." --no-workspace`

## Legacy: per-TODO `--worktree`

`track todo add --worktree` was **removed**. Existing DB rows with `worktree_requested` still work until migrated:

```bash
track migrate legacy-worktrees --dry-run
track migrate legacy-worktrees
track migrate legacy-worktrees --force
track sync
```

`track sync --legacy` still rebuilds old per-TODO jj worktrees.

## What changed from the jj-task stack

| Old (jj-task) | Now (track-owned) |
|----------------|-------------------|
| `jj-task start <slug>` | `track sync` / `track repo add` |
| `jj-task path <slug>` | `hint.next_command` / `cd "<workspace_path>"` |
| `~/.config/jj/task-workspaces.json` | Track DB + filesystem |
| `$jj` skill required for workspace | Follow stderr hint / JSON `hint` |
| Per-TODO workspaces | One workspace per track task |

## References

- [CLI.md](CLI.md) — command tables (`notes`, `import`, `config`)
- [USAGE_EXAMPLES.md](USAGE_EXAMPLES.md) — copy-paste including aggressive mode
- [LLM_INTEGRATION.md](LLM_INTEGRATION.md) — agent overview
- [skills/INSTALL.md](../skills/INSTALL.md) — optional skills (thin; follow `hint`)
