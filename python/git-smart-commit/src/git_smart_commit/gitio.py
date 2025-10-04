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


def commit_with_editor(template: str) -> None:
    """Commit using git's editor with a template file."""
    _run_git("commit", "--edit", f"--template={template}", capture_output=False)


def commit_with_file(file_path: str) -> None:
    """Commit using a message from a file."""
    _run_git("commit", "-F", file_path, capture_output=False)


def commit_single_line(message: str, edit: bool = False) -> None:
    """Create a single-line commit.

    If edit=True, opens the editor with the message pre-filled.
    Otherwise, commits directly with -m.
    """
    if edit:
        # Create temp file and use --edit --template
        temp_path = write_temp_commit(message, [])
        try:
            commit_with_editor(temp_path)
        finally:
            Path(temp_path).unlink(missing_ok=True)
    else:
        _run_git("commit", "-m", message, capture_output=False)
