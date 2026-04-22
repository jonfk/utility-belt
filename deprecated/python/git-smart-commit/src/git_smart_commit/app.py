"""Main application orchestration."""
from pathlib import Path
from typing import Optional

from . import gitio, render
from .models import (
    STAGED_SCHEMA,
    UNSTAGED_SCHEMA,
    SUMMARY_SCHEMA,
    CONTEXT_SELECTION_SCHEMA,
    Commit,
    Plan,
    StageStep,
    ExtendedContext,
    parse_staged,
    parse_unstaged,
    parse_summary,
    parse_context_selection,
)
from .prompt import (
    Context,
    build_prompt,
    build_summary_prompt,
    build_context_selection_prompt,
    build_extended_prompt,
    read_agent_context,
    read_project_guidelines,
)
from .llm.llm_cli import LlmCliClient


def gather_extended_context(
    llm_client: LlmCliClient,
    model: str,
    llm_flags: list[str],
    diff: str,
    context: Context,
    is_staged: bool,
    verbose: bool = False,
) -> Optional[dict]:
    """Gather extended context using 3-step LLM process.

    Args:
        llm_client: LLM client
        model: Model name
        llm_flags: Extra LLM flags
        diff: Current diff
        context: Regular context
        is_staged: Whether analyzing staged changes
        verbose: Show verbose output

    Returns:
        Extended context dict, or None if failed
    """
    try:
        # Step 1: Summarize current changes
        render.show_step(1, 3, "Summarizing current changes...")
        summary_prompt = build_summary_prompt(context, is_staged)

        summary_data = llm_client.generate(
            diff=diff,
            schema=SUMMARY_SCHEMA,
            prompt=summary_prompt,
            model=model,
            extra_flags=llm_flags,
        )

        render.show_json_response(summary_data, verbose=verbose)
        summary_response = parse_summary(summary_data)

        # Step 2: Select relevant context
        render.show_step(2, 3, "Selecting relevant historical context...")

        # Get git log with stats
        try:
            log_stats = gitio.log_with_stats(n=20)
        except gitio.GitError:
            log_stats = ""

        # Get file sizes for files mentioned in summary
        file_sizes = {}
        for file_sum in summary_response["file_summaries"]:
            file_path = file_sum["file"]
            size = gitio.get_file_size(file_path)
            if size > 0:
                file_sizes[file_path] = size

        # Build context selection prompt
        selection_prompt = build_context_selection_prompt(
            summary=summary_data,
            log_stats=log_stats,
            file_sizes=file_sizes,
        )

        # Get context selection from LLM (no diff needed for this step)
        selection_data = llm_client.generate(
            diff="",  # Empty diff, all info is in the prompt
            schema=CONTEXT_SELECTION_SCHEMA,
            prompt=selection_prompt,
            model=model,
            extra_flags=llm_flags,
        )

        render.show_json_response(selection_data, verbose=verbose)
        selection_response = parse_context_selection(selection_data)

        # Show what was selected
        render.show_extended_context(selection_data)

        # Step 3: Fetch selected context
        render.show_step(3, 3, "Fetching selected files and commits...")

        file_contents = {}
        commit_diffs = {}
        commit_file_contents = {}
        total_size = 0
        max_size = 50 * 1024  # 50KB limit

        # Fetch current file contents
        for file_path in selection_response.get("relevant_files", []):
            try:
                content = Path(file_path).read_text()
                size = len(content.encode('utf-8'))
                if total_size + size > max_size:
                    render.warning(f"Skipping {file_path}: would exceed size limit")
                    continue
                file_contents[file_path] = content
                total_size += size
            except Exception as e:
                render.warning(f"Could not read {file_path}: {e}")

        # Fetch commit diffs
        for commit_hash in selection_response.get("relevant_commits", []):
            try:
                diff = gitio.show_commit(commit_hash)
                size = len(diff.encode('utf-8'))
                if total_size + size > max_size:
                    render.warning(f"Skipping commit {commit_hash[:12]}: would exceed size limit")
                    continue
                commit_diffs[commit_hash] = diff
                total_size += size
            except Exception as e:
                render.warning(f"Could not fetch commit {commit_hash[:12]}: {e}")

        # Fetch specific files from commits
        for cf in selection_response.get("commit_files", []):
            commit = cf["commit"]
            file_path = cf["file"]
            try:
                content = gitio.get_file_at_commit(commit, file_path)
                size = len(content.encode('utf-8'))
                if total_size + size > max_size:
                    render.warning(f"Skipping {file_path}@{commit[:12]}: would exceed size limit")
                    continue
                commit_file_contents[(commit, file_path)] = content
                total_size += size
            except Exception as e:
                render.warning(f"Could not fetch {file_path} from {commit[:12]}: {e}")

        # Show total context size
        render.show_context_size(total_size)

        return {
            "summary": summary_response,
            "selection": selection_response,
            "file_contents": file_contents,
            "commit_diffs": commit_diffs,
            "commit_file_contents": commit_file_contents,
        }

    except Exception as e:
        render.warning(f"Extended context gathering failed: {e}")
        render.info("Falling back to normal mode")
        return None


def run(
    model: str,
    extra_prompt: str = "",
    llm_flags: Optional[list[str]] = None,
    dry_run: bool = False,
    verbose: bool = False,
    extended_context: bool = False,
) -> None:
    """Run the git-smart-commit application.

    Args:
        model: LLM model to use
        extra_prompt: Additional user-provided prompt context
        llm_flags: Extra flags to pass to the llm command
        dry_run: If True, show plan but don't execute
        verbose: Show verbose output including raw JSON
        extended_context: If True, use extended context mode with historical analysis
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
        recent_commits = gitio.log_oneline(n=10)
    except gitio.GitError:
        recent_commits = ""

    project_guidelines = read_project_guidelines()
    agent_context = read_agent_context()

    context = Context(
        recent_commits=recent_commits,
        project_guidelines=project_guidelines,
        agent_context=agent_context,
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

            # Gather extended context if requested
            extended_ctx = None
            if extended_context:
                extended_ctx = gather_extended_context(
                    llm_client=llm_client,
                    model=model,
                    llm_flags=llm_flags,
                    diff=diff,
                    context=context,
                    is_staged=True,
                    verbose=verbose,
                )

            # Build prompt based on whether we have extended context
            if extended_ctx:
                prompt = build_extended_prompt(context, extended_ctx, extra_prompt, is_staged=True)
            else:
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

            # Gather extended context if requested
            extended_ctx = None
            if extended_context:
                extended_ctx = gather_extended_context(
                    llm_client=llm_client,
                    model=model,
                    llm_flags=llm_flags,
                    diff=diff,
                    context=context,
                    is_staged=False,
                    verbose=verbose,
                )

            # Build prompt based on whether we have extended context
            if extended_ctx:
                prompt = build_extended_prompt(context, extended_ctx, extra_prompt, is_staged=False)
            else:
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

        # Ask user if they want to edit the commit message
        try:
            response = input("Edit commit message? [y/N]: ").strip().lower()
            edit = response in ['y', 'yes']
        except (EOFError, KeyboardInterrupt):
            render.info("\nAborted by user")
            raise SystemExit(130)

        # Create commit
        try:
            gitio.commit(plan.commit.message, plan.commit.body, edit=edit)
            render.success("Commit created successfully")
        except gitio.GitError as e:
            render.error(f"Commit failed: {e}")
            raise SystemExit(1)

    except gitio.GitError as e:
        render.error(f"Git operation failed: {e}")
        raise SystemExit(1)
    except KeyboardInterrupt:
        render.info("\nAborted by user")
        raise SystemExit(130)
