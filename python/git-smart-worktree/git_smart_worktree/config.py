"""Load environment variables and git metadata for runtime."""

from __future__ import annotations

import os
from pathlib import Path
from urllib.parse import urlparse

from .exceptions import GitCommandError, MissingEnvError, ValidationError
from .fs import build_repo_paths
from .git import default_branch as git_default_branch
from .git import remote_url as git_remote_url
from .git import rev_parse_toplevel
from .models import RepoContext, RepoPaths


def load_runtime(repo_override: Path | None = None) -> tuple[RepoContext, RepoPaths]:
    repo_path = resolve_repo_path(repo_override)
    remote = git_remote_url(repo_path)
    host, owner, name = parse_remote(remote)
    default = git_default_branch(repo_path)
    if not default:
        raise ValidationError("Unable to determine default branch. Fetch origin and try again.")
    context = RepoContext(
        repo_path=repo_path,
        remote_url=remote,
        host=host,
        owner=owner,
        name=name,
        default_branch=default,
    )
    admin_root, worktree_root = resolve_env_paths()
    paths = build_repo_paths(context, admin_root, worktree_root)
    return context, paths


def resolve_env_paths() -> tuple[Path, Path]:
    admin = _require_env("GIT_WORKTREE_ADMIN_ROOT")
    worktree = _require_env("GIT_WORKTREE_ROOT")
    return admin, worktree


def resolve_repo_path(repo_override: Path | None) -> Path:
    if repo_override:
        candidate = repo_override.expanduser()
        if not candidate.exists():
            raise ValidationError(f"Repository override path does not exist: {candidate}")
        cwd = candidate
    else:
        cwd = Path.cwd()
    try:
        return rev_parse_toplevel(cwd)
    except GitCommandError as exc:  # pragma: no cover - rely on git error text
        raise ValidationError("Current directory is not inside a git repository.") from exc


def parse_remote(remote: str) -> tuple[str, str, str]:
    if remote.startswith("git@"):
        host_token = remote.split("@", 1)[1]
        host, path = host_token.split(":", 1)
        path = path.rstrip("/")
    else:
        parsed = urlparse(remote)
        host = parsed.hostname or parsed.netloc
        path = parsed.path.lstrip("/")
    if not host or not path:
        raise ValidationError(f"Unsupported remote URL: {remote}")
    parts = [part for part in path.split("/") if part]
    if len(parts) < 2:
        raise ValidationError("Remote URL must look like <host>/<owner>/<repo>.")
    owner = parts[-2]
    name = parts[-1]
    if name.endswith(".git"):
        name = name[: -len(".git")]
    return host, owner, name


def _require_env(var: str) -> Path:
    raw = os.environ.get(var)
    if not raw:
        raise MissingEnvError(
            f"Environment variable {var} is required. Example: export {var}=$HOME/.git-worktrees"
        )
    return Path(raw).expanduser()
