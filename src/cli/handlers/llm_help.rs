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
| `aggressive` | per-task revision + published work record as git notes |
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
When on, each (task, repo) gets **one empty marker revision**. Commit history on `track/<slug>` is **one commit per completed TODO** (same order). Git notes (`refs/notes/track`) on the marker hold name / description / ticket / links. Each TODO's **shared** scraps are notes on **that commit**. `track scrap add` is local by default; use `--share` or `track scrap share N` for decisions. After a TODO commit is on origin, do not amend it — add a follow-up TODO (reopen is forbidden). `track notes push` discloses the ref; others `track notes fetch` then `track import` on the PR branch. Track DB remains source of truth; notes failures warn on stderr.

## Loop

1. `track status --json` (or use the last `--json` mutation)
2. Run `workflow.next_action.command` / `hint.next_command`
3. Implement in the workspace path from JSON
4. `track scrap add --json "..."` (add `--share` for decisions that should travel with the PR)
5. `track todo done --json N`
6. Repeat until `task_complete`
7. Push/merge the PR, then `track archive`

## Commands

| Command | Notes |
|---------|-------|
| `track new <name>` | Create + switch. `--ticket`, `--template`, `--json` |
| `track switch <ref>` | id, `t:TICKET`, `a:alias`, `today`. Ensures workspace when repos exist |
| `track status [--json]` | Snapshot + hint |
| `track todo add/done/update/next/delete` | Delete needs `--force` off-TTY |
| `track scrap add/list/share/unshare` | Work notes (`--share` = notes on that TODO's commit) |
| `track import` | Restore task from git notes on this branch |
| `track notes push/fetch` | Disclose / receive `refs/notes/track` |
| `track repo add [path]` | Register + create workspace |
| `track sync [--legacy]` | Ensure task workspace (legacy = old per-TODO jj worktrees) |
| `track archive [--force]` | Remove workspaces (branch/bookmark kept for PRs). `--force` skips dirty checks |
| `track config set vcs-mode git\|jj` | Switch VCS backend |
| `track config set aggressive-mode on\|off` | Task revision + published work record |
| `track config show` | Print current settings |

## Slug

`alias` → sanitized `ticket_id` → `task-<id>`. Set `track alias set <slug>` when the ticket is a bad workspace name.

## Guardrails

- Never reopen a done/cancelled TODO — add a new TODO.
- Never rewrite a published TODO commit or its notes; follow-up review is a new TODO.
- `track todo delete N --force` off-TTY.
- `track archive --force` only to skip dirty-workspace checks.
- `--worktree` on `todo add` is removed.
- `track migrate legacy-worktrees` clears old per-TODO worktree flags.

Install track skills with `npx skills add ./skills` — they tell you to follow `hint` / `next_action`.
"#
    );
    Ok(())
}
