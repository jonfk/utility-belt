"""Core business logic for worktree operations."""

from __future__ import annotations

from pathlib import Path
from typing import Sequence

from rich.console import Console
from rich.table import Table

from .config import format_worktree_path, slugify_worktree_name
from .exceptions import ValidationError
from .git import ensure_dirs, run_git
from .models import Paths, RepoContext, WorktreeEntry


def ensure_admin_clone(paths: Paths, repo_ctx: RepoContext, console: Console) -> None:
    if not paths.admin_path.exists():
        ensure_dirs([paths.admin_path.parent])
        with console.status("Cloning admin repository…"):
            run_git(
                ["clone", "--no-checkout", repo_ctx.metadata.origin_url, str(paths.admin_path)],
            )
    _ensure_detached_head(paths.admin_path)


def fetch_admin_clone(paths: Paths, console: Console) -> None:
    with console.status("Fetching refs in admin clone…"):
        run_git(["fetch", "origin", "--prune"], cwd=paths.admin_path)


def ensure_admin_ready(paths: Paths, repo_ctx: RepoContext, console: Console) -> None:
    ensure_admin_clone(paths, repo_ctx, console)


def list_worktrees(paths: Paths) -> list[WorktreeEntry]:
    output = run_git(["worktree", "list", "--porcelain"], cwd=paths.admin_path)
    entries = _parse_worktree_porcelain(output.stdout)
    # The admin clone is the control repo that spawns worktrees; it should not be displayed to users.
    return [entry for entry in entries if entry.path != paths.admin_path]


def render_worktrees_table(entries: Sequence[WorktreeEntry], console: Console) -> None:
    table = Table(title="Worktrees", show_lines=False)
    table.add_column("Name", no_wrap=True)
    table.add_column("Branch", no_wrap=True)
    table.add_column("Path")
    table.add_column("Status", no_wrap=True)
    for entry in entries:
        table.add_row(entry.name, entry.branch or "detached", str(entry.path), entry.status)
    console.print(table)


def render_worktrees_json(entries: Sequence[WorktreeEntry], console: Console) -> None:
    payload = [
        {
            "name": entry.name,
            "branch": entry.branch,
            "path": str(entry.path),
            "status": entry.status,
        }
        for entry in entries
    ]
    console.print_json(data=payload)


def add_worktree(
    paths: Paths,
    repo_ctx: RepoContext,
    *,
    worktree_name: str,
    branch: str,
    start_point: str | None,
    track: str | None,
    no_track_checkout: bool = False,
    console: Console,
) -> Path:
    slug = slugify_worktree_name(worktree_name)
    target_path = format_worktree_path(paths, slug)
    if target_path.exists():
        raise ValidationError(f"Worktree path already exists: {target_path}")
    ensure_dirs([paths.repo_worktrees_root])
    ensure_admin_clone(paths, repo_ctx, console)
    fetch_admin_clone(paths, console)

    worktree_args = ["worktree", "add", str(target_path)]
    local_branch_exists = _local_branch_exists(paths.admin_path, branch)
    remote_branch_exists = _remote_branch_exists(paths.admin_path, branch)
    if local_branch_exists:
        worktree_args.append(branch)
    else:
        inferred_start = None
        if remote_branch_exists:
            inferred_start = f"origin/{branch}"
        start = start_point or inferred_start or repo_ctx.default_branch
        worktree_args.extend(["-b", branch])
        if no_track_checkout:
            worktree_args.append("--no-track")
        worktree_args.append(start)

    with console.status(f"Adding worktree '{slug}'…"):
        run_git(worktree_args, cwd=paths.admin_path)

    if track:
        run_git([
            "branch",
            "--set-upstream-to",
            track,
            branch,
        ], cwd=target_path)

    return target_path


def remove_worktree(
    paths: Paths,
    repo_ctx: RepoContext,
    target: Path,
    *,
    force: bool,
    console: Console,
) -> None:
    ensure_admin_clone(paths, repo_ctx, console)
    args = ["worktree", "remove"]
    if force:
        args.append("--force")
    args.append(str(target))
    with console.status(f"Removing worktree '{target}'…"):
        run_git(args, cwd=paths.admin_path)
    _cleanup_empty_dirs(target, stop=paths.repo_worktrees_root)


def load_branch_summary(paths: Paths) -> dict[str, list[str]]:
    locals_ = _list_branches(paths.admin_path, remotes=False)
    remotes = _list_branches(paths.admin_path, remotes=True)
    return {"local": locals_, "remote": remotes}


def _local_branch_exists(repo_path: Path, branch: str) -> bool:
    result = run_git(
        ["rev-parse", "--verify", f"refs/heads/{branch}"],
        cwd=repo_path,
        check=False,
    )
    return result.returncode == 0


def _remote_branch_exists(repo_path: Path, branch: str) -> bool:
    result = run_git(
        ["rev-parse", "--verify", f"refs/remotes/origin/{branch}"],
        cwd=repo_path,
        check=False,
    )
    return result.returncode == 0


def _list_branches(repo_path: Path, *, remotes: bool) -> list[str]:
    if remotes:
        args = ["branch", "-r", "--format=%(refname:short)"]
    else:
        args = ["branch", "--format=%(refname:short)"]
    output = run_git(args, cwd=repo_path)
    branches = [line.strip() for line in output.stdout.splitlines() if line.strip()]
    if remotes:
        branches = [b for b in branches if b.startswith("origin/") and "->" not in b]
    return branches


def _parse_worktree_porcelain(text: str) -> list[WorktreeEntry]:
    entries: list[WorktreeEntry] = []
    current: dict[str, str | bool] = {}
    for line in text.splitlines() + [""]:
        if not line.strip():
            if current.get("worktree"):
                path = Path(str(current["worktree"]))
                branch_value = current.get("branch")
                branch = _sanitize_branch(branch_value) if branch_value else None
                entries.append(
                    WorktreeEntry(
                        path=path,
                        branch=branch,
                        is_locked=bool(current.get("locked")),
                        is_prunable=bool(current.get("prunable")),
                    )
                )
            current = {}
            continue
        key, _, value = line.partition(" ")
        if key in {"worktree", "HEAD"}:
            current[key] = value.strip()
        elif key == "branch":
            current[key] = value.strip()
        elif key in {"locked", "prunable"}:
            current[key] = True
        else:
            current[key] = value.strip()
    return entries


def _sanitize_branch(value: str) -> str:
    stripped = value.strip()
    prefix = "refs/heads/"
    if stripped.startswith(prefix):
        return stripped[len(prefix) :]
    return stripped


def _cleanup_empty_dirs(path: Path, stop: Path) -> None:
    resolved_path = path.resolve()
    resolved_stop = stop.resolve()
    if resolved_path == resolved_stop or resolved_stop not in resolved_path.parents:
        return
    current = resolved_path
    while current != resolved_stop:
        try:
            current.rmdir()
        except FileNotFoundError:
            break
        except OSError:
            break
        current = current.parent


def _ensure_detached_head(repo_path: Path) -> None:
    """Detach HEAD if it still points at a local branch."""
    result = run_git(["symbolic-ref", "-q", "HEAD"], cwd=repo_path, check=False)
    if result.returncode != 0:
        return
    head_ref = result.stdout.strip()
    if not head_ref:
        return
    commit = run_git(["rev-parse", head_ref], cwd=repo_path).stdout.strip()
    run_git(["update-ref", "--no-deref", "HEAD", commit], cwd=repo_path)


__all__ = [
    "ensure_admin_ready",
    "list_worktrees",
    "render_worktrees_table",
    "render_worktrees_json",
    "add_worktree",
    "remove_worktree",
    "load_branch_summary",
]
