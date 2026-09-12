# LLM Help Command Design

<!-- constrained-by ./JJ_INTEGRATION.md -->
<!-- constrained-by ../DESIGN.md -->

## Overview

`track llm-help` prints a Markdown guide on stdout for coding agents. It describes **track as the WHAT layer** and **`$jj` / jj-task as the HOW layer**. It is not a general human man-page; `track --help` covers that.

## Goals

- Put the two-layer stack and "do not edit repo root" rule at the top.
- Tell agents to read `track status --json` (or mutation `--json`) before guessing the next command.
- Document `jj-task start <jj.slug>`, not `track sync`, as the JJ-mode workspace step.
- Ban hanging on confirmation: `todo delete` needs `--force`; `archive --force` is not the default.

## Command specification

- **Command:** `track llm-help`
- **Output:** Markdown to stdout
- **Implementation:** `handle_llm_help()` in [`src/cli/handlers/llm_help.rs`](../src/cli/handlers/llm_help.rs) (dispatched from `src/cli/handler.rs`)

Keep this file and the printed text in sync. Prefer editing the handler string and then updating this design.

## Content structure

### 0. Mandatory: two-layer stack

<!-- dagayn: implemented-by src/cli/handlers/llm_help.rs::handle_llm_help -->

The output starts with:

1. Track = tasks / TODOs / scraps. `$jj` = commits / PR.
2. Before code changes: `track status --json` → `jj.slug` and `workflow.next_action`.
3. `jj-task start <jj.slug>` (or `cd "$(jj-task path <jj.slug>)"` if already started).
4. Work only in `.worktrees/<slug>/`. Use `$jj` for squash/commit/push/PR.

Git mode may still mention `track sync`. JJ mode must **not** tell agents that `track sync` is the first coding step.

### 1. LLM agent quick start

| Step | Action |
|------|--------|
| 1 | `track status --json` — `workflow.phase`, `next_action`, `jj.slug`, `todos_agent[].is_next` |
| 2 | `jj-task repo init` (once) then `jj-task start <slug>` when `sync_required` |
| 3 | Implement in the jj-task workspace; `$jj` for commits |
| 4 | `track scrap add --json` |
| 5 | `track todo done --json <index>` — follow returned `next_action` |
| 6 | Repeat until `task_complete`, then `$jj` merge, `jj-task done`, `track archive` |

Mutating commands return the **status snapshot** plus `ok` / `mutation`. `track list --json` is a task inventory, not that snapshot.

### 2. Task workflow (human setup, agent execute)

**Setup:** `track new`, `desc`, `ticket`, `repo add`, `todo add` (`--no-workspace` for research). Legacy `--worktree` is an error.

**Execute:** follow JSON phase. JJ: jj-task + `$jj`. Git: `track sync`. Complete TODOs in the track DB; that is not a jj commit.

### 3. Key commands

The printed table should match current clap flags, including `--json` on writes. Do not list `--worktree` as a supported add flag.

Slug for jj-task (not `track sync` bookmark names):

1. `track alias` if set
2. else sanitized `ticket_id` (`PROJ-123` → `proj-123`)
3. else `task-{id}`

### 4. Guardrails in the text

- Non-TTY stdin: confirmation fails; never wait on a prompt.
- `track todo delete N --force` always for agents.
- `track archive` without `--force` unless the user explicitly skips jj-task / dirty checks.
- `track sync` in JJ mode is legacy per-TODO worktrees only (`--legacy` or pending `worktree_requested` rows).
- `track migrate legacy-worktrees` moves old tasks onto jj-task.

## Out of scope

- Per-command JSON schemas (Beads-style). Mutations reuse status JSON.
- MCP. Skills + `llm-help` + CLI `--json` are the agent surface.
- Teaching `$jj` commit rules in this string; point at the `$jj` skill.
