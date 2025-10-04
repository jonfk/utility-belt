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
