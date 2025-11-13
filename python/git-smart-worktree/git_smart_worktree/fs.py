"""Filesystem helpers for git-smart-worktree."""

from __future__ import annotations

import re
from pathlib import Path

from .models import RepoContext, RepoPaths


_SAFE_PATTERN = re.compile(r"[^A-Za-z0-9._-]")


def slugify_branch(name: str) -> str:
    """Produce a filesystem-safe representation of a branch name."""

    slug = name.strip()
    if not slug:
        return "unnamed"
    slug = slug.replace("/", "__").replace(" ", "-")
    slug = _SAFE_PATTERN.sub("-", slug)
    return slug.lower()


def build_repo_paths(context: RepoContext, admin_root: Path, worktree_root: Path) -> RepoPaths:
    admin_repo = admin_root / context.host / context.owner / context.name
    worktree_repo_root = worktree_root / context.host / context.owner / context.name
    return RepoPaths(
        admin_root=admin_root,
        worktree_root=worktree_root,
        admin_repo=admin_repo,
        worktree_repo_root=worktree_repo_root,
    )


def ensure_directory(path: Path) -> None:
    path.mkdir(parents=True, exist_ok=True)
