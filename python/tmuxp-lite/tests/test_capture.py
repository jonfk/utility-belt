from tmuxp_lite.capture import config_from_live_state, infer_orientation
from tmuxp_lite.models import LivePane, LiveState, LiveWindow


def test_config_from_live_state_omits_shell_commands_and_preserves_window_order() -> None:
    state = LiveState(
        sessions=["alpha"],
        windows=[
            LiveWindow("alpha", 2, "server", "layout"),
            LiveWindow("alpha", 1, "editor", "layout"),
        ],
        panes=[
            LivePane("alpha", 1, 0, "/repo", "zsh", True),
            LivePane("alpha", 2, 0, "/repo/api", "node", True),
        ],
    )

    config = config_from_live_state(state)
    session = config.sessions["alpha"]

    assert session.root == "/repo"
    assert [window.name for window in session.windows] == ["editor", "server"]
    assert session.windows[0].command is None
    assert session.windows[1].command == "node"


def test_config_from_live_state_saves_simple_pane_group() -> None:
    state = LiveState(
        sessions=["alpha"],
        windows=[LiveWindow("alpha", 1, "ops", "abc,80x24,0,0{40x24,0,0,0,39x24,41,0,1}")],
        panes=[
            LivePane("alpha", 1, 0, "/repo", "zsh", True),
            LivePane("alpha", 1, 1, "/repo", "htop", False),
        ],
    )

    window = config_from_live_state(state).sessions["alpha"].windows[0]

    assert window.panes is not None
    assert window.panes.orientation == "horizontal"
    assert [pane.command for pane in window.panes.items] == [None, "htop"]


def test_infer_orientation_from_tmux_layout_container() -> None:
    assert infer_orientation("abc,80x24,0,0{40x24,0,0,0,39x24,41,0,1}") == "horizontal"
    assert infer_orientation("abc,80x24,0,0[80x12,0,0,0,80x11,0,13,1]") == "vertical"
