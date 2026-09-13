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

### Git mode (`vcs-mode=git`)

- `git worktree add` at `<repo>/.worktrees/<slug>/` on branch `track/<slug>`
- Best-effort fetch of the base branch
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

When `aggressive-mode` is `on`:

1. Each `(task, repo)` gets **one empty marker revision**, recorded in `task_revisions`. The stored git commit is immutable.
2. **Git:** empty commit on `track/<slug>` at workspace birth (ancestor of later work; included in the PR).
3. **JJ:** empty change described `[track:<slug>] <name>`. Working copy is a **child**. Bookmark `track/<slug>` starts on the working copy; after `todo done` it sits on the last TODO commit (`@-`), not the marker or the empty working copy.
4. Git notes (`refs/notes/track`): **marker** = name / description / ticket / links (frozen once published). **Each TODO commit** holds that TODO's shared scraps. `track scrap add` is local by default. Follow-up review is a new TODO + new commit + new notes (fast-forward only). `track notes push` / `fetch` move the ref. `track import` walks `marker..HEAD` for `Task-Todo` commits.
5. Turning aggressive on after a workspace exists: `track sync` backfills a marker if none is stored. Turning it off stops writing notes; marker and notes stay.

Inspired by [jjtask](https://github.com/Coobaha/jjtask) (per-task empty revisions + notes), implemented entirely inside track.

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

- [LLM_INTEGRATION.md](LLM_INTEGRATION.md) — agent overview
- [skills/INSTALL.md](../skills/INSTALL.md) — optional skills (thin; follow `hint`)
