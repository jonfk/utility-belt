"""LLM CLI adapter - shells out to the `llm` command."""
import json
import subprocess
import shutil


class LlmCliError(Exception):
    """Raised when the llm CLI command fails."""
    pass


class LlmCliClient:
    """Adapter for Simon Willison's llm CLI tool."""

    def __init__(self):
        """Initialize the client and check if llm is available."""
        if not shutil.which("llm"):
            raise LlmCliError(
                "llm command not found. Please install it first: "
                "https://llm.datasette.io/"
            )

    def generate(
        self,
        *,
        diff: str,
        schema: dict,
        prompt: str,
        model: str,
        extra_flags: list[str]
    ) -> dict:
        """Generate structured output using the llm CLI.

        Args:
            diff: The git diff to analyze
            schema: JSON schema for structured output
            prompt: The prompt text
            model: Model name/identifier
            extra_flags: Additional flags to pass to llm

        Returns:
            Parsed JSON response

        Raises:
            LlmCliError: If the command fails or returns invalid JSON
        """
        # Build the complete input text
        input_text = f"{prompt}\n\nDiff:\n{diff}"

        # Build the command
        cmd = ["llm", "-m", model, "--schema", json.dumps(schema)]
        cmd.extend(extra_flags)

        try:
            result = subprocess.run(
                cmd,
                input=input_text,
                capture_output=True,
                text=True,
                check=True,
            )

            # Parse JSON output
            if not result.stdout.strip():
                raise LlmCliError("Empty response from llm command")

            try:
                return json.loads(result.stdout)
            except json.JSONDecodeError as e:
                raise LlmCliError(f"Invalid JSON response: {e}\nOutput: {result.stdout}") from e

        except subprocess.CalledProcessError as e:
            error_msg = f"llm command failed with exit code {e.returncode}"
            if e.stderr:
                error_msg += f"\nError: {e.stderr}"
            raise LlmCliError(error_msg) from e
        except FileNotFoundError:
            raise LlmCliError("llm command not found") from None
