"""Thin wrappers around git CLI commands."""

from __future__ import annotations

import subprocess
from pathlib import Path
from typing import Iterable

from .exceptions import GitCommandError


def run_git(
    args: Iterable[str],
    *,
    cwd: Path,
    raise_on_error: bool = True,
) -> subprocess.CompletedProcess[str]:
    """Execute a git command and optionally raise on failure."""

    cmd = ["git", *args]
    proc = subprocess.run(
        cmd,
        cwd=str(cwd),
        capture_output=True,
        text=True,
        check=False,
    )
    if raise_on_error and proc.returncode != 0:
        raise GitCommandError(cmd, proc.returncode, proc.stderr)
    return proc


def rev_parse_toplevel(path: Path) -> Path:
    proc = run_git(["rev-parse", "--show-toplevel"], cwd=path)
    return Path(proc.stdout.strip())


def remote_url(path: Path, remote: str = "origin") -> str:
    proc = run_git(["remote", "get-url", remote], cwd=path)
    return proc.stdout.strip()


def default_branch(path: Path) -> str | None:
    proc = run_git(
        ["symbolic-ref", "refs/remotes/origin/HEAD"],
        cwd=path,
        raise_on_error=False,
    )
    if proc.returncode == 0:
        ref = proc.stdout.strip()
        return ref.split("/")[-1]
    # fallback heuristics
    for candidate in ("main", "master"):
        if branch_exists(path, candidate):
            return candidate
    return None


def branch_exists(path: Path, branch: str) -> bool:
    proc = run_git(
        ["show-ref", "--verify", f"refs/heads/{branch}"],
        cwd=path,
        raise_on_error=False,
    )
    return proc.returncode == 0


def remote_branch_exists(path: Path, branch: str, remote: str = "origin") -> bool:
    proc = run_git(
        ["ls-remote", "--exit-code", "--heads", remote, branch],
        cwd=path,
        raise_on_error=False,
    )
    return proc.returncode == 0


def list_branches(path: Path, include_remote: bool = True) -> list[str]:
    args = ["branch", "--format", "%(refname:short)"]
    if include_remote:
        args.insert(1, "-a")
    proc = run_git(args, cwd=path)
    reduced = set()
    for raw in proc.stdout.splitlines():
        line = raw.strip()
        if not line or line == "HEAD":
            continue
        reduced.add(normalize_branch_name(line))
    names = sorted(reduced)
    return names


def normalize_branch_name(name: str) -> str:
    if name.startswith("remotes/"):
        name = name.split("/", 2)[-1]
    return name


def fetch(path: Path, remote: str = "origin", prune: bool = True) -> None:
    args = ["fetch", remote]
    if prune:
        args.append("--prune")
    run_git(args, cwd=path)


def clone_no_checkout(remote: str, target: Path) -> None:
    run_git([
        "clone",
        "--no-checkout",
        remote,
        str(target),
    ], cwd=target.parent)


def worktree_list(path: Path) -> list[dict]:
    proc = run_git(["worktree", "list", "--porcelain"], cwd=path)
    items: list[dict] = []
    current: dict | None = None
    for raw_line in proc.stdout.splitlines():
        line = raw_line.strip()
        if not line:
            continue
        key, _, value = line.partition(" ")
        if key == "worktree":
            if current:
                items.append(current)
            current = {"path": Path(value.strip())}
        elif not current:
            continue
        elif key == "branch":
            branch = value.strip()
            if branch.startswith("refs/heads/"):
                branch = branch.split("/", 2)[-1]
            current["branch"] = branch
        elif key == "HEAD":
            current["head"] = value.strip()
        elif key == "locked":
            current["locked"] = True
        elif key == "prunable":
            current["prunable"] = True
        elif key == "detached":
            current["branch"] = None
    if current:
        items.append(current)
    return items


def worktree_add_existing(path: Path, target: Path, branch: str) -> None:
    run_git(["worktree", "add", str(target), branch], cwd=path)


def worktree_add_from_remote(path: Path, target: Path, branch: str, remote_ref: str) -> None:
    run_git(["worktree", "add", str(target), "-B", branch, remote_ref], cwd=path)


def worktree_add_new(path: Path, target: Path, branch: str, start_point: str) -> None:
    run_git(["worktree", "add", str(target), "-b", branch, start_point], cwd=path)


def worktree_remove(path: Path, target: Path, force: bool = False) -> None:
    args = ["worktree", "remove"]
    if force:
        args.append("--force")
    args.append(str(target))
    run_git(args, cwd=path)


def head_ref(path: Path) -> str | None:
    proc = run_git(["symbolic-ref", "-q", "HEAD"], cwd=path, raise_on_error=False)
    if proc.returncode == 0:
        return proc.stdout.strip()
    return None


def ensure_detached_head(path: Path) -> None:
    if head_ref(path) is None:
        return
    commit = run_git(["rev-parse", "HEAD"], cwd=path).stdout.strip()
    if not commit:
        return
    run_git(["update-ref", "--no-deref", "HEAD", commit], cwd=path)
