"""tmux subprocess integration."""

from __future__ import annotations

import shutil
import subprocess
from dataclasses import dataclass
from typing import Sequence

from .exceptions import TmuxError
from .models import LivePane, LiveState, LiveWindow

FIELD_SEP = "\x1f"
SESSION_FORMAT = "#{session_name}"
WINDOW_FORMAT = FIELD_SEP.join(
    (
        "#{session_name}",
        "#{window_index}",
        "#{window_name}",
        "#{window_layout}",
    )
)
PANE_FORMAT = FIELD_SEP.join(
    (
        "#{session_name}",
        "#{window_index}",
        "#{pane_index}",
        "#{pane_current_path}",
        "#{pane_current_command}",
        "#{pane_active}",
    )
)
ID_FORMAT = FIELD_SEP.join(("#{window_id}", "#{pane_id}"))


@dataclass(slots=True)
class CommandResult:
    stdout: str
    stderr: str
    returncode: int


class TmuxClient:
    def require_tmux(self) -> None:
        if shutil.which("tmux") is None:
            raise TmuxError("Required binary not found in PATH: tmux")

    def run(self, args: Sequence[str], *, check: bool = True) -> CommandResult:
        command = ["tmux", *args]
        try:
            result = subprocess.run(
                command,
                text=True,
                capture_output=True,
                check=False,
            )
        except OSError as exc:
            raise TmuxError(f"Failed to run tmux: {exc}") from exc
        if check and result.returncode != 0:
            details = (result.stderr or result.stdout or "tmux command failed").strip()
            raise TmuxError(details)
        return CommandResult(
            stdout=result.stdout or "",
            stderr=result.stderr or "",
            returncode=result.returncode,
        )

    def capture_live_state(self) -> LiveState:
        self.require_tmux()
        sessions = parse_sessions(
            self.run(["list-sessions", "-F", SESSION_FORMAT]).stdout
        )
        windows = parse_windows(
            self.run(["list-windows", "-a", "-F", WINDOW_FORMAT]).stdout
        )
        panes = parse_panes(
            self.run(["list-panes", "-a", "-F", PANE_FORMAT]).stdout
        )
        return LiveState(sessions=sessions, windows=windows, panes=panes)

    def has_session(self, name: str) -> bool:
        self.require_tmux()
        result = self.run(["has-session", "-t", name], check=False)
        return result.returncode == 0

    def kill_session(self, name: str) -> None:
        self.run(["kill-session", "-t", name])

    def new_session(self, session_name: str, window_name: str, cwd: str | None) -> tuple[str, str]:
        args = ["new-session", "-d", "-P", "-F", ID_FORMAT, "-s", session_name, "-n", window_name]
        if cwd:
            args.extend(["-c", cwd])
        result = self.run(args)
        return parse_id_pair(result.stdout)

    def new_window(self, session_name: str, window_name: str, cwd: str | None) -> tuple[str, str]:
        args = ["new-window", "-P", "-F", ID_FORMAT, "-t", f"{session_name}:", "-n", window_name]
        if cwd:
            args.extend(["-c", cwd])
        result = self.run(args)
        return parse_id_pair(result.stdout)

    def split_window(self, target_window_id: str, orientation: str, cwd: str | None) -> str:
        if orientation == "horizontal":
            split_flag = "-h"
        elif orientation == "vertical":
            split_flag = "-v"
        else:  # pragma: no cover - validated before execution
            raise TmuxError(f"Unknown pane orientation: {orientation}")
        args = ["split-window", "-P", "-F", "#{pane_id}", "-t", target_window_id, split_flag]
        if cwd:
            args.extend(["-c", cwd])
        result = self.run(args)
        return result.stdout.strip()

    def send_prompt_text(self, pane_id: str, text: str) -> None:
        self.run(["send-keys", "-l", "-t", pane_id, text])

    def select_pane(self, pane_id: str) -> None:
        self.run(["select-pane", "-t", pane_id])

    def select_window(self, window_id: str) -> None:
        self.run(["select-window", "-t", window_id])

    def attach_or_switch(self, session_name: str, *, inside_tmux: bool) -> None:
        if inside_tmux:
            self.run(["switch-client", "-t", session_name])
        else:
            self.run(["attach-session", "-t", session_name])


def parse_sessions(output: str) -> list[str]:
    return [line for line in output.splitlines() if line]


def parse_windows(output: str) -> list[LiveWindow]:
    windows: list[LiveWindow] = []
    for line in output.splitlines():
        if not line:
            continue
        session_name, index, name, layout = _split_fields(line, 4, "window")
        windows.append(
            LiveWindow(
                session_name=session_name,
                index=_parse_int(index, "window index"),
                name=name,
                layout=layout,
            )
        )
    return windows


def parse_panes(output: str) -> list[LivePane]:
    panes: list[LivePane] = []
    for line in output.splitlines():
        if not line:
            continue
        session_name, window_index, pane_index, cwd, command, active = _split_fields(
            line,
            6,
            "pane",
        )
        panes.append(
            LivePane(
                session_name=session_name,
                window_index=_parse_int(window_index, "window index"),
                pane_index=_parse_int(pane_index, "pane index"),
                cwd=cwd,
                command=command,
                active=active == "1",
            )
        )
    return panes


def parse_id_pair(output: str) -> tuple[str, str]:
    line = output.strip().splitlines()[0] if output.strip() else ""
    window_id, pane_id = _split_fields(line, 2, "tmux id pair")
    return window_id, pane_id


def _split_fields(line: str, expected: int, label: str) -> list[str]:
    fields = line.split(FIELD_SEP)
    if len(fields) != expected:
        raise TmuxError(f"Unexpected {label} output from tmux: {line!r}")
    return fields


def _parse_int(value: str, label: str) -> int:
    try:
        return int(value)
    except ValueError as exc:
        raise TmuxError(f"Invalid {label} from tmux: {value!r}") from exc
