"""Typer CLI entrypoint for git-worktree-utils."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Optional

import typer
from rich.console import Console

from .config import (
    build_paths,
    build_repo_context,
    load_env_config,
)
from .exceptions import GitWorktreeError, UserAbort, ValidationError
from .interactive import (
    BranchChoice,
    prompt_branch_selection,
    prompt_text,
    prompt_worktree_selection,
)
from .models import EnvConfig, Paths, RepoContext
from .worktrees import (
    add_worktree,
    ensure_admin_ready,
    list_worktrees,
    load_branch_summary,
    remove_worktree,
    render_worktrees_json,
    render_worktrees_table,
)
from ._version import __version__

app = typer.Typer(add_completion=False, no_args_is_help=True, rich_markup_mode="rich")


@dataclass(slots=True)
class AppState:
    env: EnvConfig
    repo_ctx: RepoContext
    paths: Paths
    console: Console
    verbose: bool = False


def _version_callback(value: bool) -> None:
    if value:
        typer.echo(f"git-worktree-utils {__version__}")
        raise typer.Exit()


@app.callback()
def main(
    ctx: typer.Context,
    repo: Optional[Path] = typer.Option(
        None,
        "--repo",
        "-r",
        help="Path to the repository to operate on (defaults to current working directory).",
        dir_okay=True,
        file_okay=False,
    ),
    verbose: bool = typer.Option(False, "--verbose", "-v", help="Enable verbose logging."),
    version: bool = typer.Option(
        False,
        "--version",
        callback=_version_callback,
        is_eager=True,
        help="Show the git-worktree-utils version and exit.",
    ),
) -> None:
    _ = version  # handled via callback
    console = Console()
    try:
        env = load_env_config()
        repo_ctx = build_repo_context(repo)
        paths = build_paths(env, repo_ctx.metadata)
    except GitWorktreeError as exc:
        console.print(f"[red]Error:[/red] {exc}")
        raise typer.Exit(1) from exc
    ctx.obj = AppState(env=env, repo_ctx=repo_ctx, paths=paths, console=console, verbose=verbose)


def _require_state(ctx: typer.Context) -> AppState:
    state = ctx.obj
    if not isinstance(state, AppState):  # pragma: no cover
        raise typer.Exit(1)
    return state


@app.command()
def ls(
    ctx: typer.Context,
    json_: bool = typer.Option(False, "--json", help="Output JSON instead of a table."),
) -> None:
    state = _require_state(ctx)
    ensure_admin_ready(state.paths, state.repo_ctx, state.console)
    entries = list_worktrees(state.paths)
    if not entries:
        state.console.print("No worktrees registered for this repository.")
        return
    if json_:
        render_worktrees_json(entries, state.console)
    else:
        render_worktrees_table(entries, state.console)


@app.command()
def add(
    ctx: typer.Context,
    worktree_name: Optional[str] = typer.Argument(None, help="Name for the worktree directory."),
    branch: Optional[str] = typer.Argument(None, help="Branch to checkout or create."),
    from_ref: Optional[str] = typer.Option(
        None,
        "--from",
        "--from-ref",
        help="Starting point when creating a new branch (defaults to repo default branch).",
    ),
    track: Optional[str] = typer.Option(
        None,
        "--track",
        help="Set upstream for the branch (e.g., origin/main).",
    ),
) -> None:
    state = _require_state(ctx)
    ensure_admin_ready(state.paths, state.repo_ctx, state.console)

    try:
        resolved_name = worktree_name or _prompt_worktree_name(state)
        resolved_branch, resolved_from, resolved_track = _resolve_branch_inputs(
            state,
            branch=branch,
            start_point=from_ref,
            track=track,
        )
    except (ValidationError, UserAbort) as exc:
        state.console.print(f"[yellow]{exc}[/yellow]")
        raise typer.Exit(1) from exc

    target = add_worktree(
        state.paths,
        state.repo_ctx,
        worktree_name=resolved_name,
        branch=resolved_branch,
        start_point=resolved_from,
        track=resolved_track,
        console=state.console,
    )
    state.console.print(f"[green]Worktree created at {target}[/green]")


@app.command()
def rm(
    ctx: typer.Context,
    path: Optional[Path] = typer.Argument(None, help="Path to the worktree to remove."),
    force: bool = typer.Option(False, "--force", "-f", help="Force removal even if worktree is dirty."),
    select: bool = typer.Option(False, "--select", help="Open an interactive selector to choose the worktree."),
) -> None:
    state = _require_state(ctx)
    ensure_admin_ready(state.paths, state.repo_ctx, state.console)
    target_path: Path | None = None

    if select or path is None:
        entries = list_worktrees(state.paths)
        if not entries:
            state.console.print("No worktrees found to delete.")
            raise typer.Exit(0)
        mapping = {f"{entry.name} ({entry.branch or 'detached'}) · {entry.path}": entry.path for entry in entries}
        try:
            selection = prompt_worktree_selection(list(mapping.keys()))
        except UserAbort as exc:
            state.console.print("Selection cancelled.")
            raise typer.Exit(1) from exc
        target_path = mapping[selection]
    else:
        target_path = path.expanduser().resolve()

    remove_worktree(
        state.paths,
        state.repo_ctx,
        target_path,
        force=force,
        console=state.console,
    )
    state.console.print(f"[green]Removed worktree {target_path}[/green]")


def _prompt_worktree_name(state: AppState) -> str:
    name = prompt_text("Worktree name", default=state.repo_ctx.default_branch)
    name = name.strip()
    if not name:
        raise ValidationError("Worktree name cannot be empty.")
    return name


def _resolve_branch_inputs(
    state: AppState,
    *,
    branch: Optional[str],
    start_point: Optional[str],
    track: Optional[str],
) -> tuple[str, Optional[str], Optional[str]]:
    if branch:
        return branch, start_point, track

    summary = load_branch_summary(state.paths)
    choices = _build_branch_choices(state, summary)
    selection = prompt_branch_selection(choices)
    if selection.value == "__create__":
        new_branch = prompt_text("New branch name", default=state.repo_ctx.default_branch).strip()
        if not new_branch:
            raise ValidationError("Branch name cannot be empty.")
        start = start_point or prompt_text(
            "Start point (branch, tag, or commit)",
            default=state.repo_ctx.default_branch,
        ).strip()
        if not start:
            raise ValidationError("Start point cannot be empty.")
        return new_branch, start, track

    resolved_start = start_point or selection.start_ref
    resolved_track = track or (selection.start_ref if selection.start_ref and selection.start_ref.startswith("origin/") else None)
    return selection.value, resolved_start, resolved_track


def _build_branch_choices(state: AppState, summary: dict[str, list[str]]) -> list[BranchChoice]:
    choices: list[BranchChoice] = []
    local_branches = summary.get("local", [])
    remote_branches = summary.get("remote", [])
    if state.repo_ctx.default_branch in local_branches:
        choices.append(
            BranchChoice(
                label=f"{state.repo_ctx.default_branch} · local (default)",
                value=state.repo_ctx.default_branch,
                is_default=True,
            )
        )
    for branch in local_branches:
        if branch == state.repo_ctx.default_branch:
            continue
        choices.append(BranchChoice(label=f"{branch} · local", value=branch))
    for remote in remote_branches:
        branch_name = remote.split("origin/")[-1] if remote.startswith("origin/") else remote
        choices.append(
            BranchChoice(
                label=f"{remote} · remote",
                value=branch_name,
                start_ref=remote,
            )
        )
    choices.append(BranchChoice(label="Create new branch…", value="__create__"))
    if choices and not any(choice.is_default for choice in choices):
        choices[0].is_default = True
    return choices


__all__ = ["app"]
