#!/usr/bin/env python3
"""Isolated mock-repo scenarios for track aggressive-mode (git and jj).

Does not touch the developer's real Track DB. HOME is a throwaway directory.
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
from dataclasses import asdict, dataclass, field
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
TRACK = Path(os.environ.get("TRACK_BIN", REPO / "target/debug/track"))
ROOT = Path(os.environ.get("TRACK_SIM_ROOT", "/tmp/track-sim"))
RESULTS = ROOT / "results.json"


@dataclass
class Check:
    id: str
    mode: str
    family: str
    title: str
    expected: str
    actual: str
    status: str  # pass | fail | issue | skip
    detail: str = ""


@dataclass
class Ctx:
    mode: str
    home: Path
    repo: Path
    origin: Path
    env: dict = field(default_factory=dict)
    slug: str = "sim-1"
    worktree: Path | None = None
    marker: str | None = None


def write_gitconfig(home: Path) -> None:
    (home / ".gitconfig").write_text(
        "[user]\n"
        "\tname = track-sim\n"
        "\temail = track@sim.test\n"
        "[commit]\n"
        "\tgpgsign = false\n"
        "[tag]\n"
        "\tgpgsign = false\n"
        "[init]\n"
        "\tdefaultBranch = main\n"
    )
    jj_dir = home / ".config" / "jj"
    jj_dir.mkdir(parents=True, exist_ok=True)
    (jj_dir / "config.toml").write_text(
        '[user]\nname = "track-sim"\nemail = "track@sim.test"\n'
    )


def make_env(home: Path) -> dict:
    env = os.environ.copy()
    env["HOME"] = str(home)
    env["XDG_CONFIG_HOME"] = str(home / ".config")
    env["XDG_DATA_HOME"] = str(home / ".local" / "share")
    env["GIT_CONFIG_NOSYSTEM"] = "1"
    env["GIT_CONFIG_GLOBAL"] = str(home / ".gitconfig")
    env["TRACK_HINTS"] = "0"
    env["GNUPGHOME"] = str(home / ".gnupg")
    (home / ".gnupg").mkdir(parents=True, exist_ok=True)
    return env


def run(
    args: list[str],
    *,
    cwd: Path,
    env: dict,
    check: bool = True,
    input_text: str | None = None,
) -> subprocess.CompletedProcess:
    r = subprocess.run(
        args,
        cwd=str(cwd),
        env=env,
        text=True,
        capture_output=True,
        input=input_text,
    )
    if check and r.returncode != 0:
        raise RuntimeError(
            f"{args} failed ({r.returncode})\nstdout:\n{r.stdout}\nstderr:\n{r.stderr}"
        )
    return r


def track(ctx: Ctx, args: list[str], *, cwd: Path | None = None, ok: bool = True):
    return run([str(TRACK), *args], cwd=cwd or ctx.repo, env=ctx.env, check=ok)


def git(ctx: Ctx, args: list[str], *, cwd: Path | None = None, ok: bool = True):
    return run(["git", *args], cwd=cwd or ctx.repo, env=ctx.env, check=ok)


def jj(ctx: Ctx, args: list[str], *, cwd: Path | None = None, ok: bool = True):
    return run(["jj", *args], cwd=cwd or ctx.worktree or ctx.repo, env=ctx.env, check=ok)


def track_json(ctx: Ctx, args: list[str], *, cwd: Path | None = None) -> dict:
    r = track(ctx, args, cwd=cwd)
    return json.loads(r.stdout)


def add(
    checks: list[Check],
    *,
    cid: str,
    ctx: Ctx,
    family: str,
    title: str,
    expected: str,
    actual: str,
    ok: bool,
    status_if_false: str = "fail",
    detail: str = "",
):
    checks.append(
        Check(
            id=cid,
            mode=ctx.mode,
            family=family,
            title=title,
            expected=expected,
            actual=actual,
            status="pass" if ok else status_if_false,
            detail=detail,
        )
    )


def git_cwd(ctx: Ctx) -> Path:
    """JJ workspaces have no .git; git -C walks up to repo root (usually main)."""
    if ctx.mode == "jj":
        return ctx.repo
    return ctx.worktree or ctx.repo


def task_tip(ctx: Ctx) -> str:
    if ctx.mode == "jj":
        return jj(
            ctx,
            ["log", "-r", f"track/{ctx.slug}", "--no-graph", "-T", "commit_id"],
        ).stdout.strip()
    return rev_parse(ctx, "HEAD", cwd=ctx.worktree)


def notes_show(ctx: Ctx, sha: str, cwd: Path | None = None) -> str | None:
    r = git(
        ctx,
        ["notes", "--ref", "refs/notes/track", "show", sha],
        cwd=cwd or git_cwd(ctx),
        ok=False,
    )
    if r.returncode != 0:
        return None
    return r.stdout.strip()


def rev_parse(ctx: Ctx, spec: str, cwd: Path | None = None) -> str:
    return git(ctx, ["rev-parse", spec], cwd=cwd or git_cwd(ctx)).stdout.strip()


def log_oneline(ctx: Ctx, rng: str, cwd: Path | None = None) -> list[str]:
    r = git(
        ctx,
        ["log", "--reverse", "--format=%H %s", rng],
        cwd=cwd or git_cwd(ctx),
        ok=False,
    )
    if r.returncode != 0:
        return []
    return [ln for ln in r.stdout.splitlines() if ln.strip()]


def trailers(ctx: Ctx, sha: str) -> str:
    return git(ctx, ["log", "-1", "--format=%B", sha], cwd=git_cwd(ctx)).stdout


def todo_range(ctx: Ctx) -> str:
    return f"{ctx.marker}..{task_tip(ctx)}"


def push_task_branch(ctx: Ctx) -> subprocess.CompletedProcess:
    branch = f"track/{ctx.slug}"
    if ctx.mode == "jj":
        jj(ctx, ["git", "export"], ok=False)
        r = git(ctx, ["push", "-u", "origin", branch], cwd=ctx.repo, ok=False)
        if r.returncode != 0:
            r = jj(
                ctx,
                ["git", "push", "--remote", "origin", "--bookmark", branch],
                ok=False,
            )
        jj(ctx, ["git", "fetch", "--remote", "origin"], ok=False)
    else:
        r = git(
            ctx,
            ["push", "-u", "origin", branch],
            cwd=ctx.worktree or ctx.repo,
            ok=False,
        )
    git(ctx, ["fetch", "origin"], cwd=ctx.repo, ok=False)
    return r


def is_ancestor(ctx: Ctx, anc: str, desc: str) -> bool:
    if not anc or not desc:
        return False
    return (
        git(
            ctx,
            ["merge-base", "--is-ancestor", anc, desc],
            cwd=ctx.repo,
            ok=False,
        ).returncode
        == 0
    )


def notes_ref_sha(ctx: Ctx, cwd: Path | None = None) -> str:
    r = git(
        ctx,
        ["rev-parse", "refs/notes/track"],
        cwd=cwd or git_cwd(ctx),
        ok=False,
    )
    return r.stdout.strip() if r.returncode == 0 else ""


def origin_branch_sha(ctx: Ctx, branch: str | None = None) -> str:
    b = branch or f"track/{ctx.slug}"
    r = git(
        ctx,
        ["rev-parse", f"refs/remotes/origin/{b}"],
        cwd=ctx.repo,
        ok=False,
    )
    if r.returncode == 0 and r.stdout.strip():
        return r.stdout.strip()
    if ctx.mode == "jj":
        return jj(
            ctx,
            ["log", "-r", f"{b}@origin", "--no-graph", "-T", "commit_id"],
            ok=False,
        ).stdout.strip()
    return ""


def undescribed_ancestors(ctx: Ctx, rev: str) -> str:
    if ctx.mode != "jj":
        return ""
    r = jj(
        ctx,
        [
            "log",
            "--no-graph",
            "-r",
            f'ancestors({rev}) & description(exact:"") & ~root()',
            "-T",
            'commit_id.short() ++ "\\n"',
        ],
        ok=False,
    )
    return (r.stdout or "").strip()


def unique_from_main(ctx: Ctx, tip: str) -> list[str]:
    r = git(
        ctx,
        ["rev-list", "--reverse", f"main..{tip}"],
        cwd=ctx.repo,
        ok=False,
    )
    return [ln.strip() for ln in r.stdout.splitlines() if ln.strip()]


def task_todo_indexes(ctx: Ctx) -> list[int]:
    indexes: list[int] = []
    for ln in log_oneline(ctx, todo_range(ctx)):
        body = trailers(ctx, ln.split()[0])
        for line in body.splitlines():
            if line.startswith("Task-Todo:"):
                try:
                    indexes.append(int(line.split(":", 1)[1].strip()))
                except ValueError:
                    pass
    return indexes


def task_todo_shas(ctx: Ctx) -> dict[int, str]:
    found: dict[int, str] = {}
    for ln in log_oneline(ctx, todo_range(ctx)):
        sha = ln.split()[0]
        body = trailers(ctx, sha)
        for line in body.splitlines():
            if line.startswith("Task-Todo:"):
                try:
                    found[int(line.split(":", 1)[1].strip())] = sha
                except ValueError:
                    pass
    return found


def write_ws(ctx: Ctx, rel: str, text: str) -> None:
    ws = ctx.worktree
    assert ws is not None
    path = ws / rel
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)


def merge_task_to_main(ctx: Ctx, slug: str) -> subprocess.CompletedProcess:
    git(ctx, ["fetch", "origin"], cwd=ctx.repo, ok=False)
    branch = f"track/{slug}"
    if ctx.mode == "git":
        r = git(
            ctx,
            ["merge", "--no-ff", "-m", f"Merge {branch}", branch],
            cwd=ctx.repo,
            ok=False,
        )
        git(ctx, ["push", "origin", "main"], cwd=ctx.repo, ok=False)
        return r
    r = jj(
        ctx,
        ["new", "-m", f"Merge {branch}", "main", branch],
        cwd=ctx.repo,
        ok=False,
    )
    jj(ctx, ["bookmark", "set", "main", "-r", "@"], cwd=ctx.repo, ok=False)
    jj(ctx, ["git", "push", "--remote", "origin", "--bookmark", "main"], cwd=ctx.repo, ok=False)
    git(ctx, ["fetch", "origin"], cwd=ctx.repo, ok=False)
    return r


def merge_main_into_task(ctx: Ctx) -> subprocess.CompletedProcess:
    ws = ctx.worktree
    assert ws is not None
    git(ctx, ["fetch", "origin"], cwd=ctx.repo, ok=False)
    if ctx.mode == "git":
        return git(ctx, ["merge", "--no-edit", "origin/main"], cwd=ws, ok=False)
    jj(ctx, ["git", "fetch", "--remote", "origin"], cwd=ws, ok=False)
    r = jj(ctx, ["new", "-m", "Merge main into task", "@", "main"], cwd=ws, ok=False)
    jj(ctx, ["bookmark", "set", f"track/{ctx.slug}", "-r", "@"], cwd=ws, ok=False)
    return r


def jj_conflicted(ctx: Ctx) -> bool:
    if ctx.mode != "jj":
        return False
    text = jj(
        ctx,
        ["log", "-r", "@", "--no-graph", "-T", "conflict"],
        ok=False,
    ).stdout.lower()
    return "true" in text


def workspace_conflicted(ctx: Ctx) -> bool:
    if ctx.mode == "jj":
        return jj_conflicted(ctx)
    r = git(
        ctx,
        ["rev-parse", "-q", "--verify", "MERGE_HEAD"],
        cwd=ctx.worktree,
        ok=False,
    )
    return r.returncode == 0


def commit_files(ctx: Ctx, sha: str) -> list[str]:
    r = git(ctx, ["ls-tree", "-r", "--name-only", sha], cwd=ctx.repo, ok=False)
    return [ln.strip() for ln in r.stdout.splitlines() if ln.strip()]


def commit_blob(ctx: Ctx, sha: str, path: str) -> str:
    r = git(ctx, ["show", f"{sha}:{path}"], cwd=ctx.repo, ok=False)
    return r.stdout if r.returncode == 0 else ""


def resolve_merge(ctx: Ctx, rel: str, text: str) -> subprocess.CompletedProcess:
    write_ws(ctx, rel, text)
    ws = ctx.worktree
    assert ws is not None
    if ctx.mode == "git":
        git(ctx, ["add", "--", rel], cwd=ws, ok=False)
        return git(
            ctx,
            ["commit", "--no-edit", "-m", "Resolve merge conflict"],
            cwd=ws,
            ok=False,
        )
    jj(ctx, ["diff", "--stat"], cwd=ws, ok=False)
    return jj(
        ctx,
        ["bookmark", "set", f"track/{ctx.slug}", "-r", "@"],
        cwd=ws,
        ok=False,
    )


def bind_status(ctx: Ctx, status: dict) -> None:
    g = status.get("git") or status.get("jj") or {}
    ctx.worktree = Path(g["workspace_path"])
    ctx.slug = g["slug"]
    ctx.marker = marker_sha(ctx)


def init_product(ctx: Ctx) -> None:
    ctx.origin.mkdir(parents=True)
    run(
        ["git", "init", "--bare", "-b", "main", str(ctx.origin)],
        cwd=ROOT,
        env=ctx.env,
    )
    ctx.repo.mkdir(parents=True)
    git(ctx, ["init", "-b", "main"])
    git(ctx, ["config", "user.email", "track@sim.test"])
    git(ctx, ["config", "user.name", "track-sim"])
    git(ctx, ["config", "commit.gpgsign", "false"])
    (ctx.repo / "README.md").write_text("# widgets\n")
    git(ctx, ["add", "README.md"])
    git(ctx, ["commit", "-m", "init"])
    git(ctx, ["remote", "add", "origin", str(ctx.origin)])
    git(ctx, ["push", "-u", "origin", "main"])


def boot_track(ctx: Ctx) -> None:
    track(ctx, ["config", "set", "vcs-mode", ctx.mode])
    track(ctx, ["config", "set", "aggressive-mode", "on"])


def setup_task(ctx: Ctx, name: str, ticket: str, todos: list[tuple[str, bool]]) -> dict:
    snap = track_json(ctx, ["new", name, "--ticket", ticket, "--json"])
    for text, research in todos:
        args = ["todo", "add", text, "--json"]
        if research:
            args.insert(2, "--no-workspace")
        track_json(ctx, args)
    track_json(ctx, ["repo", "add", str(ctx.repo), "--json"])
    status = track_json(ctx, ["status", "--json"])
    ws = (status.get("git") or status.get("jj") or {}).get("workspace_path")
    ctx.worktree = Path(ws)
    ctx.slug = (status.get("git") or status.get("jj") or {}).get("slug") or ticket.lower()
    return status


def marker_sha(ctx: Ctx) -> str:
    listed = git(
        ctx, ["notes", "--ref", "refs/notes/track", "list"], cwd=git_cwd(ctx), ok=False
    )
    tip = task_tip(ctx)
    candidates = []
    for line in listed.stdout.splitlines():
        parts = line.split()
        if len(parts) < 2:
            continue
        sha = parts[1]
        anc = git(
            ctx,
            ["merge-base", "--is-ancestor", sha, tip],
            cwd=git_cwd(ctx),
            ok=False,
        )
        if anc.returncode != 0:
            continue
        body = notes_show(ctx, sha) or ""
        if '"format": "track-task"' in body:
            candidates.append(sha)
    if not candidates:
        raise RuntimeError(f"no marker found\nnotes={listed.stdout}\ntip={tip}")
    oldest = candidates[0]
    for sha in candidates[1:]:
        if (
            git(
                ctx,
                ["merge-base", "--is-ancestor", sha, oldest],
                cwd=git_cwd(ctx),
                ok=False,
            ).returncode
            == 0
        ):
            oldest = sha
    return oldest


def family_birth(ctx: Ctx, checks: list[Check], status: dict) -> None:
    ws = ctx.worktree
    assert ws is not None
    add(
        checks,
        cid="A1",
        ctx=ctx,
        family="birth",
        title="Workspace exists at .worktrees/<slug>",
        expected=str(ctx.repo / ".worktrees" / ctx.slug),
        actual=str(ws),
        ok=ws.exists() and ws == ctx.repo / ".worktrees" / ctx.slug,
    )
    ctx.marker = marker_sha(ctx)
    marker_notes = notes_show(ctx, ctx.marker) or ""
    add(
        checks,
        cid="A2",
        ctx=ctx,
        family="birth",
        title="Marker notes are task identity, not scraps",
        expected="track-task with name/ticket; empty scraps",
        actual=marker_notes[:400],
        ok=(
            "OAuth refresh" in marker_notes
            and "SIM-1" in marker_notes
            and '"scraps": []' in marker_notes.replace(" ", "")
            or ('"scraps":[]' in marker_notes.replace(" ", ""))
        )
        and "chose" not in marker_notes,
    )
    # prettier check for empty scraps
    empty_scraps = '"scraps": []' in marker_notes or '"scraps":[]' in marker_notes
    if not empty_scraps:
        checks[-1].status = "fail"
        checks[-1].actual = "scraps not empty: " + marker_notes[:300]
    branch = f"track/{ctx.slug}"
    if ctx.mode == "git":
        head = rev_parse(ctx, "HEAD")
        add(
            checks,
            cid="A3",
            ctx=ctx,
            family="birth",
            title="Git: HEAD starts on the empty marker",
            expected=ctx.marker,
            actual=head,
            ok=head == ctx.marker,
        )
        cur = git(ctx, ["branch", "--show-current"], cwd=ws).stdout.strip()
        add(
            checks,
            cid="A4",
            ctx=ctx,
            family="birth",
            title="Git: worktree branch is track/<slug>",
            expected=branch,
            actual=cur,
            ok=cur == branch,
        )
    else:
        bookmark = jj(
            ctx, ["log", "-r", branch, "--no-graph", "-T", "commit_id"]
        ).stdout.strip()
        wc = jj(ctx, ["log", "-r", "@", "--no-graph", "-T", "commit_id"]).stdout.strip()
        add(
            checks,
            cid="A3",
            ctx=ctx,
            family="birth",
            title="JJ: bookmark starts on working copy, not marker",
            expected="bookmark == @ != marker",
            actual=f"bookmark={bookmark[:12]} @={wc[:12]} marker={ctx.marker[:12]}",
            ok=bookmark == wc and bookmark != ctx.marker,
        )
        parent = jj(
            ctx, ["log", "-r", "@-", "--no-graph", "-T", "commit_id"]
        ).stdout.strip()
        add(
            checks,
            cid="A4",
            ctx=ctx,
            family="birth",
            title="JJ: working copy is a child of the marker",
            expected=ctx.marker,
            actual=parent,
            ok=parent == ctx.marker,
        )
    walked = git(ctx, ["rev-parse", "HEAD"], cwd=ws, ok=False)
    if ctx.mode == "git":
        add(
            checks,
            cid="A6",
            ctx=ctx,
            family="birth",
            title="git -C worktree HEAD is the task tip",
            expected="task HEAD",
            actual=walked.stdout.strip()[:12],
            ok=walked.returncode == 0
            and walked.stdout.strip() == rev_parse(ctx, "HEAD", cwd=ws),
        )
    else:
        add(
            checks,
            cid="A6",
            ctx=ctx,
            family="birth",
            title="git -C on a jj workspace walks to repo main (track uses jj @, not this HEAD)",
            expected="git HEAD is main, not the task bookmark",
            actual=f"{walked.stdout.strip()[:12]} (git-dir={git(ctx, ['rev-parse', '--show-toplevel'], cwd=ws, ok=False).stdout.strip()})",
            ok=walked.returncode == 0 and walked.stdout.strip() != task_tip(ctx),
        )
    phase = status.get("workflow", {}).get("phase")
    add(
        checks,
        cid="A5",
        ctx=ctx,
        family="birth",
        title="workflow.phase is execute after repo add",
        expected="execute",
        actual=str(phase),
        ok=phase == "execute",
    )


def family_daily(ctx: Ctx, checks: list[Check]) -> None:
    ws = ctx.worktree
    assert ws is not None and ctx.marker
    # Trial-and-error: three commits / jj changes, plus a local scrap and a shared one.
    (ws / "src").mkdir(exist_ok=True)
    (ws / "src" / "auth.rs").write_text("fn token() {}\n")
    if ctx.mode == "git":
        git(ctx, ["add", "src/auth.rs"], cwd=ws)
        git(ctx, ["commit", "-m", "wip: sketch"], cwd=ws)
        (ws / "src" / "auth.rs").write_text("fn token() { 1 }\n")
        git(ctx, ["add", "src/auth.rs"], cwd=ws)
        git(ctx, ["commit", "-m", "wip: try int"], cwd=ws)
        (ws / "src" / "auth.rs").write_text("fn token() -> &'static str { \"ok\" }\n")
        git(ctx, ["add", "src/auth.rs"], cwd=ws)
        git(ctx, ["commit", "-m", "wip: string"], cwd=ws)
    else:
        (ws / "src" / "auth.rs").write_text("fn token() {}\n")
        jj(ctx, ["new", "-m", "wip: sketch"])
        (ws / "src" / "auth.rs").write_text("fn token() { 1 }\n")
        jj(ctx, ["new", "-m", "wip: try int"])
        (ws / "src" / "auth.rs").write_text("fn token() -> &'static str { \"ok\" }\n")
        # leave work in @ (no extra new) so describe can pick it up — AND
        # a parallel path later tests leftover wip changes.

    track(ctx, ["scrap", "add", "password in test fixture"], cwd=ws)
    track(
        ctx,
        ["scrap", "add", "--share", "chose git notes over embedding JSON in commits"],
        cwd=ws,
    )
    before_wip = log_oneline(ctx, todo_range(ctx))
    r = track(ctx, ["todo", "done", "1", "--json"], cwd=ws, ok=False)
    add(
        checks,
        cid="B0",
        ctx=ctx,
        family="daily",
        title="todo done 1 succeeds after messy WIP",
        expected="exit 0",
        actual=f"rc={r.returncode} {r.stderr[-300:]}",
        ok=r.returncode == 0,
        detail=r.stdout[-200:] if r.returncode == 0 else r.stderr,
    )
    if r.returncode != 0:
        return
    after = log_oneline(ctx, todo_range(ctx))
    todo_lines = [ln for ln in after if "Design token flow" in ln or "Task-Todo" in trailers(ctx, ln.split()[0])]
    # Count commits with Task-Todo: 1
    shas = [ln.split()[0] for ln in after]
    todo_shas = [s for s in shas if "Task-Todo: 1" in trailers(ctx, s)]
    add(
        checks,
        cid="B1",
        ctx=ctx,
        family="daily",
        title="Exactly one Task-Todo: 1 commit after first done",
        expected="1",
        actual=str(len(todo_shas)) + " / log=" + " | ".join(ln.split(" ", 1)[-1] for ln in after),
        ok=len(todo_shas) == 1,
        detail=f"wip commits before done: {len(before_wip)}",
    )
    if todo_shas:
        body = notes_show(ctx, todo_shas[0]) or ""
        add(
            checks,
            cid="B2",
            ctx=ctx,
            family="daily",
            title="Shared scrap is notes on TODO 1 commit, not marker",
            expected="chose git notes… on todo sha; absent from marker",
            actual=f"todo_notes={body[:180]!r}",
            ok="chose git notes" in body
            and "password in test" not in body
            and "password in test" not in (notes_show(ctx, ctx.marker) or ""),
        )
        marker_notes = notes_show(ctx, ctx.marker) or ""
        add(
            checks,
            cid="B3",
            ctx=ctx,
            family="daily",
            title="Marker notes still have no scraps after todo done",
            expected="no scrap text on marker",
            actual=marker_notes[:200],
            ok="chose git notes" not in marker_notes,
        )
        add(
            checks,
            cid="B4",
            ctx=ctx,
            family="daily",
            title="Local scrap never appears in git notes",
            expected="password fixture absent",
            actual="present" if "password in test" in body else "absent",
            ok="password in test" not in body,
        )
        sha1 = todo_shas[0]
    else:
        sha1 = None

    # Second TODO: append, first SHA stable
    (ws / "src" / "refresh.rs").write_text("fn refresh() {}\n")
    if ctx.mode == "git":
        git(ctx, ["add", "src/refresh.rs"], cwd=ws)
        git(ctx, ["commit", "-m", "wip refresh"], cwd=ws)
    track(
        ctx,
        ["scrap", "add", "--share", "refresh uses rotating nonce"],
        cwd=ws,
    )
    r2 = track(ctx, ["todo", "done", "2", "--json"], cwd=ws, ok=False)
    add(
        checks,
        cid="B5",
        ctx=ctx,
        family="daily",
        title="todo done 2 succeeds",
        expected="exit 0",
        actual=f"rc={r2.returncode} {r2.stderr[-200:]}",
        ok=r2.returncode == 0,
    )
    if r2.returncode == 0 and sha1:
        after2 = log_oneline(ctx, todo_range(ctx))
        shas2 = [ln.split()[0] for ln in after2]
        todo1_still = [s for s in shas2 if "Task-Todo: 1" in trailers(ctx, s)]
        todo2 = [s for s in shas2 if "Task-Todo: 2" in trailers(ctx, s)]
        add(
            checks,
            cid="B6",
            ctx=ctx,
            family="daily",
            title="Second TODO appends; first SHA unchanged",
            expected=sha1,
            actual=(todo1_still[0] if todo1_still else "missing")
            + f" todo2={len(todo2)}",
            ok=todo1_still == [sha1] and len(todo2) == 1,
        )
        if todo2:
            n2 = notes_show(ctx, todo2[0]) or ""
            n1 = notes_show(ctx, sha1) or ""
            add(
                checks,
                cid="B7",
                ctx=ctx,
                family="daily",
                title="TODO 2 notes have only TODO 2 scraps",
                expected="rotating nonce; not the first decision",
                actual=n2[:220],
                ok="rotating nonce" in n2 and "chose git notes" not in n2,
            )
            add(
                checks,
                cid="B8",
                ctx=ctx,
                family="daily",
                title="TODO 1 notes unchanged by TODO 2",
                expected="still chose git notes",
                actual=n1[:180],
                ok="chose git notes" in n1 and "rotating nonce" not in n1,
            )

    # Research TODO: still 1:1 commit because workspace exists
    r3 = track(ctx, ["todo", "done", "3", "--json"], cwd=ws, ok=False)
    add(
        checks,
        cid="B9",
        ctx=ctx,
        family="daily",
        title="--no-workspace TODO still gets a Task-Todo commit when a workspace exists",
        expected="Task-Todo: 3 commit",
        actual=r3.stderr[-150:] if r3.returncode else "ok",
        ok=r3.returncode == 0
        and any(
            "Task-Todo: 3" in trailers(ctx, ln.split()[0])
            for ln in log_oneline(ctx, todo_range(ctx))
        ),
    )


def family_publish_review(ctx: Ctx, checks: list[Check]) -> str | None:
    ws = ctx.worktree
    assert ws is not None
    branch = f"track/{ctx.slug}"
    # Record first todo sha
    after = log_oneline(ctx, todo_range(ctx))
    todo_shas = []
    for ln in after:
        sha = ln.split()[0]
        if "Task-Todo:" in trailers(ctx, sha):
            todo_shas.append(sha)
    if not todo_shas:
        add(
            checks,
            cid="C0",
            ctx=ctx,
            family="review",
            title="Need TODO commits before publish",
            expected=">=1",
            actual="0",
            ok=False,
        )
        return None
    first = todo_shas[0]
    first_notes = notes_show(ctx, first) or ""
    if ctx.mode == "jj":
        push = jj(ctx, ["git", "push", "--remote", "origin", "--bookmark", branch], ok=False)
    else:
        push = git(ctx, ["push", "-u", "origin", branch], cwd=ws, ok=False)
    add(
        checks,
        cid="C1",
        ctx=ctx,
        family="review",
        title="Push track/<slug> to origin (simulate opening a PR)",
        expected="exit 0",
        actual=f"rc={push.returncode} {push.stderr[-250:]}",
        ok=push.returncode == 0,
    )
    notes_push = track(ctx, ["notes", "push"], cwd=ws, ok=False)
    add(
        checks,
        cid="C2",
        ctx=ctx,
        family="review",
        title="notes push discloses refs/notes/track",
        expected="exit 0",
        actual=f"rc={notes_push.returncode} {notes_push.stderr[-250:]}",
        ok=notes_push.returncode == 0,
    )
    origin_tip = origin_branch_sha(ctx, branch)
    add(
        checks,
        cid="C3",
        ctx=ctx,
        family="review",
        title="origin/track/<slug> exists after push",
        expected="remote tip sha",
        actual=origin_tip[:12] or "(missing)",
        ok=bool(origin_tip),
    )

    # Follow-up TODO after publish
    track_json(ctx, ["todo", "add", "Address review: reject empty refresh", "--json"], cwd=ws)
    (ws / "src" / "refresh.rs").write_text("fn refresh() { /* reject empty */ }\n")
    track(
        ctx,
        ["scrap", "add", "--share", "review: empty refresh is 400"],
        cwd=ws,
    )
    r4 = track(ctx, ["todo", "done", "4", "--json"], cwd=ws, ok=False)
    add(
        checks,
        cid="C4",
        ctx=ctx,
        family="review",
        title="Follow-up TODO after origin push succeeds (append)",
        expected="exit 0",
        actual=f"rc={r4.returncode} {r4.stderr[-250:]}",
        ok=r4.returncode == 0,
    )
    if r4.returncode == 0:
        still = [
            ln.split()[0]
            for ln in log_oneline(ctx, todo_range(ctx))
            if "Task-Todo: 1" in trailers(ctx, ln.split()[0])
        ]
        add(
            checks,
            cid="C5",
            ctx=ctx,
            family="review",
            title="Published TODO 1 SHA is unchanged after follow-up",
            expected=first,
            actual=still[0] if still else "missing",
            ok=still == [first],
        )
        after_notes = notes_show(ctx, first) or ""
        add(
            checks,
            cid="C6",
            ctx=ctx,
            family="review",
            title="Published TODO 1 notes are not rewritten",
            expected="identical notes blob",
            actual="unchanged" if after_notes == first_notes else after_notes[:160],
            ok=after_notes == first_notes,
        )
        todo4 = [
            ln.split()[0]
            for ln in log_oneline(ctx, todo_range(ctx))
            if "Task-Todo: 4" in trailers(ctx, ln.split()[0])
        ]
        if todo4:
            anc_ok = is_ancestor(ctx, first, todo4[0])
            add(
                checks,
                cid="C7",
                ctx=ctx,
                family="review",
                title="Follow-up is a descendant (GitHub fast-forward)",
                expected="first ancestor of follow-up",
                actual="yes" if anc_ok else "no",
                ok=anc_ok,
            )
            n4 = notes_show(ctx, todo4[0]) or ""
            add(
                checks,
                cid="C8",
                ctx=ctx,
                family="review",
                title="Follow-up notes live on the new SHA only",
                expected="review scrap on TODO 4, not on TODO 1",
                actual=n4[:200],
                ok="empty refresh is 400" in n4 and "empty refresh is 400" not in after_notes,
            )
        origin_before = origin_branch_sha(ctx, branch)
        push2 = push_task_branch(ctx)
        origin_after = origin_branch_sha(ctx, branch)
        add(
            checks,
            cid="C13",
            ctx=ctx,
            family="review",
            title="Follow-up commit is fast-forward pushed",
            expected="exit 0 and previous origin tip is ancestor of new tip",
            actual=f"rc={push2.returncode} before={origin_before[:12]} after={origin_after[:12]}",
            ok=push2.returncode == 0
            and bool(origin_after)
            and is_ancestor(ctx, origin_before, origin_after),
        )
        track(ctx, ["notes", "push"], cwd=ws, ok=False)

    # Late share: scrap after a published TODO, attaching to... no pending if 4 done.
    # Add scrap now — no pending todos.
    late = track(
        ctx,
        ["scrap", "add", "--share", "late decision after everything done"],
        cwd=ws,
        ok=False,
    )
    add(
        checks,
        cid="C9",
        ctx=ctx,
        family="review",
        title="Sharing a scrap with no pending TODO still succeeds in DB",
        expected="exit 0",
        actual=f"rc={late.returncode} {late.stderr[-150:]}",
        ok=late.returncode == 0,
    )
    # That scrap cannot land on a published commit; should not rewrite first notes.
    still_notes = notes_show(ctx, first) or ""
    add(
        checks,
        cid="C10",
        ctx=ctx,
        family="review",
        title="Late shared scrap does not rewrite published notes",
        expected="TODO 1 notes unchanged; late scrap absent",
        actual="unchanged" if still_notes == first_notes else still_notes[:160],
        ok=still_notes == first_notes and "late decision" not in still_notes,
    )
    tip = task_tip(ctx)
    tip_notes = notes_show(ctx, tip) or ""
    add(
        checks,
        cid="C11",
        ctx=ctx,
        family="review",
        title="Late shared scrap has no unpublished TODO commit to attach to",
        expected="not in task-tip notes (gap: needs a follow-up TODO)",
        actual=tip_notes[:180] or "(no notes on task tip)",
        ok="late decision" not in tip_notes,
        detail="If this fails, we rewrote the tip notes for a scrap that should have required a new TODO.",
    )

    reopen = track(ctx, ["todo", "done", "1"], cwd=ws, ok=False)
    add(
        checks,
        cid="C12",
        ctx=ctx,
        family="review",
        title="Reopening a done TODO is rejected",
        expected="nonzero / invalid transition",
        actual=f"rc={reopen.returncode} {reopen.stderr[-150:]}",
        ok=reopen.returncode != 0,
    )
    return first


def family_published_wip(ctx: Ctx, checks: list[Check]) -> None:
    """New task: publish first TODO, then push extra WIP, then next todo done must refuse."""
    status = setup_task(
        ctx,
        "Published WIP trap",
        "SIM-WIP",
        [("First", False), ("Second", False)],
    )
    ctx.worktree = Path(
        (status.get("git") or status.get("jj") or {})["workspace_path"]
    )
    ctx.slug = (status.get("git") or status.get("jj") or {})["slug"]
    ctx.marker = marker_sha(ctx)
    ws = ctx.worktree
    (ws / "a.txt").write_text("a\n")
    track(ctx, ["todo", "done", "1"], cwd=ws)
    branch = f"track/{ctx.slug}"
    if ctx.mode == "jj":
        jj(ctx, ["git", "push", "--remote", "origin", "--bookmark", branch], ok=False)
    else:
        git(ctx, ["push", "-u", "origin", branch], cwd=ws, ok=False)
    (ws / "b.txt").write_text("published extra\n")
    if ctx.mode == "git":
        git(ctx, ["add", "b.txt"], cwd=ws)
        git(ctx, ["commit", "-m", "reviewer-visible extra"], cwd=ws)
        git(ctx, ["push", "origin", branch], cwd=ws)
    else:
        jj(ctx, ["describe", "-m", "reviewer-visible extra"], ok=False)
        jj(ctx, ["bookmark", "set", branch, "-r", "@"], ok=False)
        jj(ctx, ["git", "push", "--remote", "origin", "--bookmark", branch], ok=False)
    before = task_tip(ctx)
    err = track(ctx, ["todo", "done", "2"], cwd=ws, ok=False)
    add(
        checks,
        cid="P1",
        ctx=ctx,
        family="published-wip",
        title="todo done refuses to fold commits already on origin",
        expected="CannotRewritePublishedHistory",
        actual=f"rc={err.returncode} {err.stderr[-250:]} {err.stdout[-120:]}",
        ok=err.returncode != 0
        and (
            "rewrite published" in (err.stderr + err.stdout).lower()
            or "Cannot rewrite" in err.stderr + err.stdout
            or "published" in (err.stderr + err.stdout).lower()
        ),
        status_if_false="fail",
    )
    after = task_tip(ctx)
    add(
        checks,
        cid="P2",
        ctx=ctx,
        family="published-wip",
        title="Task tip unchanged when fold is refused",
        expected=before,
        actual=after,
        ok=before == after,
        status_if_false="fail",
    )


def family_import(ctx: Ctx, checks: list[Check]) -> None:
    ws = ctx.worktree
    assert ws is not None
    branch = f"track/{ctx.slug}"
    track(ctx, ["switch", "t:SIM-1"], ok=False)
    track(ctx, ["todo", "add", "Never committed plan"], ok=False)
    importer_home = ctx.home.parent / f"{ctx.mode}-importer"
    if importer_home.exists():
        shutil.rmtree(importer_home)
    importer_home.mkdir(parents=True)
    write_gitconfig(importer_home)
    imp_env = make_env(importer_home)
    clone = ROOT / f"{ctx.mode}-clone"
    if clone.exists():
        shutil.rmtree(clone)
    run(["git", "clone", str(ctx.origin), str(clone)], cwd=ROOT, env=imp_env)
    co = run(
        ["git", "checkout", branch],
        cwd=clone,
        env=imp_env,
        check=False,
    )
    add(
        checks,
        cid="D0",
        ctx=ctx,
        family="import",
        title="Importer can check out the PR branch from origin",
        expected=branch,
        actual=f"rc={co.returncode} {co.stderr[-150:]}",
        ok=co.returncode == 0,
    )
    notes = run(
        [str(TRACK), "notes", "fetch"],
        cwd=clone,
        env=imp_env,
        check=False,
    )
    add(
        checks,
        cid="D1",
        ctx=ctx,
        family="import",
        title="Importer restores notes via track notes fetch",
        expected="exit 0",
        actual=f"rc={notes.returncode} {notes.stderr[-200:]}",
        ok=notes.returncode == 0,
    )
    # Pending TODO that was never completed should be missing from import
    # (happy path completed 1-4; all done). Record count.
    imp = run(
        [str(TRACK), "import", "--json"],
        cwd=clone,
        env=imp_env,
        check=False,
    )
    add(
        checks,
        cid="D2",
        ctx=ctx,
        family="import",
        title="track import from PR branch restores a local task",
        expected="ok + todos from Task-Todo commits",
        actual=f"rc={imp.returncode} {imp.stderr[-200:]} {imp.stdout[:300]}",
        ok=imp.returncode == 0,
    )
    if imp.returncode == 0:
        data = json.loads(imp.stdout)
        todos = data.get("todos") or data.get("todos_agent") or []
        # mutation snapshot uses todos
        names = [t.get("content") or t.get("text") for t in todos]
        add(
            checks,
            cid="D3",
            ctx=ctx,
            family="import",
            title="Imported TODOs match completed commits (not local-only journal)",
            expected="Design token flow, Implement refresh, Write docs, Address review",
            actual=str(names),
            ok=any("Design token" in str(n) for n in names)
            and any("Implement refresh" in str(n) for n in names)
            and any("Address review" in str(n) for n in names),
        )
        scraps = data.get("scraps") or []
        texts = [s.get("content") for s in scraps]
        add(
            checks,
            cid="D4",
            ctx=ctx,
            family="import",
            title="Shared scraps import; local password scrap does not",
            expected="chose git notes; no password fixture",
            actual=str(texts),
            ok=any("chose git notes" in str(t) for t in texts)
            and any("empty refresh is 400" in str(t) for t in texts)
            and not any("password" in str(t).lower() for t in texts),
        )
        add(
            checks,
            cid="D5",
            ctx=ctx,
            family="import",
            title="Pending TODOs never committed are absent after import",
            expected="only done TODOs; no 'Never committed plan'",
            actual=f"{len(todos)} todos names={names} statuses={[t.get('status') for t in todos]}",
            ok=not any("Never committed" in str(n) for n in names)
            and all((t.get("status") or "done") == "done" for t in todos),
            detail="Contract: import restores completed work from history, not unpublished plan.",
        )

    ws_home = ctx.home.parent / f"{ctx.mode}-ws-importer"
    if ws_home.exists():
        shutil.rmtree(ws_home)
    ws_home.mkdir(parents=True)
    write_gitconfig(ws_home)
    ws_env = make_env(ws_home)
    ws_imp = run(
        [str(TRACK), "import", "--json"],
        cwd=ws,
        env=ws_env,
        check=False,
    )
    ws_data = {}
    if ws_imp.returncode == 0:
        try:
            ws_data = json.loads(ws_imp.stdout)
        except json.JSONDecodeError:
            ws_data = {}
    ws_names = [
        t.get("content") or t.get("text")
        for t in (ws_data.get("todos") or ws_data.get("todos_agent") or [])
    ]
    add(
        checks,
        cid="D6",
        ctx=ctx,
        family="import",
        title="track import from the task workspace uses the task tip (not repo main)",
        expected="exit 0 and Address review present",
        actual=f"rc={ws_imp.returncode} names={ws_names} {ws_imp.stderr[-150:]}",
        ok=ws_imp.returncode == 0 and any("Address review" in str(n) for n in ws_names),
    )
    main_home = ctx.home.parent / f"{ctx.mode}-main-importer"
    if main_home.exists():
        shutil.rmtree(main_home)
    main_home.mkdir(parents=True)
    write_gitconfig(main_home)
    main_env = make_env(main_home)
    main_imp = run(
        [str(TRACK), "import", "--json"],
        cwd=ctx.repo,
        env=main_env,
        check=False,
    )
    add(
        checks,
        cid="D7",
        ctx=ctx,
        family="import",
        title="track import from repo main does not restore the task branch",
        expected="nonzero or no Address review",
        actual=f"rc={main_imp.returncode} {main_imp.stdout[:180]} {main_imp.stderr[-120:]}",
        ok=main_imp.returncode != 0
        or "Address review" not in main_imp.stdout,
    )


def family_out_of_order(ctx: Ctx, checks: list[Check]) -> None:
    status = setup_task(
        ctx,
        "Out of order",
        "SIM-OO",
        [("One", False), ("Two", False)],
    )
    ctx.worktree = Path((status.get("git") or status.get("jj") or {})["workspace_path"])
    ctx.slug = (status.get("git") or status.get("jj") or {})["slug"]
    ctx.marker = marker_sha(ctx)
    ws = ctx.worktree
    r = track(ctx, ["todo", "done", "2"], cwd=ws, ok=False)
    r2 = track(ctx, ["todo", "done", "1"], cwd=ws, ok=False)
    add(
        checks,
        cid="O1",
        ctx=ctx,
        family="order",
        title="Completing TODO 2 before TODO 1 is allowed",
        expected="both succeed",
        actual=f"2={r.returncode} 1={r2.returncode}",
        ok=r.returncode == 0 and r2.returncode == 0,
        status_if_false="issue",
    )
    lines = log_oneline(ctx, todo_range(ctx))
    order = []
    for ln in lines:
        body = trailers(ctx, ln.split()[0])
        if "Task-Todo: 2" in body:
            order.append(2)
        if "Task-Todo: 1" in body:
            order.append(1)
    add(
        checks,
        cid="O2",
        ctx=ctx,
        family="order",
        title="Commit order follows completion order, not task_index",
        expected="[2, 1] (completion order) — may diverge from TODO list order",
        actual=str(order),
        ok=order == [2, 1],
        detail="Replay walks git history; imported TODO order may not match original task_index intent.",
    )


def family_backfill(ctx: Ctx, checks: list[Check]) -> None:
    track(ctx, ["config", "set", "aggressive-mode", "off"])
    status = setup_task(ctx, "Late aggressive", "SIM-BF", [("Work", False)])
    ctx.worktree = Path((status.get("git") or status.get("jj") or {})["workspace_path"])
    ctx.slug = (status.get("git") or status.get("jj") or {})["slug"]
    ws = ctx.worktree
    (ws / "x.txt").write_text("work\n")
    if ctx.mode == "git":
        git(ctx, ["add", "x.txt"], cwd=ws)
        git(ctx, ["commit", "-m", "real work"], cwd=ws)
    else:
        jj(ctx, ["describe", "-m", "real work"], ok=False)
        jj(ctx, ["new"], ok=False)
    track(ctx, ["config", "set", "aggressive-mode", "on"])
    track(ctx, ["sync"], cwd=ctx.repo)
    tip = task_tip(ctx)
    try:
        ctx.marker = marker_sha(ctx)
        marker_ok = ctx.marker
    except Exception as e:
        marker_ok = ""
        marker_err = str(e)[:200]
    else:
        marker_err = ""
    add(
        checks,
        cid="F1",
        ctx=ctx,
        family="backfill",
        title="Turning aggressive on keeps the work file and does not sit the marker on the tip",
        expected="x.txt present, marker != tip",
        actual=f"marker={marker_ok[:12]} tip={tip[:12]} x.txt={(ws / 'x.txt').exists()} {marker_err}",
        ok=(ws / "x.txt").exists() and bool(marker_ok) and marker_ok != tip,
    )
    if marker_ok:
        add(
            checks,
            cid="F2",
            ctx=ctx,
            family="backfill",
            title="Backfilled marker is an ancestor of the task tip",
            expected="marker ancestor of tip, marker != tip",
            actual=f"marker={ctx.marker[:12]} tip={tip[:12]}",
            ok=git(
                ctx,
                ["merge-base", "--is-ancestor", ctx.marker, tip],
                cwd=ctx.repo,
                ok=False,
            ).returncode
            == 0
            and ctx.marker != tip,
        )
    else:
        add(
            checks,
            cid="F2",
            ctx=ctx,
            family="backfill",
            title="Backfilled marker is an ancestor of the task tip",
            expected="marker found",
            actual=marker_err,
            ok=False,
        )
    track(ctx, ["config", "set", "aggressive-mode", "on"])  # keep on for later


def family_two_tasks(ctx: Ctx, checks: list[Check]) -> None:
    track(ctx, ["config", "set", "aggressive-mode", "on"])
    s1 = setup_task(ctx, "Task A", "SIM-A", [("Do A", False)])
    ws_a = Path((s1.get("git") or s1.get("jj") or {})["workspace_path"])
    slug_a = (s1.get("git") or s1.get("jj") or {})["slug"]
    s2 = track_json(ctx, ["new", "Task B", "--ticket", "SIM-B", "--json"])
    track_json(ctx, ["todo", "add", "Do B", "--json"])
    r = track(ctx, ["repo", "add", str(ctx.repo), "--json"], ok=False)
    ws_b = ctx.repo / ".worktrees" / "sim-b"
    add(
        checks,
        cid="T1",
        ctx=ctx,
        family="multi-task",
        title="Second task on the same repo gets its own worktree",
        expected="exit 0 and .worktrees/sim-b exists",
        actual=f"rc={r.returncode} {r.stderr[-200:]} a_exists={ws_a.exists()}",
        ok=r.returncode == 0 and ws_b.exists() and ws_a.exists(),
    )


def family_push_birth(ctx: Ctx, checks: list[Check]) -> None:
    """Push immediately after workspace birth — not the happy-path SIM-1 branch."""
    status = setup_task(ctx, "Push at birth", "SIM-BR", [("Later", False)])
    ctx.worktree = Path((status.get("git") or status.get("jj") or {})["workspace_path"])
    ctx.slug = (status.get("git") or status.get("jj") or {})["slug"]
    ctx.marker = marker_sha(ctx)
    empty = undescribed_ancestors(ctx, f"track/{ctx.slug}")
    push = push_task_branch(ctx)
    add(
        checks,
        cid="A7",
        ctx=ctx,
        family="birth-push",
        title="track/<slug> is pushable at birth (no undescribed ancestors)",
        expected="undescribed=0 and push exit 0",
        actual=f"undescribed={empty!r} rc={push.returncode} {push.stderr[-180:]}",
        ok=push.returncode == 0 and not empty,
    )


def family_share_after_done(ctx: Ctx, checks: list[Check]) -> None:
    status = setup_task(
        ctx,
        "Share after done",
        "SIM-SH",
        [("First", False), ("Second", False)],
    )
    ctx.worktree = Path((status.get("git") or status.get("jj") or {})["workspace_path"])
    ctx.slug = (status.get("git") or status.get("jj") or {})["slug"]
    ctx.marker = marker_sha(ctx)
    ws = ctx.worktree
    (ws / "s.txt").write_text("share\n")
    track(ctx, ["scrap", "add", "local then share this"], cwd=ws)
    track(ctx, ["todo", "done", "1"], cwd=ws)
    todo1 = [
        ln.split()[0]
        for ln in log_oneline(ctx, todo_range(ctx))
        if "Task-Todo: 1" in trailers(ctx, ln.split()[0])
    ]
    share = track(ctx, ["scrap", "share", "1"], cwd=ws, ok=False)
    n1 = notes_show(ctx, todo1[0]) if todo1 else None
    add(
        checks,
        cid="S1",
        ctx=ctx,
        family="share-after-done",
        title="scrap share on an unpublished TODO commit writes notes on that SHA",
        expected="local then share this on TODO 1",
        actual=f"rc={share.returncode} notes={(n1 or '')[:180]}",
        ok=share.returncode == 0 and todo1 and n1 is not None and "local then share this" in n1,
    )
    blob_before = n1 or ""
    ref_before = notes_ref_sha(ctx)
    push_task_branch(ctx)
    track(ctx, ["notes", "push"], cwd=ws, ok=False)
    after_pub = track(ctx, ["scrap", "add", "--share", "must not rewrite published TODO"], cwd=ws)
    blob_after = notes_show(ctx, todo1[0]) if todo1 else None
    ref_after = notes_ref_sha(ctx)
    add(
        checks,
        cid="S2",
        ctx=ctx,
        family="share-after-done",
        title="After origin, sharing does not rewrite published TODO notes or notes ref",
        expected="blob and refs/notes/track unchanged",
        actual=f"blob_eq={blob_after == blob_before} ref {ref_before[:12]}->{ref_after[:12]}",
        ok=blob_after == blob_before
        and "must not rewrite published TODO" not in (blob_after or "")
        and (not ref_before or ref_after == ref_before),
    )
    _ = after_pub


def family_empty_followup(ctx: Ctx, checks: list[Check]) -> None:
    status = setup_task(ctx, "Empty follow-up", "SIM-EF", [("First", False)])
    ctx.worktree = Path((status.get("git") or status.get("jj") or {})["workspace_path"])
    ctx.slug = (status.get("git") or status.get("jj") or {})["slug"]
    ctx.marker = marker_sha(ctx)
    ws = ctx.worktree
    (ws / "e.txt").write_text("first\n")
    track(ctx, ["todo", "done", "1"], cwd=ws)
    first = [
        ln.split()[0]
        for ln in log_oneline(ctx, todo_range(ctx))
        if "Task-Todo: 1" in trailers(ctx, ln.split()[0])
    ]
    notes1 = notes_show(ctx, first[0]) if first else None
    push_task_branch(ctx)
    origin_before = origin_branch_sha(ctx)
    track_json(ctx, ["todo", "add", "Empty follow-up", "--json"])
    r = track(ctx, ["todo", "done", "2"], cwd=ws, ok=False)
    after = log_oneline(ctx, todo_range(ctx))
    todo2 = [ln.split()[0] for ln in after if "Task-Todo: 2" in trailers(ctx, ln.split()[0])]
    still1 = [ln.split()[0] for ln in after if "Task-Todo: 1" in trailers(ctx, ln.split()[0])]
    add(
        checks,
        cid="E1",
        ctx=ctx,
        family="empty-followup",
        title="todo done with no new files appends an empty follow-up commit",
        expected="exit 0, new Task-Todo: 2, first SHA unchanged",
        actual=f"rc={r.returncode} todo2={todo2[:1]} still1={still1[:1]}",
        ok=r.returncode == 0 and len(todo2) == 1 and still1 == first,
    )
    if first and todo2:
        add(
            checks,
            cid="E2",
            ctx=ctx,
            family="empty-followup",
            title="Empty follow-up is a fast-forward descendant; old notes unchanged",
            expected="ancestor + identical TODO 1 notes",
            actual=f"anc={is_ancestor(ctx, first[0], todo2[0])}",
            ok=is_ancestor(ctx, first[0], todo2[0])
            and (notes_show(ctx, first[0]) or "") == (notes1 or ""),
        )
        push = push_task_branch(ctx)
        origin_after = origin_branch_sha(ctx)
        add(
            checks,
            cid="E3",
            ctx=ctx,
            family="empty-followup",
            title="Empty follow-up fast-forwards origin",
            expected="push 0 and origin ancestor",
            actual=f"rc={push.returncode} {origin_before[:12]}->{origin_after[:12]}",
            ok=push.returncode == 0 and is_ancestor(ctx, origin_before, origin_after),
        )


def family_published_backfill(ctx: Ctx, checks: list[Check]) -> None:
    track(ctx, ["config", "set", "aggressive-mode", "off"])
    status = setup_task(ctx, "Published backfill", "SIM-PB", [("Work", False)])
    ctx.worktree = Path((status.get("git") or status.get("jj") or {})["workspace_path"])
    ctx.slug = (status.get("git") or status.get("jj") or {})["slug"]
    ws = ctx.worktree
    (ws / "p.txt").write_text("published work\n")
    if ctx.mode == "git":
        git(ctx, ["add", "p.txt"], cwd=ws)
        git(ctx, ["commit", "-m", "real work"], cwd=ws)
        git(ctx, ["commit", "--allow-empty", "-m", "second unique"], cwd=ws)
    else:
        jj(ctx, ["describe", "-m", "real work"], ok=False)
        jj(ctx, ["new", "-m", "second unique"], ok=False)
        jj(ctx, ["bookmark", "set", f"track/{ctx.slug}", "-r", "@"], ok=False)
        jj(ctx, ["git", "export"], ok=False)
    tip = task_tip(ctx)
    unique_before = unique_from_main(ctx, tip)
    push = git(
        ctx,
        ["push", "-u", "origin", f"track/{ctx.slug}"],
        cwd=ctx.repo,
        ok=False,
    )
    if push.returncode != 0 and ctx.mode == "jj":
        push = jj(
            ctx,
            ["git", "push", "--remote", "origin", "--bookmark", f"track/{ctx.slug}"],
            ok=False,
        )
    if ctx.mode == "jj":
        jj(ctx, ["git", "fetch", "--remote", "origin"], ok=False)
    git(ctx, ["fetch", "origin"], cwd=ctx.repo, ok=False)
    origin = origin_branch_sha(ctx)
    push_out = ((push.stdout or "") + (push.stderr or "")).strip()[-300:]
    add(
        checks,
        cid="PB0",
        ctx=ctx,
        family="published-backfill",
        title="Unique commits are on origin before aggressive backfill",
        expected="origin/track/<slug> exists and push exit 0",
        actual=f"origin={origin[:12] or '(missing)'} rc={push.returncode} {push_out}",
        ok=push.returncode == 0 and bool(origin),
    )
    published = set(unique_before)
    track(ctx, ["config", "set", "aggressive-mode", "on"])
    track(ctx, ["sync"], cwd=ctx.repo)
    tip_after = task_tip(ctx)
    unique_after = set(unique_from_main(ctx, tip_after))
    add(
        checks,
        cid="PB1",
        ctx=ctx,
        family="published-backfill",
        title="Backfill after origin does not rewrite published unique SHAs",
        expected="published unique set is a subset of unique after sync",
        actual=f"before={len(published)} after={len(unique_after)} missing={sorted(s[:8] for s in published - unique_after)}",
        ok=bool(published) and published <= unique_after,
    )
    try:
        ctx.marker = marker_sha(ctx)
        add(
            checks,
            cid="PB2",
            ctx=ctx,
            family="published-backfill",
            title="Published backfill uses an existing unique commit as marker (no new parent)",
            expected="marker in the pre-sync unique set",
            actual=ctx.marker[:12],
            ok=ctx.marker in published,
        )
    except Exception as e:
        add(
            checks,
            cid="PB2",
            ctx=ctx,
            family="published-backfill",
            title="Published backfill uses an existing unique commit as marker (no new parent)",
            expected="marker found in published set",
            actual=str(e)[:200],
            ok=False,
        )
    track(ctx, ["config", "set", "aggressive-mode", "on"])


def family_sibling_backfill(ctx: Ctx, checks: list[Check]) -> None:
    track(ctx, ["config", "set", "aggressive-mode", "off"])
    a = setup_task(ctx, "Sibling A", "SIM-SA", [("Do A", False)])
    ws_a = Path((a.get("git") or a.get("jj") or {})["workspace_path"])
    slug_a = (a.get("git") or a.get("jj") or {})["slug"]
    (ws_a / "a-work.txt").write_text("A unpublished\n")
    if ctx.mode == "git":
        git(ctx, ["add", "a-work.txt"], cwd=ws_a)
        git(ctx, ["commit", "-m", "work on A"], cwd=ws_a)
        tip_a = rev_parse(ctx, "HEAD", cwd=ws_a)
    else:
        jj(ctx, ["describe", "-m", "work on A"], cwd=ws_a, ok=False)
        tip_a = jj(
            ctx,
            ["log", "-r", "@", "--no-graph", "-T", "commit_id"],
            cwd=ws_a,
        ).stdout.strip()
    b = setup_task(ctx, "Sibling B", "SIM-SB", [("Do B", False)])
    ws_b = Path((b.get("git") or b.get("jj") or {})["workspace_path"])
    (ws_b / "b-work.txt").write_text("B unpublished\n")
    if ctx.mode == "git":
        git(ctx, ["add", "b-work.txt"], cwd=ws_b)
        git(ctx, ["commit", "-m", "work on B"], cwd=ws_b)
    else:
        jj(ctx, ["describe", "-m", "work on B"], cwd=ws_b, ok=False)
    track(ctx, ["config", "set", "aggressive-mode", "on"])
    track(ctx, ["switch", "t:SIM-SB"], ok=False)
    track(ctx, ["sync"], cwd=ctx.repo)
    if ctx.mode == "git":
        tip_a_after = rev_parse(ctx, f"track/{slug_a}", cwd=ctx.repo)
    else:
        tip_a_after = jj(
            ctx,
            ["log", "-r", f"track/{slug_a}", "--no-graph", "-T", "commit_id"],
            cwd=ws_a,
        ).stdout.strip()
    add(
        checks,
        cid="SB1",
        ctx=ctx,
        family="sibling-backfill",
        title="Backfilling task B does not rewrite unpublished task A",
        expected=tip_a,
        actual=tip_a_after,
        ok=bool(tip_a) and tip_a == tip_a_after and (ws_a / "a-work.txt").exists(),
    )
    track(ctx, ["config", "set", "aggressive-mode", "on"])


def family_archive_pr_head(ctx: Ctx, checks: list[Check]) -> None:
    status = setup_task(ctx, "Archive me", "SIM-AR", [("Ship", False)])
    ctx.worktree = Path((status.get("git") or status.get("jj") or {})["workspace_path"])
    ctx.slug = (status.get("git") or status.get("jj") or {})["slug"]
    ws = ctx.worktree
    (ws / "ship.txt").write_text("ship\n")
    track(ctx, ["todo", "done", "1"], cwd=ws)
    push_task_branch(ctx)
    branch = f"track/{ctx.slug}"
    dirty = setup_task(ctx, "Dirty archive", "SIM-AD", [("Stay", False)])
    ws_d = Path((dirty.get("git") or dirty.get("jj") or {})["workspace_path"])
    (ws_d / "dirty.txt").write_text("unstaged\n")
    if ctx.mode == "jj":
        # jj WC includes the file as a change
        pass
    blocked = track(ctx, ["archive"], ok=False)
    add(
        checks,
        cid="AR1",
        ctx=ctx,
        family="archive",
        title="archive without --force refuses a dirty workspace (non-TTY)",
        expected="nonzero",
        actual=f"rc={blocked.returncode} {blocked.stderr[-180:]}",
        ok=blocked.returncode != 0,
    )
    track(ctx, ["switch", "t:SIM-AR"], ok=False)
    arch = track(ctx, ["archive", "--force"], ok=False)
    add(
        checks,
        cid="AR2",
        ctx=ctx,
        family="archive",
        title="archive --force removes the workspace and keeps track/<slug> for the PR",
        expected="workspace gone, branch/bookmark remains",
        actual=f"rc={arch.returncode} ws={ws.exists()} {arch.stderr[-120:]}",
        ok=arch.returncode == 0 and not ws.exists(),
    )
    if ctx.mode == "git":
        ref = git(ctx, ["show-ref", "--verify", f"refs/heads/{branch}"], ok=False)
        still = ref.returncode == 0
    else:
        listed = jj(
            ctx,
            ["bookmark", "list", branch],
            cwd=ctx.repo,
            ok=False,
        ).stdout
        still = branch in listed
    add(
        checks,
        cid="AR3",
        ctx=ctx,
        family="archive",
        title="PR head track/<slug> survives archive",
        expected=branch,
        actual="present" if still else "missing",
        ok=still,
    )


def family_two_repo_task(ctx: Ctx, checks: list[Check]) -> None:
    origin2 = ROOT / f"{ctx.mode}-origin2.git"
    repo2 = ROOT / f"{ctx.mode}-repo2"
    if origin2.exists():
        shutil.rmtree(origin2)
    if repo2.exists():
        shutil.rmtree(repo2)
    origin2.mkdir(parents=True)
    run(["git", "init", "--bare", "-b", "main", str(origin2)], cwd=ROOT, env=ctx.env)
    repo2.mkdir(parents=True)
    git_r2 = lambda args, ok=True: run(
        ["git", *args], cwd=repo2, env=ctx.env, check=ok
    )
    git_r2(["init", "-b", "main"])
    git_r2(["config", "user.email", "track@sim.test"])
    git_r2(["config", "user.name", "track-sim"])
    git_r2(["config", "commit.gpgsign", "false"])
    (repo2 / "README.md").write_text("# other\n")
    git_r2(["add", "README.md"])
    git_r2(["commit", "-m", "init"])
    git_r2(["remote", "add", "origin", str(origin2)])
    git_r2(["push", "-u", "origin", "main"])
    if ctx.mode == "jj":
        run(["jj", "git", "init", "--colocate", str(repo2)], cwd=repo2, env=ctx.env, check=False)

    snap = track_json(ctx, ["new", "Two repos", "--ticket", "SIM-2R", "--json"])
    track_json(ctx, ["todo", "add", "Touch both", "--json"])
    track_json(ctx, ["repo", "add", str(ctx.repo), "--json"])
    track_json(ctx, ["repo", "add", str(repo2), "--json"])
    status = track_json(ctx, ["status", "--json"])
    ctx.slug = (status.get("git") or status.get("jj") or {}).get("slug") or "sim-2r"
    ws1 = ctx.repo / ".worktrees" / ctx.slug
    ws2 = repo2 / ".worktrees" / ctx.slug
    (ws1 / "r1.txt").write_text("one\n")
    (ws2 / "r2.txt").write_text("two\n")
    track(ctx, ["scrap", "add", "--share", "same scrap on both repos"], cwd=ws1)
    ctx.worktree = ws1
    try:
        ctx.marker = marker_sha(ctx)
    except Exception:
        ctx.marker = None
    done = track(ctx, ["todo", "done", "1"], cwd=ws1, ok=False)

    def commits_with_todo(repo: Path, ws: Path) -> list[str]:
        if ctx.mode == "jj":
            listed = jj(
                ctx,
                [
                    "log",
                    "--no-graph",
                    "-r",
                    f"main..track/{ctx.slug}",
                    "-T",
                    'commit_id ++ " " ++ description.first_line() ++ "\\n"',
                ],
                cwd=ws,
                ok=False,
            )
            out = []
            for ln in listed.stdout.splitlines():
                if not ln.strip():
                    continue
                sha = ln.split()[0]
                body = jj(
                    ctx,
                    ["log", "-r", sha, "--no-graph", "-T", "description"],
                    cwd=ws,
                    ok=False,
                ).stdout
                if "Task-Todo: 1" in body:
                    out.append(sha)
            return out
        listed = git(
            ctx,
            ["log", "--reverse", "--format=%H %s", f"main..track/{ctx.slug}"],
            cwd=repo,
            ok=False,
        )
        out = []
        for ln in listed.stdout.splitlines():
            if not ln.strip():
                continue
            sha = ln.split()[0]
            body = git(ctx, ["log", "-1", "--format=%B", sha], cwd=repo, ok=False).stdout
            if "Task-Todo: 1" in body:
                out.append(sha)
        return out

    c1 = commits_with_todo(ctx.repo, ws1)
    c2 = commits_with_todo(repo2, ws2)
    n1 = notes_show(ctx, c1[0], cwd=ctx.repo) if c1 else None
    n2 = git(
        ctx,
        ["notes", "--ref", "refs/notes/track", "show", c2[0]],
        cwd=repo2,
        ok=False,
    )
    n2t = n2.stdout.strip() if n2.returncode == 0 else ""
    add(
        checks,
        cid="TR1",
        ctx=ctx,
        family="two-repo",
        title="todo done writes one Task-Todo commit in each registered repo",
        expected="1 commit per repo",
        actual=f"rc={done.returncode} repo1={len(c1)} repo2={len(c2)}",
        ok=done.returncode == 0 and len(c1) == 1 and len(c2) == 1,
    )
    add(
        checks,
        cid="TR2",
        ctx=ctx,
        family="two-repo",
        title="Shared scrap notes land on both repos' TODO commits",
        expected="same scrap on both",
        actual=f"n1={(n1 or '')[:80]!r} n2={n2t[:80]!r}",
        ok="same scrap on both repos" in (n1 or "") and "same scrap on both repos" in n2t,
    )
    _ = snap


def family_vcs_switch(ctx: Ctx, checks: list[Check]) -> None:
    other = "jj" if ctx.mode == "git" else "git"
    sw = track(ctx, ["config", "set", "vcs-mode", other], ok=False)
    # config set itself may succeed; sync should fail
    syn = track(ctx, ["sync"], ok=False)
    add(
        checks,
        cid="V1",
        ctx=ctx,
        family="vcs-switch",
        title="Switching vcs-mode against an existing workspace errors on sync",
        expected="WorkspaceVcsMismatch",
        actual=f"config_rc={sw.returncode} sync_rc={syn.returncode} {syn.stderr[-200:]}",
        ok=syn.returncode != 0
        and (
            "mismatch" in (syn.stderr + syn.stdout).lower()
            or "VCS" in syn.stderr + syn.stdout
            or "jj" in (syn.stderr + syn.stdout).lower()
        ),
    )
    track(ctx, ["config", "set", "vcs-mode", ctx.mode], ok=False)


def family_jj_fold_gap(ctx: Ctx, checks: list[Check]) -> None:
    if ctx.mode != "jj":
        return
    status = setup_task(
        ctx, "JJ fold", "SIM-JJ", [("Implement", False)]
    )
    ctx.worktree = Path((status.get("git") or status.get("jj") or {})["workspace_path"])
    ctx.slug = (status.get("git") or status.get("jj") or {})["slug"]
    ctx.marker = marker_sha(ctx)
    ws = ctx.worktree
    (ws / "wip1.txt").write_text("one\n")
    jj(ctx, ["new", "-m", "experiment 1"])
    (ws / "wip2.txt").write_text("two\n")
    jj(ctx, ["new", "-m", "experiment 2"])
    (ws / "wip3.txt").write_text("three\n")
    # @ now has wip3; two parent changes remain
    track(ctx, ["todo", "done", "1"], cwd=ws)
    log = log_oneline(ctx, todo_range(ctx))
    subjects = [ln.split(" ", 1)[-1] for ln in log]
    add(
        checks,
        cid="J1",
        ctx=ctx,
        family="jj-fold",
        title="JJ trial-and-error `jj new` is folded into one TODO commit (parity with git reset --soft)",
        expected="exactly 1 commit whose subject is Implement (no leftover experiment commits on the bookmark)",
        actual=" | ".join(subjects),
        ok=len([s for s in subjects if s.startswith("Implement")]) == 1
        and not any("experiment" in s for s in subjects),
    )
    bookmark = jj(
        ctx,
        ["log", "-r", f"track/{ctx.slug}", "--no-graph", "-T", "commit_id"],
    ).stdout.strip()
    wc = jj(ctx, ["log", "-r", "@", "--no-graph", "-T", "commit_id"]).stdout.strip()
    add(
        checks,
        cid="J2",
        ctx=ctx,
        family="jj-fold",
        title="After todo done, JJ bookmark sits on the TODO commit (@-), not empty @",
        expected="bookmark != @",
        actual=f"bookmark={bookmark[:12]} @={wc[:12]}",
        ok=bookmark != wc,
    )


def family_many_todos(ctx: Ctx, checks: list[Check]) -> None:
    status = setup_task(
        ctx,
        "Many TODOs",
        "SIM-MT",
        [
            ("One", False),
            ("Two cancel", False),
            ("Three", False),
            ("Four delete", False),
            ("Five", False),
        ],
    )
    bind_status(ctx, status)
    write_ws(ctx, "one.txt", "one\n")
    track(ctx, ["todo", "done", "1"], cwd=ctx.worktree)
    cancel = track(ctx, ["todo", "update", "2", "cancelled"], cwd=ctx.worktree, ok=False)
    deleted = track(ctx, ["todo", "delete", "4", "--force"], cwd=ctx.worktree, ok=False)
    write_ws(ctx, "three.txt", "three\n")
    track(ctx, ["todo", "done", "3"], cwd=ctx.worktree)
    write_ws(ctx, "five.txt", "five\n")
    track(ctx, ["todo", "done", "5"], cwd=ctx.worktree)
    indexes = task_todo_indexes(ctx)
    add(
        checks,
        cid="MT1",
        ctx=ctx,
        family="many-todos",
        title="Sequential done TODOs are one commit each, same order",
        expected="[1, 3, 5]",
        actual=str(indexes),
        ok=indexes == [1, 3, 5],
    )
    add(
        checks,
        cid="MT2",
        ctx=ctx,
        family="many-todos",
        title="Cancelled TODO does not get a Task-Todo commit",
        expected="no Task-Todo: 2; update exit 0",
        actual=f"rc={cancel.returncode} indexes={indexes}",
        ok=cancel.returncode == 0 and 2 not in indexes,
    )
    add(
        checks,
        cid="MT3",
        ctx=ctx,
        family="many-todos",
        title="Deleted pending TODO does not get a Task-Todo commit",
        expected="no Task-Todo: 4; delete --force exit 0",
        actual=f"rc={deleted.returncode} indexes={indexes}",
        ok=deleted.returncode == 0 and 4 not in indexes,
    )

    extra = setup_task(
        ctx,
        "Add after done",
        "SIM-MT2",
        [("First", False), ("Second", False)],
    )
    bind_status(ctx, extra)
    write_ws(ctx, "first.txt", "first\n")
    track(ctx, ["todo", "done", "1"], cwd=ctx.worktree)
    track_json(ctx, ["todo", "add", "Late addition", "--json"])
    write_ws(ctx, "late.txt", "late\n")
    track(ctx, ["todo", "done", "3"], cwd=ctx.worktree)
    indexes2 = task_todo_indexes(ctx)
    add(
        checks,
        cid="MT4",
        ctx=ctx,
        family="many-todos",
        title="TODO added after earlier dones appends a new Task-Todo index",
        expected="[1, 3]",
        actual=str(indexes2),
        ok=indexes2 == [1, 3],
    )

    mixed = setup_task(
        ctx,
        "Mixed research",
        "SIM-MT3",
        [("Code", False), ("Read papers", True), ("More code", False)],
    )
    bind_status(ctx, mixed)
    write_ws(ctx, "code.txt", "code\n")
    track(ctx, ["todo", "done", "1"], cwd=ctx.worktree)
    track(ctx, ["todo", "done", "2"], cwd=ctx.worktree)
    write_ws(ctx, "more.txt", "more\n")
    track(ctx, ["todo", "done", "3"], cwd=ctx.worktree)
    mixed_idx = task_todo_indexes(ctx)
    add(
        checks,
        cid="MT5",
        ctx=ctx,
        family="many-todos",
        title="Research TODOs still get a Task-Todo commit when a workspace exists",
        expected="[1, 2, 3]",
        actual=str(mixed_idx),
        ok=mixed_idx == [1, 2, 3],
    )

    nxt = setup_task(
        ctx,
        "Next reorder",
        "SIM-MT4",
        [("Alpha", False), ("Bravo", False), ("Charlie", False)],
    )
    bind_status(ctx, nxt)
    track(ctx, ["todo", "next", "3"], cwd=ctx.worktree)
    write_ws(ctx, "charlie.txt", "charlie first\n")
    track(ctx, ["todo", "done", "1"], cwd=ctx.worktree)
    shas = task_todo_shas(ctx)
    body = trailers(ctx, shas[1]) if 1 in shas else ""
    add(
        checks,
        cid="MT6",
        ctx=ctx,
        family="many-todos",
        title="todo next then done 1 records the moved TODO as Task-Todo: 1",
        expected="Charlie in Task-Todo: 1 body",
        actual=body[:200],
        ok=1 in shas and "Charlie" in body,
    )


def family_notes_ops(ctx: Ctx, checks: list[Check]) -> None:
    status = setup_task(ctx, "Notes ops", "SIM-N", [("Ship notes", False)])
    bind_status(ctx, status)
    ws = ctx.worktree
    assert ws is not None
    track(ctx, ["desc", "first draft"], cwd=ws)
    marker_v1 = notes_show(ctx, ctx.marker) or ""
    write_ws(ctx, "n.txt", "notes work\n")
    track(ctx, ["scrap", "add", "--share", "secret sauce"], cwd=ws)
    track(ctx, ["todo", "done", "1"], cwd=ws)
    shas = task_todo_shas(ctx)
    todo_sha = shas.get(1, "")
    before_unshare = notes_show(ctx, todo_sha) or ""
    unshare = track(ctx, ["scrap", "unshare", "1"], cwd=ws, ok=False)
    after_unshare = notes_show(ctx, todo_sha) or ""
    add(
        checks,
        cid="N5",
        ctx=ctx,
        family="notes-ops",
        title="Unshare on an unpublished TODO commit removes the scrap from notes",
        expected="secret sauce gone after unshare",
        actual=f"rc={unshare.returncode} before={('secret sauce' in before_unshare)} after={('secret sauce' in after_unshare)}",
        ok=unshare.returncode == 0
        and "secret sauce" in before_unshare
        and "secret sauce" not in after_unshare,
    )
    track(ctx, ["desc", "second draft"], cwd=ws)
    marker_v2 = notes_show(ctx, ctx.marker) or ""
    push_task_branch(ctx)
    notes_push = track(ctx, ["notes", "push"], cwd=ws, ok=False)
    track(ctx, ["desc", "third draft"], cwd=ws)
    marker_v3 = notes_show(ctx, ctx.marker) or ""
    add(
        checks,
        cid="N4",
        ctx=ctx,
        family="notes-ops",
        title="Marker notes update while unpublished; freeze after origin",
        expected="v2 on marker after push; v3 does not rewrite",
        actual=f"v1={('first draft' in marker_v1)} v2={('second draft' in marker_v2)} v3={('third draft' in marker_v3)}",
        ok="first draft" in marker_v1
        and "second draft" in marker_v2
        and "second draft" in marker_v3
        and "third draft" not in marker_v3
        and notes_push.returncode == 0,
    )

    clone = ROOT / f"{ctx.mode}-notes-clone"
    if clone.exists():
        shutil.rmtree(clone)
    imp_home = ctx.home.parent / f"{ctx.mode}-notes-home"
    if imp_home.exists():
        shutil.rmtree(imp_home)
    imp_home.mkdir(parents=True)
    write_gitconfig(imp_home)
    imp_env = make_env(imp_home)
    run(["git", "clone", str(ctx.origin), str(clone)], cwd=ROOT, env=imp_env)
    run(
        ["git", "checkout", f"track/{ctx.slug}"],
        cwd=clone,
        env=imp_env,
        check=False,
    )
    before_fetch = run(
        ["git", "notes", "--ref", "refs/notes/track", "show", todo_sha],
        cwd=clone,
        env=imp_env,
        check=False,
    )
    fetched = run(
        [str(TRACK), "notes", "fetch"],
        cwd=clone,
        env=imp_env,
        check=False,
    )
    after_fetch = run(
        ["git", "notes", "--ref", "refs/notes/track", "show", ctx.marker or "HEAD"],
        cwd=clone,
        env=imp_env,
        check=False,
    )
    add(
        checks,
        cid="N1",
        ctx=ctx,
        family="notes-ops",
        title="notes fetch on a clone restores refs/notes/track",
        expected="fetch exit 0; marker notes present; were missing before",
        actual=f"before_rc={before_fetch.returncode} fetch_rc={fetched.returncode} after={after_fetch.stdout[:80]!r}",
        ok=before_fetch.returncode != 0
        and fetched.returncode == 0
        and after_fetch.returncode == 0
        and "second draft" in after_fetch.stdout,
    )

    hijack = run(
        [
            "git",
            "notes",
            "--ref",
            "refs/notes/track",
            "add",
            "-f",
            "-m",
            "hijacked notes",
            ctx.marker or "HEAD",
        ],
        cwd=clone,
        env=imp_env,
        check=False,
    )
    push_div = run(
        [str(TRACK), "notes", "push"],
        cwd=clone,
        env=imp_env,
        check=False,
    )
    add(
        checks,
        cid="N2",
        ctx=ctx,
        family="notes-ops",
        title="Diverged notes push is rejected (no force)",
        expected="nonzero push",
        actual=f"hijack_rc={hijack.returncode} push_rc={push_div.returncode} {push_div.stderr[-180:]}",
        ok=hijack.returncode == 0 and push_div.returncode != 0,
    )
    hijack_todo = run(
        [
            "git",
            "notes",
            "--ref",
            "refs/notes/track",
            "add",
            "-f",
            "-m",
            "hijacked todo notes",
            todo_sha,
        ],
        cwd=clone,
        env=imp_env,
        check=False,
    )
    push_todo = run(
        [str(TRACK), "notes", "push"],
        cwd=clone,
        env=imp_env,
        check=False,
    )
    origin_todo = run(
        [
            "git",
            "--git-dir",
            str(ctx.origin),
            "notes",
            "--ref",
            "refs/notes/track",
            "show",
            todo_sha,
        ],
        cwd=ROOT,
        env=ctx.env,
        check=False,
    )
    add(
        checks,
        cid="N6",
        ctx=ctx,
        family="notes-ops",
        title="notes add -f on a published TODO SHA cannot be pushed",
        expected="nonzero push; origin blob unchanged",
        actual=(
            f"hijack_rc={hijack_todo.returncode} push_rc={push_todo.returncode} "
            f"origin={origin_todo.stdout[:80]!r} {push_todo.stderr[-120:]}"
        ),
        ok=hijack_todo.returncode == 0
        and push_todo.returncode != 0
        and "hijacked todo notes" not in origin_todo.stdout,
    )
    fetch_div = run(
        [str(TRACK), "notes", "fetch"],
        cwd=clone,
        env=imp_env,
        check=False,
    )
    still = run(
        ["git", "notes", "--ref", "refs/notes/track", "show", ctx.marker or "HEAD"],
        cwd=clone,
        env=imp_env,
        check=False,
    )
    add(
        checks,
        cid="N3",
        ctx=ctx,
        family="notes-ops",
        title="notes fetch does not replace diverged local notes with origin",
        expected="local hijack remains",
        actual=f"fetch_rc={fetch_div.returncode} notes={still.stdout[:80]!r}",
        ok="hijacked notes" in still.stdout,
    )
    restored = run(
        [
            "git",
            "fetch",
            "origin",
            "+refs/notes/track:refs/notes/track",
        ],
        cwd=clone,
        env=imp_env,
        check=False,
    )
    after_restore = run(
        ["git", "notes", "--ref", "refs/notes/track", "show", ctx.marker or "HEAD"],
        cwd=clone,
        env=imp_env,
        check=False,
    )
    after_restore_todo = run(
        ["git", "notes", "--ref", "refs/notes/track", "show", todo_sha],
        cwd=clone,
        env=imp_env,
        check=False,
    )
    add(
        checks,
        cid="N8",
        ctx=ctx,
        family="notes-ops",
        title="Manual force-fetch of origin notes restores hijacked local notes",
        expected="second draft back; hijack gone",
        actual=(
            f"rc={restored.returncode} marker={after_restore.stdout[:80]!r} "
            f"todo={after_restore_todo.stdout[:80]!r}"
        ),
        ok=restored.returncode == 0
        and "second draft" in after_restore.stdout
        and "hijacked notes" not in after_restore.stdout
        and "hijacked todo notes" not in after_restore_todo.stdout,
    )

    orig_todo_notes = notes_show(ctx, todo_sha) or ""
    track_json(ctx, ["todo", "add", "Notes follow-up", "--json"])
    write_ws(ctx, "follow-n.txt", "follow notes\n")
    track(ctx, ["scrap", "add", "--share", "new secret"], cwd=ws)
    track(ctx, ["todo", "done", "2"], cwd=ws)
    follow_shas = task_todo_shas(ctx)
    follow_sha = follow_shas.get(2, "")
    follow_notes = notes_show(ctx, follow_sha) or ""
    after_todo_notes = notes_show(ctx, todo_sha) or ""
    follow_push = track(ctx, ["notes", "push"], cwd=ws, ok=False)
    add(
        checks,
        cid="N7",
        ctx=ctx,
        family="notes-ops",
        title="Follow-up TODO notes still push after origin exists",
        expected="push exit 0; new secret on new SHA; published TODO notes unchanged",
        actual=(
            f"push_rc={follow_push.returncode} follow={follow_sha[:12]} "
            f"new={'new secret' in follow_notes} "
            f"old_same={after_todo_notes == orig_todo_notes}"
        ),
        ok=follow_push.returncode == 0
        and bool(follow_sha)
        and "new secret" in follow_notes
        and after_todo_notes == orig_todo_notes
        and "new secret" not in after_todo_notes,
    )


def family_merge_two_tasks(ctx: Ctx, checks: list[Check]) -> None:
    a = setup_task(ctx, "Merge Alpha", "SIM-MA", [("Do A", False)])
    bind_status(ctx, a)
    ws_a = ctx.worktree
    slug_a = ctx.slug
    marker_a = ctx.marker
    write_ws(ctx, "a.txt", "alpha\n")
    track(ctx, ["scrap", "add", "--share", "A decision"], cwd=ws_a)
    track(ctx, ["todo", "done", "1"], cwd=ws_a)
    sha_a = task_todo_shas(ctx).get(1, "")
    notes_a = notes_show(ctx, sha_a) or ""
    push_task_branch(ctx)
    track(ctx, ["notes", "push"], cwd=ws_a, ok=False)

    b = setup_task(ctx, "Merge Beta", "SIM-MB", [("Do B", False)])
    bind_status(ctx, b)
    ws_b = ctx.worktree
    slug_b = ctx.slug
    write_ws(ctx, "b.txt", "beta\n")
    track(ctx, ["scrap", "add", "--share", "B decision"], cwd=ws_b)
    track(ctx, ["todo", "done", "1"], cwd=ws_b)
    sha_b = task_todo_shas(ctx).get(1, "")
    notes_b = notes_show(ctx, sha_b) or ""
    push_task_branch(ctx)
    track(ctx, ["notes", "push"], cwd=ws_b, ok=False)

    merged = merge_task_to_main(ctx, slug_a)
    notes_a_after = notes_show(ctx, sha_a) or ""
    add(
        checks,
        cid="MG1",
        ctx=ctx,
        family="merge-tasks",
        title="Merging task A into main keeps A's TODO SHA and notes",
        expected="merge ok; A notes still on A SHA",
        actual=f"merge_rc={merged.returncode} sha={sha_a[:12]} notes={('A decision' in notes_a_after)}",
        ok=merged.returncode == 0
        and bool(sha_a)
        and "A decision" in notes_a
        and notes_a_after == notes_a,
    )

    ctx.worktree = ws_b
    ctx.slug = slug_b
    into_b = merge_main_into_task(ctx)
    sha_b_after = sha_b if git(ctx, ["cat-file", "-t", sha_b], cwd=ctx.repo, ok=False).stdout.strip() == "commit" else ""
    add(
        checks,
        cid="MG2",
        ctx=ctx,
        family="merge-tasks",
        title="Merging main into B does not rewrite B's unpublished-at-birth unique TODO SHA",
        expected="B TODO SHA still a commit; merge clean or conflict-free",
        actual=f"merge_rc={into_b.returncode} still={sha_b_after[:12]} err={into_b.stderr[-120:]}",
        ok=into_b.returncode == 0 and sha_b_after == sha_b,
    )
    notes_b_after = notes_show(ctx, sha_b) or ""
    add(
        checks,
        cid="MG3",
        ctx=ctx,
        family="merge-tasks",
        title="After merging main, A and B notes stay on their own SHAs",
        expected="A decision on A; B decision on B",
        actual=f"A={('A decision' in (notes_show(ctx, sha_a) or ''))} B={('B decision' in notes_b_after)}",
        ok="A decision" in (notes_show(ctx, sha_a) or "")
        and "B decision" in notes_b_after
        and notes_b_after == notes_b,
    )

    imp_home = ctx.home.parent / f"{ctx.mode}-merge-importer"
    if imp_home.exists():
        shutil.rmtree(imp_home)
    imp_home.mkdir(parents=True)
    write_gitconfig(imp_home)
    imp_env = make_env(imp_home)
    imp = run(
        [str(TRACK), "notes", "fetch"],
        cwd=ws_b,
        env=imp_env,
        check=False,
    )
    imported = run(
        [str(TRACK), "import", "--json"],
        cwd=ws_b,
        env=imp_env,
        check=False,
    )
    names: list[str] = []
    todos: list[str] = []
    if imported.returncode == 0:
        try:
            data = json.loads(imported.stdout)
        except json.JSONDecodeError:
            data = {}
        task = data.get("task") or {}
        names = [str(task.get("name") or data.get("name") or "")]
        todos = [
            str(t.get("content") or t.get("text") or "")
            for t in (data.get("todos") or data.get("todos_agent") or [])
        ]
    add(
        checks,
        cid="MG4",
        ctx=ctx,
        family="merge-tasks",
        title="import on B after merging main restores B, not merged sibling A",
        expected="Merge Beta / Do B; not Do A",
        actual=f"fetch_rc={imp.returncode} import_rc={imported.returncode} names={names} todos={todos} {imported.stderr[-120:]}",
        ok=imported.returncode == 0
        and any("Beta" in n for n in names)
        and any("Do B" in t for t in todos)
        and not any("Do A" in t for t in todos),
    )
    _ = marker_a

    c = setup_task(ctx, "Conflict Charlie", "SIM-MC", [("Do C", False)])
    bind_status(ctx, c)
    slug_c = ctx.slug
    write_ws(ctx, "conflict.txt", "from C\n")
    track(ctx, ["todo", "done", "1"], cwd=ctx.worktree)
    push_task_branch(ctx)

    d = setup_task(ctx, "Conflict Delta", "SIM-MD", [("Do D", False)])
    bind_status(ctx, d)
    sha_d = ""
    write_ws(ctx, "conflict.txt", "from D\n")
    track(ctx, ["todo", "done", "1"], cwd=ctx.worktree)
    sha_d = task_todo_shas(ctx).get(1, "")
    notes_d = notes_show(ctx, sha_d) or ""
    push_task_branch(ctx)
    merge_task_to_main(ctx, slug_c)
    conflicted = merge_main_into_task(ctx)
    is_conflict = conflicted.returncode != 0 or jj_conflicted(ctx)
    add(
        checks,
        cid="MG5",
        ctx=ctx,
        family="merge-tasks",
        title="Same-file edits from two tasks conflict when merging main into D",
        expected="merge conflict",
        actual=f"rc={conflicted.returncode} jj_conflict={jj_conflicted(ctx)} {conflicted.stderr[-150:]}",
        ok=is_conflict,
    )
    still_d = (
        git(ctx, ["cat-file", "-t", sha_d], cwd=ctx.repo, ok=False).stdout.strip() == "commit"
        if sha_d
        else False
    )
    add(
        checks,
        cid="MG6",
        ctx=ctx,
        family="merge-tasks",
        title="Conflict does not drop D's TODO commit or its notes",
        expected="D SHA exists; notes readable",
        actual=f"sha={sha_d[:12]} still={still_d} notes={('Do D' in notes_d) or bool(notes_show(ctx, sha_d))}",
        ok=still_d and (notes_show(ctx, sha_d) is not None or bool(notes_d)),
    )

    notes_d_before = notes_show(ctx, sha_d) or notes_d
    track_json(ctx, ["todo", "add", "After conflict", "--json"])
    write_ws(ctx, "after-merge.txt", "follow after resolve\n")
    during = track(ctx, ["todo", "done", "2"], cwd=ctx.worktree, ok=False)
    add(
        checks,
        cid="MG7",
        ctx=ctx,
        family="merge-tasks",
        title="todo done refuses a conflicted workspace",
        expected="nonzero; D SHA unchanged",
        actual=f"rc={during.returncode} conflicted={workspace_conflicted(ctx)} {during.stderr[-150:]}",
        ok=during.returncode != 0
        and workspace_conflicted(ctx)
        and git(ctx, ["cat-file", "-t", sha_d], cwd=ctx.repo, ok=False).stdout.strip()
        == "commit",
    )
    resolved = resolve_merge(ctx, "conflict.txt", "from C and D\n")
    after = track(ctx, ["todo", "done", "2"], cwd=ctx.worktree, ok=False)
    follow_shas = task_todo_shas(ctx)
    follow_sha = follow_shas.get(2, "")
    add(
        checks,
        cid="MG8",
        ctx=ctx,
        family="merge-tasks",
        title="After resolving the conflict, a follow-up TODO appends",
        expected="todo done 2 exit 0; Task-Todo: 2 exists",
        actual=(
            f"resolve_rc={resolved.returncode} done_rc={after.returncode} "
            f"indexes={list(follow_shas)} {after.stderr[-120:]}"
        ),
        ok=after.returncode == 0 and 2 in follow_shas,
    )
    notes_d_after = notes_show(ctx, sha_d) or ""
    blob = commit_blob(ctx, follow_sha, "conflict.txt") if follow_sha else ""
    files = commit_files(ctx, follow_sha) if follow_sha else []
    add(
        checks,
        cid="MG9",
        ctx=ctx,
        family="merge-tasks",
        title="Follow-up keeps D's SHA/notes and records the resolution",
        expected="D ancestor of TODO 2; D notes same; conflict.txt resolved; after-merge.txt present",
        actual=(
            f"anc={is_ancestor(ctx, sha_d, follow_sha)} notes_same={notes_d_after == notes_d_before} "
            f"blob={blob[:40]!r} files={files}"
        ),
        ok=bool(follow_sha)
        and is_ancestor(ctx, sha_d, follow_sha)
        and notes_d_after == notes_d_before
        and "from C and D" in blob
        and "<<<<<<" not in blob
        and "after-merge.txt" in files,
    )


def family_root_mistake(ctx: Ctx, checks: list[Check]) -> None:
    """Working in repo root instead of the worktree — a common agent mistake."""
    status = setup_task(
        ctx, "Root mistake", "SIM-R", [("Ship from workspace", False)]
    )
    bind_status(ctx, status)
    accident = ctx.repo / "ACCIDENT.txt"
    accident.write_text("wrote at root\n")
    dirty = git(ctx, ["status", "--porcelain"], cwd=ctx.repo).stdout
    add(
        checks,
        cid="R1",
        ctx=ctx,
        family="root-mistake",
        title="Repo root is dirty if the agent forgets the worktree (observation)",
        expected="ACCIDENT.txt visible at repo root; worktree is the intended place",
        actual=dirty.strip()[:200],
        ok="ACCIDENT.txt" in dirty,
        status_if_false="issue",
        detail="Not a track bug; documents that todo done will not pick up root dirty files.",
    )
    write_ws(ctx, "real.txt", "from worktree\n")
    done = track(ctx, ["todo", "done", "1"], cwd=ctx.repo, ok=False)
    sha = task_todo_shas(ctx).get(1, "")
    files = commit_files(ctx, sha) if sha else []
    add(
        checks,
        cid="R2",
        ctx=ctx,
        family="root-mistake",
        title="todo done from repo root folds only the worktree, not root dirty files",
        expected="real.txt in commit; ACCIDENT.txt not in commit; done exit 0",
        actual=f"rc={done.returncode} sha={sha[:12]} files={files}",
        ok=done.returncode == 0
        and "real.txt" in files
        and "ACCIDENT.txt" not in files,
    )
    still = git(ctx, ["status", "--porcelain"], cwd=ctx.repo).stdout
    add(
        checks,
        cid="R3",
        ctx=ctx,
        family="root-mistake",
        title="Root ACCIDENT.txt stays uncommitted on main after todo done",
        expected="ACCIDENT.txt still dirty at repo root",
        actual=still.strip()[:200],
        ok="ACCIDENT.txt" in still,
    )
    accident.unlink(missing_ok=True)


def new_ctx(mode: str) -> Ctx:
    home = ROOT / f"{mode}-home"
    if home.exists():
        shutil.rmtree(home)
    home.mkdir(parents=True)
    write_gitconfig(home)
    repo = ROOT / f"{mode}-repo"
    origin = ROOT / f"{mode}-origin.git"
    if repo.exists():
        shutil.rmtree(repo)
    if origin.exists():
        shutil.rmtree(origin)
    ctx = Ctx(mode=mode, home=home, repo=repo, origin=origin, env=make_env(home))
    init_product(ctx)
    boot_track(ctx)
    return ctx


def run_mode(mode: str) -> list[Check]:
    checks: list[Check] = []
    ctx = new_ctx(mode)
    try:
        status = setup_task(
            ctx,
            "OAuth refresh",
            "SIM-1",
            [
                ("Design token flow", False),
                ("Implement refresh", False),
                ("Write docs", True),
            ],
        )
        family_birth(ctx, checks, status)
        family_daily(ctx, checks)
        family_publish_review(ctx, checks)
        family_import(ctx, checks)
        family_root_mistake(ctx, checks)
        family_published_wip(ctx, checks)
        family_out_of_order(ctx, checks)
        family_backfill(ctx, checks)
        family_published_backfill(ctx, checks)
        family_push_birth(ctx, checks)
        family_share_after_done(ctx, checks)
        family_empty_followup(ctx, checks)
        family_two_tasks(ctx, checks)
        family_sibling_backfill(ctx, checks)
        family_two_repo_task(ctx, checks)
        family_vcs_switch(ctx, checks)
        family_jj_fold_gap(ctx, checks)
        family_many_todos(ctx, checks)
        family_notes_ops(ctx, checks)
        family_merge_two_tasks(ctx, checks)
        family_archive_pr_head(ctx, checks)
    except Exception as e:
        checks.append(
            Check(
                id="XX",
                mode=mode,
                family="harness",
                title="Uncaught harness error",
                expected="scenario run completes",
                actual=repr(e)[:400],
                status="fail",
                detail="traceback truncated",
            )
        )
    return checks


def main() -> int:
    if not TRACK.exists():
        print("missing track binary", TRACK, file=sys.stderr)
        return 2
    ROOT.mkdir(parents=True, exist_ok=True)
    all_checks: list[Check] = []
    for mode in ("git", "jj"):
        print(f"=== {mode} ===", flush=True)
        all_checks.extend(run_mode(mode))
        n = [c for c in all_checks if c.mode == mode]
        print(
            f"  {sum(c.status=='pass' for c in n)} pass / "
            f"{sum(c.status=='fail' for c in n)} fail / "
            f"{sum(c.status=='issue' for c in n)} issue / "
            f"{len(n)} total",
            flush=True,
        )
    RESULTS.write_text(json.dumps([asdict(c) for c in all_checks], indent=2))
    print("wrote", RESULTS)
    fails = [c for c in all_checks if c.status == "fail"]
    return 1 if fails else 0


if __name__ == "__main__":
    sys.exit(main())
