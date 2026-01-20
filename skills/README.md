# Track CLI Skills for LLM Agents

Official [Agent Skills](https://github.com/anthropics/skills) for the `track` CLI tool.

## What are Agent Skills?

Agent Skills are structured workflow guides following the official Agent Skills specification. They help LLM agents (Claude Code, Cline, Cursor, etc.) effectively use the `track` CLI through progressive disclosure:

- **Level 1**: Metadata (loaded at startup, ~100 tokens)
- **Level 2**: Quick start and overview (loaded when skill active, ~1000 tokens)
- **Level 3**: Detailed references (loaded only when needed)

## Available Skills

### [track-task-management](track-task-management/SKILL.md)

**Purpose**: Manages development tasks with integrated JJ workspaces, WebUI, and link management.

**Use when**: Creating tasks, adding TODOs, working through task lists, managing references, or managing development workflows.

**Key features:**
- Task and TODO management with task-scoped indices
- JJ workspace integration for parallel development
- Ticket integration (Jira, GitHub, GitLab)
- Progress tracking with Markdown-enabled scraps
- Link management for references
- Template support for recurring workflows
- Alias support for easy task switching
- Web-based UI with real-time updates

**Quick start:**
```bash
# Create task with ticket
track new "Feature name" --ticket PROJ-123 --ticket-url <url>

# Or create from template
track new "Sprint 2" --template t:PROJ-100

# Add TODOs and links
track todo add "Implement core logic" --worktree
track todo add "Write tests"
track link add https://docs.example.com/api --title "API Docs"

# Sync and work
track sync
cd "$(track todo workspace 1)"
jj status
jj describe -m "Implement core logic"
track todo done 1
```

---

## Installation

Skills are automatically available when you have access to the track repository.

For detailed installation instructions for specific LLM agents, see [INSTALL.md](INSTALL.md).

### Quick Setup

**For most LLM agents:**

1. Skills are located at: `skills/track-task-management/`
2. Agent will automatically discover and load `SKILL.md`
3. Reference files loaded only when needed

**For Claude Code / Cline:**
- Skills auto-detected in workspace
- No configuration needed

**For Cursor / VS Code Copilot:**
- Reference skills using `@Files` or workspace context

---

## Usage Examples

### Creating a Task

```
Agent: Following the track-task-management skill, create a task for implementing user authentication
```

The agent will:
1. Read `SKILL.md` for overview
2. Reference `references/creating-tasks.md` for details
3. Execute workflow steps

### Executing TODOs

```
Agent: Continue working on the current track task
```

The agent will:
1. Check current state with `track status`
2. Reference `references/executing-tasks.md`
3. Work through pending TODOs

### Advanced Workflows

```
Agent: Set up a multi-repo task for API and frontend changes
```

The agent will:
1. Reference `references/advanced-workflows.md`
2. Follow multi-repo pattern
3. Set up coordinated changes

---

## Skill Structure

Following Agent Skills standard:

```
track-task-management/
├── SKILL.md              # Main skill file (required)
│                         # - YAML frontmatter with metadata
│                         # - Quick start guide
│                         # - References to detailed docs
└── references/           # Detailed documentation (optional)
    ├── creating-tasks.md      # Task creation workflow
    ├── executing-tasks.md     # Task execution workflow
    └── advanced-workflows.md  # Advanced patterns
```

---

## For LLM Agents

### Standard Pattern

1. **Run `track sync`** BEFORE any code changes (MANDATORY)
2. **Verify bookmark** with `jj status` (must be task bookmark)
3. **Check `SKILL.md`** for overview and quick start
4. **Identify user's goal** (creating task vs. executing vs. advanced)
5. **Load relevant reference** only if needed:
    - Creating task → `references/creating-tasks.md`
    - Executing task → `references/executing-tasks.md`
    - Advanced use case → `references/advanced-workflows.md`
6. **Follow workflow** step-by-step
7. **Use examples** as templates

### Quick Commands

Always available in `SKILL.md`:
- `track sync` - **MANDATORY FIRST STEP** - Sync bookmarks and create workspaces
- `track status` - Check current state
- `track new` - Create task
- `track todo add` - Add TODO
- `track todo workspace` - Show TODO workspace
- `track todo done` - Complete TODO
- `track link add` - Add reference link
- `track scrap add` - Record note
- `track webui` - Start web UI
- `track llm-help` - Show comprehensive help


---

## Comparison with Previous Version

### What Changed

**Old structure** (non-standard):
```
skills/
├── create-task-workflow.md    (300+ lines)
├── execute-task-workflow.md   (350+ lines)
├── README.md
└── INSTALL.md
```

**New structure** (Agent Skills standard):
```
skills/
├── track-task-management/
│   ├── SKILL.md               (100 lines, with YAML)
│   └── references/
│       ├── creating-tasks.md  (200 lines)
│       ├── executing-tasks.md (250 lines)
│       └── advanced-workflows.md (250 lines)
├── README.md (this file)
└── INSTALL.md
```

### Benefits

- **Progressive Disclosure**: Only loads what's needed
- **Better organization**: Domain-separated references
- **Standard compliance**: Works with all Agent Skills platforms
- **Token efficiency**: ~90% reduction in baseline tokens
- **Portability**: Can be shared via Skills Marketplace

---

## Additional Resources

- **Installation Guide**: [INSTALL.md](INSTALL.md) - Detailed setup for different agents
- **Quick Reference**: Run `track llm-help` for CLI reference
- **Full Documentation**: See main [README.md](../README.md)
- **Agent Skills Spec**: https://github.com/anthropics/skills

---

## Contributing

To improve or extend skills:

1. Edit `track-task-management/SKILL.md` for quick start
2. Add/edit files in `references/` for detailed guides
3. Keep SKILL.md under 5000 tokens
4. Follow progressive disclosure principles
5. Test with multiple LLM agents

---

## License

MIT License (same as track project)
