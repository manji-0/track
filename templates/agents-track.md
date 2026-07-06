## Track task management

Use [track](https://github.com/manji0/track) for development task / TODO context when this repo (or your workflow) uses it.

### Session start (mandatory)

1. Run `track status --json` and read `workflow.phase`, `workflow.next_action`, and `jj.slug` (if present).
2. Follow `workflow.next_action.command` — do not guess the next step.
3. For JJ mode: use **agent-skill-jj** (`$jj` skill + `jj-task`) for all commits and PR work.

### Two-layer stack

| Layer | Tool | Responsibility |
|-------|------|----------------|
| WHAT | **track** | Tasks, TODOs, scraps, workflow phase |
| HOW | **$jj** (agent-skill-jj) | jj-task workspace, squash/commit, PR, push |

### Agent loop

```bash
track status --json
jj-task start <jj.slug>          # when workflow.phase is sync_required
cd "$(jj-task path <jj.slug>)"   # work only in the jj-task workspace
# implement + test; commits via $jj skill
track scrap add "<note>"
track todo done <index>
```

### Skills (recommended)

```bash
npx skills add manji-0/track \
  -s track -s track-task-setup -s track-task-execute -s track-advanced \
  -g -a cursor -a claude-code -a codex -y
npx skills add manji-0/agent-skill-jj -s jj -g -y
```

Route by `workflow.phase`: `setup` → track-task-setup; `sync_required` / `execute` → track-task-execute; `task_complete` → track-advanced.

### References

- `track llm-help` — full CLI workflow for agents
- `skills/INSTALL.md` in the track repo — skill install guide
- Re-run `track install agents --global` to refresh this section
