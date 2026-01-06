# Today Task Feature

## Overview

The "Today Task" is a special task type designed for daily task management. It automatically creates a new task for each day and inherits incomplete work from the previous day.

## Key Features

### 1. Automatic Task Creation

When you run `track switch today`, the system:
- Checks if a today task exists for the current date
- If not, creates a new task named "Today: YYYY-MM-DD"
- Automatically switches to this task

### 2. Inheritance from Previous Day

When creating a new today task, the system automatically:
- Copies all **pending** (incomplete) TODOs from the previous day's today task
- Copies **scraps** that are linked to the inherited TODOs
- Maintains the todo-scrap linkage with updated references

### 3. Specialized Web UI

The today task has a customized Web UI layout:
- **Calendar View**: Displays your Google Calendar (when configured)
- **Links Section**: Shows reference links side-by-side with the calendar
- **No Description/Ticket/Repository Cards**: Simplified layout focused on daily tasks
- **Dark Mode Support**: Calendar automatically adapts to the current theme

## Usage

### Basic Workflow

```bash
# Switch to today's task (creates if doesn't exist)
track switch today

# Add TODOs for today
track todo add "Review pull requests"
track todo add "Update documentation"
track todo add "Team meeting at 2pm"

# Add work notes
track scrap add "Meeting notes: discussed new feature requirements"

# View in Web UI
track webui --open
```

### Calendar Integration

To display your Google Calendar in the today task view:

```bash
# Set your calendar ID
track config set-calendar "your-email@example.com"

# Or use a specific calendar ID
track config set-calendar "calendar-id@group.calendar.google.com"

# View current configuration
track config show
```

**Finding Your Calendar ID:**

1. Open Google Calendar in your browser
2. Click the three dots next to your calendar name
3. Select "Settings and sharing"
4. Scroll down to "Integrate calendar"
5. Copy the "Calendar ID"

**Calendar Permissions:**

For the calendar to display correctly:
- The calendar must be shared with appropriate permissions
- For Google Workspace calendars, ensure iframe embedding is allowed
- You must be logged into your Google account in the browser

### Web UI Features

When viewing a today task in the Web UI:

- **Calendar Display**: Shows your daily schedule in agenda view
- **Automatic Theme Matching**: Calendar colors adapt to dark/light mode
- **2-Pane Layout**: Calendar on the left, links on the right
- **Focus Mode**: Hide the overview section to focus on TODOs and scraps

## Implementation Details

### Database Schema

The `tasks` table includes an `is_today_task` column:
- `is_today_task = 1`: This is a today task
- `is_today_task = 0`: Regular task

Only one task can have `is_today_task = 1` at a time.

### Task Naming Convention

Today tasks are named: `Today: YYYY-MM-DD`

Example: `Today: 2026-01-06`

### Inheritance Logic

1. **Find Previous Today Task**: Searches for the most recent task with `is_today_task = 1`
2. **Copy Pending TODOs**: Only TODOs with `status = 'pending'` are copied
3. **Copy Linked Scraps**: Scraps with `active_todo_id` matching inherited TODOs are copied
4. **Update References**: The `active_todo_id` in copied scraps is updated to match the new TODO IDs

### Configuration Storage

Calendar configuration is stored in the `app_state` table:
- Key: `calendar_id`
- Value: Google Calendar ID

## Tips and Best Practices

### Daily Workflow

1. **Start of Day**: Run `track switch today` to create/switch to today's task
2. **Review Inherited TODOs**: Check what was carried over from yesterday
3. **Add New TODOs**: Add any new tasks for today
4. **Complete TODOs**: Mark tasks as done throughout the day
5. **Add Notes**: Use scraps to record important information

### Managing Incomplete Work

- Incomplete TODOs automatically carry over to the next day
- Review inherited TODOs each morning and decide if they're still relevant
- Use `track todo delete <id>` to remove TODOs that are no longer needed

### Calendar Integration

- Use the calendar view to see your scheduled meetings and appointments
- Add TODOs based on your calendar events
- The agenda view shows today and upcoming events

### Switching Between Tasks

```bash
# Work on today's task
track switch today

# Switch to a specific project task
track switch 5

# Return to today's task
track switch today
```

## Troubleshooting

### Calendar Not Displaying

**Issue**: Calendar shows "No calendar configured" message

**Solution**:
```bash
track config set-calendar "your-calendar-id"
```

**Issue**: Calendar is blank or shows an error

**Solutions**:
- Ensure you're logged into Google in your browser
- Check calendar sharing permissions
- Verify the calendar ID is correct
- For Google Workspace, check if iframe embedding is allowed

### TODOs Not Inheriting

**Issue**: Previous day's TODOs not showing up

**Possible Causes**:
- Previous day's task is not marked as `is_today_task`
- TODOs were marked as "done" or "cancelled"
- Database migration didn't run correctly

**Solution**:
```bash
# Check if migration ran
# The is_today_task column should exist in the tasks table
```

### Theme Issues

**Issue**: Calendar colors don't match the theme

**Solution**:
- Hard refresh the browser: `Cmd+Shift+R` (Mac) or `Ctrl+Shift+R` (Windows/Linux)
- Clear browser cache
- The calendar uses CSS filters to adapt to dark mode

## Examples

### Example 1: Daily Standup Preparation

```bash
# Morning routine
track switch today

# Add standup items
track todo add "Yesterday: Completed authentication feature"
track todo add "Today: Work on API documentation"
track todo add "Blockers: Need design review for new UI"

# Add meeting notes after standup
track scrap add "Team feedback: Focus on error handling in API"
```

### Example 2: Calendar-Driven Workflow

```bash
# Set up calendar
track config set-calendar "work@company.com"

# Switch to today
track switch today

# Open Web UI to see calendar
track webui --open

# Add TODOs based on calendar events
track todo add "Prepare for 10am design review"
track todo add "Attend 2pm team meeting"
track todo add "Follow up on client feedback"
```

### Example 3: End of Day Review

```bash
# Review today's task
track status

# Mark completed items
track todo done 1
track todo done 3

# Add notes for tomorrow
track scrap add "Tomorrow: Continue work on feature X, blocked on API response"

# Incomplete TODO #2 will automatically carry over to tomorrow
```

## Related Commands

- `track new <name>` - Create a regular task
- `track switch <task_id>` - Switch to a specific task
- `track todo add <text>` - Add a TODO
- `track scrap add <content>` - Add a work note
- `track config show` - View current configuration
- `track webui --open` - Open Web UI

## See Also

- [USAGE_EXAMPLES.md](USAGE_EXAMPLES.md) - General usage examples
- [FUNCTIONAL_SPEC.md](FUNCTIONAL_SPEC.md) - Functional specification
- [README.md](../README.md) - Main documentation
