# Advanced Workflows — Detailed Patterns

Advanced patterns for multi-repo, hotfixes, and long-running work.

**Skill:** [track-advanced](../SKILL.md) · **Setup:** [track-task-setup](../../track-task-setup/SKILL.md) · **Execute:** [track-task-execute](../../track-task-execute/SKILL.md)

## Multi-Repository Tasks

```bash
track new "Cross-service feature" --ticket PROJ-789
track repo add /path/to/frontend
track repo add /path/to/backend
track todo add "Add sync API endpoint"
track todo add "Implement sync client"
```

Each repo gets `.worktrees/<slug>` on `track/<slug>` (same slug). Work through TODOs sequentially via **track-task-execute**.

---

## Hotfix Workflow

```bash
track new "Fix critical auth bug" --ticket BUG-999
track alias set fix-auth-bug
track todo add "Fix token refresh logic"
# follow hint: cd .worktrees/fix-auth-bug
# fix, test, commit, push track/fix-auth-bug
track todo done 1
```

---

## Research and Planning Tasks

```bash
track new "Research authentication providers"
track todo add "Compare Okta vs Auth0" --no-workspace
track scrap add "Finding: ..."
track todo done 1
```

Research TODOs skip workspace sync and go straight to `execute`.

---

## Team Collaboration

```bash
# push track/<slug>, open PR
track status --json    # share slug + TODOs
track scrap list
```

Teammate switches task and continues in the track-owned workspace:

```bash
track switch t:PROJ-123
track status --json
track sync             # if needed
```

---

## Archive Completed Task

```bash
track status --json   # confirm task_complete
# merge PR if needed
track archive
```

`track archive` checks dirty workspaces. On a TTY it prompts; without a TTY it errors instead of hanging. Use `track archive --force` only when you intentionally skip those checks. Branch/bookmark `track/<slug>` is kept for PRs.

---

## Best Practices

1. Use tickets or `track alias set` for stable slugs
2. One workspace per track task — sequential TODOs, not per-TODO workspaces
3. Follow `hint` / `workflow.next_action` instead of inventing git/jj workspace commands
4. Document decisions with `track scrap add`
5. Read `track status --json` before and after work
6. Archive completed tasks regularly
