from tmuxp_lite.models import Config, PaneGroupSpec, PaneSpec, SessionSpec, WindowSpec
from tmuxp_lite.restore import restore_sessions
from tmuxp_lite.tmux import TmuxClient


def test_restore_dry_run_plans_ordered_windows_and_panes() -> None:
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
    client = TmuxClient(dry_run=True)

    summary = restore_sessions(config, [], client)

    assert summary.restored == ["alpha"]
    assert [command[:2] for command in client.dry_run_commands[:3]] == [
        ["tmux", "has-session"],
        ["tmux", "new-session"],
        ["tmux", "send-keys"],
    ]
    assert any(command[1] == "new-window" and "ops" in command for command in client.dry_run_commands)
    assert any(command[1] == "split-window" and "-v" in command for command in client.dry_run_commands)
    assert any(command[1] == "send-keys" and "htop" in command for command in client.dry_run_commands)


def test_restore_selected_unknown_session_fails() -> None:
    config = Config(sessions={})
    client = TmuxClient(dry_run=True)

    try:
        restore_sessions(config, ["missing"], client)
    except Exception as exc:
        assert "Unknown saved session" in str(exc)
    else:
        raise AssertionError("restore should fail for unknown saved sessions")
