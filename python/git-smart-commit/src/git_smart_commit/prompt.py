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
