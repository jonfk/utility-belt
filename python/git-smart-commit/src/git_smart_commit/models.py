"""Data models and JSON schemas for LLM responses."""
from dataclasses import dataclass
from typing import TypedDict


# JSON Schemas for LLM structured output
STAGED_SCHEMA = {
    "type": "object",
    "properties": {
        "message": {
            "type": "string",
            "description": "Concise conventional commit message under 50 characters when possible",
            "minLength": 1,
            "maxLength": 100
        },
        "body": {
            "type": "array",
            "items": {
                "type": "string",
                "minLength": 1
            },
            "description": "Array of detailed explanation lines. Each item in the array is a line in the commit body. Only include if changes need detailed explanation"
        }
    },
    "required": ["message"],
    "additionalProperties": False
}

UNSTAGED_SCHEMA = {
    "type": "object",
    "properties": {
        "files": {
            "type": "array",
            "items": {
                "type": "string",
                "minLength": 1
            },
            "minItems": 1,
            "description": "Array of file paths to stage together - only include related files that serve a common purpose"
        },
        "message": {
            "type": "string",
            "description": "Concise conventional commit message under 50 characters when possible",
            "minLength": 1,
            "maxLength": 100
        },
        "body": {
            "type": "array",
            "items": {
                "type": "string",
                "minLength": 1
            },
            "description": "Array of detailed explanation lines. Each item in the array is a line in the commit body. Only include if changes need detailed explanation"
        }
    },
    "required": ["files", "message"],
    "additionalProperties": False
}

SUMMARY_SCHEMA = {
    "type": "object",
    "properties": {
        "overall_summary": {
            "type": "string",
            "description": "Brief overall summary of what changed and why (1-2 sentences)",
            "minLength": 1,
            "maxLength": 300
        },
        "file_summaries": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "file": {
                        "type": "string",
                        "description": "File path",
                        "minLength": 1
                    },
                    "summary": {
                        "type": "string",
                        "description": "What changed in this file and why (1 sentence)",
                        "minLength": 1,
                        "maxLength": 200
                    }
                },
                "required": ["file", "summary"],
                "additionalProperties": False
            },
            "description": "Summary of changes for each modified file"
        }
    },
    "required": ["overall_summary", "file_summaries"],
    "additionalProperties": False
}

CONTEXT_SELECTION_SCHEMA = {
    "type": "object",
    "properties": {
        "relevant_files": {
            "type": "array",
            "items": {
                "type": "string",
                "minLength": 1
            },
            "description": "Current repository files that need historical context (select carefully to avoid too much context)"
        },
        "relevant_commits": {
            "type": "array",
            "items": {
                "type": "string",
                "minLength": 1
            },
            "description": "Commit hashes that provide relevant context (full commit diffs will be included)"
        },
        "commit_files": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "commit": {
                        "type": "string",
                        "description": "Commit hash",
                        "minLength": 1
                    },
                    "file": {
                        "type": "string",
                        "description": "File path in that commit",
                        "minLength": 1
                    }
                },
                "required": ["commit", "file"],
                "additionalProperties": False
            },
            "description": "Specific files from specific commits that provide relevant context"
        },
        "reasoning": {
            "type": "string",
            "description": "Brief explanation of why this context is relevant (1-2 sentences)",
            "minLength": 1,
            "maxLength": 300
        }
    },
    "required": ["reasoning"],
    "additionalProperties": False
}


# Response types
class StagedResponse(TypedDict, total=False):
    """Response from LLM for staged changes."""
    message: str
    body: list[str]


class UnstagedResponse(TypedDict, total=False):
    """Response from LLM for unstaged changes."""
    files: list[str]
    message: str
    body: list[str]


class FileSummary(TypedDict):
    """Summary of changes for a single file."""
    file: str
    summary: str


class SummaryResponse(TypedDict):
    """Response from LLM for change summarization."""
    overall_summary: str
    file_summaries: list[FileSummary]


class CommitFile(TypedDict):
    """Reference to a file in a specific commit."""
    commit: str
    file: str


class ContextSelectionResponse(TypedDict, total=False):
    """Response from LLM for context selection."""
    relevant_files: list[str]
    relevant_commits: list[str]
    commit_files: list[CommitFile]
    reasoning: str


@dataclass
class Commit:
    """Represents a commit to be created."""
    message: str
    body: list[str]


@dataclass
class StageStep:
    """Represents files to stage."""
    files: list[str]


@dataclass
class Plan:
    """Execution plan for staging and committing."""
    steps: list[StageStep]
    commit: Commit


@dataclass
class ExtendedContext:
    """Extended context from repository history."""
    summary: SummaryResponse
    selection: ContextSelectionResponse
    file_contents: dict[str, str]  # file_path -> content
    commit_diffs: dict[str, str]  # commit_hash -> diff
    commit_file_contents: dict[tuple[str, str], str]  # (commit, file) -> content


def parse_staged(data: dict) -> tuple[str, list[str]]:
    """Parse and validate staged response from LLM.

    Returns:
        Tuple of (message, body_lines)
    """
    message = data.get("message", "").strip()
    if not message:
        raise ValueError("Commit message is required")

    # Parse body lines, filtering out empty ones
    body = data.get("body", [])
    body_lines = [line.strip() for line in body if line.strip()]

    # Warn if message is too long (handled by caller)
    if len(message) > 72:
        pass  # Caller will handle warning

    return message, body_lines


def parse_unstaged(data: dict) -> tuple[list[str], str, list[str]]:
    """Parse and validate unstaged response from LLM.

    Returns:
        Tuple of (files, message, body_lines)
    """
    files = data.get("files", [])
    if not files:
        raise ValueError("At least one file is required")

    # Filter out empty file paths
    files = [f.strip() for f in files if f.strip()]
    if not files:
        raise ValueError("No valid files specified")

    message = data.get("message", "").strip()
    if not message:
        raise ValueError("Commit message is required")

    # Parse body lines, filtering out empty ones
    body = data.get("body", [])
    body_lines = [line.strip() for line in body if line.strip()]

    # Warn if message is too long (handled by caller)
    if len(message) > 72:
        pass  # Caller will handle warning

    return files, message, body_lines


def parse_summary(data: dict) -> SummaryResponse:
    """Parse and validate summary response from LLM.

    Returns:
        SummaryResponse with overall_summary and file_summaries
    """
    overall_summary = data.get("overall_summary", "").strip()
    if not overall_summary:
        raise ValueError("Overall summary is required")

    file_summaries_raw = data.get("file_summaries", [])
    file_summaries: list[FileSummary] = []

    for item in file_summaries_raw:
        file = item.get("file", "").strip()
        summary = item.get("summary", "").strip()
        if file and summary:
            file_summaries.append(FileSummary(file=file, summary=summary))

    return SummaryResponse(
        overall_summary=overall_summary,
        file_summaries=file_summaries
    )


def parse_context_selection(data: dict) -> ContextSelectionResponse:
    """Parse and validate context selection response from LLM.

    Returns:
        ContextSelectionResponse with selected files, commits, and reasoning
    """
    reasoning = data.get("reasoning", "").strip()
    if not reasoning:
        raise ValueError("Reasoning is required")

    relevant_files = [f.strip() for f in data.get("relevant_files", []) if f.strip()]
    relevant_commits = [c.strip() for c in data.get("relevant_commits", []) if c.strip()]

    commit_files_raw = data.get("commit_files", [])
    commit_files: list[CommitFile] = []
    for item in commit_files_raw:
        commit = item.get("commit", "").strip()
        file = item.get("file", "").strip()
        if commit and file:
            commit_files.append(CommitFile(commit=commit, file=file))

    return ContextSelectionResponse(
        relevant_files=relevant_files,
        relevant_commits=relevant_commits,
        commit_files=commit_files,
        reasoning=reasoning
    )
