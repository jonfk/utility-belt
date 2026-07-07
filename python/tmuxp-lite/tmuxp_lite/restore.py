"""Restore saved tmux sessions."""

from __future__ import annotations

from dataclasses import dataclass, field

from .config import expand_runtime_path
from .exceptions import ConfigError
from .models import Config, PaneSpec, SessionSpec, WindowSpec
from .tmux import TmuxClient


@dataclass(slots=True)
class RestoreSummary:
    restored: list[str] = field(default_factory=list)
    skipped_existing: list[str] = field(default_factory=list)


def restore_sessions(
    config: Config,
    session_names: list[str],
    client: TmuxClient,
    *,
    kill_existing: bool = False,
    attach: bool = False,
    inside_tmux: bool = False,
) -> RestoreSummary:
    selected_names = session_names or list(config.sessions.keys())
    missing = [name for name in selected_names if name not in config.sessions]
    if missing:
        raise ConfigError(f"Unknown saved session(s): {', '.join(missing)}")

    summary = RestoreSummary()
    attach_target: str | None = None
    for session_name in selected_names:
        session = config.sessions[session_name]
        if client.has_session(session_name):
            if not kill_existing:
                summary.skipped_existing.append(session_name)
                continue
            client.kill_session(session_name)
        _restore_session(session, client)
        summary.restored.append(session_name)
        if attach_target is None:
            attach_target = session_name

    if attach and attach_target:
        client.attach_or_switch(attach_target, inside_tmux=inside_tmux)
    return summary


def _restore_session(session: SessionSpec, client: TmuxClient) -> None:
    if not session.windows:
        raise ConfigError(f"Session has no windows: {session.name}")

    first_window = session.windows[0]
    first_window_id, first_pane_id = client.new_session(
        session.name,
        first_window.name,
        _cwd(first_window, session),
    )
    _restore_window_content(
        first_window,
        session,
        client,
        window_id=first_window_id,
        first_pane_id=first_pane_id,
    )

    for window in session.windows[1:]:
        window_id, pane_id = client.new_window(
            session.name,
            window.name,
            _cwd(window, session),
        )
        _restore_window_content(
            window,
            session,
            client,
            window_id=window_id,
            first_pane_id=pane_id,
        )

    client.select_window(first_window_id)


def _restore_window_content(
    window: WindowSpec,
    session: SessionSpec,
    client: TmuxClient,
    *,
    window_id: str,
    first_pane_id: str,
) -> None:
    pane_ids: list[str] = [first_pane_id]
    pane_specs: list[PaneSpec]
    if window.panes:
        pane_specs = window.panes.items
        for pane in pane_specs[1:]:
            pane_ids.append(
                client.split_window(
                    window_id,
                    window.panes.orientation,
                    expand_runtime_path(pane.cwd, fallback=_cwd(window, session)),
                )
            )
    else:
        pane_specs = [
            PaneSpec(
                cwd=window.cwd,
                command=window.command,
            )
        ]

    for pane_id, pane in zip(pane_ids, pane_specs):
        if pane.command:
            client.send_command(pane_id, pane.command)

    client.select_pane(first_pane_id)


def _cwd(window: WindowSpec, session: SessionSpec) -> str | None:
    return expand_runtime_path(window.cwd, fallback=session.root)
