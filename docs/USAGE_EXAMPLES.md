# Usage Examples

This document provides detailed usage examples for common Track workflows.

## Example 1: Bug Fix Workflow

```bash
# 1. Create task for bug fix
track new "Fix authentication timeout" \
  --ticket BUG-456 \
  --ticket-url https://github.com/myorg/myrepo/issues/456

# 2. Add investigation notes
track scrap add "Issue occurs after 30 minutes of inactivity"
track scrap add "Likely related to JWT expiration handling"

# 3. Add TODO with worktree for isolated work
track todo add "Fix token refresh logic" --worktree

# 4. Setup worktree
track repo add .
track sync

# 5. Work in isolation
cd task/BUG-456-todo-1
# ... make changes, test, commit ...

# 6. Complete and merge
track todo done 1  # Automatically merges and cleans up

# 7. Archive when done
track archive t:BUG-456
```

## Example 2: Feature Development with Multiple TODOs

```bash
# 1. Create feature task
track new "Add user profile page" --ticket FEAT-789

# 2. Break down into TODOs
track todo add "Design profile UI mockup"
track todo add "Implement backend API" --worktree
track todo add "Create frontend components" --worktree
track todo add "Write tests"

# 3. Add reference materials
track link add https://figma.com/design/profile "UI Design"
track link add https://api-docs.example.com "API Spec"

# 4. Work through TODOs
track todo done 1  # Complete design
track sync         # Create worktrees for #2 and #3

# Work on backend
cd task/FEAT-789-todo-2
# ... implement API ...
track scrap add "Using PostgreSQL for user data storage"
track todo done 2

# Work on frontend
cd ../task/FEAT-789-todo-3
# ... build components ...
track todo done 3

# 5. Check progress
track status
```

## Example 3: Managing Multiple Tasks

```bash
# Create multiple tasks
track new "Refactor authentication module" --ticket TECH-101
track new "Update documentation" --ticket DOC-202
track new "Performance optimization" --ticket PERF-303

# View all tasks
track list

# Switch between tasks
track switch t:TECH-101
track todo add "Extract auth logic to separate service"
track scrap add "Current code is in src/auth/legacy.rs"

track switch t:DOC-202
track todo add "Update API documentation"
track link add https://swagger.io "Swagger Editor"

# Check current task
track status
```

## Workflow with Git Worktrees

```bash
# Register repository and sync (creates task branches and worktrees)
track repo add /path/to/repo
track sync

# This creates:
# - Branch: task/AUTH-456 (base task branch)
# - Worktree: /path/to/repo/task/AUTH-456-todo-2 (for TODO #2)

# Navigate to worktree and work on TODO
cd /path/to/repo/task/AUTH-456-todo-2
# ... make changes, commit ...

# Complete TODO (automatically merges and cleans up worktree)
track todo done 2
```

## Task Aliases

Assign human-readable aliases to tasks for easier reference:

```bash
# Set an alias for the current task
track alias set daily-report

# Now you can reference the task by alias
track switch daily-report
track status daily-report
track archive daily-report

# Remove alias
track alias remove
```

**Task Reference Priority:**
1. Numeric ID (e.g., `3`)
2. Ticket reference (e.g., `t:PROJ-123`)
3. Task alias (e.g., `daily-report`)

**Alias Rules:**
- Alphanumeric characters, hyphens, and underscores only
- 1-50 characters
- Must be unique (unless using `--force` to overwrite)
- Cannot use reserved command names (new, list, status, etc.)

**Overwriting Aliases:**

If you try to set an alias that's already in use by another task, you'll get an error:

```bash
# Task 1 has alias "daily-report"
track switch 2
track alias set daily-report
# Error: Alias 'daily-report' is already in use by task #1. Use --force to overwrite.

# Use --force to move the alias to the current task
track alias set daily-report --force
# The alias is removed from task #1 and set on task #2
```

## Task Templates

Create new tasks from existing task templates to reuse TODO lists for recurring workflows:

```bash
# Create a template task with common TODOs
track new "Daily Report Template"
track alias set daily-template
track todo add "Collect metrics"
track todo add "Analyze data"
track todo add "Write summary"
track todo add "Send to team"

# Create new task from template
track new "Daily Report 2026-01-04" --template daily-template

# All TODOs are copied with 'pending' status
# You can also use task ID or ticket reference
track new "Daily Report 2026-01-05" --template 3
track new "Daily Report 2026-01-06" --template t:TEMPLATE-001
```

**Use Cases:**
- Daily/weekly reports
- Release checklists
- Code review processes
- Onboarding workflows

## Ticket Reference

You can reference tasks by ticket ID instead of task ID:

```bash
# Switch task by ticket ID
track switch t:PROJ-123

# Archive by ticket ID
track archive t:PROJ-123
```

## Todo Prioritization

Move todos to the front of your work queue to change priorities dynamically:

```bash
# Create a task with multiple todos
track new "Sprint Planning"
track todo add "Review backlog"
track todo add "Estimate stories"
track todo add "Plan sprint"
track todo add "Update roadmap"

# List todos (shows current order)
track todo list
# 1. Review backlog (pending)
# 2. Estimate stories (pending)
# 3. Plan sprint (pending)
# 4. Update roadmap (pending)

# Urgent: Need to update roadmap first
track todo next 4

# New order:
track todo list
# 1. Update roadmap (pending)  ← Moved to front
# 2. Review backlog (pending)
# 3. Estimate stories (pending)
# 4. Plan sprint (pending)
```

**Web UI:** Click the ⋮ menu on any pending todo and select "⬆️ Make Next" to move it to the front.

## Scrap Linking

Scraps (work notes) are automatically linked to the active todo when created. This helps you track which notes belong to which work item:

```bash
# Working on first todo
track todo add "Implement authentication"
track todo add "Add user profile"
track todo add "Write tests"

# Add notes while working on todo #1
track scrap add "Using JWT for token-based auth"
track scrap add "Storing tokens in httpOnly cookies"

# Complete first todo and move to next
track todo done 1

# Add notes for todo #2
track scrap add "Profile includes avatar, bio, and preferences"
track scrap add "Using Cloudinary for image storage"

# View all scraps
track scrap list
# [2026-01-06 01:00:00] Using JWT for token-based auth (linked to todo #1)
# [2026-01-06 01:01:00] Storing tokens in httpOnly cookies (linked to todo #1)
# [2026-01-06 01:05:00] Profile includes avatar, bio, and preferences (linked to todo #2)
# [2026-01-06 01:06:00] Using Cloudinary for image storage (linked to todo #2)
```

**Web UI:** Click the 📝 button on any todo to jump to its related scraps. The scrap will be highlighted and scrolled into view.

## Branch Naming Convention

For tasks with registered tickets, the ticket ID is automatically used in branch names:

```bash
# When ticket PROJ-123 is registered and sync is run:
track sync
# → Creates branch: task/PROJ-123 (base task branch)

# When TODO #1 has --worktree flag:
track todo add "Implement login" --worktree
track sync
# → Creates branch: task/PROJ-123-todo-1 (TODO work branch)
```
