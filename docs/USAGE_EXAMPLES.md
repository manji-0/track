# Usage Examples

<!-- constrained-by ./JJ_INTEGRATION.md -->

Copy-paste workflows for Track. **Git mode is the default**: track stores **what** to do and **creates** `.worktrees/<slug>` on `track/<slug>`. Switch with `track config set vcs-mode git|jj`. See [JJ_INTEGRATION.md](JJ_INTEGRATION.md).

## Bug fix (git, default)

```bash
track new "Fix authentication timeout" \
  --ticket BUG-456 \
  --ticket-url https://github.com/myorg/myrepo/issues/456

track repo add .
track todo add "Fix token refresh logic"
track todo add "Compare expiry settings" --no-workspace

track status --json   # read hint / workflow.next_action
# footer: next: cd "/path/to/repo/.worktrees/bug-456"
# implement + test; commit in that worktree; git push -u origin track/bug-456

track scrap add --json "Timeout was idle JWT, not server session"
track todo done --json 1

# when workflow.phase is task_complete:
# push/merge PR, then:
track archive t:BUG-456
```

## Same flow in jj mode

```bash
track config set vcs-mode jj
track repo add .          # colocates if needed; creates .worktrees/<slug>
# commit with jj in the workspace; PR head is bookmark track/<slug>
# jj git push --named track/<slug>
```

## Feature with several TODOs

```bash
track new "Add user profile page" --ticket FEAT-789
track repo add .
track todo add "Design profile UI mockup" --no-workspace
track todo add "Implement backend API"
track todo add "Create frontend components"
track link add https://figma.com/design/profile --title "UI Design"

track todo done --json 1          # research, no workspace
# one workspace for the whole task — not one per TODO

track scrap add --json "Postgres for profile rows"
track todo done --json 2
track todo done --json 3
track status --json
```

## Several personal tasks

```bash
track new "Refactor authentication module" --ticket TECH-101
track new "Update documentation" --ticket DOC-202

track list --json
track switch t:TECH-101
track todo add "Extract auth logic"
track scrap add "Current code is in src/auth/legacy.rs"

track switch t:DOC-202
track todo add "Update API documentation"
track status
```

## Aliases

```bash
track alias set daily-report
track switch a:daily-report
track alias set daily-report --force   # move alias from another task
track alias remove
```

References resolve in this order: numeric id, `t:<ticket>`, alias.

## Templates

```bash
track new "Daily Report Template"
track alias set daily-template
track todo add "Collect metrics"
track todo add "Write summary"

track new "Daily Report 2026-09-13" --template daily-template
track new "Daily Report 2026-09-14" --template a:daily-template
```

## Todo order

```bash
track todo add "Review backlog"
track todo add "Estimate stories"
track todo add "Update roadmap"
track todo next 3    # roadmap becomes #1
```

## Scraps

Scraps attach to the oldest pending TODO at insert time. Local is the default. With `track config set aggressive-mode on`, **shared** scraps (`--share` / `track scrap share`) are git notes on **that TODO's commit**, not on the marker. See [JJ_INTEGRATION.md](JJ_INTEGRATION.md#commit--notes-strategy).

```bash
track todo add "Implement authentication"
track scrap add "Using JWT"
track todo done 1
track scrap list
```

## Workspace slug

JSON `git.slug` / `jj.slug` is:

1. `track alias` if set
2. else sanitized ticket (`PROJ-123` → `proj-123`)
3. else `task-{id}`

```bash
track alias set fix-oauth-refresh
```

The directory is always `.worktrees/<slug>/` and the PR head is `track/<slug>`.

## Aggressive mode

```bash
track config set aggressive-mode on
track repo add .     # creates an empty task revision
track scrap add --share "decision"   # git notes on the matching TODO commit
track notes push
# other machine, on the PR branch:
track notes fetch
track import
```

## Legacy per-TODO worktrees

`--worktree` is removed. Existing DB rows: `track migrate legacy-worktrees`, then `track sync`.
