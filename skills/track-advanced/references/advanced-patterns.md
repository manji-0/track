# Advanced Workflows — Detailed Patterns

Advanced patterns for multi-repo, parallel, and long-running work.

**Skill:** [track-advanced](../SKILL.md) · **Setup:** [track-task-setup](../../track-task-setup/SKILL.md) · **Execute:** [track-task-execute](../../track-task-execute/SKILL.md)

## Multi-Repository Tasks

```bash
track new "Cross-service feature" --ticket PROJ-789
track repo add /path/to/frontend
track repo add /path/to/backend
track todo add "Add sync API endpoint" --worktree
track todo add "Implement sync client" --worktree
track sync
```

Work each TODO via **track-task-execute** — `track sync` touches all repos.

---

## Parallel Development

**Terminal 1:**
```bash
cd "$(track todo workspace 2)"
# ... work ...
track todo done 2
```

**Terminal 2:**
```bash
cd "$(track todo workspace 3)"
# ... work ...
track todo done 3
```

---

## Ticket-Based Organization

```bash
track new "Fix memory leak" --ticket GH-456
track switch t:PROJ-123
track archive t:PROJ-123
```

Bookmarks: `task/<ticket>` and `task/<ticket>-todo-<n>`.

---

## Hotfix Workflow

```bash
track new "Fix critical auth bug" --ticket BUG-999
track todo add "Fix token refresh logic" --worktree
track sync
cd "$(track todo workspace 1)"
# fix, test, describe, done
track todo done 1
jj git push --bookmark task/BUG-999
```

---

## Research and Planning Tasks

```bash
track new "Research authentication providers"
track todo add "Compare Okta vs Auth0"    # no --worktree
track scrap add "Finding: ..."
track todo done 1
```

---

## Team Collaboration

```bash
jj git push --bookmark task/PROJ-123
# teammate:
track switch t:PROJ-123
track status --json
track scrap list
```

---

## Archive Completed Task

```bash
track status --json   # confirm task_complete
track archive
```

Dirty workspaces block archive — commit or discard first.

---

## Best Practices

1. Use tickets for consistent bookmark naming
2. Use `--worktree` for complex or parallel work
3. Describe frequently with `jj describe`
4. Document decisions with `track scrap add`
5. Read `track status --json` before and after work
6. Archive completed tasks regularly
