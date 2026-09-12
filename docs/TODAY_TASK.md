# Today Task

<!-- constrained-by ../DESIGN.md#Implicit current task -->
<!-- dagayn: implemented-by src/use_cases/create_today_task.rs::CreateTodayTaskUseCase -->

`track switch today` switches to a **daily personal inbox**: one task named `Today: YYYY-MM-DD` (local date). It is the same kind of task as any other — TODOs, scraps, links — not a separate product.

Today task is **not** a planner graph and **not** a jj-task workspace by itself. Add `track repo add` / workspace-requiring TODOs only when that day's work needs a coding workspace.

## Switching

```bash
track switch today
```

If today's task already exists and is `active`, track reuses it and sets `current_task_id`.

Otherwise `CreateTodayTaskUseCase` runs in one transaction:

1. Find the previous row with `is_today_task = 1` and `status = active`.
2. Clear `is_today_task` on all tasks.
3. Insert `Today: YYYY-MM-DD` with `is_today_task = 1`.
4. Copy **incomplete** TODOs from the previous today task (status reset to pending).
5. Copy scraps that were linked to those TODOs, remapping `active_todo_id`.
6. Set `current_task_id` to the new row.

Done/cancelled TODOs stay on the old day. The old today task remains in the list until you archive it.

## Web UI

The today-task view can embed a Google Calendar iframe when a calendar ID is stored:

```bash
track config set-calendar <calendar-id>
track config show
track webui
```

Share the calendar with whatever identity the browser uses. The ID lives in `app_state.calendar_id`.

## Related commands

| Command | Role |
|---------|------|
| `track switch today` | Get or create today's task |
| `track todo add` / `done` | Same as any task |
| `track scrap add` | Notes; linked to the current pending TODO when present |
| `track archive` | Close a past today task when you no longer need it |

## Implementation

- Flag: `tasks.is_today_task`
- Use case: `src/use_cases/create_today_task.rs`
- Switch keyword: `today` in `handle_switch`
