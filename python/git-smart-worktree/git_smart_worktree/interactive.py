"""Interactive prompt helpers built on InquirerPy."""

from __future__ import annotations

import sys
from typing import Any, Iterable, Sequence

from InquirerPy import inquirer
from InquirerPy.base.control import Choice

from .exceptions import ValidationError


def _ensure_tty() -> None:
    if not sys.stdin.isatty():
        raise ValidationError(
            "Interactive mode requires a TTY. Provide the missing arguments to run non-interactively."
        )


def fuzzy_select(message: str, choices: Sequence[Choice | str]) -> Any:
    _ensure_tty()
    return inquirer.fuzzy(message=message, choices=choices).execute()


def text_input(message: str, default: str | None = None) -> str:
    _ensure_tty()
    return inquirer.text(message=message, default=default).execute().strip()


def confirm(message: str, default: bool = True) -> bool:
    _ensure_tty()
    return bool(inquirer.confirm(message=message, default=default).execute())


def build_choices(options: Iterable[str], *, highlight: Sequence[str] | None = None) -> list[Choice]:
    """Return Choice objects with highlighted defaults placed first."""

    highlight = highlight or []
    result: list[Choice] = []
    seen: set[str] = set()
    for item in list(highlight) + list(options):
        if not item or item in seen:
            continue
        seen.add(item)
        result.append(Choice(value=item, name=item))
    return result
