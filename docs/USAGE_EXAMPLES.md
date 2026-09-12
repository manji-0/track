# Usage Examples

<!-- constrained-by ./JJ_INTEGRATION.md -->

Copy-paste workflows for Track. JJ mode is the default: track stores **what** to do; **jj-task** + **`$jj`** create the workspace and commits. See [JJ_INTEGRATION.md](JJ_INTEGRATION.md).

## Bug fix (JJ)

```bash
track new "Fix authentication timeout" \
  --ticket BUG-456 \
  --ticket-url https://github.com/myorg/myrepo/issues/456

track repo add .
track todo add "Fix token refresh logic"
track todo add "Compare expiry settings" --no-workspace

track status --json   # read jj.slug and workflow.next_action
jj-task repo init     # once per repo
jj-task start bug-456
cd "$(jj-task path bug-456)"
# implement + test; use $jj skill for squash/commit/PR

track scrap add --json "Timeout was idle JWT, not server session"
track todo done --json 1

# when workflow.phase is task_complete:
# $jj merge/PR, then:
jj-task done bug-456
track archive t:BUG-456
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
jj-task start feat-789
cd "$(jj-task path feat-789)"
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

Scraps attach to the oldest pending TODO at insert time.

```bash
track todo add "Implement authentication"
track scrap add "Using JWT"
track todo done 1
track scrap list
```

## jj-task slug

Not `task/PROJ-123` from legacy `track sync`. JSON `jj.slug` is:

1. `track alias` if set
2. else sanitized ticket (`PROJ-123` → `proj-123`)
3. else `task-{id}`

```bash
track alias set fix-oauth-refresh
# jj.slug → fix-oauth-refresh
```

## Git mode (optional)

```bash
track config set vcs-mode git
track repo add .
track sync    # creates .worktrees/<slug> on track/<slug>
```

## Legacy per-TODO worktrees

`--worktree` is removed. Existing DB rows: `track migrate legacy-worktrees`, then jj-task.
