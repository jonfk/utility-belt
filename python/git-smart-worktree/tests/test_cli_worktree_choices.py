"""Tests for the worktree choice helper used by the CLI."""

from __future__ import annotations

import unittest
from pathlib import Path

from git_smart_worktree.cli import _build_worktree_choice_data
from git_smart_worktree.exceptions import ValidationError
from git_smart_worktree.models import WorktreeEntry


class BuildWorktreeChoiceDataTests(unittest.TestCase):
    def setUp(self) -> None:
        self.entries = [
            WorktreeEntry(path=Path("/tmp/worktrees/experiment/foo"), branch="foo", context="experiment", status="active"),
            WorktreeEntry(path=Path("/tmp/worktrees/feature/bar"), branch="bar", context="feature", status="locked"),
        ]

    def test_returns_lookup_and_choices(self) -> None:
        choices, lookup = _build_worktree_choice_data(self.entries)

        self.assertEqual(len(choices), len(self.entries))
        self.assertEqual(set(lookup.keys()), {str(entry.path) for entry in self.entries})

        first_choice_value = choices[0].value
        self.assertIn(first_choice_value, lookup)
        self.assertIs(lookup[first_choice_value], self.entries[0])

    def test_duplicate_paths_raise_validation_error(self) -> None:
        duplicate_entries = self.entries + [
            WorktreeEntry(path=self.entries[0].path, branch="other", context="experiment", status="active")
        ]

        with self.assertRaises(ValidationError):
            _build_worktree_choice_data(duplicate_entries)


if __name__ == "__main__":
    unittest.main()
