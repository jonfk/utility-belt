"""Prompt assembly for LLM requests."""
from pathlib import Path
from dataclasses import dataclass


@dataclass
class Context:
    """Context information for prompt building."""
    recent_commits: str = ""
    project_guidelines: str = ""


BASE_PROMPT = """You are an AI engineering assistant helping with git commit creation.

Create a conventional commit message that describes the PRIMARY purpose of these changes.

Use conventional commit format with types like: feat, fix, docs, style, refactor, test, chore, perf, ci, build

Guidelines:
- Keep the main message concise and under 50 characters when possible
- Use imperative mood ("add" not "added" or "adds")
- Include scope in parentheses when appropriate: type(scope): description
- Only use body lines for complex changes that need detailed explanation
- Focus on WHAT changed and WHY, not HOW"""


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

    # Add additional user context if provided
    if additional_prompt:
        parts.append(f"\nAdditional context: {additional_prompt}")

    # Add specific instructions based on staged vs unstaged
    if is_staged:
        parts.append("\nAnalyze the following staged diff:")
    else:
        parts.append("\nAnalyze the following diff and identify ONLY RELATED changes that should be committed together. DO NOT add all files - select only files with related changes that serve a common purpose.\n\nDiff:")

    return "\n".join(parts)
