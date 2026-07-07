from tmuxp_lite.tmux import FIELD_SEP, parse_panes, parse_windows


def test_parse_windows_reads_machine_formatted_tmux_output() -> None:
    output = FIELD_SEP.join(("alpha", "1", "editor", "abc,80x24,0,0")) + "\n"

    windows = parse_windows(output)

    assert len(windows) == 1
    assert windows[0].session_name == "alpha"
    assert windows[0].index == 1
    assert windows[0].name == "editor"
    assert windows[0].layout == "abc,80x24,0,0"


def test_parse_panes_reads_machine_formatted_tmux_output() -> None:
    output = FIELD_SEP.join(("alpha", "1", "0", "/tmp/project", "nvim", "1")) + "\n"

    panes = parse_panes(output)

    assert len(panes) == 1
    assert panes[0].session_name == "alpha"
    assert panes[0].window_index == 1
    assert panes[0].pane_index == 0
    assert panes[0].cwd == "/tmp/project"
    assert panes[0].command == "nvim"
    assert panes[0].active is True


def test_parse_windows_rejects_unexpected_fields() -> None:
    try:
        parse_windows("too few fields\n")
    except Exception as exc:
        assert "Unexpected window output" in str(exc)
    else:
        raise AssertionError("parse_windows should reject malformed output")
