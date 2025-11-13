"""Custom error hierarchy for git-worktree-utils."""

from __future__ import annotations

from dataclasses import dataclass


class GitWorktreeError(RuntimeError):
    """Base error for the CLI."""


class MissingEnvError(GitWorktreeError):
    """Raised when required environment variables are missing or invalid."""


class RepoDetectionError(GitWorktreeError):
    """Raised when we cannot resolve repository metadata."""


class GitCommandError(GitWorktreeError):
    """Raised when an underlying git command fails."""

    def __init__(self, command: list[str], returncode: int, stderr: str | None = None):
        self.command = command
        self.returncode = returncode
        self.stderr = stderr or ""
        message = f"git command failed (exit {returncode}): {' '.join(command)}"
        if self.stderr:
            message = f"{message}\n{self.stderr.strip()}"
        super().__init__(message)


class ValidationError(GitWorktreeError):
    """Raised when user input fails validation."""


class UserAbort(GitWorktreeError):
    """Raised when the user cancels an interactive flow."""


@dataclass(slots=True)
class FriendlyMessage:
    """Reusable structure for presenting actionable errors."""

    heading: str
    details: str


__all__ = [
    "GitWorktreeError",
    "MissingEnvError",
    "RepoDetectionError",
    "GitCommandError",
    "ValidationError",
    "UserAbort",
    "FriendlyMessage",
]
