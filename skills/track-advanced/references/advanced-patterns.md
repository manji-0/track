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

Each repo: `jj-task repo init` once, then `jj-task start <slug>` per repo (same slug, separate jj-task maps).

Work through TODOs sequentially in the jj-task workspace(s) via **track-task-execute** and **`$jj`**.

---

## Hotfix Workflow

```bash
track new "Fix critical auth bug" --ticket BUG-999
track alias set fix-auth-bug
track todo add "Fix token refresh logic"
jj-task start fix-auth-bug
cd "$(jj-task path fix-auth-bug)"
# fix, test — $jj skill for squash/commit/PR
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
# $jj skill: push bookmark, open PR
track status --json    # share jj.slug + TODOs
track scrap list
```

Teammate switches task and continues in their jj-task workspace:

```bash
track switch t:PROJ-123
track status --json
jj-task start <jj.slug>
cd "$(jj-task path <jj.slug>)"
```

---

## Archive Completed Task

```bash
track status --json   # confirm task_complete
# $jj skill: merge PR if needed
jj-task done <jj.slug>
track archive
```

`track archive` validates jj-task phase and dirty workspaces. Use `track archive --force` only when you intentionally skip those checks.

---

## Best Practices

1. Use tickets or `track alias set` for stable jj-task slugs
2. One jj-task workspace per track task — sequential TODOs, not per-TODO workspaces
3. All jj commit/PR work through **`$jj`** skill
4. Document decisions with `track scrap add`
5. Read `track status --json` before and after work — follow `workflow.checklist`
6. Archive completed tasks regularly
