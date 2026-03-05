"""Haiku API wrapper for code generation tests.

Backward-compatible alias for AnthropicModelClient with hardcoded Haiku model.
"""

from __future__ import annotations

HAIKU_MODEL = "claude-haiku-4-5-20251001"

from .model_client import AnthropicModelClient


class HaikuClient:
    """Backward-compatible alias for AnthropicModelClient using hardcoded Haiku model."""

    def __init__(self, api_key: str) -> None:
        assert api_key, "api_key must be non-empty"
        self._inner = AnthropicModelClient(api_key=api_key, model=HAIKU_MODEL)

    def generate_code(self, prompt: str, max_tokens: int = 1024) -> str:
        """Generate code from a single prompt using Haiku."""
        return self._inner.generate_code(prompt, max_tokens=max_tokens)

    def generate_code_n_times(
        self, prompt: str, n: int, max_tokens: int = 1024
    ) -> list[str]:
        """Generate code n times, collecting all outputs."""
        return self._inner.generate_code_n_times(prompt, n, max_tokens=max_tokens)

    def get_model_name(self) -> str:
        """Return the hardcoded Haiku model identifier."""
        result = self._inner.get_model_name()
        assert result, "model name must be non-empty"
        assert result == "claude-haiku-4-5-20251001"
        return result
