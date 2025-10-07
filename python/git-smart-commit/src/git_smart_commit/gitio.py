"""Git subprocess wrappers."""
import subprocess
import tempfile
from pathlib import Path


class GitError(Exception):
    """Raised when a git command fails."""
    pass


def _run_git(*args: str, check: bool = True, capture_output: bool = True) -> subprocess.CompletedProcess:
    """Run a git command and return the result."""
    try:
        result = subprocess.run(
            ["git", *args],
            capture_output=capture_output,
            text=True,
            check=check,
        )
        return result
    except subprocess.CalledProcessError as e:
        raise GitError(f"Git command failed: {e.stderr}") from e


def ensure_repo() -> None:
    """Ensure we're inside a git repository."""
    result = _run_git("rev-parse", "--is-inside-work-tree", check=False)
    if result.returncode != 0:
        raise GitError("Not in a git repository")


def has_staged_changes() -> bool:
    """Check if there are staged changes."""
    result = _run_git("diff", "--staged", "--quiet", "--exit-code", check=False)
    return result.returncode != 0


def has_unstaged_changes() -> bool:
    """Check if there are unstaged changes."""
    result = _run_git("diff", "--quiet", "--exit-code", check=False)
    return result.returncode != 0


def log_oneline(n: int = 5) -> str:
    """Get recent commit history."""
    result = _run_git("log", "--oneline", f"-{n}")
    return result.stdout.strip()


def diff_staged() -> str:
    """Get the staged diff."""
    result = _run_git("diff", "--staged")
    return result.stdout


def diff_unstaged() -> str:
    """Get the unstaged diff."""
    result = _run_git("diff")
    return result.stdout


def get_changed_files() -> list[str]:
    """Get list of changed (unstaged) files."""
    result = _run_git("diff", "--name-only")
    files = result.stdout.strip().split("\n")
    return [f for f in files if f]


def stage(files: list[str]) -> None:
    """Stage the given files.

    Only stages files that actually exist in the changed files list.
    """
    if not files:
        raise GitError("No files to stage")

    # Verify files exist in changed files
    changed_files = set(get_changed_files())
    valid_files = [f for f in files if f in changed_files]

    if not valid_files:
        raise GitError("No valid files to stage (files not found in changes)")

    invalid_files = set(files) - set(valid_files)
    if invalid_files:
        # Warning will be handled by caller
        pass

    _run_git("add", *valid_files)


def write_temp_commit(message: str, body: list[str]) -> str:
    """Write a temporary commit message file and return the path."""
    fd, path = tempfile.mkstemp(suffix=".txt", prefix="git-commit-")
    try:
        with open(fd, "w") as f:
            f.write(message + "\n")
            if body:
                f.write("\n")
                for line in body:
                    f.write(line + "\n")
        return path
    except Exception:
        Path(path).unlink(missing_ok=True)
        raise


def log_with_stats(n: int = 20) -> str:
    """Get recent commit history with file statistics.

    Returns output similar to: git log --stat -n <n>
    """
    result = _run_git("log", "--stat", f"-{n}")
    return result.stdout.strip()


def show_commit(commit_id: str) -> str:
    """Get full commit diff for a specific commit.

    Args:
        commit_id: The commit hash or reference

    Returns:
        Full commit diff output
    """
    result = _run_git("show", commit_id)
    return result.stdout


def get_file_at_commit(commit_id: str, file_path: str) -> str:
    """Get the contents of a file at a specific commit.

    Args:
        commit_id: The commit hash or reference
        file_path: Path to the file in the repository

    Returns:
        File contents at that commit

    Raises:
        GitError: If the file doesn't exist at that commit
    """
    result = _run_git("show", f"{commit_id}:{file_path}")
    return result.stdout


def get_file_size(file_path: str) -> int:
    """Get the size of a file in bytes.

    Args:
        file_path: Path to the file

    Returns:
        File size in bytes, or 0 if file doesn't exist
    """
    try:
        return Path(file_path).stat().st_size
    except (FileNotFoundError, OSError):
        return 0


def commit(message: str, body: list[str] | None = None, edit: bool = False) -> None:
    """Create a commit with the given message and optional body.

    Args:
        message: The commit message (first line)
        body: Optional additional lines for the commit body
        edit: If True, opens the editor for editing the message

    The message and body are written to a temporary file, then committed.
    If edit=True, the editor is opened. Otherwise, the commit is created directly.
    """
    body = body or []
    temp_path = write_temp_commit(message, body)
    try:
        if edit:
            _run_git("commit", "--edit", f"--template={temp_path}", capture_output=False)
        else:
            _run_git("commit", "-F", temp_path, capture_output=False)
    finally:
        Path(temp_path).unlink(missing_ok=True)
