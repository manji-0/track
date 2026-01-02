# Skill: Execute Task and Complete TODOs Workflow

## Purpose
This skill guides LLM agents through the execution phase of a task: working on TODOs, recording progress, and marking items complete using the `track` CLI.

## When to Use
- After task setup is complete (task created, TODOs added)
- Ready to start coding and implementing changes
- Working through a list of actionable items

## Prerequisites
- Task has been created with `track new`
- TODOs have been added with `track todo add`
- Repositories registered and synced (if using worktrees)

## Workflow Steps

### Step 1: Check Current Task State

```bash
track status
```

**Example output:**
```
=== Task #7: Implement user authentication ===
Ticket: PROJ-123
URL: https://jira.company.com/browse/PROJ-123
Created: 2026-01-02 14:30:00

Description:
Add OAuth2 authentication with Google and GitHub providers

[ TODOs ]
  [ ] [1] Set up OAuth2 configuration
  [ ] [2] Implement Google OAuth flow
      Worktree: /home/user/projects/myapp/task/PROJ-123-todo-2
  [✓] [3] Implement GitHub OAuth flow
  [ ] [4] Write integration tests

[ Recent Scraps ]
  [2026-01-02 15:45] Completed GitHub OAuth implementation
  [2026-01-02 15:20] Found issue with redirect URIs, fixed in config
```

**Analyze this output:**
- Identify pending TODOs (marked with `[ ]`)
- Note worktree paths if they exist
- Review recent scraps for context
- Determine which TODO to work on next

---

### Step 2: Select Next TODO

**Strategy for selection:**
1. **Priority**: Work on TODOs in logical order (e.g., dependencies first)
2. **Worktrees**: Check if TODO has an associated worktree
3. **Complexity**: Consider starting with simpler TODOs for momentum

**Common selection patterns:**
- Sequential: Work through TODOs 1 → 2 → 3
- Parallel: If multiple worktrees exist, work on any pending TODO
- Dependency-based: Complete prerequisites before dependent tasks

---

### Step 3: Navigate to Work Location

**If TODO has a worktree:**
```bash
cd /path/to/worktree
```

**Example:**
```bash
# From track status output, worktree path shown as:
# Worktree: /home/user/projects/myapp/task/PROJ-123-todo-2

cd /home/user/projects/myapp/task/PROJ-123-todo-2
```

**If no worktree (working in main task branch):**
```bash
cd /path/to/repository
git checkout task/<branch-name>
```

**Verify you're in the right place:**
```bash
pwd           # Check current directory
git branch    # Verify current branch
```

---

### Step 4: Implement the TODO

**Follow standard development practices:**

1. **Understand requirements**
   - Review TODO description
   - Check task description for context
   - Review any related scraps

2. **Write code**
   - Make necessary changes
   - Follow project conventions
   - Keep changes focused on the TODO

3. **Test your changes**
   ```bash
   # Run relevant tests
   cargo test           # Rust
   npm test            # Node.js
   pytest              # Python
   go test ./...       # Go
   
   # Build if applicable
   cargo build
   npm run build
   ```

4. **Verify code quality**
   ```bash
   # Linting
   cargo clippy        # Rust
   npm run lint        # Node.js
   pylint .            # Python
   
   # Formatting
   cargo fmt
   npm run format
   ```

---

### Step 5: Commit Changes

**Important:** All changes must be committed before marking TODO as done.

```bash
# Stage changes
git add .

# Commit with descriptive message
git commit -m "Implement OAuth2 Google provider flow

- Add GoogleOAuthClient class
- Implement token exchange
- Add redirect URI handling
- Update configuration with Google credentials

Refs: PROJ-123"
```

**Commit message best practices:**
- Use imperative mood ("Add feature" not "Added feature")
- Include brief summary in first line
- Add details in body if needed
- Reference ticket ID

---

### Step 6: Record Progress (Scraps)

Use `track scrap add` to document findings, decisions, or progress:

```bash
track scrap add "<note>"
```

**When to add scraps:**
- **Findings**: Discovered issues, edge cases, or insights
- **Decisions**: Technical choices made during implementation
- **Progress**: Milestones or completion notes
- **Questions**: Issues to address or clarify later
- **Links**: References to documentation or resources

**Examples:**
```bash
track scrap add "Completed Google OAuth flow. Used official google-auth-library. Token refresh handled automatically."

track scrap add "Found issue: Redirect URI must match exactly in Google Console. Updated config to use https://app.company.com/auth/callback"

track scrap add "Decision: Using server-side OAuth flow for better security. Client-side tokens expose risks."

track scrap add "TODO 2 complete. All tests passing. Ready to move to GitHub provider."

track scrap add "Reference: Google OAuth2 docs at https://developers.google.com/identity/protocols/oauth2"
```

---

### Step 7: Complete the TODO

```bash
track todo done <index>
```

**Example:**
```bash
track todo done 2
```

**What happens when you run this:**
1. **Checks for uncommitted changes**: If any exist, operation aborts (you must commit first)
2. **Merges worktree**: If TODO had a worktree, merges its branch into the task base branch
3. **Removes worktree**: Deletes the worktree directory and database record
4. **Updates status**: Marks TODO as `done` in database

**Output:**
```
TODO #2 marked as done
Merged task/PROJ-123-todo-2 into task/PROJ-123
Removed worktree at /home/user/projects/myapp/task/PROJ-123-todo-2
```

---

### Step 8: Verify and Continue

```bash
track status
```

**Check:**
- TODO is now marked as done `[✓]`
- Next pending TODO is identified
- Worktree is removed (if applicable)

**Then:**
- If more TODOs remain: **Return to Step 2**
- If all TODOs done: **Task is complete** (proceed to finalization)

---

## Complete Workflow Example

```bash
# 1. Check state
track status

# 2. Navigate to worktree for TODO #2
cd /home/user/projects/myapp/task/PROJ-123-todo-2

# 3. Implement changes
# ... edit code ...
cargo test
cargo clippy

# 4. Commit
git add .
git commit -m "Implement Google OAuth flow"

# 5. Record progress
track scrap add "Completed Google OAuth. Tests passing. Used google-auth-library."

# 6. Mark complete
track todo done 2

# 7. Check next TODO
track status

# 8. Repeat for next TODO
```

---

## Task Completion

When all TODOs are marked as done:

1. **Final verification**
   ```bash
   track status    # All TODOs should show [✓]
   ```

2. **Review scraps**
   ```bash
   track scrap list
   ```

3. **Final testing**
   - Run full test suite
   - Verify build succeeds
   - Check code quality

4. **Push changes** (if ready for review/merge)
   ```bash
   cd /path/to/repository
   git checkout task/PROJ-123
   git push origin task/PROJ-123
   ```

5. **Archive or keep task**
   - Use `track archive` if task is fully complete
   - Or keep task active for reference

---

## Common Patterns

### Pattern 1: Simple Sequential Execution
```bash
track status                           # View TODOs
# Work on TODO 1
git add . && git commit -m "..."
track todo done 1
# Work on TODO 2
git add . && git commit -m "..."
track scrap add "Completed TODO 2"
track todo done 2
# ...continue until done
```

### Pattern 2: Parallel Work with Worktrees
```bash
track status                           # Shows multiple worktrees

# Work on TODO 3 in terminal 1
cd /path/to/worktree-todo-3
# ... implement ...
git commit -m "..."
track todo done 3

# Work on TODO 4 in terminal 2 (in parallel)
cd /path/to/worktree-todo-4
# ... implement ...
git commit -m "..."
track todo done 4
```

### Pattern 3: Incremental Progress
```bash
# Start TODO
cd /path/to/worktree

# Make some progress
# ... partial implementation ...
git add . && git commit -m "WIP: Implement OAuth client setup"
track scrap add "Progress: Set up OAuth client, working on token exchange"

# Continue work
# ... more implementation ...
git add . && git commit -m "Complete OAuth flow implementation"
track scrap add "Completed OAuth flow. All tests passing."

# Mark done
track todo done 2
```

---

## Tips for LLM Agents

1. **Always check status first**: Run `track status` before making assumptions
2. **Commit frequently**: Make logical, incremental commits
3. **Document as you go**: Add scraps for important findings or decisions
4. **Test before completing**: Verify tests pass before `track todo done`
5. **Clean state required**: Ensure all changes are committed before marking TODO done
6. **Follow TODO order**: Unless there are dependencies, work sequentially
7. **Use scraps for continuity**: Help future you (or other agents) understand context

---

## Troubleshooting

**Problem: `track todo done` fails with "uncommitted changes"**
- **Solution**: Commit or stash all changes first
  ```bash
  git status
  git add .
  git commit -m "..."
  track todo done <index>
  ```

**Problem: Don't know which TODO to work on**
- **Solution**: Run `track status` to see all pending TODOs and their context
- Start with lowest numbered pending TODO, or follow dependency order

**Problem: Worktree path doesn't exist**
- **Solution**: Run `track sync` to create pending worktrees
  ```bash
  track sync
  track status    # Verify worktrees created
  ```

**Problem: Want to skip a TODO**
- **Solution**: Work on TODOs in any order, just use the correct index
  ```bash
  # Can complete TODO 3 before TODO 2
  track todo done 3
  ```

**Problem: Tests fail after merge**
- **Solution**: Before running `track todo done`, ensure tests pass
  ```bash
  cargo test      # or appropriate test command
  # Fix any failures
  git add . && git commit -m "Fix test failures"
  track todo done <index>
  ```

---

## Reference Commands

| Command | Purpose |
|---------|---------|
| `track status` | View current task, TODOs, and worktrees |
| `track scrap add "<note>"` | Record progress or findings |
| `track scrap list` | View all scraps for current task |
| `track todo done <index>` | Complete TODO and merge worktree |
| `track todo list` | List all TODOs |
| `git status` | Check for uncommitted changes |
| `git commit -m "..."` | Commit changes |

---

## Integration with Create Task Workflow

This skill follows the **Create Task and List TODOs Workflow**:

1. **Create Task Workflow** → Task setup complete
2. **Execute Task Workflow** → (This skill) Work through TODOs
3. **Result** → All TODOs complete, task ready for review/merge
