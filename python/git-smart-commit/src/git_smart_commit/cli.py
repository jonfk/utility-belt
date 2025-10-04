"""CLI interface using Typer."""
import sys
from typing import Optional, Annotated
import typer

from . import config
from .app import run


app = typer.Typer(
    help="AI-powered git commit message generator",
    add_completion=False,
)


def parse_args_with_separator(args: list[str]) -> tuple[list[str], list[str]]:
    """Split arguments on '--' separator.

    Returns:
        Tuple of (args_before, args_after)
    """
    if "--" in args:
        sep_idx = args.index("--")
        return args[:sep_idx], args[sep_idx + 1 :]
    return args, []


@app.command()
def main(
    model: Annotated[
        Optional[str],
        typer.Option("-m", "--model", help="LLM model to use")
    ] = None,
    dry_run: Annotated[
        bool,
        typer.Option("--dry-run", help="Show plan without making changes")
    ] = False,
    verbose: Annotated[
        bool,
        typer.Option("--verbose", "-v", help="Show verbose output including raw JSON")
    ] = False,
    ctx: typer.Context = typer.Context,
) -> None:
    """Generate intelligent git commit messages using AI.

    Additional text before -- is used as extra prompt context.
    Arguments after -- are passed to the llm command.

    Examples:

        git-smart-commit

        git-smart-commit -m claude-3-5-sonnet-20240307

        git-smart-commit focus on security fixes -- --temperature 0.7
    """
    # Parse additional prompt and llm flags
    # Typer doesn't support -- separator natively, so we parse sys.argv directly
    our_args, llm_flags = parse_args_with_separator(sys.argv[1:])

    # Extract additional prompt from positional arguments
    # These are args that aren't flags
    additional_prompt_parts = []
    skip_next = False

    for i, arg in enumerate(our_args):
        if skip_next:
            skip_next = False
            continue

        # Skip known flags and their values
        if arg in {"-m", "--model"}:
            skip_next = True
            continue
        if arg in {"--dry-run", "--verbose", "-v"}:
            continue

        # This is a positional arg (part of additional prompt)
        additional_prompt_parts.append(arg)

    additional_prompt = " ".join(additional_prompt_parts)

    # Get model from args, env, or default
    model_to_use = model or config.get_default_model()

    # Merge default llm flags with user-provided ones
    default_llm_flags = config.get_default_llm_flags()
    all_llm_flags = default_llm_flags + llm_flags

    # Run the application
    run(
        model=model_to_use,
        extra_prompt=additional_prompt,
        llm_flags=all_llm_flags,
        dry_run=dry_run,
        verbose=verbose,
    )


if __name__ == "__main__":
    app()
