"""High-level orchestration for worktree operations."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Iterable

from . import git
from .exceptions import ValidationError
from .fs import ensure_directory, slugify_branch
from .models import RepoContext, RepoPaths, WorktreeEntry

DEFAULT_CONTEXTS = ["main", "feature", "review", "release", "hotfix", "experiment"]


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
                context_name, _ = derive_context(path, self.paths.worktree_repo_root)
                status = "locked" if raw.get("locked") else "prunable" if raw.get("prunable") else "active"
                entry = WorktreeEntry(
                    path=path,
                    branch=raw.get("branch"),
                    context=context_name,
                    status=status,
                )
                entries[path] = entry
        if include_all:
            for path in iter_worktree_directories(self.paths.worktree_repo_root):
                if path in entries:
                    continue
                context_name, branch_slug = derive_context(path, self.paths.worktree_repo_root)
                entries[path] = WorktreeEntry(
                    path=path,
                    branch=branch_slug,
                    context=context_name,
                    status="unknown",
                )
        return sorted(entries.values(), key=lambda e: (e.context or "", e.branch or ""))

    def list_contexts(self) -> list[str]:
        contexts = set(DEFAULT_CONTEXTS)
        root = self.paths.worktree_repo_root
        if root.exists():
            for child in root.iterdir():
                if child.is_dir():
                    contexts.add(child.name)
        for entry in self.list_worktrees():
            if entry.context:
                contexts.add(entry.context)
        return sorted(contexts)

    def add_worktree(self, context_name: str, branch: str, start_point: str | None = None) -> Path:
        self._validate_context_name(context_name)
        target_dir = self._target_path(context_name, branch)
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

    def _target_path(self, context_name: str, branch: str) -> Path:
        slug = slugify_branch(branch)
        return self.paths.worktree_repo_root / context_name / slug

    @staticmethod
    def _validate_context_name(value: str) -> None:
        if not value.strip():
            raise ValidationError("Context name cannot be empty.")
        if "/" in value or "\0" in value:
            raise ValidationError("Context name cannot contain '/' or null characters.")

    def _ensure_detached_head(self, repo: Path) -> None:
        git.ensure_detached_head(repo)


def derive_context(path: Path, repo_root: Path) -> tuple[str | None, str | None]:
    try:
        relative = path.relative_to(repo_root)
    except ValueError:
        return None, None
    parts = relative.parts
    if not parts:
        return None, None
    context = parts[0]
    branch = parts[1] if len(parts) > 1 else None
    return context, branch


def iter_worktree_directories(root: Path) -> Iterable[Path]:
    if not root.exists():
        return
    for context_dir in root.iterdir():
        if not context_dir.is_dir():
            continue
        for branch_dir in context_dir.iterdir():
            if branch_dir.is_dir():
                yield branch_dir
