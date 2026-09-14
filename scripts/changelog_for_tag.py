#!/usr/bin/env python3
"""Print the Keep a Changelog section for a git tag (v0.9.0 or 0.9.0)."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path


def section_for(changelog: str, version: str) -> str:
    heading = f"## [{version}]"
    lines = changelog.splitlines()
    start = None
    for i, line in enumerate(lines):
        if line.startswith(heading):
            start = i
            break
    if start is None:
        raise SystemExit(f"no changelog heading {heading!r}")

    end = len(lines)
    for j in range(start + 1, len(lines)):
        if lines[j].startswith("## ["):
            end = j
            break
    body = "\n".join(lines[start:end]).strip()
    return body + "\n"


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("tag")
    parser.add_argument(
        "--changelog",
        type=Path,
        default=Path("CHANGELOG.md"),
    )
    args = parser.parse_args()
    version = args.tag.lstrip("v")
    text = args.changelog.read_text(encoding="utf-8")
    sys.stdout.write(section_for(text, version))


if __name__ == "__main__":
    main()
