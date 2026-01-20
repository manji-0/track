# LLM Integration

Track provides **Agent Skills** following the [official Agent Skills specification](https://github.com/anthropics/skills) for LLM agents (Claude Code, Cline, Cursor, etc.).

## Quick Start for Agents

```bash
# Always start with status
track status

# Reference the main skill
skills/track-task-management/SKILL.md
```

Skills use progressive disclosure:
- **SKILL.md**: Quick start and overview (~1000 tokens)
- **references/**: Detailed guides loaded only when needed

## Main Skill: track-task-management

**Purpose**: Manages development tasks with integrated JJ workspaces.

**Use when**: Creating tasks, adding TODOs, working through task lists, or managing development workflows.

**Quick commands:**
```bash
track new "<task>"              # Create task
track todo add "<item>"         # Add TODO  
track todo done <index>         # Complete TODO
track scrap add "<note>"        # Record progress
track status                    # Check state
```

## Detailed References

The main skill references detailed guides for specific workflows:

| Reference | When to Use |
|-----------|-------------|
| [creating-tasks.md](../skills/track-task-management/references/creating-tasks.md) | Setting up new tasks |
| [executing-tasks.md](../skills/track-task-management/references/executing-tasks.md) | Working through TODOs |
| [advanced-workflows.md](../skills/track-task-management/references/advanced-workflows.md) | Multi-repo, parallel work |

## LLM Help Command

For quick CLI reference:

```bash
track llm-help
```

Outputs comprehensive guide with commands, ticket integration, and workspace details.
Includes JJ bookmark verification steps (`jj status`, `jj bookmark list -r @`).

## Installation

**No setup required** - Skills auto-detected in workspace.

For agent-specific configuration, see [skills/INSTALL.md](../skills/INSTALL.md).

Full skill documentation: [skills/README.md](../skills/README.md)
