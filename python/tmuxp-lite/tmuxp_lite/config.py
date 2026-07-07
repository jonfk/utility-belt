"""YAML config loading and saving."""

from __future__ import annotations

import os
from pathlib import Path

import yaml

from .exceptions import ConfigError
from .models import Config


class _IndentedDumper(yaml.SafeDumper):
    def increase_indent(self, flow: bool = False, indentless: bool = False) -> object:
        return super().increase_indent(flow, False)


def default_config_path() -> Path:
    config_home = os.environ.get("XDG_CONFIG_HOME")
    if config_home:
        return Path(config_home).expanduser() / "tmuxp-lite" / "sessions.yaml"
    return Path.home() / ".config" / "tmuxp-lite" / "sessions.yaml"


def expand_config_path(path: Path | None) -> Path:
    return (path or default_config_path()).expanduser()


def expand_runtime_path(value: str | None, *, fallback: str | None = None) -> str | None:
    candidate = value or fallback
    if not candidate:
        return None
    return os.path.expandvars(os.path.expanduser(candidate))


def load_config(path: Path) -> Config:
    if not path.exists():
        raise ConfigError(f"Config file not found: {path}")
    if not path.is_file():
        raise ConfigError(f"Config path is not a file: {path}")
    try:
        with path.open("r", encoding="utf-8") as fh:
            data = yaml.safe_load(fh)
    except yaml.YAMLError as exc:
        raise ConfigError(f"Failed to parse YAML config: {exc}") from exc
    except OSError as exc:
        raise ConfigError(f"Failed to read config: {exc}") from exc

    if data is None:
        data = {"version": 1, "sessions": {}}
    try:
        return Config.from_dict(data)
    except ValueError as exc:
        raise ConfigError(str(exc)) from exc


def save_config(config: Config, path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    try:
        with path.open("w", encoding="utf-8") as fh:
            yaml.dump(
                config.to_dict(),
                fh,
                Dumper=_IndentedDumper,
                sort_keys=False,
                default_flow_style=False,
            )
    except OSError as exc:
        raise ConfigError(f"Failed to write config: {exc}") from exc


def ensure_editable_config(path: Path) -> None:
    if path.exists():
        if not path.is_file():
            raise ConfigError(f"Config path is not a file: {path}")
        return
    save_config(Config(), path)
