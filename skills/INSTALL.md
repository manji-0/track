# Installation Guide for LLM Agent Skills

This guide explains how to install and configure the `track` CLI skills for various LLM agents and development environments.

## Overview

Track CLI skills are structured workflow guides in markdown format that help LLM agents understand how to use the `track` tool effectively. These skills can be integrated into your development workflow in several ways.

---

## Quick Start

### For Most Users (Direct File Access)

If your LLM agent can access project files, no installation is needed:

1. **Skills location**: `skills/` directory in the track repository
2. **Usage**: Agent reads files on-demand when working with track

**That's it!** The skills are ready to use.

---

## Installation Methods by Agent Type

### 1. Cline / Claude Code Assistant

Cline can reference files directly from your workspace.

**Method A: Direct Reference (Recommended)**

No setup required. Skills are automatically available when working in the track project.

**Method B: Add to Custom Instructions**

1. Open Cline settings
2. Navigate to "Custom Instructions" or "System Prompt"
3. Add the following:

```
When working with the track CLI tool:
- For creating tasks: Reference /path/to/track/skills/create-task-workflow.md
- For executing tasks: Reference /path/to/track/skills/execute-task-workflow.md
- Always run `track status` before starting work
- Use `track llm-help` for quick command reference
```

Replace `/path/to/track` with your actual path.

---

### 2. Aider

Aider can read files from the repository.

**Method: Add Skills to Context**

When starting a session, include skill files in context:

```bash
aider /path/to/track/skills/create-task-workflow.md \
      /path/to/track/skills/execute-task-workflow.md
```

Or add to your `.aider.conf.yml`:

```yaml
read:
  - skills/create-task-workflow.md
  - skills/execute-task-workflow.md
```

---

### 3. GitHub Copilot / Copilot Chat

**Method: Use @workspace References**

In Copilot Chat, reference skills when asking questions:

```
@workspace Using the skills/create-task-workflow.md guide, help me create a new task for [feature name]
```

Or reference in comments:

```python
# Following skills/execute-task-workflow.md:
# 1. Check status with track status
# 2. Navigate to worktree
# 3. Implement changes
```

---

### 4. Cursor AI

Cursor can reference files in the workspace.

**Method A: Direct Reference**

Use `@Files` or `@Codebase` to reference skills:

```
@Files skills/create-task-workflow.md

Help me create a new feature task following this workflow
```

**Method B: Add to Rules for AI**

1. Open Cursor Settings → Features → Rules for AI
2. Add:

```
When working with track CLI:
- Refer to skills/create-task-workflow.md for task creation
- Refer to skills/execute-task-workflow.md for task execution
- Always check current state with `track status` first
```

---

### 5. ChatGPT / GPT-4 (Web Interface)

**Method: Copy-Paste or Upload**

1. Copy skill content from markdown files
2. Paste into conversation, or
3. Use file upload feature (if available) to upload skill files

Example prompt:

```
I'm using the track CLI tool. Here's the workflow guide:

[paste create-task-workflow.md content]

Help me create a task for implementing [feature]
```

---

### 6. Other LLM Agents

**General Method: File System Access**

Most modern LLM coding agents support file system access. Skills are located at:

```
<track-repo>/skills/
  ├── README.md                    # Overview and usage guide
  ├── create-task-workflow.md      # Task creation workflow
  └── execute-task-workflow.md     # Task execution workflow
```

Configure your agent to:
1. Read these files when working with track
2. Follow the documented workflows
3. Reference commands and examples as needed

---

## Verification

After installation, verify skills are working:

### Test 1: Check File Access

Try this prompt with your LLM agent:

```
Can you read the file at skills/create-task-workflow.md and summarize the main workflow steps?
```

**Expected response**: Agent summarizes the 6 main steps (create task, add description, register repo, add TODOs, review, sync).

### Test 2: Use a Skill

```
Following the create-task-workflow skill, help me create a task for adding a dark mode feature.
```

**Expected behavior**: Agent should:
1. Run `track new "Add dark mode"`
2. Ask for description
3. Prompt for ticket info (optional)
4. Suggest TODOs
5. Show final status

---

## Usage Tips

### Best Practices

1. **Start with `track status`**: Always have your agent check current state first
2. **Reference skills by name**: "Following create-task-workflow..."
3. **Combine skills**: Use create-task-workflow → execute-task-workflow for full cycle
4. **Use llm-help**: Quick reference with `track llm-help` command

### Common Workflows

#### Creating a New Task
```
Agent: Following skills/create-task-workflow.md, create a task for [feature]
```

#### Continuing Work on Existing Task
```
Agent: Following skills/execute-task-workflow.md, continue working on the current task
```

#### Full Development Cycle
```
Agent: Using both workflow skills, set up and complete a task for [feature]
```

---

## Advanced: Custom Integration

### Creating Agent-Specific Skills

You can create custom skills tailored to your workflow:

1. Copy an existing skill as a template
2. Modify workflow steps for your use case
3. Save in `skills/` directory
4. Reference in your agent configuration

Example custom skill ideas:
- `hotfix-workflow.md` - For urgent bug fixes
- `pr-preparation-workflow.md` - Preparing tasks for pull requests
- `multi-repo-workflow.md` - Managing cross-repo tasks

### Environment Variables

Set these for consistent paths:

```bash
export TRACK_SKILLS_DIR="/path/to/track/skills"
```

Then reference in agent config:
```
Skills directory: $TRACK_SKILLS_DIR
```

---

## Troubleshooting

### Problem: Agent can't find skill files

**Solution**: Verify file paths

```bash
# Check skills directory exists
ls -la skills/

# Verify files present
ls skills/*.md
```

Provide absolute paths if relative paths don't work:
```
/home/user/projects/track/skills/create-task-workflow.md
```

### Problem: Agent doesn't follow workflow

**Solution**: Be explicit in prompts

Instead of:
```
Create a task for feature X
```

Use:
```
Following the create-task-workflow.md skill, create a task for feature X. Follow each step in order.
```

### Problem: Skills outdated after track update

**Solution**: Pull latest changes

```bash
cd /path/to/track
git pull origin main
```

Skills are version-controlled with the repository.

---

## Support

- **Main Documentation**: See [skills/README.md](README.md) for skill overview
- **Command Reference**: Run `track llm-help` for quick reference
- **Issues**: Report problems at the track repository

---

## Contributing

Want to improve skills or add new ones?

1. Fork the track repository
2. Add/modify skill files in `skills/`
3. Follow the existing structure:
   - Purpose
   - Prerequisites
   - Workflow Steps
   - Examples
   - Tips
   - Reference Commands
4. Update `skills/README.md` with new skill info
5. Submit a pull request

---

## Additional Resources

- **track CLI Documentation**: See main [README.md](../README.md)
- **Functional Spec**: See [docs/FUNCTIONAL_SPEC.md](../docs/FUNCTIONAL_SPEC.md)
- **LLM Help Design**: See [docs/LLM_HELP_DESIGN.md](../docs/LLM_HELP_DESIGN.md)

---

## Changelog

- **2026-01-02**: Initial release with two core skills (create-task, execute-task)
