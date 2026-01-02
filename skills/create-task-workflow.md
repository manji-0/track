# Skill: Create Task and List TODOs Workflow

## Purpose
This skill guides LLM agents through the standard workflow of creating a new development task and organizing it with TODOs using the `track` CLI.

## When to Use
- Starting a new feature or bug fix
- Breaking down a large task into actionable items
- Setting up a structured development workflow

## Prerequisites
- `track` CLI is installed and initialized
- User has provided task name and scope

## Workflow Steps

### Step 1: Create the Task

**Basic task creation:**
```bash
track new "<task_name>"
```

**With ticket integration (recommended):**
```bash
track new "<task_name>" --ticket <TICKET_ID> --ticket-url <URL>
```

**Examples:**
```bash
# Without ticket
track new "Add user authentication"

# With Jira ticket
track new "Fix login bug" --ticket PROJ-123 --ticket-url https://jira.company.com/browse/PROJ-123

# With GitHub issue
track new "Implement dark mode" --ticket GH-456 --ticket-url https://github.com/org/repo/issues/456
```

**What happens:**
- Creates a new task in the track database
- Automatically switches to the new task as the current active task
- If ticket info provided, links the task to the external ticket (enables ticket-based branch naming)

---

### Step 2: Add Task Description

```bash
track desc "<detailed_description>"
```

**Example:**
```bash
track desc "Implement OAuth2-based authentication for the web app. Support Google and GitHub providers. Store tokens securely in database."
```

**Best practices:**
- Include what needs to be done
- Mention key technical requirements
- Reference related tickets or documentation

**What happens:**
- Stores a detailed description for the task
- This description is visible in `track status` for context

---

### Step 3: Register Repository (if needed)

```bash
track repo add [path]
```

**Examples:**
```bash
# Register current directory
track repo add

# Register specific repository
track repo add /home/user/projects/my-app

# Register multiple repositories for multi-repo tasks
track repo add /home/user/projects/frontend
track repo add /home/user/projects/backend
```

**What happens:**
- Registers the repository for this task
- Later, `track sync` will create branches and worktrees in these repos

**When to do this:**
- If you haven't already registered the repository
- When working with multiple repositories

---

### Step 4: Break Down into TODOs

**Add simple TODO:**
```bash
track todo add "<todo_description>"
```

**Add TODO with worktree (for isolated development):**
```bash
track todo add "<todo_description>" --worktree
```

**Examples:**
```bash
# Simple TODOs
track todo add "Set up OAuth2 configuration"
track todo add "Create user authentication models"
track todo add "Write integration tests"

# TODOs with worktrees (for complex changes)
track todo add "Implement Google OAuth flow" --worktree
track todo add "Implement GitHub OAuth flow" --worktree
track todo add "Add token refresh logic" --worktree
```

**When to use `--worktree`:**
- Changes require isolated development environment
- Working on multiple TODOs in parallel
- Need to keep changes separate before merging

**What happens:**
- Adds TODO to the current task
- If `--worktree` flag used, marks TODO for worktree creation (created later by `track sync`)
- Each TODO gets a task-scoped index (1, 2, 3, ...)

---

### Step 5: Review Task Setup

```bash
track status
```

**Example output:**
```
=== Task #5: Add user authentication ===
Ticket: PROJ-123
URL: https://jira.company.com/browse/PROJ-123
Created: 2026-01-02 10:30:00

Description:
Implement OAuth2-based authentication for the web app...

[ TODOs ]
  [ ] [1] Set up OAuth2 configuration
  [ ] [2] Implement Google OAuth flow (worktree)
  [ ] [3] Implement GitHub OAuth flow (worktree)
  [ ] [4] Add token refresh logic (worktree)
  [ ] [5] Write integration tests
```

**Verify:**
- Task name and description are correct
- All TODOs are listed
- Worktree markers are shown where expected

---

### Step 6: Sync Repositories (Optional, prepare for execution)

```bash
track sync
```

**What happens:**
- Creates task branch on all registered repositories
- Creates worktrees for TODOs marked with `--worktree`
- Prepares the development environment for task execution

**Note:** This step is typically done when you're ready to start coding. It can be deferred until task execution phase.

---

## Expected Outcome

After completing this workflow, you will have:

1. ✅ A new task created and set as current
2. ✅ Task description documented
3. ✅ Repository(ies) registered for the task
4. ✅ TODO list with clear, actionable items
5. ✅ (Optional) Branches and worktrees created for development

## Next Steps

After task setup is complete:
- Use `track status` to view current state
- Navigate to worktree directories if created
- Execute TODOs one by one
- Use `track scrap add` to document progress
- Mark TODOs complete with `track todo done <index>`

## Common Patterns

### Pattern 1: Simple Task (No Worktrees)
```bash
track new "Update documentation"
track desc "Update README and add API examples"
track todo add "Update README with new features"
track todo add "Add API usage examples"
track todo add "Update changelog"
```

### Pattern 2: Complex Feature with Worktrees
```bash
track new "Add payment integration" --ticket PROJ-789 --ticket-url <url>
track desc "Integrate Stripe payment processing"
track repo add
track todo add "Set up Stripe SDK" --worktree
track todo add "Create payment models" --worktree
track todo add "Implement checkout flow" --worktree
track todo add "Add webhook handlers" --worktree
track todo add "Write tests"
track sync
```

### Pattern 3: Multi-Repo Task
```bash
track new "Sync user data between services" --ticket PROJ-555 --ticket-url <url>
track desc "Implement user data synchronization between frontend and backend"
track repo add /home/user/projects/frontend
track repo add /home/user/projects/backend
track todo add "Add sync API endpoint in backend" --worktree
track todo add "Implement sync client in frontend" --worktree
track todo add "Add integration tests"
track sync
```

## Tips for LLM Agents

1. **Always ask for task name**: If user doesn't provide, ask before creating task
2. **Offer ticket integration**: If user mentions a ticket ID, offer to link it
3. **Suggest meaningful TODOs**: Break down work into logical, testable chunks
4. **Use worktrees strategically**: Suggest `--worktree` for complex or parallel work
5. **Review before sync**: Show planned structure before running `track sync`
6. **Document everything**: Add task description capturing user intent

## Troubleshooting

**Problem: Task already exists with same name**
- Use `track list` to see existing tasks
- Choose a more specific name or switch to existing task

**Problem: Repo not registered**
- Run `track repo add [path]` before `track sync`
- Verify with `track repo list`

**Problem: TODOs not created with worktrees**
- Make sure you used `--worktree` flag with `track todo add`
- Run `track sync` to actually create the worktrees

## Reference Commands

| Command | Purpose |
|---------|---------|
| `track new "<name>"` | Create new task |
| `track new "<name>" --ticket <id> --ticket-url <url>` | Create task with ticket |
| `track desc "<text>"` | Add task description |
| `track repo add [path]` | Register repository |
| `track todo add "<text>"` | Add simple TODO |
| `track todo add "<text>" --worktree` | Add TODO with worktree |
| `track status` | View current task and TODOs |
| `track sync` | Create branches and worktrees |
