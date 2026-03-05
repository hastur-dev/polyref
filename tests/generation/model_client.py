"""Multi-provider model client abstraction for code generation."""

from __future__ import annotations

import logging
import os
from typing import Protocol

import anthropic
import openai

logger = logging.getLogger(__name__)


class ModelClient(Protocol):
    """Protocol for any model provider that can generate code."""

    def generate_code(self, prompt: str, max_tokens: int = 1024) -> str:
        """Generate code from a single prompt."""
        ...

    def generate_code_n_times(
        self, prompt: str, n: int, max_tokens: int = 1024
    ) -> list[str]:
        """Generate code n times, collecting all outputs."""
        ...

    def get_model_name(self) -> str:
        """Return the model identifier."""
        ...


class AnthropicModelClient:
    """Anthropic API backend. Supports any claude-* model."""

    def __init__(self, api_key: str, model: str = "claude-haiku-4-5-20251001") -> None:
        assert api_key, "api_key must be non-empty"
        assert model, "model must be non-empty"
        self._client = anthropic.Anthropic(api_key=api_key)
        self._model = model

    def generate_code(self, prompt: str, max_tokens: int = 1024) -> str:
        """Generate code from a single prompt using Anthropic."""
        assert prompt, "prompt must be non-empty"
        assert max_tokens > 0, "max_tokens must be positive"
        response = self._client.messages.create(
            model=self._model,
            max_tokens=max_tokens,
            system=(
                "You are a code generator. Output only a single fenced code block"
                " containing the requested code. No explanations."
            ),
            messages=[{"role": "user", "content": prompt}],
        )
        text = response.content[0].text  # type: ignore[union-attr]
        assert isinstance(text, str)
        assert text, "Model returned empty response"
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
        """Return the model identifier."""
        assert self._model, "model name must be non-empty"
        return self._model


class OpenAIModelClient:
    """OpenAI API backend. base_url allows Ollama or any OpenAI-compat endpoint."""

    def __init__(
        self, api_key: str, model: str = "gpt-4o-mini", base_url: str | None = None
    ) -> None:
        assert api_key, "api_key must be non-empty"
        assert model, "model must be non-empty"
        self._model = model
        self._client = openai.OpenAI(api_key=api_key, base_url=base_url)

    def generate_code(self, prompt: str, max_tokens: int = 1024) -> str:
        """Generate code from a single prompt using OpenAI."""
        assert prompt, "prompt must be non-empty"
        assert max_tokens > 0, "max_tokens must be positive"
        response = self._client.chat.completions.create(
            model=self._model,
            max_tokens=max_tokens,
            system=(
                "You are a code generator. Output only a single fenced code block"
                " containing the requested code. No explanations."
            ),
            messages=[{"role": "user", "content": prompt}],
        )
        text = response.choices[0].message.content
        assert isinstance(text, str)
        assert text, "Model returned empty response"
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
        """Return the model identifier."""
        assert self._model, "model name must be non-empty"
        return self._model


def create_model_client() -> ModelClient:
    """
    Factory: reads POLYREF_PROVIDER, POLYREF_MODEL env vars to create the appropriate client.

    Environment variables:
        POLYREF_PROVIDER: "anthropic" (default) | "openai" | "openai_compat" | "ollama"
        POLYREF_MODEL: model name (overrides defaults)
        ANTHROPIC_API_KEY: for Anthropic backend
        OPENAI_API_KEY: for OpenAI / OpenAI-compat backends
        OPENAI_BASE_URL: optional custom base_url for OpenAI-compat

    Ollama uses OpenAI-compat backend with base_url="http://localhost:11434/v1"
    """
    provider = os.environ.get("POLYREF_PROVIDER", "anthropic").lower()
    model = os.environ.get("POLYREF_MODEL", "").strip()

    if provider == "anthropic":
        api_key = os.environ.get("ANTHROPIC_API_KEY", "")
        if not api_key:
            raise ValueError("ANTHROPIC_API_KEY environment variable not set")
        default_model = "claude-haiku-4-5-20251001"
        return AnthropicModelClient(api_key=api_key, model=model or default_model)

    elif provider == "openai":
        api_key = os.environ.get("OPENAI_API_KEY", "")
        if not api_key:
            raise ValueError("OPENAI_API_KEY environment variable not set")
        default_model = "gpt-4o-mini"
        base_url = os.environ.get("OPENAI_BASE_URL", "").strip() or None
        return OpenAIModelClient(
            api_key=api_key, model=model or default_model, base_url=base_url
        )

    elif provider == "openai_compat":
        api_key = os.environ.get("OPENAI_API_KEY", "")
        if not api_key:
            raise ValueError("OPENAI_API_KEY environment variable not set")
        base_url = os.environ.get("OPENAI_BASE_URL", "").strip()
        if not base_url:
            raise ValueError("OPENAI_BASE_URL environment variable not set for openai_compat")
        default_model = "gpt-4o-mini"
        return OpenAIModelClient(
            api_key=api_key, model=model or default_model, base_url=base_url
        )

    elif provider == "ollama":
        api_key = os.environ.get("OPENAI_API_KEY", "ollama")
        base_url = "http://localhost:11434/v1"
        default_model = "llama2"
        return OpenAIModelClient(
            api_key=api_key, model=model or default_model, base_url=base_url
        )

    else:
        raise ValueError(f"Unknown POLYREF_PROVIDER: {provider}")
