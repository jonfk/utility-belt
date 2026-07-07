"""User-facing exceptions for tmuxp-lite."""

from __future__ import annotations


class TmuxpLiteError(RuntimeError):
    """Base exception for expected command failures."""


class ConfigError(TmuxpLiteError):
    """Raised when the YAML config is missing or invalid."""


class TmuxError(TmuxpLiteError):
    """Raised when tmux cannot complete an operation."""
