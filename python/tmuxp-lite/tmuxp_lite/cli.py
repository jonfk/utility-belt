"""Command line interface for tmuxp-lite."""

from __future__ import annotations

import argparse
import os
import shlex
import subprocess
import sys
from pathlib import Path
from typing import Sequence

from .capture import config_from_live_state
from .config import ensure_editable_config, expand_config_path, load_config, save_config
from .exceptions import ConfigError, TmuxpLiteError
from .restore import restore_sessions
from .tmux import TmuxClient


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="tmuxp-lite",
        description="Save and restore lightweight tmux sessions from one YAML file.",
    )
    subparsers = parser.add_subparsers(dest="command", required=True)
    common = argparse.ArgumentParser(add_help=False)
    common.add_argument(
        "--config",
        type=Path,
        help="Path to the YAML config file.",
    )

    subparsers.add_parser(
        "capture",
        parents=[common],
        help="Replace the config file with the current tmux server state.",
    )
    subparsers.add_parser(
        "list",
        parents=[common],
        help="List saved session names.",
    )
    subparsers.add_parser(
        "edit",
        parents=[common],
        help="Open the config file in $EDITOR.",
    )

    restore_parser = subparsers.add_parser(
        "restore",
        parents=[common],
        help="Restore all saved sessions or selected sessions.",
    )
    restore_parser.add_argument(
        "sessions",
        nargs="*",
        help="Saved session names to restore. Defaults to all saved sessions.",
    )
    restore_parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Print tmux commands without executing them.",
    )
    restore_parser.add_argument(
        "--kill-existing",
        action="store_true",
        help="Kill and recreate live sessions with matching names.",
    )
    restore_parser.add_argument(
        "--attach",
        action="store_true",
        help="Attach or switch to the first restored session.",
    )

    return parser


def main(argv: Sequence[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    try:
        config_path = expand_config_path(args.config)
        if args.command == "capture":
            return _capture(config_path)
        if args.command == "list":
            return _list(config_path)
        if args.command == "edit":
            return _edit(config_path)
        if args.command == "restore":
            return _restore(args, config_path)
    except TmuxpLiteError as exc:
        print(f"tmuxp-lite: {exc}", file=sys.stderr)
        return 1
    return 1


def _capture(config_path: Path) -> int:
    client = TmuxClient()
    state = client.capture_live_state()
    config = config_from_live_state(state)
    save_config(config, config_path)
    count = len(config.sessions)
    plural = "session" if count == 1 else "sessions"
    print(f"Captured {count} {plural} to {config_path}")
    return 0


def _list(config_path: Path) -> int:
    config = load_config(config_path)
    if not config.sessions:
        print("No saved sessions.")
        return 0
    for session_name in config.sessions:
        print(session_name)
    return 0


def _edit(config_path: Path) -> int:
    ensure_editable_config(config_path)
    editor = os.environ.get("EDITOR")
    if not editor:
        raise ConfigError("$EDITOR is not set")
    command = [*shlex.split(editor), str(config_path)]
    try:
        return subprocess.run(command, check=False).returncode
    except OSError as exc:
        raise ConfigError(f"Failed to run editor: {exc}") from exc


def _restore(args: argparse.Namespace, config_path: Path) -> int:
    config = load_config(config_path)
    client = TmuxClient(dry_run=args.dry_run)
    summary = restore_sessions(
        config,
        list(args.sessions),
        client,
        kill_existing=args.kill_existing,
        attach=args.attach,
        inside_tmux=bool(os.environ.get("TMUX")),
    )
    if args.dry_run:
        if client.dry_run_commands:
            for command in client.dry_run_commands:
                print(shlex.join(command))
        else:
            print("No tmux commands would be run.")
        return 0

    for session_name in summary.skipped_existing:
        print(f"Skipped existing session: {session_name}")
    for session_name in summary.restored:
        print(f"Restored session: {session_name}")
    if not summary.skipped_existing and not summary.restored:
        print("No sessions restored.")
    return 0
