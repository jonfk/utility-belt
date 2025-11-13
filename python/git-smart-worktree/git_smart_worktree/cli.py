"""Typer-based CLI for git-smart-worktree."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import typer
from rich.console import Console
from rich.table import Table

from .config import load_runtime
from .exceptions import GitCommandError, MissingEnvError, ValidationError, WorktreeError
from .interactive import Choice, build_choices, fuzzy_select, text_input
from .models import WorktreeEntry
from .worktrees import DEFAULT_CONTEXTS, WorktreeService

app = typer.Typer(help="Manage git worktrees with opinionated layouts")
console = Console()


@app.callback()
def main(
    ctx: typer.Context,
    repo: Path | None = typer.Option(
        None,
        "--repo",
        help="Path to a git repository whose worktrees should be managed.",
        exists=False,
        dir_okay=True,
        file_okay=False,
    ),
    verbose: bool = typer.Option(False, "--verbose", help="Show additional debug information."),
) -> None:
    ctx.ensure_object(dict)
    ctx.obj["repo_override"] = repo
    ctx.obj["verbose"] = verbose


@app.command(help="List worktrees for the resolved repository")
def ls(
    ctx: typer.Context,
    show_all: bool = typer.Option(False, "--all", help="Include filesystem directories not tracked by git."),
    context_filter: str | None = typer.Option(None, "--context", help="Filter results by context folder."),
    as_json: bool = typer.Option(False, "--json", help="Output JSON for scripting."),
) -> None:
    repo_ctx, service = _build_service(ctx)
    entries = service.list_worktrees(include_all=show_all)
    if context_filter:
        entries = [entry for entry in entries if entry.context == context_filter]
    if as_json:
        data = [
            {
                "context": entry.context,
                "branch": entry.branch,
                "path": str(entry.path),
                "status": entry.status,
            }
            for entry in entries
        ]
        typer.echo(json.dumps(data, indent=2))
        return
    if not entries:
        console.print("No worktrees found.")
        return
    table = Table(show_header=True, header_style="bold")
    table.add_column("Context")
    table.add_column("Branch")
    table.add_column("Status")
    table.add_column("Path")
    for entry in entries:
        table.add_row(entry.context or "?", entry.branch or "(detached)", entry.status, str(entry.path))
    console.print(table)


@app.command(help="Add a new worktree")
def add(
    ctx: typer.Context,
    context_name: str | None = typer.Argument(None, help="Context folder (feature/review/etc)."),
    branch: str | None = typer.Argument(None, help="Branch to checkout or create."),
    from_ref: str | None = typer.Option(
        None,
        "--from",
        help="Starting point for new branches (commit, tag, or reference).",
    ),
) -> None:
    repo_ctx, service = _build_service(ctx)
    context_name = context_name or _prompt_context(service)
    branch = branch or _prompt_branch(service, repo_ctx.default_branch)
    local_exists, remote_exists = service.branch_status(branch)
    start_point = from_ref
    if not local_exists and not remote_exists and not start_point:
        start_point = _prompt_start_point(service, repo_ctx.default_branch)
    target = service.add_worktree(context_name, branch, start_point=start_point)
    console.print(f"Created worktree at {target}")


@app.command(help="Remove an existing worktree")
def rm(
    ctx: typer.Context,
    path: Path | None = typer.Argument(
        None,
        help="Path to the worktree. If omitted, an interactive picker is shown.",
    ),
    force: bool = typer.Option(False, "--force", help="Force removal even if the worktree is dirty."),
) -> None:
    repo_ctx, service = _build_service(ctx)
    admin_repo_path = service.paths.admin_repo
    target_path = path
    if target_path is None:
        entries = service.list_worktrees()
        entries = _filter_removable_worktrees(entries, admin_repo_path)
        if not entries:
            raise ValidationError("No removable worktrees available.")
        target_entry = _prompt_worktree(entries)
        target_path = target_entry.path
    if Path(target_path).expanduser() == admin_repo_path:
        raise ValidationError("Cannot remove the admin repository worktree.")
    target_path = target_path.expanduser()
    service.remove_worktree(target_path, force=force)
    console.print(f"Removed worktree at {target_path}")


def _build_service(ctx: typer.Context) -> tuple[Any, WorktreeService]:
    try:
        repo_ctx, repo_paths = load_runtime(ctx.obj.get("repo_override"))
        service = WorktreeService(repo_ctx, repo_paths)
        return repo_ctx, service
    except WorktreeError as err:
        _fail(str(err))


def _prompt_context(service: WorktreeService) -> str:
    contexts = service.list_contexts()
    choices = [Choice(value=ctx, name=ctx) for ctx in contexts]
    create_choice = Choice(value="__new__", name="Create new context…")
    selection = fuzzy_select("Select context", choices + [create_choice])
    if selection == "__new__":
        return text_input("Context name")
    return str(selection)


def _prompt_branch(service: WorktreeService, default_branch: str) -> str:
    branches = service.branch_suggestions()
    branch_usage = service.branches_in_use()
    available = [branch for branch in branches if branch not in branch_usage]
    highlight = [default_branch] if default_branch in available else []
    choices = build_choices(available, highlight=highlight)
    if not choices:
        console.print("No available branches detected. Create a new branch or provide one manually.")
    choices.append(Choice(value="__new__", name="Create new branch…"))
    if branch_usage:
        console.print("")
        console.print("Branches already in use (not selectable):")
        for branch in sorted(branch_usage):
            locations = ", ".join(str(path) for path in branch_usage[branch])
            console.print(f"  • {branch}: {locations}")
        console.print("")
    selection = fuzzy_select("Select branch", choices)
    if selection == "__new__":
        return text_input("Branch name")
    return str(selection)


def _prompt_start_point(service: WorktreeService, default_branch: str) -> str:
    branches = service.branch_suggestions()
    choices = build_choices(branches, highlight=[default_branch])
    choices.append(Choice(value="__custom__", name="Custom reference…"))
    selection = fuzzy_select("Starting point", choices)
    if selection == "__custom__":
        return text_input("Enter commit, tag, or ref")
    return str(selection)


def _prompt_worktree(entries: list[WorktreeEntry]) -> WorktreeEntry:
    choices, lookup = _build_worktree_choice_data(entries)
    selection = fuzzy_select("Select worktree", choices)
    try:
        return lookup[str(selection)]
    except KeyError as exc:
        raise ValidationError("Selected worktree could not be resolved.") from exc


def _build_worktree_choice_data(entries: list[WorktreeEntry]) -> tuple[list[Choice], dict[str, WorktreeEntry]]:
    """Return the choice list used for prompts plus a lookup keyed by path."""

    lookup: dict[str, WorktreeEntry] = {}
    path_choices: list[Choice] = []
    for entry in entries:
        key = str(entry.path)
        if key in lookup:
            raise ValidationError(f"Duplicate worktree path detected: {key}")
        lookup[key] = entry
        path_choices.append(Choice(value=key, name=f"{entry.display_name} · {entry.path}"))
    return path_choices, lookup


def _filter_removable_worktrees(entries: list[WorktreeEntry], admin_repo: Path) -> list[WorktreeEntry]:
    """Exclude admin repository entries from the removable list."""

    admin_repo = admin_repo.expanduser()
    return [entry for entry in entries if Path(entry.path).expanduser() != admin_repo]


def _fail(message: str, code: int = 1) -> None:
    typer.secho(message, err=True, fg=typer.colors.RED)
    raise typer.Exit(code)


if __name__ == "__main__":
    app()
