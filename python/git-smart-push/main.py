#!/usr/bin/env python3

from __future__ import annotations

import re
import shutil
import subprocess
import sys
from typing import Optional, Sequence

PULL_NEW_URL_PATTERN = re.compile(r"https?://[^\s'\"<>]*?/pull/new[^\s'\"<>]*")


def run_git_push(push_args: Sequence[str]) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", "push", *push_args],
        text=True,
        capture_output=True,
        check=False,
    )


def echo_git_output(result: subprocess.CompletedProcess[str]) -> str:
    stdout = result.stdout or ""
    stderr = result.stderr or ""

    if stdout:
        print(stdout, end="")
    if stderr:
        print(stderr, end="", file=sys.stderr)

    return "\n".join(part for part in (stdout, stderr) if part)


def find_pull_new_url(command_output: str) -> Optional[str]:
    match = PULL_NEW_URL_PATTERN.search(command_output)
    if not match:
        return None

    # Some terminals may include trailing punctuation around detected URLs.
    return match.group(0).rstrip(".,;:)]}>'\"")


def confirm_open_url(url: str) -> bool:
    print(f"Detected pull request URL: {url}")
    try:
        answer = input("Open this URL in browser? [Y/n] ")
    except EOFError:
        return True
    except KeyboardInterrupt:
        print("", file=sys.stderr)
        return False

    normalized = answer.strip().lower()
    return normalized in {"", "y", "yes"}


def open_url(url: str) -> None:
    if shutil.which("open") is None:
        print("Cannot open URL automatically because 'open' was not found in PATH.", file=sys.stderr)
        return

    result = subprocess.run(["open", url], text=True, capture_output=True, check=False)
    if result.returncode == 0:
        return

    details = (result.stderr or result.stdout or "").strip()
    if not details:
        details = f"open exited with code {result.returncode}"
    print(f"Failed to open URL: {details}", file=sys.stderr)


def main(argv: Optional[Sequence[str]] = None) -> int:
    push_args = list(sys.argv[1:] if argv is None else argv)

    try:
        push_result = run_git_push(push_args)
    except FileNotFoundError:
        print("git command not found in PATH.", file=sys.stderr)
        return 1

    command_output = echo_git_output(push_result)
    pull_new_url = find_pull_new_url(command_output)

    if pull_new_url and confirm_open_url(pull_new_url):
        open_url(pull_new_url)

    return push_result.returncode


if __name__ == "__main__":
    raise SystemExit(main())
