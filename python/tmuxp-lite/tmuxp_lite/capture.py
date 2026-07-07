"""Build saved config from live tmux state."""

from __future__ import annotations

from collections import defaultdict

from .models import (
    Config,
    LivePane,
    LiveState,
    LiveWindow,
    PaneGroupSpec,
    PaneSpec,
    SessionSpec,
    WindowSpec,
)

SHELL_COMMANDS = {
    "ash",
    "bash",
    "csh",
    "dash",
    "fish",
    "ksh",
    "sh",
    "tcsh",
    "zsh",
}


def config_from_live_state(state: LiveState) -> Config:
    windows_by_session: dict[str, list[LiveWindow]] = defaultdict(list)
    panes_by_window: dict[tuple[str, int], list[LivePane]] = defaultdict(list)

    for window in state.windows:
        windows_by_session[window.session_name].append(window)
    for pane in state.panes:
        panes_by_window[(pane.session_name, pane.window_index)].append(pane)

    sessions: dict[str, SessionSpec] = {}
    for session_name in state.sessions:
        live_windows = sorted(windows_by_session.get(session_name, []), key=lambda item: item.index)
        if not live_windows:
            continue
        saved_windows: list[WindowSpec] = []
        for live_window in live_windows:
            panes = sorted(
                panes_by_window.get((session_name, live_window.index), []),
                key=lambda item: item.pane_index,
            )
            if not panes:
                continue
            saved_windows.append(_window_from_live(live_window, panes))
        if not saved_windows:
            continue
        sessions[session_name] = SessionSpec(
            name=session_name,
            root=saved_windows[0].cwd,
            windows=saved_windows,
        )
    return Config(sessions=sessions)


def _window_from_live(window: LiveWindow, panes: list[LivePane]) -> WindowSpec:
    first_pane = panes[0]
    if len(panes) == 1:
        return WindowSpec(
            name=window.name,
            cwd=first_pane.cwd,
            command=_saved_command(first_pane.command),
        )
    return WindowSpec(
        name=window.name,
        cwd=first_pane.cwd,
        panes=PaneGroupSpec(
            orientation=infer_orientation(window.layout),
            items=[
                PaneSpec(cwd=pane.cwd, command=_saved_command(pane.command))
                for pane in panes
            ],
        ),
    )


def infer_orientation(layout: str) -> str:
    for char in layout:
        if char == "{":
            return "horizontal"
        if char == "[":
            return "vertical"
    return "horizontal"


def _saved_command(command: str) -> str | None:
    if not command or command in SHELL_COMMANDS:
        return None
    return command
