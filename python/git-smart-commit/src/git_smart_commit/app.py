"""Main application orchestration."""
from pathlib import Path
from typing import Optional

from . import gitio, render
from .models import (
    STAGED_SCHEMA,
    UNSTAGED_SCHEMA,
    Commit,
    Plan,
    StageStep,
    parse_staged,
    parse_unstaged,
)
from .prompt import Context, build_prompt, read_project_guidelines
from .llm.llm_cli import LlmCliClient


def run(
    model: str,
    extra_prompt: str = "",
    llm_flags: Optional[list[str]] = None,
    edit: bool = True,
    dry_run: bool = False,
    verbose: bool = False,
) -> None:
    """Run the git-smart-commit application.

    Args:
        model: LLM model to use
        extra_prompt: Additional user-provided prompt context
        llm_flags: Extra flags to pass to the llm command
        edit: Whether to open editor for commit message
        dry_run: If True, show plan but don't execute
        verbose: Show verbose output including raw JSON
    """
    llm_flags = llm_flags or []

    # Check requirements
    try:
        gitio.ensure_repo()
    except gitio.GitError as e:
        render.error(str(e))
        raise SystemExit(1)

    # Initialize LLM client
    try:
        llm_client = LlmCliClient()
    except Exception as e:
        render.error(str(e))
        raise SystemExit(1)

    # Build context
    try:
        recent_commits = gitio.log_oneline(n=5)
    except gitio.GitError:
        recent_commits = ""

    project_guidelines = read_project_guidelines()

    context = Context(
        recent_commits=recent_commits,
        project_guidelines=project_guidelines,
    )

    # Determine what to do based on changes
    has_staged = gitio.has_staged_changes()
    has_unstaged = gitio.has_unstaged_changes()

    if not has_staged and not has_unstaged:
        render.info("No changes detected")
        return

    render.show_model(model)

    # Generate plan
    try:
        if has_staged:
            # Staged changes: just generate commit message
            render.show_analyzing(is_staged=True)
            diff = gitio.diff_staged()
            prompt = build_prompt(context, extra_prompt, is_staged=True)

            data = llm_client.generate(
                diff=diff,
                schema=STAGED_SCHEMA,
                prompt=prompt,
                model=model,
                extra_flags=llm_flags,
            )

            render.show_json_response(data, verbose=verbose)

            message, body = parse_staged(data)
            plan = Plan(steps=[], commit=Commit(message, body))

        else:
            # Unstaged changes: select files and generate commit message
            render.show_analyzing(is_staged=False)
            diff = gitio.diff_unstaged()
            prompt = build_prompt(context, extra_prompt, is_staged=False)

            data = llm_client.generate(
                diff=diff,
                schema=UNSTAGED_SCHEMA,
                prompt=prompt,
                model=model,
                extra_flags=llm_flags,
            )

            render.show_json_response(data, verbose=verbose)

            files, message, body = parse_unstaged(data)
            plan = Plan(
                steps=[StageStep(files)],
                commit=Commit(message, body),
            )

    except Exception as e:
        render.error(f"Failed to generate commit: {e}")
        raise SystemExit(1)

    # Preview the plan
    changed_files = gitio.get_changed_files() if plan.steps else None
    render.preview_plan(plan, changed_files)

    if dry_run:
        render.info("Dry run - no changes made")
        return

    # Execute the plan
    try:
        # Stage files if needed
        for step in plan.steps:
            changed_set = set(gitio.get_changed_files())
            valid_files = [f for f in step.files if f in changed_set]
            invalid_files = set(step.files) - set(valid_files)

            if invalid_files:
                for f in invalid_files:
                    render.warning(f"File '{f}' not found in changes or already staged")

            if not valid_files:
                render.error("No valid files to stage")
                raise SystemExit(1)

            gitio.stage(valid_files)
            render.success(f"Staged {len(valid_files)} file(s)")

        # Create commit
        if plan.commit.body:
            # Multi-line commit with editor or direct file
            temp_path = gitio.write_temp_commit(plan.commit.message, plan.commit.body)
            try:
                if edit:
                    gitio.commit_with_editor(temp_path)
                else:
                    gitio.commit_with_file(temp_path)
                render.success("Commit created successfully")
            except gitio.GitError as e:
                # If commit fails (e.g., user aborted in editor), don't delete temp file
                # Show them the command they can run manually
                render.error(f"Commit failed: {e}")
                render.info(f"You can commit manually with: git commit -F {temp_path}")
                raise SystemExit(1)
            finally:
                # Clean up temp file only if commit succeeded
                Path(temp_path).unlink(missing_ok=True)
        else:
            # Single-line commit
            gitio.commit_single_line(plan.commit.message, edit=edit)
            render.success("Commit created successfully")

    except gitio.GitError as e:
        render.error(f"Git operation failed: {e}")
        raise SystemExit(1)
    except KeyboardInterrupt:
        render.info("\nAborted by user")
        raise SystemExit(130)
