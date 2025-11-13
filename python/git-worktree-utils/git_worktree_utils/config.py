"""Environment and repository configuration helpers."""

from __future__ import annotations

import os
import re
from pathlib import Path
from typing import Iterable

from .exceptions import GitCommandError, MissingEnvError, RepoDetectionError
from .git import run_git
from .models import EnvConfig, Paths, RepoContext, RepoMetadata

_GIT_URL_RE = re.compile(
    r"^(?:(?P<protocol>https?|git)://|git@)(?P<host>[^/:]+)[:/](?P<owner>[^/]+)/(?P<repo>[^/]+?)(?:\.git)?$"
)


def load_env_config() -> EnvConfig:
    admin_root = _get_and_validate_path("GIT_WORKTREE_ADMIN_ROOT")
    worktree_root = _get_and_validate_path("GIT_WORKTREE_ROOT")
    return EnvConfig(admin_root=admin_root, worktree_root=worktree_root)


def _get_and_validate_path(var_name: str) -> Path:
    raw = os.environ.get(var_name)
    if not raw:
        raise MissingEnvError(
            f"Environment variable {var_name} is required. Export it before running the CLI."
        )
    path = Path(raw).expanduser().resolve()
    if not path.exists():
        raise MissingEnvError(
            f"Path from {var_name} does not exist: {path}. Create it or update the variable."
        )
    if not os.access(path, os.W_OK | os.X_OK):
        raise MissingEnvError(
            f"Path from {var_name} is not writable: {path}. Adjust permissions or pick another location."
        )
    return path


def resolve_repo_path(repo_override: Path | None) -> Path:
    if repo_override:
        repo_path = repo_override.expanduser().resolve()
    else:
        repo_path = Path.cwd()
    output = run_git(["rev-parse", "--show-toplevel"], cwd=repo_path)
    return Path(output.stdout.strip())


def discover_repo_metadata(repo_path: Path) -> RepoMetadata:
    origin = run_git(["remote", "get-url", "origin"], cwd=repo_path).stdout.strip()
    match = _GIT_URL_RE.match(origin)
    if not match:
        raise RepoDetectionError(
            "Unable to parse origin URL. Supported formats include git@host:owner/repo.git and https URLs."
        )
    host = match.group("host")
    owner = match.group("owner")
    repo = match.group("repo")
    return RepoMetadata(host=host, owner=owner, name=repo, origin_url=origin)


def detect_default_branch(repo_path: Path) -> str:
    try:
        ref = run_git(["symbolic-ref", "refs/remotes/origin/HEAD"], cwd=repo_path).stdout.strip()
        _, branch = ref.rsplit("/", 1)
        return branch
    except GitCommandError:
        for fallback in ("main", "master"):
            if branch_exists(repo_path, fallback):
                return fallback
        raise RepoDetectionError(
            "Unable to detect default branch from origin/HEAD and neither 'main' nor 'master' exist."
        )


def branch_exists(repo_path: Path, branch: str) -> bool:
    result = run_git([
        "rev-parse",
        "--verify",
        f"refs/heads/{branch}",
    ],
        cwd=repo_path,
        check=False,
    )
    return result.returncode == 0


def build_repo_context(repo_override: Path | None) -> RepoContext:
    repo_path = resolve_repo_path(repo_override)
    metadata = discover_repo_metadata(repo_path)
    default_branch = detect_default_branch(repo_path)
    return RepoContext(repo_path=repo_path, metadata=metadata, default_branch=default_branch)


def build_paths(env: EnvConfig, metadata: RepoMetadata) -> Paths:
    admin_path = env.admin_root / metadata.host / metadata.owner / metadata.name
    worktree_root = env.worktree_root / metadata.host / metadata.owner / metadata.name
    return Paths(admin_path=admin_path, repo_worktrees_root=worktree_root)


def slugify_worktree_name(name: str) -> str:
    if not name:
        raise ValueError("Worktree name cannot be empty")
    slug = name.strip().replace("/", "__")
    slug = re.sub(r"\s+", "-", slug)
    slug = re.sub(r"[^A-Za-z0-9_.-]", "-", slug)
    slug = re.sub(r"-+", "-", slug)
    return slug


def format_worktree_path(paths: Paths, slug: str) -> Path:
    return paths.repo_worktrees_root / slug


__all__ = [
    "load_env_config",
    "build_repo_context",
    "build_paths",
    "slugify_worktree_name",
    "format_worktree_path",
]
