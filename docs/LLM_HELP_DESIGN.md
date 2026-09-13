# LLM Help Command Design

<!-- constrained-by ./JJ_INTEGRATION.md -->
<!-- constrained-by ../DESIGN.md -->

## Overview

`track llm-help` prints a Markdown guide on stdout for coding agents. It describes **track as owning task state and the coding workspace**. It is not a general human man-page; `track --help` covers that.

## Goals

- Put "follow `hint` / `workflow.next_action`" and "do not edit repo root" at the top.
- Tell agents to read `track status --json` (or mutation `--json`) before guessing the next command.
- Document `track sync` / `track repo add` as the workspace step in **both** git and jj modes.
- Ban hanging on confirmation: `todo delete` needs `--force`; `archive --force` is not the default.
- Do not send agents to jj-task or other external workspace maps.

## Command specification

- **Command:** `track llm-help`
- **Output:** Markdown to stdout
- **Implementation:** `handle_llm_help()` in [`src/cli/handlers/llm_help.rs`](../src/cli/handlers/llm_help.rs) (dispatched from `src/cli/handler.rs`)

Keep this file and the printed text in sync. Prefer editing the handler string and then updating this design.

## Content structure

### 0. Mandatory: track-owned workspace

<!-- dagayn: implemented-by src/cli/handlers/llm_help.rs::handle_llm_help -->

The output starts with:

1. Track = tasks / TODOs / scraps **and** the coding workspace.
2. Before code changes: `track status --json` → `hint` and `workflow.next_action`.
3. `track repo add` / `track sync` create `.worktrees/<slug>/` on `track/<slug>`.
4. Work only in that path. Commit and push with git or jj from the workspace.

Human commands also print a stderr footer (`hint:` / `next:`). `--json` includes `hint`. `TRACK_HINTS=0` hides the footer.

### 1. LLM agent quick start

| Step | Action |
|------|--------|
| 1 | `track status --json` — `workflow.phase`, `hint`, `next_action`, `todos_agent[].is_next` |
| 2 | Run `hint.next_command` (`track repo add`, `track sync`, or `cd "<path>"`) |
| 3 | Implement in `.worktrees/<slug>/`; commit from that workspace |
| 4 | `track scrap add --json` |
| 5 | `track todo done --json <index>` — follow returned `next_action` |
| 6 | Repeat until `task_complete`, then push `track/<slug>`, `track archive` |

Mutating commands return the **status snapshot** plus `ok` / `mutation`. `track list --json` is a task inventory, not that snapshot.

### 2. Task workflow (human setup, agent execute)

**Setup:** `track new`, `desc`, `ticket`, `repo add`, `todo add` (`--no-workspace` for research). Legacy `--worktree` is an error.

**Execute:** follow JSON phase. Workspace: `track sync`. Complete TODOs in the track DB; that is not a git/jj commit.

### 3. Key commands

The printed table should match current clap flags, including `--json` on writes. Do not list `--worktree` as a supported add flag.

Slug (workspace directory and `track/<slug>` branch/bookmark):

1. `track alias` if set
2. else sanitized `ticket_id` (`PROJ-123` → `proj-123`)
3. else `task-{id}`

### 4. Guardrails in the text

- Non-TTY stdin: confirmation fails; never wait on a prompt.
- `track todo delete N --force` always for agents.
- `track archive` without `--force` unless the user explicitly skips dirty checks.
- `track sync --legacy` is leftover per-TODO jj worktrees only.
- `track migrate legacy-worktrees` clears old per-TODO worktree flags.

## Out of scope

- Per-command JSON schemas (Beads-style). Mutations reuse status JSON.
- MCP. Skills + `llm-help` + CLI `--json` are the agent surface.
- Teaching git/jj commit conventions; point at the workspace path and PR head `track/<slug>`.
