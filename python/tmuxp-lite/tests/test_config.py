from pathlib import Path

from tmuxp_lite.config import load_config, save_config
from tmuxp_lite.models import Config, SessionSpec, WindowSpec


def test_load_config_preserves_session_and_window_order(tmp_path: Path) -> None:
    config_path = tmp_path / "sessions.yaml"
    config_path.write_text(
        """
version: 1
sessions:
  alpha:
    root: ~/alpha
    windows:
      - name: editor
        cwd: ~/alpha
      - name: server
        cwd: ~/alpha/server
        command: npm run dev
  beta:
    windows:
      - name: shell
""",
        encoding="utf-8",
    )

    config = load_config(config_path)

    assert list(config.sessions) == ["alpha", "beta"]
    assert [window.name for window in config.sessions["alpha"].windows] == [
        "editor",
        "server",
    ]


def test_save_config_omits_empty_optional_fields(tmp_path: Path) -> None:
    config_path = tmp_path / "sessions.yaml"
    config = Config(
        sessions={
            "alpha": SessionSpec(
                name="alpha",
                root="~/alpha",
                windows=[WindowSpec(name="shell", cwd="~/alpha")],
            )
        }
    )

    save_config(config, config_path)

    assert config_path.read_text(encoding="utf-8") == (
        "version: 1\n"
        "sessions:\n"
        "  alpha:\n"
        "    root: ~/alpha\n"
        "    windows:\n"
        "      - name: shell\n"
        "        cwd: ~/alpha\n"
    )
