"""Domain models for tmuxp-lite."""

from __future__ import annotations

from dataclasses import dataclass, field


@dataclass(slots=True)
class PaneSpec:
    cwd: str
    command: str | None = None

    @classmethod
    def from_dict(cls, data: object, *, path: str) -> "PaneSpec":
        if not isinstance(data, dict):
            raise ValueError(f"{path} must be a mapping")
        _reject_unknown(data, {"cwd", "command"}, path)
        return cls(
            cwd=_required_str(data.get("cwd"), f"{path}.cwd"),
            command=_optional_str(data.get("command"), f"{path}.command"),
        )

    def to_dict(self) -> dict[str, str]:
        data: dict[str, str] = {}
        if self.cwd:
            data["cwd"] = self.cwd
        if self.command:
            data["command"] = self.command
        return data


@dataclass(slots=True)
class PaneGroupSpec:
    orientation: str
    items: list[PaneSpec]

    @classmethod
    def from_dict(cls, data: object, *, path: str) -> "PaneGroupSpec":
        if not isinstance(data, dict):
            raise ValueError(f"{path} must be a mapping")
        _reject_unknown(data, {"orientation", "items"}, path)
        orientation = _required_str(data.get("orientation"), f"{path}.orientation")
        if orientation not in {"horizontal", "vertical"}:
            raise ValueError(f"{path}.orientation must be 'horizontal' or 'vertical'")
        raw_items = data.get("items")
        if not isinstance(raw_items, list) or not raw_items:
            raise ValueError(f"{path}.items must be a non-empty list")
        items = [
            PaneSpec.from_dict(item, path=f"{path}.items[{index}]")
            for index, item in enumerate(raw_items)
        ]
        return cls(orientation=orientation, items=items)

    def to_dict(self) -> dict[str, object]:
        return {
            "orientation": self.orientation,
            "items": [item.to_dict() for item in self.items],
        }


@dataclass(slots=True)
class WindowSpec:
    name: str
    cwd: str | None = None
    command: str | None = None
    panes: PaneGroupSpec | None = None

    @classmethod
    def from_dict(cls, data: object, *, path: str) -> "WindowSpec":
        if not isinstance(data, dict):
            raise ValueError(f"{path} must be a mapping")
        _reject_unknown(data, {"name", "cwd", "command", "panes"}, path)
        panes = None
        if "panes" in data:
            panes = PaneGroupSpec.from_dict(data["panes"], path=f"{path}.panes")
        if panes and data.get("command") is not None:
            raise ValueError(f"{path} cannot set both command and panes")
        if panes and data.get("cwd") is not None:
            raise ValueError(f"{path} cannot set both cwd and panes")
        return cls(
            name=_required_str(data.get("name"), f"{path}.name"),
            cwd=_optional_str(data.get("cwd"), f"{path}.cwd"),
            command=_optional_str(data.get("command"), f"{path}.command"),
            panes=panes,
        )

    def to_dict(self) -> dict[str, object]:
        data: dict[str, object] = {"name": self.name}
        if self.cwd:
            data["cwd"] = self.cwd
        if self.command:
            data["command"] = self.command
        if self.panes:
            data["panes"] = self.panes.to_dict()
        return data


@dataclass(slots=True)
class SessionSpec:
    name: str
    root: str | None = None
    windows: list[WindowSpec] = field(default_factory=list)

    @classmethod
    def from_dict(cls, name: str, data: object, *, path: str) -> "SessionSpec":
        if not isinstance(data, dict):
            raise ValueError(f"{path} must be a mapping")
        _reject_unknown(data, {"root", "windows"}, path)
        raw_windows = data.get("windows")
        if not isinstance(raw_windows, list) or not raw_windows:
            raise ValueError(f"{path}.windows must be a non-empty list")
        windows = [
            WindowSpec.from_dict(item, path=f"{path}.windows[{index}]")
            for index, item in enumerate(raw_windows)
        ]
        return cls(
            name=name,
            root=_optional_str(data.get("root"), f"{path}.root"),
            windows=windows,
        )

    def to_dict(self) -> dict[str, object]:
        data: dict[str, object] = {}
        if self.root:
            data["root"] = self.root
        data["windows"] = [window.to_dict() for window in self.windows]
        return data


@dataclass(slots=True)
class Config:
    sessions: dict[str, SessionSpec] = field(default_factory=dict)
    version: int = 1

    @classmethod
    def from_dict(cls, data: object) -> "Config":
        if not isinstance(data, dict):
            raise ValueError("config must be a mapping")
        _reject_unknown(data, {"version", "sessions"}, "config")
        version = data.get("version")
        if version != 1:
            raise ValueError("config.version must be 1")
        raw_sessions = data.get("sessions")
        if raw_sessions is None:
            raw_sessions = {}
        if not isinstance(raw_sessions, dict):
            raise ValueError("config.sessions must be a mapping")
        sessions: dict[str, SessionSpec] = {}
        for name, session_data in raw_sessions.items():
            if not isinstance(name, str) or not name:
                raise ValueError("session names must be non-empty strings")
            sessions[name] = SessionSpec.from_dict(
                name,
                session_data,
                path=f"config.sessions.{name}",
            )
        return cls(sessions=sessions, version=1)

    def to_dict(self) -> dict[str, object]:
        return {
            "version": self.version,
            "sessions": {
                name: session.to_dict()
                for name, session in self.sessions.items()
            },
        }


@dataclass(slots=True)
class LiveWindow:
    session_name: str
    index: int
    name: str
    layout: str


@dataclass(slots=True)
class LivePane:
    session_name: str
    window_index: int
    pane_index: int
    cwd: str
    command: str
    active: bool


@dataclass(slots=True)
class LiveState:
    sessions: list[str]
    windows: list[LiveWindow]
    panes: list[LivePane]


def _required_str(value: object, path: str) -> str:
    if not isinstance(value, str) or not value:
        raise ValueError(f"{path} must be a non-empty string")
    return value


def _optional_str(value: object, path: str) -> str | None:
    if value is None:
        return None
    if not isinstance(value, str):
        raise ValueError(f"{path} must be a string")
    return value or None


def _reject_unknown(data: dict[object, object], allowed: set[str], path: str) -> None:
    unknown = sorted((key for key in data if key not in allowed), key=str)
    if unknown:
        joined = ", ".join(str(key) for key in unknown)
        raise ValueError(f"{path} has unknown field(s): {joined}")
