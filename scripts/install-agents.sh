#!/usr/bin/env bash
# Append or refresh the managed Track section in AGENTS.md.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/manji-0/track/master/scripts/install-agents.sh | bash
#   ./scripts/install-agents.sh [--global|--project] [--path FILE] [--dry-run] [--skills]
#
# Prefer `track install agents` when the track CLI is already installed.

set -euo pipefail

SCOPE="global"
TARGET_PATH=""
DRY_RUN=0
INSTALL_SKILLS=0

usage() {
  cat <<'EOF'
Usage: install-agents.sh [options]

Append or refresh the Track managed section in AGENTS.md.

Options:
  --global          Write to ~/.agents/AGENTS.md (default)
  --project         Write to ./AGENTS.md in the current directory
  --path FILE       Write to a custom AGENTS.md path
  --dry-run         Print actions without writing files
  --skills          Also run npx skills add for track + agent-skill-jj
  -h, --help        Show this help

When track is on PATH, this script delegates to:
  track install agents ...
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --global)
      SCOPE="global"
      shift
      ;;
    --project)
      SCOPE="project"
      shift
      ;;
    --path)
      TARGET_PATH="${2:-}"
      if [[ -z "$TARGET_PATH" ]]; then
        echo "error: --path requires a value" >&2
        exit 1
      fi
      SCOPE="custom"
      shift 2
      ;;
    --dry-run)
      DRY_RUN=1
      shift
      ;;
    --skills)
      INSTALL_SKILLS=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown option: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

if command -v track >/dev/null 2>&1; then
  args=(install agents)
  case "$SCOPE" in
    global) args+=(--global) ;;
    project) args+=(--project) ;;
    custom) args+=(--path "$TARGET_PATH") ;;
  esac
  if [[ "$DRY_RUN" -eq 1 ]]; then
    args+=(--dry-run)
  fi
  if [[ "$INSTALL_SKILLS" -eq 1 ]]; then
    args+=(--skills)
  fi
  exec track "${args[@]}"
fi

MARKER_START='<!-- track:agents:start v1 -->'
MARKER_END='<!-- track:agents:end -->'

resolve_target() {
  case "$SCOPE" in
    global)
      if [[ -n "${HOME:-}" ]]; then
        printf '%s/.agents/AGENTS.md' "$HOME"
      else
        echo "error: HOME is not set" >&2
        exit 1
      fi
      ;;
    project)
      printf '%s/AGENTS.md' "$PWD"
      ;;
    custom)
      printf '%s' "$TARGET_PATH"
      ;;
  esac
}

fetch_body() {
  if [[ -f templates/agents-track.md ]]; then
    cat templates/agents-track.md
    return
  fi

  local url='https://raw.githubusercontent.com/manji-0/track/master/templates/agents-track.md'
  if ! command -v curl >/dev/null 2>&1; then
    echo "error: track CLI not found and curl is unavailable" >&2
    echo "Install track, or run this script from the track repository checkout." >&2
    exit 1
  fi
  curl -fsSL "$url"
}

TARGET="$(resolve_target)"
BODY="$(fetch_body)"
SECTION="${MARKER_START}
${BODY}
${MARKER_END}
"

export TARGET DRY_RUN SECTION MARKER_START MARKER_END
python3 <<'PY'
import os
import pathlib

target = pathlib.Path(os.environ["TARGET"])
dry_run = os.environ["DRY_RUN"] == "1"
section = os.environ["SECTION"]
start = os.environ["MARKER_START"]
end = os.environ["MARKER_END"]

existing = target.read_text(encoding="utf-8") if target.exists() else ""

def normalize_prefix(before: str) -> str:
    if not before:
        return ""
    return before if before.endswith("\n") else before + "\n"

def normalize_suffix(after: str) -> str:
    if not after:
        return ""
    return after if after.startswith("\n") else "\n" + after

if start in existing and end in existing:
    before, rest = existing.split(start, 1)
    _, after = rest.split(end, 1)
    merged = normalize_prefix(before) + section + normalize_suffix(after)
    action = "updated" if merged != existing else "unchanged"
else:
    if existing.strip():
        merged = existing.rstrip() + ("\n" if existing.endswith("\n") else "\n\n") + section
        action = "updated"
    else:
        merged = section
        action = "created"

if dry_run:
    verb = {"created": "create", "updated": "update", "unchanged": "leave unchanged"}[action]
    print(f"Would {verb}: {target}")
    print("\nManaged section preview:\n")
    print(section, end="")
elif action == "unchanged":
    print(f"Already up to date: {target}")
else:
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(merged, encoding="utf-8")
    print(f"{action.capitalize()}: {target}")
PY

if [[ "$INSTALL_SKILLS" -eq 1 ]]; then
  if [[ "$DRY_RUN" -eq 1 ]]; then
    echo "Skipping skill install in --dry-run mode"
    echo "Would run: npx skills add manji-0/track -s track -s track-task-execute -g -y"
    exit 0
  fi
  if ! command -v npx >/dev/null 2>&1; then
    echo "error: npx not found (install Node.js 18+ for --skills)" >&2
    exit 1
  fi
  npx skills add manji-0/track \
    -s track -s track-task-setup -s track-task-execute -s track-advanced \
    -g -a cursor -a claude-code -a codex -y
  npx skills add manji-0/agent-skill-jj -s jj -g -y
fi
