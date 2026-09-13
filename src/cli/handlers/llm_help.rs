use crate::cli::handlers::CommandCtx;
use crate::utils::Result;

pub fn handle_llm_help(_ctx: &CommandCtx) -> Result<()> {
    println!(
        r#"# Track CLI Help for LLM Agents

Track owns **what** you are doing and **where** the coding workspace lives.
Follow `workflow.next_action` and the `hint` object — do not look up jj-task or other external maps.

## First step

```bash
track status --json
```

Human commands also print a stderr footer:

```
hint: vcs=git aggressive=off | workspace ready at /repo/.worktrees/slug
next: cd "/repo/.worktrees/slug"
```

Set `TRACK_HINTS=0` to hide the footer. `--json` already includes `hint`.

| Field | Use |
|-------|-----|
| `vcs_mode` | `git` (default) or `jj` |
| `aggressive` | per-task revision + scraps as git notes |
| `workflow.phase` | setup · sync_required · execute · task_complete · archived |
| `workflow.next_action.command` | Next command to run |
| `hint.post_state` / `hint.next_command` | Same as the stderr footer |
| `git.workspace_path` / `jj.workspace_path` | `.worktrees/<slug>` |
| `todos_agent[].is_next` | Current TODO |

Mutating commands accept `--json` and return the **same snapshot** plus `ok` and `mutation`. Prefer `--json` on writes so you do not need a second `status` call.

## Workspace (track owns this)

Layout is the same in both modes:

- path: `<repo>/.worktrees/<slug>/`
- git branch / jj bookmark: `track/<slug>` (GitHub PR head)

```bash
track config set vcs-mode git          # default
track config set vcs-mode jj           # colocated jj
track config set aggressive-mode on    # task revision + git notes
track repo add .                       # registers repo and creates the workspace
track sync                             # ensure workspace exists
```

Work **only** in the task workspace, never at repo root.

### Git mode
Commit and push from the worktree: `git push -u origin track/<slug>` then `gh pr create`.

### JJ mode
Track runs `jj git init --colocate` when needed and `jj workspace add`.
Push the PR bookmark: `jj git push --named track/<slug>` then `gh pr create`.

### Aggressive mode
When on, each (task, repo) gets **one empty marker revision**. `track scrap add` writes `refs/notes/track` on that marker (not HEAD). Git: marker is the first empty commit on `track/<slug>`. JJ: marker is a parent change; bookmark `track/<slug>` stays on the working-copy child (PR head). Turning it on later: `track sync` backfills a marker without moving an existing one. Notes failures warn on stderr; the track DB stays source of truth.

## Loop

1. `track status --json` (or use the last `--json` mutation)
2. Run `workflow.next_action.command` / `hint.next_command`
3. Implement in the workspace path from JSON
4. `track scrap add --json "..."` and `track todo done --json N`
5. Repeat until `task_complete`
6. Push/merge the PR, then `track archive`

## Commands

| Command | Notes |
|---------|-------|
| `track new <name>` | Create + switch. `--ticket`, `--template`, `--json` |
| `track switch <ref>` | id, `t:TICKET`, `a:alias`, `today`. Ensures workspace when repos exist |
| `track status [--json]` | Snapshot + hint |
| `track todo add/done/update/next/delete` | Delete needs `--force` off-TTY |
| `track scrap add/list` | Work notes (git notes when aggressive) |
| `track repo add [path]` | Register + create workspace |
| `track sync [--legacy]` | Ensure task workspace (legacy = old per-TODO jj worktrees) |
| `track archive [--force]` | Remove workspaces (branch/bookmark kept for PRs). `--force` skips dirty checks |
| `track config set vcs-mode git\|jj` | Switch VCS backend |
| `track config set aggressive-mode on\|off` | Task revision + git notes |
| `track config show` | Print current settings |

## Slug

`alias` → sanitized `ticket_id` → `task-<id>`. Set `track alias set <slug>` when the ticket is a bad workspace name.

## Guardrails

- Never reopen a done/cancelled TODO — add a new TODO.
- `track todo delete N --force` off-TTY.
- `track archive --force` only to skip dirty-workspace checks.
- `--worktree` on `todo add` is removed.
- `track migrate legacy-worktrees` clears old per-TODO worktree flags.

Install track skills with `npx skills add ./skills` — they tell you to follow `hint` / `next_action`.
"#
    );
    Ok(())
}
