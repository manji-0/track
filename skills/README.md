# Track CLI Skills for LLM Agents

This directory contains skill documents designed to help LLM agents effectively use the `track` CLI tool for task and TODO management.

## What are Skills?

Skills are structured guides that describe common workflows and best practices for using `track`. Each skill provides:

- **Clear workflow steps** with examples
- **When to use** the skill
- **Expected outcomes** 
- **Troubleshooting tips**
- **Reference commands**

LLM agents can reference these skills to understand how to accomplish specific goals with `track`.

## Available Skills

### 1. [Create Task and List TODOs Workflow](./create-task-workflow.md)

**Purpose**: Guide for creating a new task and organizing it with TODOs.

**Use this skill when:**
- Starting a new feature or bug fix
- Breaking down a large task into actionable items
- Setting up a structured development workflow

**Key steps:**
1. Create task with `track new`
2. Add description with `track desc`
3. Register repositories with `track repo add`
4. Add TODOs with `track todo add`
5. Review setup with `track status`
6. Sync repositories with `track sync`

---

### 2. [Execute Task and Complete TODOs Workflow](./execute-task-workflow.md)

**Purpose**: Guide for working through TODOs and completing a task.

**Use this skill when:**
- Ready to start implementing changes
- Working through a list of actionable items
- Need to track progress and complete TODOs

**Key steps:**
1. Check state with `track status`
2. Select next TODO to work on
3. Navigate to worktree (if applicable)
4. Implement and test changes
5. Commit changes
6. Record progress with `track scrap add`
7. Complete TODO with `track todo done`
8. Repeat until all TODOs are done

---

## How to Use These Skills

### For LLM Agents (Cline, Aider, etc.)

1. **Read the skill document** relevant to your current goal
2. **Follow the workflow steps** in order
3. **Use the examples** as templates for commands
4. **Refer to troubleshooting** if you encounter issues
5. **Use reference commands** for quick lookup

### For Human Developers

These skills also serve as comprehensive guides for using `track` effectively:

- **New users**: Read skills in order to learn the complete workflow
- **Experienced users**: Use as reference for specific commands or patterns
- **Team leads**: Share skills with team members for consistent usage

---

## Installation / Setup for LLM Agents

**📖 For detailed installation instructions for specific LLM agents (Cline, Aider, Copilot, Cursor, etc.), see [INSTALL.md](./INSTALL.md)**

### Option 1: Direct File Access (Recommended)

If your LLM agent has access to the project files:

1. Skills are located at `skills/` in the track repository
2. Read files directly when needed:
   - `skills/create-task-workflow.md`
   - `skills/execute-task-workflow.md`

### Option 2: Include in System Prompt

Add the following to your LLM agent's system prompt or custom instructions:

```
When working with the `track` CLI tool:

1. For creating tasks and adding TODOs, refer to the "Create Task Workflow" skill
2. For executing tasks and completing TODOs, refer to the "Execute Task Workflow" skill

Skill documents are available at:
- Create Task: /path/to/track/skills/create-task-workflow.md
- Execute Task: /path/to/track/skills/execute-task-workflow.md

Before starting work, always run `track status` to understand current state.
```

### Option 3: Copy to Agent Configuration

Some LLM agents (like Cline) support custom knowledge bases:

1. Copy skill markdown files to your agent's knowledge directory
2. Reference them by name when needed
3. Agent will have skills available for all projects

---

## Skill Usage Examples

### Example 1: Creating a New Feature Task

**User request:** "Create a task for implementing dark mode"

**Agent should:**
1. Reference **Create Task Workflow** skill
2. Run: `track new "Implement dark mode"`
3. Ask user for description
4. Run: `track desc "Add dark mode theme with automatic switching based on system preference"`
5. Add TODOs:
   ```bash
   track todo add "Create dark mode CSS variables"
   track todo add "Implement theme switcher component" --worktree
   track todo add "Add system preference detection" --worktree
   track todo add "Update all components to use CSS variables"
   track todo add "Write tests for theme switching"
   ```
6. Show final status with `track status`

---

### Example 2: Working Through TODOs

**User request:** "Continue working on the current task"

**Agent should:**
1. Reference **Execute Task Workflow** skill
2. Run: `track status` to check current state
3. Identify next pending TODO
4. If worktree exists, navigate to it
5. Ask user what changes to implement, or proceed with implementation
6. After changes: commit, add scrap, run `track todo done <index>`
7. Repeat for next TODO

---

### Example 3: Multi-Repo Task

**User request:** "Set up a task for syncing data between frontend and backend repos"

**Agent should:**
1. Reference **Create Task Workflow**, specifically "Pattern 3: Multi-Repo Task"
2. Create task:
   ```bash
   track new "Sync user data between services" --ticket PROJ-555 --ticket-url <url>
   track desc "Implement user data synchronization between frontend and backend"
   ```
3. Register both repositories:
   ```bash
   track repo add /path/to/frontend
   track repo add /path/to/backend
   ```
4. Add TODOs for both repos:
   ```bash
   track todo add "Add sync API endpoint in backend" --worktree
   track todo add "Implement sync client in frontend" --worktree
   track todo add "Add integration tests"
   ```
5. Run `track sync` to prepare both repositories

---

## Quick Reference

### Essential Commands

| Command | When to Use |
|---------|-------------|
| `track status` | Check current task state (do this first!) |
| `track new "<name>"` | Start a new task |
| `track todo add "<text>"` | Add a TODO item |
| `track todo done <index>` | Complete a TODO |
| `track scrap add "<note>"` | Record progress or findings |
| `track sync` | Create branches and worktrees |

### Skill Selection Guide

| Your Goal | Use This Skill |
|-----------|----------------|
| Starting new work | [Create Task Workflow](./create-task-workflow.md) |
| Implementing changes | [Execute Task Workflow](./execute-task-workflow.md) |
| Both (full cycle) | Read both skills in order |

---

## Tips for LLM Agents

1. **Always start with `track status`**: Understand current state before taking action
2. **Follow workflows sequentially**: Don't skip steps unless user explicitly requests
3. **Ask for clarification**: If user request is ambiguous, ask before creating tasks/TODOs
4. **Use examples as templates**: Adapt examples from skills to user's specific needs
5. **Record progress**: Use `track scrap add` to document decisions and findings
6. **Verify before completing**: Always test changes before running `track todo done`

---

## Contributing New Skills

To add a new skill:

1. Create a new markdown file in `skills/`
2. Follow the structure of existing skills:
   - Purpose
   - When to Use
   - Prerequisites
   - Workflow Steps (detailed)
   - Examples
   - Tips
   - Troubleshooting
   - Reference Commands
3. Update this README with skill description
4. Submit a pull request

**Skill ideas:**
- Managing multiple tasks with `track switch`
- Archiving completed tasks
- Using ticket references effectively
- Troubleshooting common issues
- Advanced worktree management

---

## Additional Resources

- **Main CLI Help**: Run `track llm-help` for comprehensive workflow guide
- **Command Reference**: Run `track --help` for all available commands
- **Project README**: See main repository README for installation and setup

---

## License

These skills are part of the `track` CLI project and share the same license (MIT).
