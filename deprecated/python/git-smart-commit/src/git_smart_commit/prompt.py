"""Prompt assembly for LLM requests."""
from pathlib import Path
from dataclasses import dataclass


@dataclass
class Context:
    """Context information for prompt building."""
    recent_commits: str = ""
    project_guidelines: str = ""
    agent_context: str = ""


BASE_PROMPT = """You are an AI engineering assistant helping with git commit creation.

Create a conventional commit message that describes the PRIMARY purpose of these changes.

Use conventional commit format with types like: feat, fix, docs, style, refactor, test, chore, perf, ci, build

Guidelines:
- Keep the main message concise and under 50 characters when possible
- Use imperative mood ("add" not "added" or "adds")
- Include scope in parentheses when appropriate: type(scope): description
- If the scope is ambiguous, don't include the scope
- Only use body lines for complex changes that need detailed explanation
- Focus on WHAT changed and WHY, not HOW"""


# Maximum characters for general documentation files (non-agent-specific)
MAX_GENERAL_DOC_CHARS = 3000


def read_agent_context(repo_path: Path = Path(".")) -> str:
    """Read agent context from priority-ordered files.

    Checks files in priority order and returns the first one found.
    Agent-specific files are loaded fully, while general docs are truncated
    if they exceed MAX_GENERAL_DOC_CHARS.

    Priority order:
    1. CLAUDE.md (agent-specific, no truncation)
    2. AGENTS.md (agent-specific, no truncation)
    3. .claude.md (agent-specific, no truncation)
    4. .agents.md (agent-specific, no truncation)
    5. CONTRIBUTING.md (general doc, truncate if > limit)
    6. .github/CONTRIBUTING.md (general doc, truncate if > limit)
    7. CONVENTIONS.md (general doc, truncate if > limit)
    8. README.md (general doc, truncate if > limit)
    """
    # (filename, is_agent_specific)
    candidates = [
        ("CLAUDE.md", True),
        ("AGENTS.md", True),
        (".claude.md", True),
        (".agents.md", True),
        ("CONTRIBUTING.md", False),
        (".github/CONTRIBUTING.md", False),
        ("CONVENTIONS.md", False),
        ("README.md", False),
    ]

    for filename, is_agent_specific in candidates:
        file_path = repo_path / filename
        if file_path.exists():
            try:
                content = file_path.read_text().strip()
                if not content:
                    continue

                # Truncate general docs if they exceed the limit
                if not is_agent_specific and len(content) > MAX_GENERAL_DOC_CHARS:
                    content = content[:MAX_GENERAL_DOC_CHARS] + "\n\n...[truncated]"

                return content
            except Exception:
                continue

    return ""


def read_project_guidelines(repo_path: Path = Path(".")) -> str:
    """Read project-specific commit guidelines from .git-commit-ai-prompt.txt."""
    guidelines_file = repo_path / ".git-commit-ai-prompt.txt"
    if guidelines_file.exists():
        try:
            return guidelines_file.read_text().strip()
        except Exception:
            return ""
    return ""


def build_prompt(context: Context, additional_prompt: str = "", is_staged: bool = True) -> str:
    """Build the complete prompt for the LLM.

    Args:
        context: Context information (recent commits, guidelines)
        additional_prompt: Additional user-provided context
        is_staged: Whether analyzing staged or unstaged changes

    Returns:
        Complete prompt string
    """
    parts = []

    # Start with base prompt
    parts.append(BASE_PROMPT)

    # Add recent commits context if available
    if context.recent_commits:
        parts.append("\nRecent commits in this project:")
        parts.append(context.recent_commits)

    # Add project-specific guidelines if available
    if context.project_guidelines:
        parts.append("\nProject-specific commit guidelines:")
        parts.append(context.project_guidelines)

    # Add agent context if available
    if context.agent_context:
        parts.append("\nRepository context:")
        parts.append(context.agent_context)

    # Add additional user context if provided
    if additional_prompt:
        parts.append(f"\nAdditional context: {additional_prompt}")

    # Add specific instructions based on staged vs unstaged
    if is_staged:
        parts.append("\nAnalyze the following staged diff:")
    else:
        parts.append("\nAnalyze the following diff and identify ONLY RELATED changes that should be committed together. DO NOT add all files - select only files with related changes that serve a common purpose.\n\nDiff:")

    return "\n".join(parts)


def build_summary_prompt(context: Context, is_staged: bool) -> str:
    """Build prompt for summarizing current changes.

    Args:
        context: Context information
        is_staged: Whether analyzing staged or unstaged changes

    Returns:
        Prompt for change summarization
    """
    parts = [
        "You are an AI assistant analyzing git changes.",
        "",
        "Your task: Create a brief summary of the changes shown in the diff below.",
        "",
        "Guidelines:",
        "- Provide a 1-2 sentence overall summary of what changed and why",
        "- For each modified file, write a single sentence explaining what changed",
        "- Focus on the purpose and intent, not implementation details",
        "- Keep summaries concise and clear",
    ]

    # Add context if available
    if context.recent_commits:
        parts.append("\nRecent commits in this project:")
        parts.append(context.recent_commits)

    if context.agent_context:
        parts.append("\nRepository context:")
        parts.append(context.agent_context)

    change_type = "staged" if is_staged else "unstaged"
    parts.append(f"\nAnalyze the following {change_type} diff:")

    return "\n".join(parts)


def build_context_selection_prompt(
    summary: dict,
    log_stats: str,
    file_sizes: dict[str, int]
) -> str:
    """Build prompt for selecting relevant historical context.

    Args:
        summary: The SummaryResponse from the previous step
        log_stats: Output from git log --stat
        file_sizes: Dictionary mapping file paths to sizes in bytes

    Returns:
        Prompt for context selection
    """
    parts = [
        "You are an AI assistant helping select relevant historical context for commit message generation.",
        "",
        "Your task: Based on the summary of current changes, identify which files and commits from recent history would provide useful context.",
        "",
        "Current changes summary:",
        f"Overall: {summary.get('overall_summary', '')}",
        "",
        "Files being changed:",
    ]

    for file_sum in summary.get('file_summaries', []):
        parts.append(f"  - {file_sum.get('file', '')}: {file_sum.get('summary', '')}")

    parts.extend([
        "",
        "Guidelines for selection:",
        "- Be VERY selective - only include context that directly relates to these changes",
        "- Consider implementation history, related functionality, or dependencies",
        "- Avoid large files (>10KB) unless absolutely necessary",
        "- Prefer recent commits over older ones",
        "- Total context should not exceed 50KB",
        "",
        "Available options:",
        "- relevant_files: Current files that need their full content for context",
        "- relevant_commits: Commit hashes to include full diffs",
        "- commit_files: Specific files from specific commits",
        "",
    ])

    # Add file size information
    if file_sizes:
        parts.append("Current file sizes:")
        for file, size in sorted(file_sizes.items()):
            size_kb = size / 1024
            parts.append(f"  - {file}: {size_kb:.1f}KB")
        parts.append("")

    parts.append("Recent commit history with file stats:")
    parts.append(log_stats)

    return "\n".join(parts)


def build_extended_prompt(
    context: Context,
    extended_context: dict,
    additional_prompt: str = "",
    is_staged: bool = True
) -> str:
    """Build enhanced prompt with historical context.

    Args:
        context: Regular context information
        extended_context: Extended context with historical data
        additional_prompt: User-provided additional context
        is_staged: Whether analyzing staged or unstaged changes

    Returns:
        Enhanced prompt with historical context
    """
    parts = []

    # Start with base prompt
    parts.append(BASE_PROMPT)

    # Add recent commits
    if context.recent_commits:
        parts.append("\nRecent commits in this project:")
        parts.append(context.recent_commits)

    # Add project guidelines
    if context.project_guidelines:
        parts.append("\nProject-specific commit guidelines:")
        parts.append(context.project_guidelines)

    # Add agent context
    if context.agent_context:
        parts.append("\nRepository context:")
        parts.append(context.agent_context)

    # Add extended context - summary and reasoning
    summary = extended_context.get("summary", {})
    if summary:
        parts.append("\nSummary of current changes:")
        parts.append(f"Overall: {summary.get('overall_summary', '')}")

    selection = extended_context.get("selection", {})
    if selection and selection.get("reasoning"):
        parts.append(f"\nContext selection reasoning: {selection['reasoning']}")

    # Add historical file contents
    file_contents = extended_context.get("file_contents", {})
    if file_contents:
        parts.append("\nRelevant current file contents:")
        for file_path, content in file_contents.items():
            parts.append(f"\n--- {file_path} ---")
            parts.append(content[:10000])  # Limit each file to 10KB
            if len(content) > 10000:
                parts.append("\n[... truncated ...]")

    # Add historical commit diffs
    commit_diffs = extended_context.get("commit_diffs", {})
    if commit_diffs:
        parts.append("\nRelevant historical commits:")
        for commit_hash, diff in commit_diffs.items():
            parts.append(f"\n--- Commit {commit_hash} ---")
            parts.append(diff[:10000])  # Limit each diff to 10KB
            if len(diff) > 10000:
                parts.append("\n[... truncated ...]")

    # Add specific files from commits
    commit_file_contents = extended_context.get("commit_file_contents", {})
    if commit_file_contents:
        parts.append("\nRelevant files from historical commits:")
        for (commit, file_path), content in commit_file_contents.items():
            parts.append(f"\n--- {file_path} at {commit} ---")
            parts.append(content[:10000])  # Limit to 10KB
            if len(content) > 10000:
                parts.append("\n[... truncated ...]")

    # Add user context
    if additional_prompt:
        parts.append(f"\nAdditional context: {additional_prompt}")

    # Add final instructions
    if is_staged:
        parts.append("\nWith this historical context in mind, analyze the following staged diff and create an appropriate commit message:")
    else:
        parts.append("\nWith this historical context in mind, analyze the following diff and identify ONLY RELATED changes that should be committed together:")

    return "\n".join(parts)
