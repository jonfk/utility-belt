"""Protocol definition for LLM clients."""
from typing import Protocol


class LLMClient(Protocol):
    """Protocol for LLM client implementations."""

    def generate(
        self,
        *,
        diff: str,
        schema: dict,
        prompt: str,
        model: str,
        extra_flags: list[str]
    ) -> dict:
        """Generate structured output from the LLM.

        Args:
            diff: The git diff to analyze
            prompt: The prompt text to send to the LLM
            schema: JSON schema for structured output
            model: Model name/identifier
            extra_flags: Additional flags to pass to the LLM

        Returns:
            Parsed JSON response as a dict

        Raises:
            Exception: If the LLM request fails or returns invalid output
        """
        ...
