"""Unit tests for multi-provider model client (using mocks, no real API calls)."""

from __future__ import annotations

import os
from unittest.mock import MagicMock, patch

import pytest

from .haiku_client import HAIKU_MODEL, HaikuClient
from .model_client import AnthropicModelClient, OpenAIModelClient, create_model_client


class TestAnthropicModelClient:
    """Tests for AnthropicModelClient."""

    def test_init_requires_api_key(self) -> None:
        """AnthropicModelClient requires non-empty api_key."""
        with pytest.raises(AssertionError):
            AnthropicModelClient(api_key="")

    def test_init_requires_model(self) -> None:
        """AnthropicModelClient requires non-empty model."""
        with pytest.raises(AssertionError):
            AnthropicModelClient(api_key="test", model="")

    @patch("anthropic.Anthropic")
    def test_generate_code(self, mock_anthropic_class: MagicMock) -> None:
        """generate_code calls Anthropic API and returns text."""
        mock_client = MagicMock()
        mock_anthropic_class.return_value = mock_client

        mock_response = MagicMock()
        mock_response.content = [MagicMock(text="generated code")]
        mock_client.messages.create.return_value = mock_response

        client = AnthropicModelClient(api_key="test-key", model="claude-test")
        result = client.generate_code("test prompt")

        assert result == "generated code"
        mock_client.messages.create.assert_called_once()
        call_kwargs = mock_client.messages.create.call_args[1]
        assert call_kwargs["model"] == "claude-test"
        assert call_kwargs["max_tokens"] == 1024

    @patch("anthropic.Anthropic")
    def test_generate_code_n_times(self, mock_anthropic_class: MagicMock) -> None:
        """generate_code_n_times returns correct count of outputs."""
        mock_client = MagicMock()
        mock_anthropic_class.return_value = mock_client

        mock_response = MagicMock()
        mock_response.content = [MagicMock(text="generated code")]
        mock_client.messages.create.return_value = mock_response

        client = AnthropicModelClient(api_key="test-key", model="claude-test")
        results = client.generate_code_n_times("test prompt", n=3)

        assert len(results) == 3
        assert all(r == "generated code" for r in results)

    @patch("anthropic.Anthropic")
    def test_generate_code_n_times_with_failure(
        self, mock_anthropic_class: MagicMock
    ) -> None:
        """generate_code_n_times handles failures gracefully."""
        mock_client = MagicMock()
        mock_anthropic_class.return_value = mock_client

        # Fail once, then succeed
        mock_response = MagicMock()
        mock_response.content = [MagicMock(text="generated code")]
        mock_client.messages.create.side_effect = [
            Exception("API error"),
            mock_response,
            mock_response,
        ]

        client = AnthropicModelClient(api_key="test-key", model="claude-test")
        results = client.generate_code_n_times("test prompt", n=3)

        assert len(results) == 3
        assert results[0] == ""  # Failed attempt
        assert results[1] == "generated code"
        assert results[2] == "generated code"

    @patch("anthropic.Anthropic")
    def test_get_model_name(self, mock_anthropic_class: MagicMock) -> None:
        """get_model_name returns the configured model."""
        mock_anthropic_class.return_value = MagicMock()
        client = AnthropicModelClient(api_key="test-key", model="claude-test-model")
        assert client.get_model_name() == "claude-test-model"


class TestOpenAIModelClient:
    """Tests for OpenAIModelClient."""

    def test_init_requires_api_key(self) -> None:
        """OpenAIModelClient requires non-empty api_key."""
        with pytest.raises(AssertionError):
            OpenAIModelClient(api_key="")

    def test_init_requires_model(self) -> None:
        """OpenAIModelClient requires non-empty model."""
        with pytest.raises(AssertionError):
            OpenAIModelClient(api_key="test", model="")

    @patch("openai.OpenAI")
    def test_generate_code(self, mock_openai_class: MagicMock) -> None:
        """generate_code calls OpenAI API and returns text."""
        mock_client = MagicMock()
        mock_openai_class.return_value = mock_client

        mock_response = MagicMock()
        mock_response.choices = [MagicMock(message=MagicMock(content="generated code"))]
        mock_client.chat.completions.create.return_value = mock_response

        client = OpenAIModelClient(api_key="test-key", model="gpt-test")
        result = client.generate_code("test prompt")

        assert result == "generated code"
        mock_client.chat.completions.create.assert_called_once()
        call_kwargs = mock_client.chat.completions.create.call_args[1]
        assert call_kwargs["model"] == "gpt-test"
        assert call_kwargs["max_tokens"] == 1024

    @patch("openai.OpenAI")
    def test_generate_code_with_custom_base_url(
        self, mock_openai_class: MagicMock
    ) -> None:
        """OpenAIModelClient accepts custom base_url for Ollama/compat endpoints."""
        mock_openai_class.return_value = MagicMock()

        client = OpenAIModelClient(
            api_key="test-key",
            model="llama2",
            base_url="http://localhost:11434/v1",
        )

        # Verify base_url was passed to OpenAI constructor
        mock_openai_class.assert_called_once()
        call_kwargs = mock_openai_class.call_args[1]
        assert call_kwargs["base_url"] == "http://localhost:11434/v1"

    @patch("openai.OpenAI")
    def test_generate_code_n_times(self, mock_openai_class: MagicMock) -> None:
        """generate_code_n_times returns correct count of outputs."""
        mock_client = MagicMock()
        mock_openai_class.return_value = mock_client

        mock_response = MagicMock()
        mock_response.choices = [MagicMock(message=MagicMock(content="generated code"))]
        mock_client.chat.completions.create.return_value = mock_response

        client = OpenAIModelClient(api_key="test-key", model="gpt-test")
        results = client.generate_code_n_times("test prompt", n=3)

        assert len(results) == 3
        assert all(r == "generated code" for r in results)

    @patch("openai.OpenAI")
    def test_get_model_name(self, mock_openai_class: MagicMock) -> None:
        """get_model_name returns the configured model."""
        mock_openai_class.return_value = MagicMock()
        client = OpenAIModelClient(api_key="test-key", model="gpt-test-model")
        assert client.get_model_name() == "gpt-test-model"


class TestCreateModelClientFactory:
    """Tests for create_model_client() factory function."""

    def test_defaults_to_anthropic(self) -> None:
        """create_model_client defaults to Anthropic when no env set."""
        with patch.dict(
            os.environ,
            {"ANTHROPIC_API_KEY": "test-key"},
            clear=True,
        ):
            with patch("anthropic.Anthropic"):
                client = create_model_client()
                assert isinstance(client, AnthropicModelClient)

    def test_anthropic_provider_with_env(self) -> None:
        """POLYREF_PROVIDER=anthropic creates AnthropicModelClient."""
        with patch.dict(
            os.environ,
            {
                "POLYREF_PROVIDER": "anthropic",
                "ANTHROPIC_API_KEY": "test-key",
            },
            clear=True,
        ):
            with patch("anthropic.Anthropic"):
                client = create_model_client()
                assert isinstance(client, AnthropicModelClient)

    def test_anthropic_missing_api_key(self) -> None:
        """Anthropic provider requires ANTHROPIC_API_KEY."""
        with patch.dict(
            os.environ,
            {"POLYREF_PROVIDER": "anthropic"},
            clear=True,
        ):
            with pytest.raises(ValueError, match="ANTHROPIC_API_KEY"):
                create_model_client()

    def test_openai_provider_with_env(self) -> None:
        """POLYREF_PROVIDER=openai creates OpenAIModelClient."""
        with patch.dict(
            os.environ,
            {
                "POLYREF_PROVIDER": "openai",
                "OPENAI_API_KEY": "test-key",
            },
            clear=True,
        ):
            with patch("openai.OpenAI"):
                client = create_model_client()
                assert isinstance(client, OpenAIModelClient)

    def test_openai_missing_api_key(self) -> None:
        """OpenAI provider requires OPENAI_API_KEY."""
        with patch.dict(
            os.environ,
            {"POLYREF_PROVIDER": "openai"},
            clear=True,
        ):
            with pytest.raises(ValueError, match="OPENAI_API_KEY"):
                create_model_client()

    def test_openai_compat_provider(self) -> None:
        """POLYREF_PROVIDER=openai_compat requires OPENAI_BASE_URL."""
        with patch.dict(
            os.environ,
            {
                "POLYREF_PROVIDER": "openai_compat",
                "OPENAI_API_KEY": "test-key",
                "OPENAI_BASE_URL": "http://custom:8000/v1",
            },
            clear=True,
        ):
            with patch("openai.OpenAI"):
                client = create_model_client()
                assert isinstance(client, OpenAIModelClient)

    def test_openai_compat_missing_base_url(self) -> None:
        """openai_compat provider requires OPENAI_BASE_URL."""
        with patch.dict(
            os.environ,
            {
                "POLYREF_PROVIDER": "openai_compat",
                "OPENAI_API_KEY": "test-key",
            },
            clear=True,
        ):
            with pytest.raises(ValueError, match="OPENAI_BASE_URL"):
                create_model_client()

    def test_ollama_provider(self) -> None:
        """POLYREF_PROVIDER=ollama uses OpenAI-compat with localhost:11434/v1."""
        with patch.dict(
            os.environ,
            {
                "POLYREF_PROVIDER": "ollama",
            },
            clear=True,
        ):
            with patch("openai.OpenAI") as mock_openai:
                client = create_model_client()
                assert isinstance(client, OpenAIModelClient)
                # Verify base_url is set
                mock_openai.assert_called_once()
                call_kwargs = mock_openai.call_args[1]
                assert call_kwargs["base_url"] == "http://localhost:11434/v1"

    def test_unknown_provider(self) -> None:
        """Unknown POLYREF_PROVIDER raises ValueError."""
        with patch.dict(
            os.environ,
            {"POLYREF_PROVIDER": "unknown_provider"},
            clear=True,
        ):
            with pytest.raises(ValueError, match="Unknown POLYREF_PROVIDER"):
                create_model_client()

    def test_polyref_model_env_override(self) -> None:
        """POLYREF_MODEL env var overrides default model."""
        with patch.dict(
            os.environ,
            {
                "POLYREF_PROVIDER": "anthropic",
                "POLYREF_MODEL": "claude-opus-4",
                "ANTHROPIC_API_KEY": "test-key",
            },
            clear=True,
        ):
            with patch("anthropic.Anthropic"):
                client = create_model_client()
                assert isinstance(client, AnthropicModelClient)
                assert client.get_model_name() == "claude-opus-4"


class TestHaikuClientBackwardCompat:
    """Tests for HaikuClient backward compatibility."""

    def test_haiku_client_uses_hardcoded_model(self) -> None:
        """HaikuClient uses hardcoded HAIKU_MODEL."""
        # Test that get_model_name returns the hardcoded HAIKU_MODEL constant
        assert HAIKU_MODEL == "claude-haiku-4-5-20251001"

    def test_haiku_client_api_key_required(self) -> None:
        """HaikuClient requires non-empty api_key."""
        with pytest.raises(AssertionError, match="api_key must be non-empty"):
            HaikuClient(api_key="")

    def test_haiku_client_is_wrapper_for_anthropic(self) -> None:
        """HaikuClient creates an AnthropicModelClient internally."""
        with patch("anthropic.Anthropic") as mock_anthropic:
            client = HaikuClient(api_key="test-key")
            # Verify that AnthropicModelClient's __init__ was called
            # (which calls anthropic.Anthropic internally)
            mock_anthropic.assert_called_once()
            # Verify the hardcoded model is used
            assert client.get_model_name() == HAIKU_MODEL
