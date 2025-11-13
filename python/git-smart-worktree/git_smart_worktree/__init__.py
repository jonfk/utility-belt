"""Top-level package for git-smart-worktree."""

from importlib import metadata


try:  # pragma: no cover - best effort metadata lookup
    __version__ = metadata.version("git-smart-worktree")
except metadata.PackageNotFoundError:  # pragma: no cover
    __version__ = "0.0.0"

__all__ = ["__version__"]
