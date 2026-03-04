"""Haiku API wrapper for code generation tests."""

from __future__ import annotations

import logging

import anthropic

logger = logging.getLogger(__name__)

HAIKU_MODEL = "claude-haiku-4-5-20251001"

_SYSTEM_PROMPT = (
    "You are a code generator. Output only a single fenced code block"
    " containing the requested code. No explanations."
)


class HaikuClient:
    """Thin wrapper around the Anthropic API for Haiku code generation."""

    def __init__(self, api_key: str) -> None:
        assert api_key, "api_key must be non-empty"
        self._client = anthropic.Anthropic(api_key=api_key)

    def generate_code(self, prompt: str, max_tokens: int = 1024) -> str:
        """Generate code from a single prompt using Haiku."""
        assert prompt, "prompt must be non-empty"
        response = self._client.messages.create(
            model=HAIKU_MODEL,
            max_tokens=max_tokens,
            system=_SYSTEM_PROMPT,
            messages=[{"role": "user", "content": prompt}],
        )
        text = response.content[0].text  # type: ignore[union-attr]
        assert isinstance(text, str)
        assert text, "Haiku returned empty response"
        return text

    def generate_code_n_times(
        self, prompt: str, n: int, max_tokens: int = 1024
    ) -> list[str]:
        """Generate code n times, collecting all outputs."""
        assert n >= 1, f"n must be >= 1, got {n}"
        results: list[str] = []
        for i in range(n):
            try:
                output = self.generate_code(prompt, max_tokens=max_tokens)
            except Exception:
                logger.warning("Generation %d/%d failed", i + 1, n, exc_info=True)
                output = ""
            results.append(output)
        assert len(results) == n, f"expected {n} results, got {len(results)}"
        return results

    def get_model_name(self) -> str:
        """Return the hardcoded Haiku model identifier."""
        result = HAIKU_MODEL
        assert result, "model name must be non-empty"
        assert result == "claude-haiku-4-5-20251001"
        return result
