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
