"""Rich UI helpers for terminal output."""
import json
from typing import Optional
from rich.console import Console
from rich.panel import Panel
from rich.table import Table
from rich.markdown import Markdown
from rich import print as rprint

from .models import Plan


console = Console()


def info(message: str) -> None:
    """Print an info message."""
    console.print(f"[blue]ℹ[/blue] {message}")


def success(message: str) -> None:
    """Print a success message."""
    console.print(f"[green]✓[/green] {message}")


def warning(message: str) -> None:
    """Print a warning message."""
    console.print(f"[yellow]⚠[/yellow] {message}")


def error(message: str) -> None:
    """Print an error message."""
    console.print(f"[red]✗[/red] {message}", style="red")


def show_model(model: str) -> None:
    """Show which model is being used."""
    info(f"Using model: [bold]{model}[/bold]")


def show_json_response(data: dict, verbose: bool = False) -> None:
    """Show the JSON response from the LLM."""
    if verbose:
        console.print("\n[bold]Generated response:[/bold]")
        console.print(json.dumps(data, indent=2))
        console.print()


def preview_plan(plan: Plan, changed_files: Optional[list[str]] = None) -> None:
    """Preview the staging and commit plan.

    Args:
        plan: The execution plan
        changed_files: List of actually changed files (for validation)
    """
    console.print()

    # Show files to be staged if any
    if plan.steps:
        for step in plan.steps:
            table = Table(title="Files to Stage", show_header=True, header_style="bold cyan")
            table.add_column("File", style="cyan")
            table.add_column("Status", justify="center")

            changed_set = set(changed_files or [])
            for file in step.files:
                if file in changed_set:
                    table.add_row(file, "[green]✓[/green]")
                else:
                    table.add_row(file, "[yellow]⚠ not in changes[/yellow]")

            console.print(table)
            console.print()

    # Show commit message preview
    commit_preview = f"[bold green]{plan.commit.message}[/bold green]"

    if plan.commit.body:
        commit_preview += "\n\n"
        commit_preview += "\n".join(plan.commit.body)

    panel = Panel(
        commit_preview,
        title="Commit Message Preview",
        border_style="green",
        padding=(1, 2),
    )
    console.print(panel)
    console.print()

    # Show warnings
    msg_len = len(plan.commit.message)
    if msg_len > 72:
        warning(f"Commit message is {msg_len} characters (recommended: 50-72)")
    elif msg_len > 50:
        info(f"Commit message is {msg_len} characters (ideal: ≤50)")


def show_analyzing(is_staged: bool) -> None:
    """Show that we're analyzing changes."""
    change_type = "staged" if is_staged else "unstaged"
    info(f"Generating commit message for {change_type} changes...")


def show_commit_preview(message: str, body: list[str]) -> None:
    """Show commit message preview in a simple format."""
    console.print("\n[bold]Commit message preview:[/bold]")
    console.print("----------------------")
    console.print(message)
    if body:
        console.print()
        for line in body:
            console.print(line)
    console.print("----------------------\n")


def show_step(step: int, total: int, description: str) -> None:
    """Show progress step in extended context mode.

    Args:
        step: Current step number
        total: Total number of steps
        description: Description of what this step does
    """
    console.print(f"[bold cyan]Step {step}/{total}:[/bold cyan] {description}")


def show_extended_context(selection: dict) -> None:
    """Display what extended context was selected.

    Args:
        selection: ContextSelectionResponse dict
    """
    console.print("\n[bold]Selected Context:[/bold]")

    reasoning = selection.get("reasoning", "")
    if reasoning:
        console.print(f"[dim]Reasoning: {reasoning}[/dim]\n")

    relevant_files = selection.get("relevant_files", [])
    if relevant_files:
        console.print("[cyan]Current files for context:[/cyan]")
        for file in relevant_files:
            console.print(f"  • {file}")

    relevant_commits = selection.get("relevant_commits", [])
    if relevant_commits:
        console.print("\n[cyan]Historical commits:[/cyan]")
        for commit in relevant_commits:
            console.print(f"  • {commit[:12]}")

    commit_files = selection.get("commit_files", [])
    if commit_files:
        console.print("\n[cyan]Specific files from commits:[/cyan]")
        for cf in commit_files:
            commit = cf.get("commit", "")[:12]
            file = cf.get("file", "")
            console.print(f"  • {file} @ {commit}")

    if not (relevant_files or relevant_commits or commit_files):
        console.print("[dim]No additional context selected[/dim]")

    console.print()


def show_context_size(size_bytes: int, max_bytes: int = 50 * 1024) -> None:
    """Display context size and warn if it's large.

    Args:
        size_bytes: Size of the extended context in bytes
        max_bytes: Maximum recommended size (default 50KB)
    """
    size_kb = size_bytes / 1024

    if size_bytes > max_bytes:
        warning(f"Extended context is large: {size_kb:.1f}KB (recommended: <{max_bytes/1024:.0f}KB)")
    else:
        info(f"Extended context size: {size_kb:.1f}KB")
