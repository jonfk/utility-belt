"""Custom exception hierarchy for git-smart-worktree."""


class WorktreeError(Exception):
    """Base error for all custom exceptions."""


class MissingEnvError(WorktreeError):
    """Raised when the required environment variables are absent."""


class GitCommandError(WorktreeError):
    """Raised when a git invocation fails."""

    def __init__(self, command: list[str], returncode: int, stderr: str | None = None):
        message = "Git command failed"
        if command:
            message = f"Git command failed: {' '.join(command)}"
        super().__init__(message)
        self.command = command
        self.returncode = returncode
        self.stderr = stderr or ""


class ValidationError(WorktreeError):
    """Raised when user input is invalid."""


class UserAbort(WorktreeError):
    """Raised when the user cancels an interactive flow."""
