# Creating Tasks - Detailed Guide

Complete workflow for creating and setting up development tasks.

## When to Use

- Starting a new feature or bug fix
- Breaking down a large project into actionable items
- Setting up a structured development workflow

## Prerequisites

- `track` CLI installed and initialized

## Step-by-Step Workflow

### Step 1: Create the Task

**Basic creation:**
```bash
track new "<task_name>"
```

**With ticket (recommended):**
```bash
track new "<task_name>" --ticket <TICKET_ID> --ticket-url <URL>
```

**Examples:**
```bash
# Simple task
track new "Add user authentication"

# With Jira
track new "Fix login bug" --ticket PROJ-123 \
  --ticket-url https://jira.company.com/browse/PROJ-123

# With GitHub issue
track new "Dark mode" --ticket GH-456 \
  --ticket-url https://github.com/org/repo/issues/456
```

**What happens:**
- Creates new task in database
- Switches to new task (sets as current)
- Links ticket if provided (enables ticket-based bookmark naming)

---

### Step 2: Add Description

```bash
track desc "<detailed_description>"
```

**Example:**
```bash
track desc "Implement OAuth2-based authentication. Support Google and GitHub providers. Store tokens securely."
```

**Best practices:**
- Be specific about what needs to be done
- Mention key technical requirements
- Reference related documentation

---

### Step 3: Register Repository

**If not already registered:**
```bash
track repo add [path]
```

**Examples:**
```bash
# Current directory
track repo add

# Specific path
track repo add /home/user/projects/my-app

# With specific base bookmark
track repo add --base develop

# Multiple repositories
track repo add /home/user/projects/frontend
track repo add /home/user/projects/backend --base main
```

**What happens:**
- Registers repository for current task
- Optionally specifies base bookmark (where TODOs will rebase onto)
- Later, `track sync` creates bookmarks and workspaces in these repos

---

### Step 4: Add TODOs

**Simple TODO:**
```bash
track todo add "<description>"
```

**TODO with workspace (for isolated work):**
```bash
track todo add "<description>" --worktree
```

**Examples:**
```bash
# Simple TODOs
track todo add "Set up OAuth2 configuration"
track todo add "Write tests"

# Complex changes needing isolation
track todo add "Implement Google OAuth flow" --worktree
track todo add "Implement GitHub OAuth flow" --worktree
```

**When to use `--worktree`:**
- Complex changes requiring isolation
- Working on multiple TODOs in parallel
- Need to keep changes separate before rebasing

---

### Step 5: Review Setup

```bash
track status
```

**Verify:**
- Task name and description are correct
- All TODOs are listed
- Workspace markers shown where expected

---

### Step 6: Sync Repositories (Optional)

```bash
track sync
```

**What happens:**
- Creates task bookmarks on all registered repos
- Creates workspaces for TODOs with `--worktree` flag
- Moves each workspace to the task bookmark (verify with `jj bookmark list -r @`)
- Prepares development environment

**Note:** Can be deferred until ready to start coding.

---

## Common Patterns

### Simple Task (No Workspaces)
```bash
track new "Update documentation"
track desc "Update README and add API examples"
track todo add "Update README with new features"
track todo add "Add API usage examples"
track todo add "Update changelog"
```

### Complex Feature with Workspaces
```bash
track new "Add payment integration" \
  --ticket PROJ-789 \
  --ticket-url https://jira.example.com/browse/PROJ-789

track desc "Integrate Stripe payment processing"
track repo add

track todo add "Set up Stripe SDK" --worktree
track todo add "Create payment models" --worktree
track todo add "Implement checkout flow" --worktree
track todo add "Add webhook handlers" --worktree
track todo add "Write tests"

track sync
```

### Multi-Repo Task
```bash
track new "Sync user data" \
  --ticket PROJ-555 \
  --ticket-url https://example.com/PROJ-555

track desc "Implement user data sync between frontend and backend"

track repo add /home/user/projects/frontend
track repo add /home/user/projects/backend

track todo add "Add sync API in backend" --worktree
track todo add "Implement sync client in frontend" --worktree
track todo add "Add integration tests"

track sync
```

---

## Expected Outcome

After completing this workflow:

- ✅ New task created and set as current
- ✅ Task description documented
- ✅ Repository(ies) registered
- ✅ TODO list with actionable items
- ✅ (Optional) Bookmarks and workspaces created

## Next Steps

See [executing-tasks.md](executing-tasks.md) for working through TODOs.

---

## Quick Reference

| Command | Purpose |
|---------|---------|
| `track new "<name>"` | Create task |
| `track new "<name>" --ticket <id> --ticket-url <url>` | Create with ticket |
| `track desc "<text>"` | Add description |
| `track repo add [path]` | Register repository |
| `track repo add --base <bookmark>` | Register repository with base bookmark |
| `track todo add "<text>" --worktree` | Add TODO with workspace |
| `track status` | Review setup |
| `track sync` | Create bookmarks/workspaces |

---

## Troubleshooting

**Task already exists:**
- Use `track list` to see existing tasks
- Choose more specific name or switch to existing

**Repo not registered:**
- Run `track repo add [path]` before `track sync`
- Verify with `track repo list`

**Workspaces not created:**
- Ensure `--worktree` flag used with `track todo add`
- Run `track sync` to create workspaces
