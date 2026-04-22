#!/usr/bin/env python3

# See https://developers.openai.com/codex/config-advanced#notifications

import json
import os
import subprocess
import sys
from pathlib import Path


INSTALL_SOUND_DIR = Path.home() / ".local" / "share" / "codex-notify"
REPO_SOUND_DIR = Path(__file__).resolve().with_name("codex-notify")
APP_SOUND_NAME = "scarlett-her-notify.wav"
CLI_SOUND_NAME = "aoe-wololo-notify.mp3"
FALLBACK_SOUND_NAME = "hal-9000-cant-do-that.wav"


def resolve_sound_dir() -> Path:
    configured_dir = os.environ.get("CODEX_NOTIFY_SOUND_DIR")
    if configured_dir:
        return Path(configured_dir).expanduser()

    if INSTALL_SOUND_DIR.exists():
        return INSTALL_SOUND_DIR

    return REPO_SOUND_DIR


def read_notification() -> dict:
    if len(sys.argv) < 2:
        return {}

    try:
        payload = json.loads(sys.argv[1])
    except json.JSONDecodeError:
        return {}

    return payload if isinstance(payload, dict) else {}


def read_parent_command() -> str:
    try:
        result = subprocess.run(
            ["ps", "-p", str(os.getppid()), "-o", "command="],
            check=True,
            capture_output=True,
            text=True,
        )
    except (OSError, subprocess.SubprocessError):
        return ""

    return result.stdout.strip()


def select_sound(parent_command: str) -> Path:
    normalized = parent_command.lower()
    sound_dir = resolve_sound_dir()

    if "/applications/codex.app/" in normalized:
        return sound_dir / APP_SOUND_NAME

    cli_markers = (
        "/node_modules/@openai/codex/",
        "/@openai/codex-",
        "/vendor/aarch64-apple-darwin/codex/codex",
        "/vendor/x86_64-apple-darwin/codex/codex",
        "/bin/codex",
    )
    if any(marker in normalized for marker in cli_markers):
        return sound_dir / CLI_SOUND_NAME

    return sound_dir / FALLBACK_SOUND_NAME


def play_sound(sound_path: Path) -> int:
    if not sound_path.exists():
        return 1

    try:
        subprocess.Popen(
            ["afplay", str(sound_path)],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            start_new_session=True,
        )
    except OSError:
        return 1

    return 0


def main() -> int:
    notification = read_notification()
    if notification.get("type") != "agent-turn-complete":
        return 0

    parent_command = read_parent_command()
    sound_path = select_sound(parent_command)
    return play_sound(sound_path)


if __name__ == "__main__":
    raise SystemExit(main())
