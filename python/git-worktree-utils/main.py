"""Entry point shim for `python -m git_worktree_utils`."""

from __future__ import annotations

from git_worktree_utils.cli import app


def main() -> None:
    app()


if __name__ == "__main__":
    main()
