# Advanced Workflows

Advanced patterns and use cases for track CLI.

## Multi-Repository Tasks

Working across multiple repositories simultaneously.

### Setup

```bash
track new "Cross-service feature" \
  --ticket PROJ-789 \
  --ticket-url https://jira.example.com/browse/PROJ-789

track desc "Implement user sync between frontend and backend"

# Register multiple repos
track repo add /home/user/projects/frontend
track repo add /home/user/projects/backend

# Add TODOs for each repo
track todo add "Add sync API endpoint in backend" --worktree
track todo add "Implement sync client in frontend" --worktree
track todo add "Add integration tests"

# Create bookmarks and workspaces in all repos
track sync
```

### Working Across Repos

```bash
# Backend work
cd "$(track todo workspace 1)"
jj status
# ... implement API ...
jj describe -m "Add sync API endpoint"
track scrap add "Backend API complete. Returns user delta."
track todo done 1

# Frontend work
cd "$(track todo workspace 2)"
jj status
# ... implement client ...
jj describe -m "Implement sync client"
track todo done 2

# Integration tests (no workspace)
cd /home/user/projects/backend
# ... write tests ...
jj describe -m "Add integration tests"
track todo done 3
```

---

## Parallel Development

Working on multiple TODOs simultaneously.

### Terminal Setup

**Terminal 1: TODO 2**
```bash
track status
cd "$(track todo workspace 2)"
jj status
# ... work on feature A ...
jj describe -m "Implement feature A"
track scrap add "Feature A complete"
track todo done 2
```

**Terminal 2: TODO 3** (parallel)
```bash
track status
cd "$(track todo workspace 3)"
jj status
# ... work on feature B ...
jj describe -m "Implement feature B"
track scrap add "Feature B complete"
track todo done 3
```

### Benefits

- Work on independent features simultaneously
- Avoid bookmark switching overhead
- Test features in isolation
- Merge when ready

---

## Ticket-Based Organization

Leveraging ticket integration for better organization.

### GitHub Issues

```bash
track new "Fix memory leak" \
  --ticket GH-456 \
  --ticket-url https://github.com/org/repo/issues/456

# Bookmark created: task/GH-456
```

### Jira Integration

```bash
track new "API v2 migration" \
  --ticket TECH-789 \
  --ticket-url https://jira.company.com/browse/TECH-789

# Bookmark created: task/TECH-789
```

### Switching by Ticket

```bash
# Switch to task by ticket ID
track switch t:PROJ-123

# Archive by ticket ID
track archive t:PROJ-123
```

---

## Task Switching Patterns

Managing multiple active tasks.

### Context Switching

```bash
# Create multiple tasks
track new "Refactor auth" --ticket TECH-101
track todo add "Extract auth logic"
track todo add "Add unit tests"

track new "Update docs" --ticket DOC-202
track todo add "Update README"
track todo add "Add API examples"

# Switch between tasks
track switch t:TECH-101
track status  # Shows auth task

track switch t:DOC-202
track status  # Shows docs task
```

### Viewing All Tasks

```bash
track list

# Output:
# +---+----+----------+---------------------------+
# |   | ID | Ticket   | Name                      |
# +---+----+----------+---------------------------+
# | * | 5  | DOC-202  | Update docs               |
# |   | 4  | TECH-101 | Refactor auth             |
# +---+----+----------+---------------------------+
```

---

## Incremental Development

Breaking work into small, testable commits.

### Pattern

```bash
track todo add "Implement OAuth2 flow" --worktree
track sync

cd "$(track todo workspace 1)"
jj status

# Step 1: Setup
# ... add dependencies ...
jj describe -m "Add OAuth2 dependencies"
track scrap add "Added google-auth-library v2.5.0"

# Step 2: Basic flow
# ... implement core logic ...
jj describe -m "Implement basic OAuth2 flow"
track scrap add "Core flow working, needs error handling"

# Step 3: Error handling
# ... add error handling ...
jj describe -m "Add OAuth2 error handling"

# Step 4: Tests
# ... write tests ...
jj describe -m "Add OAuth2 tests"
track scrap add "All tests passing. Coverage 95%."

# Complete
track todo done 1
```

---

## Hotfix Workflow

Quick fixes for urgent issues.

### Setup

```bash
track new "Fix critical auth bug" \
  --ticket BUG-999 \
  --ticket-url https://jira.company.com/browse/BUG-999

track desc "Users unable to login after 30min inactivity"

# Single TODO for quick fix
track todo add "Fix token refresh logic" --worktree

track sync
```

### Implementation

```bash
cd "$(track todo workspace 1)"
jj status

# Quick investigation
track scrap add "Issue: JWT refresh token not updating expiry"

# Fix
# ... implement fix ...
jj describe -m "Fix JWT refresh token expiry update"

# Test
cargo test
track scrap add "Fix confirmed. All auth tests passing."

# Complete
track todo done 1

# Push for immediate deployment
cd /path/to/main/repo
jj git push --bookmark task/BUG-999
```

---

## Research and Planning Tasks

Using track for non-code tasks.

### Pattern

```bash
track new "Research authentication providers"

track desc "Evaluate OAuth2 providers for enterprise SSO"

# Research TODOs (no workspaces needed)
track todo add "Research Okta integration options"
track todo add "Evaluate Auth0 pricing"
track todo add "Compare Azure AD features"
track todo add "Document findings and recommendation"

# Work through research
track scrap add "Okta: Strong enterprise features, $2/user/month"
track todo done 1

track scrap add "Auth0: Good DX, but $23/user/month for enterprise"
track todo done 2

track scrap add "Azure AD: Free for existing Microsoft customers"
track todo done 3

# Create summary document
# ... write comparison doc ...
jj describe -m "Add provider comparison document"
track scrap add "Recommendation: Azure AD for cost, Okta for features"
track todo done 4
```

---

## Team Collaboration

Using track in team environments.

### Branch Naming

With ticket integration, bookmarks are consistent:

```bash
# Task created: PROJ-123
# Base bookmark: task/PROJ-123
# TODO bookmarks: task/PROJ-123-todo-1, task/PROJ-123-todo-2, etc.
```

### Sharing Work

```bash
# Push task bookmark
jj git push --bookmark task/PROJ-123

# Team member can pull and continue
jj git fetch

# Use track to see context
track switch t:PROJ-123
track status  # See TODOs and scraps
track scrap list  # View team's notes
```

---

## Long-Running Tasks

Managing tasks over days or weeks.

### Pattern

```bash
# Create task with many TODOs
track new "API v2 migration" --ticket PROJ-500

track todo add "Audit v1 API endpoints"
track todo add "Design v2 API schema"
track todo add "Implement core endpoints" --worktree
track todo add "Migrate auth endpoints" --worktree
track todo add "Migrate data endpoints" --worktree
track todo add "Update client SDKs"
track todo add "Write migration guide"
track todo add "Deploy to staging"
track todo add "Run load tests"
track todo add "Deploy to production"

# Work incrementally
track status  # See progress
# Work on TODO 1
track todo done 1
# Next day, continue
# Work on TODO 2
track todo done 2

# View progress history
track scrap list
```

---

## Quick Reference

### Multi-Repo
```bash
track repo add /path/to/repo1
track repo add /path/to/repo2
track sync
```

### Parallel Work
```bash
# Terminal 1
cd "$(track todo workspace 2)"
# Terminal 2
cd "$(track todo workspace 3)"
```

### Ticket Switching
```bash
track switch t:PROJ-123
track archive t:PROJ-456
```

### Incremental Changes
```bash
jj describe -m "Step 1: ..."
track scrap add "Progress note"
jj describe -m "Step 2: ..."
```

---

## Best Practices

1. **Use tickets**: Enable consistent bookmark naming
2. **Use workspaces**: For complex or parallel work
3. **Describe frequently**: Small, logical changes
4. **Use scraps liberally**: Document decisions and progress
5. **Review status often**: `track status` before and after work
6. **Clean up regularly**: Archive completed tasks

---

## See Also

- [Creating Tasks](creating-tasks.md) - Basic task setup
- [Executing Tasks](executing-tasks.md) - Working through TODOs
- Main [SKILL.md](../SKILL.md) - Quick reference
