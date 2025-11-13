"""Dataclasses shared across modules."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class RepoContext:
    """Metadata resolved from the user's git repository."""

    repo_path: Path
    remote_url: str
    host: str
    owner: str
    name: str
    default_branch: str

    @property
    def slug(self) -> str:
        return f"{self.host}/{self.owner}/{self.name}"


@dataclass(frozen=True)
class RepoPaths:
    """Concrete filesystem locations derived from env vars."""

    admin_root: Path
    worktree_root: Path
    admin_repo: Path
    worktree_repo_root: Path


@dataclass(frozen=True)
class WorktreeEntry:
    """Represents a single worktree tracked by git."""

    path: Path
    branch: str | None
    context: str | None
    status: str

    @property
    def display_name(self) -> str:
        context = self.context or "?"
        branch = self.branch or "(detached)"
        return f"{context}/{branch}"
