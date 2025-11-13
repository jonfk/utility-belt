"""Shared dataclasses used throughout the CLI."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path


@dataclass(slots=True)
class EnvConfig:
    admin_root: Path
    worktree_root: Path


@dataclass(slots=True)
class RepoMetadata:
    host: str
    owner: str
    name: str
    origin_url: str


@dataclass(slots=True)
class RepoContext:
    repo_path: Path
    metadata: RepoMetadata
    default_branch: str


@dataclass(slots=True)
class Paths:
    admin_path: Path
    repo_worktrees_root: Path


@dataclass(slots=True)
class WorktreeEntry:
    path: Path
    branch: str | None
    is_locked: bool
    is_prunable: bool

    @property
    def name(self) -> str:
        return self.path.name

    @property
    def status(self) -> str:
        if self.is_locked:
            return "locked"
        if self.is_prunable:
            return "prunable"
        return "active"


__all__ = [
    "EnvConfig",
    "RepoMetadata",
    "RepoContext",
    "Paths",
    "WorktreeEntry",
]
