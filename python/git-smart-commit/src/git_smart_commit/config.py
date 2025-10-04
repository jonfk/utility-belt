"""Configuration management."""
import os
from dataclasses import dataclass


@dataclass
class Config:
    """Application configuration."""
    default_model: str = "gemini-2.5-flash"
    default_backend: str = "llm"  # Future: could be "litellm"


def get_default_model() -> str:
    """Get the default model from env or config."""
    return os.getenv("GSC_MODEL", Config.default_model)


def get_default_llm_flags() -> list[str]:
    """Get default LLM flags from environment."""
    flags_str = os.getenv("GSC_LLM_FLAGS", "")
    if flags_str:
        return flags_str.split()
    return []


def get_backend() -> str:
    """Get the LLM backend to use."""
    return os.getenv("GSC_BACKEND", Config.default_backend)
