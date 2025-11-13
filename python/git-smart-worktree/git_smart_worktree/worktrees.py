"""High-level orchestration for worktree operations."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Iterable

from . import git
from .exceptions import ValidationError
from .fs import ensure_directory, slugify_branch
from .models import RepoContext, RepoPaths, WorktreeEntry


@dataclass
class WorktreeService:
    context: RepoContext
    paths: RepoPaths

    def admin_exists(self) -> bool:
        return (self.paths.admin_repo / ".git").exists()

    def ensure_admin_repo(self) -> Path:
        admin_repo = self.paths.admin_repo
        if self.admin_exists():
            self._ensure_detached_head(admin_repo)
            return admin_repo
        ensure_directory(admin_repo.parent)
        git.clone_no_checkout(self.context.remote_url, admin_repo)
        self._ensure_detached_head(admin_repo)
        return admin_repo

    def list_worktrees(self, include_all: bool = False) -> list[WorktreeEntry]:
        entries: dict[Path, WorktreeEntry] = {}
        if self.admin_exists():
            for raw in git.worktree_list(self.paths.admin_repo):
                path = raw.get("path")
                if not path:
                    continue
                name = derive_worktree_name(path, self.paths.worktree_repo_root)
                status = "locked" if raw.get("locked") else "prunable" if raw.get("prunable") else "active"
                entry = WorktreeEntry(
                    path=path,
                    name=name,
                    branch=raw.get("branch"),
                    status=status,
                )
                entries[path] = entry
        if include_all:
            for path in iter_worktree_directories(self.paths.worktree_repo_root):
                if path in entries:
                    continue
                name = derive_worktree_name(path, self.paths.worktree_repo_root)
                entries[path] = WorktreeEntry(
                    path=path,
                    name=name,
                    branch=None,
                    status="unknown",
                )
        return sorted(entries.values(), key=lambda e: (e.name or "", e.branch or ""))

    def add_worktree(self, worktree_name: str, branch: str, start_point: str | None = None) -> Path:
        self.validate_worktree_name(worktree_name)
        target_dir = self._target_path(worktree_name)
        if target_dir.exists():
            raise ValidationError(f"Worktree path already exists: {target_dir}")
        admin_repo = self.ensure_admin_repo()
        ensure_directory(target_dir.parent)
        git.fetch(admin_repo)
        branch_usage = self.branches_in_use()
        if branch in branch_usage:
            locations = ", ".join(str(path) for path in branch_usage[branch])
            raise ValidationError(
                f"Branch '{branch}' is already attached to worktree(s): {locations}. "
                "Detach or remove the existing worktree before continuing."
            )
        branch_exists = git.branch_exists(admin_repo, branch)
        remote_exists = git.remote_branch_exists(admin_repo, branch)
        if branch_exists:
            git.worktree_add_existing(admin_repo, target_dir, branch)
        elif remote_exists:
            remote_ref = f"origin/{branch}"
            git.worktree_add_from_remote(admin_repo, target_dir, branch, remote_ref)
        else:
            start = start_point or self.context.default_branch
            git.worktree_add_new(admin_repo, target_dir, branch, start)
        return target_dir

    def remove_worktree(self, path: Path, *, force: bool = False) -> None:
        admin_repo = self.ensure_admin_repo()
        git.worktree_remove(admin_repo, path, force=force)

    def branch_status(self, branch: str) -> tuple[bool, bool]:
        repo_path = self.paths.admin_repo if self.admin_exists() else self.context.repo_path
        local_exists = git.branch_exists(repo_path, branch)
        remote_exists = git.remote_branch_exists(repo_path, branch)
        return local_exists, remote_exists

    def branch_suggestions(self) -> list[str]:
        repo_path = self.paths.admin_repo if self.admin_exists() else self.context.repo_path
        return git.list_branches(repo_path, include_remote=True)

    def branches_in_use(self) -> dict[str, list[Path]]:
        usage: dict[str, list[Path]] = {}
        if not self.admin_exists():
            return usage
        for raw in git.worktree_list(self.paths.admin_repo):
            branch = raw.get("branch")
            path = raw.get("path")
            if branch and path:
                usage.setdefault(branch, []).append(path)
        return usage

    def _target_path(self, worktree_name: str) -> Path:
        slug = slugify_branch(worktree_name)
        return self.paths.worktree_repo_root / slug

    @staticmethod
    def validate_worktree_name(value: str) -> None:
        if not value.strip():
            raise ValidationError("Worktree name cannot be empty.")
        if "/" in value or "\0" in value:
            raise ValidationError("Worktree name cannot contain '/' or null characters.")

    def _ensure_detached_head(self, repo: Path) -> None:
        git.ensure_detached_head(repo)


def derive_worktree_name(path: Path, repo_root: Path) -> str | None:
    try:
        relative = path.relative_to(repo_root)
    except ValueError:
        return None
    parts = relative.parts
    if not parts:
        return None
    return parts[0]


def iter_worktree_directories(root: Path) -> Iterable[Path]:
    if not root.exists():
        return
    for candidate in root.iterdir():
        if candidate.is_dir():
            yield candidate
