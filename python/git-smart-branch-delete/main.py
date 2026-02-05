#!/usr/bin/env python3

from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
from typing import List, Optional, Sequence


DEFAULT_PROTECTED_BRANCHES = ("main", "master")


class UserFacingError(RuntimeError):
    pass


def run_command(
    command: Sequence[str],
    *,
    input_text: Optional[str] = None,
    check: bool = True,
) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        list(command),
        input=input_text,
        text=True,
        capture_output=True,
        check=check,
    )


def require_binary(name: str) -> None:
    if shutil.which(name) is None:
        raise UserFacingError(f"Required binary not found in PATH: {name}")


def git_stdout(args: Sequence[str]) -> str:
    try:
        result = run_command(["git", *args], check=True)
    except subprocess.CalledProcessError as exc:  # pragma: no cover
        stderr = (exc.stderr or "").strip()
        stdout = (exc.stdout or "").strip()
        details = stderr or stdout or "Unknown git error."
        raise UserFacingError(details) from exc
    return (result.stdout or "").rstrip("\n")


def ensure_git_repo() -> None:
    try:
        value = git_stdout(["rev-parse", "--is-inside-work-tree"]).strip()
    except UserFacingError as exc:
        raise UserFacingError("Not a git repository (or any of the parent directories).") from exc
    if value.lower() != "true":
        raise UserFacingError("Not a git repository (or any of the parent directories).")


def current_branch() -> Optional[str]:
    # Empty output when in detached HEAD state.
    value = git_stdout(["branch", "--show-current"]).strip()
    return value or None


def list_local_branches() -> List[str]:
    output = git_stdout(["for-each-ref", "refs/heads", "--format=%(refname:short)"])
    branches = [line.strip() for line in output.splitlines() if line.strip()]
    return branches


def select_branches(branches: Sequence[str]) -> List[str]:
    selection_input = "\n".join(branches)
    result = run_command(
        [
            "fzf",
            "--multi",
            "--prompt",
            "Delete branches> ",
            "--header",
            "TAB to toggle selection; ENTER to confirm; ESC to cancel.",
        ],
        input_text=selection_input,
        check=False,
    )

    if result.returncode != 0:
        if result.returncode == 130:
            # User cancelled via ESC / Ctrl+C.
            return []
        stderr = (result.stderr or "").strip()
        raise UserFacingError(stderr or "fzf selection failed.")

    return [line.strip() for line in (result.stdout or "").splitlines() if line.strip()]


def confirm_deletion(branches: Sequence[str]) -> bool:
    count = len(branches)
    plural = "branch" if count == 1 else "branches"
    prompt = f"Delete {count} {plural}? [Y/n] "
    try:
        answer = input(prompt)
    except EOFError:
        return False
    normalized = answer.strip().lower()
    if normalized == "":
        return True
    return normalized in {"y", "yes"}


def delete_branches(branches: Sequence[str], *, dry_run: bool, force: bool) -> None:
    if dry_run:
        for branch in branches:
            if force:
                print(f"DRY RUN: git branch --delete --force -- {branch}")
            else:
                print(f"DRY RUN: git branch --delete -- {branch}")
        return

    failures: List[str] = []
    for branch in branches:
        command = ["git", "branch", "--delete"]
        if force:
            command.append("--force")
        command.extend(["--", branch])
        result = run_command(command, check=False)
        if result.returncode == 0:
            stdout = (result.stdout or "").strip()
            if stdout:
                print(stdout)
            continue

        failures.append(branch)
        stderr = (result.stderr or "").strip()
        stdout = (result.stdout or "").strip()
        details = stderr or stdout or "Unknown git error."
        print(f"Failed to delete {branch}: {details}", file=sys.stderr)

    if failures:
        failed_csv = ", ".join(failures)
        raise UserFacingError(
            "Some branches could not be deleted. "
            "They may be unmerged, checked out in another worktree, or otherwise protected by git. "
            f"Failed: {failed_csv}"
        )


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description=(
            "Interactively delete local git branches using fzf. "
            "Uses 'git branch --delete' by default."
        )
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Print what would be deleted without deleting anything.",
    )
    parser.add_argument(
        "--force",
        "-f",
        action="store_true",
        help="Force delete branches (equivalent to git branch -D).",
    )
    parser.add_argument(
        "--yes",
        action="store_true",
        help="Skip the confirmation prompt after selection.",
    )
    parser.add_argument(
        "--protect",
        action="append",
        default=[],
        metavar="BRANCH",
        help=(
            "Protect a branch from deletion (repeatable). "
            f"Defaults include: {', '.join(DEFAULT_PROTECTED_BRANCHES)} (plus the current branch)."
        ),
    )
    return parser


def main(argv: Optional[Sequence[str]] = None) -> int:
    args = build_parser().parse_args(argv)

    try:
        require_binary("git")
        require_binary("fzf")
        ensure_git_repo()

        protected = set(DEFAULT_PROTECTED_BRANCHES)
        protected.update(args.protect)
        head = current_branch()
        if head:
            protected.add(head)

        candidates = [b for b in list_local_branches() if b not in protected]
        if not candidates:
            protected_list = ", ".join(sorted(protected))
            print(f"No deletable local branches found. Protected: {protected_list}")
            return 0

        selected = select_branches(candidates)
        if not selected:
            print("No branches selected.")
            return 0

        print("Selected branches:")
        for branch in selected:
            print(f"- {branch}")

        if not args.yes and not confirm_deletion(selected):
            print("Cancelled.")
            return 0

        delete_branches(selected, dry_run=args.dry_run, force=args.force)
        return 0
    except UserFacingError as exc:
        print(str(exc), file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
