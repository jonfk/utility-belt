"""Minimal utilities for invoking git commands."""

from __future__ import annotations

import subprocess
from pathlib import Path
from typing import Iterable, Sequence

from .exceptions import GitCommandError


def run_git(
    args: Sequence[str],
    *,
    cwd: Path | None = None,
    env: dict[str, str] | None = None,
    check: bool = True,
) -> subprocess.CompletedProcess[str]:
    command = ["git", *args]
    result = subprocess.run(
        command,
        cwd=str(cwd) if cwd else None,
        env=env,
        text=True,
        capture_output=True,
    )
    if check and result.returncode != 0:
        raise GitCommandError(command, result.returncode, result.stderr)
    return result


def ensure_dirs(paths: Iterable[Path]) -> None:
    for path in paths:
        path.mkdir(parents=True, exist_ok=True)


__all__ = ["run_git", "ensure_dirs"]
