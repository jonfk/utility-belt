from tmuxp_lite.models import Config, PaneGroupSpec, PaneSpec, SessionSpec, WindowSpec
from tmuxp_lite.restore import restore_sessions


class RecordingTmuxClient:
    def __init__(self) -> None:
        self.commands: list[list[str | None]] = []
        self.window_counter = 0
        self.pane_counter = 0

    def has_session(self, name: str) -> bool:
        self.commands.append(["has-session", name])
        return False

    def kill_session(self, name: str) -> None:
        self.commands.append(["kill-session", name])

    def new_session(self, session_name: str, window_name: str, cwd: str | None) -> tuple[str, str]:
        self.commands.append(["new-session", session_name, window_name, cwd])
        return self._next_window(), self._next_pane()

    def new_window(self, session_name: str, window_name: str, cwd: str | None) -> tuple[str, str]:
        self.commands.append(["new-window", session_name, window_name, cwd])
        return self._next_window(), self._next_pane()

    def split_window(self, target_window_id: str, orientation: str, cwd: str | None) -> str:
        pane_id = self._next_pane()
        self.commands.append(["split-window", target_window_id, orientation, cwd, pane_id])
        return pane_id

    def send_prompt_text(self, pane_id: str, text: str) -> None:
        self.commands.append(["send-prompt-text", pane_id, text])

    def select_pane(self, pane_id: str) -> None:
        self.commands.append(["select-pane", pane_id])

    def select_window(self, window_id: str) -> None:
        self.commands.append(["select-window", window_id])

    def attach_or_switch(self, session_name: str, *, inside_tmux: bool) -> None:
        self.commands.append(["attach-or-switch", session_name, str(inside_tmux)])

    def _next_window(self) -> str:
        self.window_counter += 1
        return f"@{self.window_counter}"

    def _next_pane(self) -> str:
        self.pane_counter += 1
        return f"%{self.pane_counter}"


def test_restore_plans_ordered_windows_and_prompt_hints() -> None:
    config = Config(
        sessions={
            "alpha": SessionSpec(
                name="alpha",
                root="/repo",
                windows=[
                    WindowSpec(name="editor", cwd="/repo", command="nvim"),
                    WindowSpec(
                        name="ops",
                        cwd="/repo",
                        panes=PaneGroupSpec(
                            orientation="vertical",
                            items=[
                                PaneSpec(cwd="/repo", command=None),
                                PaneSpec(cwd="/repo", command="htop"),
                            ],
                        ),
                    ),
                ],
            )
        }
    )
    client = RecordingTmuxClient()

    summary = restore_sessions(config, [], client)

    assert summary.restored == ["alpha"]
    assert [command[0] for command in client.commands[:3]] == [
        "has-session",
        "new-session",
        "send-prompt-text",
    ]
    assert ["new-window", "alpha", "ops", "/repo"] in client.commands
    assert any(command[:4] == ["split-window", "@2", "vertical", "/repo"] for command in client.commands)
    assert ["send-prompt-text", "%3", "htop"] in client.commands


def test_restore_selected_unknown_session_fails() -> None:
    config = Config(sessions={})
    client = RecordingTmuxClient()

    try:
        restore_sessions(config, ["missing"], client)
    except Exception as exc:
        assert "Unknown saved session" in str(exc)
    else:
        raise AssertionError("restore should fail for unknown saved sessions")
