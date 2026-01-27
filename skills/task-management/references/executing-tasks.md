# Executing Tasks - Detailed Guide

Complete workflow for working through TODOs and completing tasks.

## When to Use

- Ready to start implementing changes
- Working through a task's TODO list
- Need to track progress and complete items

## Prerequisites

- Task created with `track new`
- TODOs added with `track todo add`
- Repositories synced (if using workspaces)

## Step-by-Step Workflow

### Step 1: Sync and Verify Bookmark

```bash
track sync
jj status
jj bookmark list -r @
```

**Verify:**
- You are on the task bookmark (not main/master/develop)
- Workspace is clean and ready for work

---

### Step 2: Check Current State

```bash
track status [id]
```

**Analyze output:**
- Identify pending TODOs (marked `[ ]`)
- Note workspace paths if they exist
- Review recent scraps for context
- Determine which TODO to tackle next

---

### Step 3: Select Next TODO

**Selection strategies:**
1. **Sequential**: Work through 1 → 2 → 3
2. **Priority**: Handle dependencies first
3. **Parallel**: If workspaces exist, work on any pending TODO

---

### Step 4: Navigate to Work Location

**If TODO has workspace:**
```bash
cd "$(track todo workspace <index>)"
```

Example from `track status` output:
```bash
# Workspace shown as: /home/user/projects/myapp/task/PROJ-123-todo-2
cd "$(track todo workspace 2)"
```

**If no workspace:**
```bash
cd /path/to/repository
```

**Verify location:**
```bash
pwd        # Check directory
jj status  # Verify bookmark
jj bookmark list -r @
```

---

### Step 5: Implement Changes

1. **Understand requirements**
   - Review TODO description
   - Check task description
   - Review related scraps

2. **Write code**
   - Make necessary changes
   - Follow project conventions
   - Keep changes focused

3. **Describe changes**
   ```bash
   jj describe -m "<summary>"
   ```

4. **Test changes**
   ```bash
   # Run tests
   cargo test       # Rust
   npm test        # Node.js
   pytest          # Python
   
   # Build
   cargo build
   npm run build
   ```

5. **Verify quality**
   ```bash
   # Lint
   cargo clippy
   npm run lint
   
   # Format
   cargo fmt
   npm run format
   ```

---

### Step 6: Describe Changes

**Describe changes before marking TODO done.**

```bash
jj describe -m "Implement OAuth2 Google provider

- Add GoogleOAuthClient class
- Implement token exchange
- Add redirect URI handling
- Update config with credentials

Refs: PROJ-123"
```

**Description best practices:**
- Imperative mood ("Add" not "Added")
- Brief summary in first line
- Details in body if needed
- Reference ticket ID

---

### Step 7: Record Progress

```bash
track scrap add "<note>"
```

**When to add scraps:**
- **Findings**: Issues, edge cases, insights discovered
- **Decisions**: Technical choices made
- **Progress**: Milestones or completion notes
- **Questions**: Issues to address later
- **References**: Links to documentation

**Examples:**
```bash
track scrap add "Completed Google OAuth. Used google-auth-library. Token refresh automatic."

track scrap add "Issue found: Redirect URI must match exactly in Google Console. Updated config."

track scrap add "Decision: Using server-side OAuth for better security."

track scrap add "TODO 2 complete. All tests passing."
```

---

### Step 8: Complete the TODO

```bash
track todo done <index>
```

**Example:**
```bash
track todo done 2
```

**What happens:**
1. Checks for uncommitted changes (aborts if any exist)
2. Rebases workspace bookmark onto task bookmark (if workspace exists)
3. Moves the task bookmark to the rebased change
4. Removes workspace directory and database record
5. Marks TODO as `done`

**Output:**
```
TODO #2 marked as done
Rebased task/PROJ-123-todo-2 onto task/PROJ-123
Removed workspace at /home/user/projects/myapp/task/PROJ-123-todo-2
```

---

### Step 9: Continue with Next TODO

```bash
track status
```

**Verify:**
- TODO marked as done `[✓]`
- Workspace removed (if applicable)
- Next pending TODO identified

**Then:**
- If more TODOs: **Return to Step 3**
- If all done: **Task complete**

---

## Complete Example

```bash
# 1. Check state
track status

# 2. Navigate to workspace
cd "$(track todo workspace 2)"

# 3. Implement
# ... edit code ...
cargo test
cargo clippy

# 4. Describe changes

jj describe -m "Implement Google OAuth flow"


# 5. Record progress
track scrap add "Completed Google OAuth. Tests passing."

# 6. Mark complete
track todo done 2

# 7. Check next
track status

# 8. Repeat
```

---

## Task Completion

When all TODOs marked done:

1. **Verify completion**
   ```bash
   track status  # All TODOs show [✓]
   ```

2. **Review work**
   ```bash
   track scrap list
   ```

3. **Final testing**
   - Run full test suite
   - Verify build succeeds

4. **Push changes**
   ```bash
   cd /path/to/repository
   jj git push --bookmark task/PROJ-123
   ```

5. **Archive or keep**
   - Use `track archive` to archive current task
   - Or keep for reference

---

## Common Patterns

### Sequential Execution
```bash
track status
# Work on TODO 1
jj describe -m "..."
track todo done 1
# Work on TODO 2
jj describe -m "..."
track scrap add "Completed TODO 2"
track todo done 2
```

### Parallel Work (Multiple Terminals)
```bash
# Terminal 1: TODO 3
cd "$(track todo workspace 3)"
# ... implement ...
track todo done 3

# Terminal 2: TODO 4 (parallel)
cd "$(track todo workspace 4)"
# ... implement ...
track todo done 4
```

### Incremental Progress
```bash
# Start work
cd "$(track todo workspace 2)"

# Partial progress
jj describe -m "WIP: OAuth client setup"
track scrap add "Progress: Client setup done, working on token exchange"

# Continue
jj describe -m "Complete OAuth flow"
track scrap add "OAuth flow complete. Tests passing."

# Finish
track todo done 2
```

---

## For LLM Agents

1. **Always check status first**: `track status`
2. **Describe frequently**: Logical, incremental change summaries
3. **Document as you go**: Add scraps for important decisions
4. **Test before completing**: Verify tests pass
5. **Clean state required**: All changes described before `track todo done`
6. **Use scraps for context**: Help future understanding

---

## Quick Reference

| Command | Purpose |
|---------|---------|
| `track status` | View task and TODOs |
| `track scrap add "<note>"` | Record progress |
| `track scrap list` | View all scraps |
| `track todo done <index>` | Complete TODO |
| `jj status` | Check uncommitted changes |
| `jj describe -m "..."` | Describe changes |

---

## Troubleshooting

**`track todo done` fails with "uncommitted changes":**
```bash
jj status
jj describe -m "..."
track todo done <index>
```

**Don't know which TODO to work on:**
```bash
track status  # See all pending TODOs
# Start with lowest numbered or follow dependencies
```

**Workspace path doesn't exist:**
```bash
track sync         # Create workspaces
track status       # Verify created
```

**Want to skip a TODO:**
```bash
# Work in any order
track todo done 3  # Can complete before TODO 2
```

**Tests fail after changes:**
```bash
# Before marking done
cargo test
# Fix failures
jj describe -m "Fix tests"
track todo done <index>
```
