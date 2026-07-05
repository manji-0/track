---
name: track-task-management
description: DEPRECATED — use split track skills instead. Redirects to track (router), track-task-setup, track-task-execute, and track-advanced.
license: MIT
compatibility: Requires track CLI installed
metadata:
  author: track
  version: 2.0.0
  deprecated: true
  tags: [deprecated, task-management]
---

# Track Task Management (Deprecated)

This monolithic skill was split into purpose-specific skills in v2.0. **Install the new skills instead.**

## Install (recommended)

```bash
npx skills add ./skills \
  -s track -s track-task-setup -s track-task-execute -s track-advanced \
  -g -a cursor -a claude-code -a codex -y
```

See [../INSTALL.md](../INSTALL.md).

## Where did content go?

| Old section | New skill |
|-------------|-----------|
| Quick start / creating tasks | [track-task-setup](../track-task-setup/SKILL.md) |
| Agent execution loop | [track-task-execute](../track-task-execute/SKILL.md) |
| Multi-repo / parallel / archive | [track-advanced](../track-advanced/SKILL.md) |
| Routing by `workflow.phase` | [track](../track/SKILL.md) |

## References (moved)

| Old path | New path |
|----------|----------|
| `references/creating-tasks.md` | [track-task-setup/references/setup-workflow.md](../track-task-setup/references/setup-workflow.md) |
| `references/executing-tasks.md` | [track-task-execute/references/execution-workflow.md](../track-task-execute/references/execution-workflow.md) |
| `references/advanced-workflows.md` | [track-advanced/references/advanced-patterns.md](../track-advanced/references/advanced-patterns.md) |

## Migrate

```bash
npx skills remove track-task-management -y
npx skills add ./skills \
  -s track -s track-task-setup -s track-task-execute -s track-advanced \
  -g -a cursor -a claude-code -a codex -y
```
