#!/usr/bin/env bash
# Fill GitHub Release bodies from CHANGELOG.md (source of truth).
# Needs `gh` with repo write access. Intended for maintainers; CI already
# attaches notes on new tags via .github/workflows/release.yml.
set -euo pipefail
cd "$(dirname "$0")/.."

tags=("$@")
if [ "${#tags[@]}" -eq 0 ]; then
  # Default: tags that were published with empty/stub notes.
  tags=(v0.9.0 v0.8.0 v0.7.0 v0.6.1 v0.6.0 v0.5.0 v0.2.0 v0.1.0)
fi

prev_of() {
  case "$1" in
    v0.9.0) echo v0.8.0 ;;
    v0.8.0) echo v0.7.0 ;;
    v0.7.0) echo v0.6.1 ;;
    v0.6.1) echo v0.6.0 ;;
    v0.6.0) echo v0.5.0 ;;
    v0.5.0) echo v0.4.2 ;;
    v0.2.0) echo v0.1.0 ;;
    *) echo "" ;;
  esac
}

tmp="$(mktemp)"
trap 'rm -f "$tmp"' EXIT

for tag in "${tags[@]}"; do
  python3 scripts/changelog_for_tag.py "$tag" >"$tmp"
  prev="$(prev_of "$tag")"
  echo >>"$tmp"
  if [ -n "$prev" ]; then
    echo "**Full Changelog**: https://github.com/manji-0/track/compare/${prev}...${tag}" >>"$tmp"
  else
    echo "**Full Changelog**: https://github.com/manji-0/track/commits/${tag}" >>"$tmp"
  fi
  echo "Updating $tag"
  gh release edit "$tag" --notes-file "$tmp"
done
