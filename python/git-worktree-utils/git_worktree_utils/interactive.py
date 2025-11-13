"""Interactive prompt helpers built on InquirerPy with graceful fallbacks."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Iterable, Sequence

from .exceptions import UserAbort

try:
    from InquirerPy import inquirer

    _HAS_INQUIRER = True
except ImportError:  # pragma: no cover - fallback path
    inquirer = None
    _HAS_INQUIRER = False


@dataclass(slots=True)
class BranchChoice:
    label: str
    value: str
    is_default: bool = False
    start_ref: str | None = None


def _ensure_choices(choices: Sequence[str]) -> list[str]:
    if not choices:
        raise UserAbort("No options available for selection.")
    return list(choices)


def prompt_text(message: str, default: str | None = None) -> str:
    try:
        if _HAS_INQUIRER:
            return inquirer.text(message=message, default=default).execute()
        raw = input(f"{message} [{default or ''}]: ")
        return raw.strip() or (default or "")
    except KeyboardInterrupt as exc:  # pragma: no cover - user cancel
        raise UserAbort("User cancelled the prompt.") from exc


def prompt_list(message: str, choices: Sequence[str], default: str | None = None) -> str:
    _ensure_choices(choices)
    try:
        if _HAS_INQUIRER:
            return inquirer.select(
                message=message,
                choices=list(choices),
                default=default,
                qmark="›",
            ).execute()
        print(message)
        for idx, choice in enumerate(choices, start=1):
            prefix = "*" if choice == default else " "
            print(f" {idx:2d}. {prefix} {choice}")
        selection = input("Select option #: ").strip()
        selected_index = int(selection) - 1 if selection else 0
        return list(choices)[selected_index]
    except KeyboardInterrupt as exc:  # pragma: no cover - user cancel
        raise UserAbort("User cancelled the prompt.") from exc


def prompt_branch_selection(branches: Sequence[BranchChoice]) -> BranchChoice:
    labeled = [b.label for b in branches]
    default_label = next((b.label for b in branches if b.is_default), None)
    selected_label = prompt_list("Select branch", labeled, default_label)
    for branch in branches:
        if branch.label == selected_label:
            return branch
    raise UserAbort("Invalid branch selection.")


def prompt_start_point_selection(
    branches: Sequence[BranchChoice],
    manual_label: str = "Enter ref manually…",
) -> str | None:
    labels = [b.label for b in branches]
    default_label = next((b.label for b in branches if b.is_default), None)
    if not default_label:
        if labels:
            default_label = labels[0]
        else:
            default_label = manual_label
    selection = prompt_list("Start point", labels + [manual_label], default_label)
    if selection == manual_label:
        return None
    for branch in branches:
        if branch.label == selection:
            return branch.value
    raise UserAbort("Invalid start point selection.")


def prompt_branch_mode() -> str:
    options = [
        ("existing", "Use existing branch"),
        ("new", "Create new branch"),
    ]
    labels = [label for _, label in options]
    selected = prompt_list("Branch workflow", labels, default=labels[0])
    for value, label in options:
        if label == selected:
            return value
    raise UserAbort("Invalid branch workflow selection.")


def prompt_worktree_selection(options: Sequence[str]) -> str:
    return prompt_list("Select worktree", options)


__all__ = [
    "prompt_text",
    "prompt_list",
    "prompt_branch_selection",
    "prompt_start_point_selection",
    "prompt_worktree_selection",
    "prompt_branch_mode",
    "BranchChoice",
]
