#!/usr/bin/env python3
"""Smoke-test the track skill plugin package without external dependencies."""

from __future__ import annotations

import json
import py_compile
import re
import sys
from pathlib import Path
from urllib.parse import unquote

ROOT = Path(__file__).resolve().parents[1]
SKILLS_ROOT = ROOT / "skills"

ACTIVE_SKILLS = [
    "track",
    "track-task-setup",
    "track-task-execute",
    "track-advanced",
]


def fail(errors: list[str], message: str) -> None:
    errors.append(message)


def parse_frontmatter(path: Path, errors: list[str]) -> dict[str, str]:
    text = path.read_text(encoding="utf-8")
    if not text.startswith("---\n"):
        fail(errors, f"{path.relative_to(ROOT)}: missing YAML frontmatter")
        return {}

    end = text.find("\n---", 4)
    if end == -1:
        fail(errors, f"{path.relative_to(ROOT)}: unterminated YAML frontmatter")
        return {}

    data: dict[str, str] = {}
    current_key: str | None = None
    for raw in text[4:end].splitlines():
        if not raw.strip():
            continue
        if raw.startswith((" ", "\t")):
            continue
        if ":" not in raw:
            fail(errors, f"{path.relative_to(ROOT)}: invalid frontmatter line: {raw}")
            continue
        key, value = raw.split(":", 1)
        current_key = key.strip()
        data[current_key] = value.strip().strip('"').strip("'")

    if current_key is None:
        fail(errors, f"{path.relative_to(ROOT)}: empty frontmatter")
    return data


def check_json(errors: list[str]) -> None:
    for path in [
        ROOT / ".claude-plugin" / "plugin.json",
        ROOT / ".claude-plugin" / "marketplace.json",
        ROOT / ".codex-plugin" / "plugin.json",
        ROOT / ".agents" / "plugins" / "marketplace.json",
    ]:
        try:
            json.loads(path.read_text(encoding="utf-8"))
        except FileNotFoundError:
            fail(errors, f"{path.relative_to(ROOT)}: missing JSON file")
        except json.JSONDecodeError as exc:
            fail(
                errors,
                f"{path.relative_to(ROOT)}: invalid JSON at {exc.lineno}:{exc.colno}: {exc.msg}",
            )


def as_list(value: object) -> list[str]:
    if isinstance(value, str):
        return [value]
    if isinstance(value, list) and all(isinstance(item, str) for item in value):
        return list(value)
    return []


def has_skill_file(path: Path) -> bool:
    if path.is_file():
        return path.name == "SKILL.md"
    if (path / "SKILL.md").is_file():
        return True
    return (
        any(child.is_dir() and (child / "SKILL.md").is_file() for child in path.iterdir())
        if path.is_dir()
        else False
    )


def check_codex_interface(errors: list[str]) -> None:
    manifest = ROOT / ".codex-plugin" / "plugin.json"
    data = json.loads(manifest.read_text(encoding="utf-8"))
    interface = data.get("interface")
    if not isinstance(interface, dict):
        fail(errors, f"{manifest.relative_to(ROOT)}: missing top-level interface object")
        return
    for key in [
        "displayName",
        "shortDescription",
        "longDescription",
        "developerName",
        "category",
        "capabilities",
        "defaultPrompt",
    ]:
        if not interface.get(key):
            fail(errors, f"{manifest.relative_to(ROOT)}: missing interface.{key}")


def check_manifest_skill_paths(errors: list[str]) -> None:
    manifests = [
        ROOT / ".claude-plugin" / "plugin.json",
        ROOT / ".codex-plugin" / "plugin.json",
    ]
    for manifest in manifests:
        data = json.loads(manifest.read_text(encoding="utf-8"))
        skills = as_list(data.get("skills"))
        if not skills:
            fail(errors, f"{manifest.relative_to(ROOT)}: missing top-level skills path")
        for entry in skills:
            path = (ROOT / entry).resolve()
            if ROOT not in path.parents and path != ROOT:
                fail(errors, f"{manifest.relative_to(ROOT)}: skill path escapes repo: {entry}")
            elif not has_skill_file(path):
                fail(errors, f"{manifest.relative_to(ROOT)}: skill path has no SKILL.md: {entry}")

    marketplace = ROOT / ".claude-plugin" / "marketplace.json"
    data = json.loads(marketplace.read_text(encoding="utf-8"))
    for plugin in data.get("plugins", []):
        if not isinstance(plugin, dict):
            continue
        skills = as_list(plugin.get("skills"))
        if plugin.get("name") == "track" and not skills:
            fail(errors, f"{marketplace.relative_to(ROOT)}: track entry missing skills")
        for entry in skills:
            path = (ROOT / entry).resolve()
            if not has_skill_file(path):
                fail(errors, f"{marketplace.relative_to(ROOT)}: skill path has no SKILL.md: {entry}")


def check_skill_frontmatter(errors: list[str]) -> None:
    for skill in ACTIVE_SKILLS:
        path = SKILLS_ROOT / skill / "SKILL.md"
        if not path.is_file():
            fail(errors, f"{path.relative_to(ROOT)}: missing SKILL.md")
            continue
        data = parse_frontmatter(path, errors)
        rel = path.relative_to(ROOT)
        for key in ["name", "description"]:
            if not data.get(key):
                fail(errors, f"{rel}: missing required frontmatter field {key}")
        expected = path.parent.name
        if data.get("name") and data["name"] != expected:
            fail(
                errors,
                f"{rel}: frontmatter name {data['name']!r} does not match directory {expected!r}",
            )


def check_openai_agent_metadata(errors: list[str]) -> None:
    for skill in ACTIVE_SKILLS:
        path = SKILLS_ROOT / skill / "agents" / "openai.yaml"
        if not path.is_file():
            fail(errors, f"{path.relative_to(ROOT)}: missing agents/openai.yaml")
            continue
        text = path.read_text(encoding="utf-8")
        for key in ["display_name:", "short_description:", "default_prompt:"]:
            if key not in text:
                fail(errors, f"{path.relative_to(ROOT)}: missing {key.rstrip(':')}")


def slugify(heading: str) -> str:
    result: list[str] = []
    for char in heading.strip().lower():
        if char.isalnum() or char == "_":
            result.append(char)
        elif char in {" ", "-"}:
            result.append("-")
    return "".join(result)


def heading_slugs(path: Path) -> set[str]:
    counts: dict[str, int] = {}
    slugs: set[str] = set()
    for line in path.read_text(encoding="utf-8").splitlines():
        match = re.match(r"^\s{0,3}#{1,6}\s+(.+?)\s*#*\s*$", line)
        if not match:
            continue
        base = slugify(match.group(1))
        count = counts.get(base, 0)
        counts[base] = count + 1
        slugs.add(base if count == 0 else f"{base}-{count}")
    return slugs


LINK_RE = re.compile(r"\[[^\]]+\]\(([^)]+)\)")
REF_LINK_RE = re.compile(r"^\[[^\]]+\]:\s+(\S+)", re.MULTILINE)
DIRECTIVE_RE = re.compile(r"@(?:include|import)\s+([^\s]+)")


def split_link_target(target: str) -> tuple[str, str]:
    target = unquote(target.strip().split()[0])
    if "#" in target:
        path, anchor = target.split("#", 1)
        return path, anchor
    return target, ""


def should_skip_target(target: str) -> bool:
    return bool(re.match(r"^[a-z][a-z0-9+.-]*:", target)) or target.startswith("#")


def check_markdown_links(errors: list[str]) -> None:
    for path in sorted(SKILLS_ROOT.rglob("*.md")):
        text = path.read_text(encoding="utf-8")
        targets = LINK_RE.findall(text) + REF_LINK_RE.findall(text) + DIRECTIVE_RE.findall(text)
        for raw_target in targets:
            target_path, anchor = split_link_target(raw_target)
            if should_skip_target(target_path):
                continue
            if not target_path:
                target = path
            else:
                target = (path.parent / target_path).resolve()
            rel = path.relative_to(ROOT)
            if ROOT not in target.parents and target != ROOT:
                fail(errors, f"{rel}: relative Markdown target escapes repo: {raw_target}")
                continue
            if not target.exists():
                fail(errors, f"{rel}: missing relative Markdown target: {raw_target}")
                continue
            if anchor and target.suffix == ".md":
                slugs = heading_slugs(target)
                if slugify(anchor) not in slugs:
                    fail(errors, f"{rel}: missing Markdown anchor {raw_target}")


def check_python_scripts(errors: list[str]) -> None:
    script_dir = ROOT / "scripts"
    if not script_dir.is_dir():
        return
    for path in sorted(script_dir.glob("*.py")):
        try:
            py_compile.compile(str(path), doraise=True)
        except py_compile.PyCompileError as exc:
            fail(errors, f"{path.relative_to(ROOT)}: Python syntax error: {exc.msg}")


def main() -> int:
    errors: list[str] = []
    check_json(errors)
    if not errors:
        check_codex_interface(errors)
        check_manifest_skill_paths(errors)
        check_skill_frontmatter(errors)
        check_openai_agent_metadata(errors)
        check_markdown_links(errors)
        check_python_scripts(errors)

    if errors:
        print("Package smoke test failed:")
        for error in errors:
            print(f"- {error}")
        return 1

    print("Package smoke test passed.")
    print(
        "Checked JSON manifests, skill frontmatter, agents/openai.yaml, "
        "Markdown links, manifest skill paths, and Python scripts."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
