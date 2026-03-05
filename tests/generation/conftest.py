"""Pytest fixtures for generation tests — API client and reference loading."""

from __future__ import annotations

import os
from pathlib import Path

import pytest
from dotenv import load_dotenv
from polyref_py.models import ReferenceFile
from polyref_py.ref_parser import load_reference_file

from .haiku_client import HaikuClient
from .model_client import ModelClient, create_model_client

_REFS_DIR = Path(__file__).resolve().parent.parent.parent / "refs"


@pytest.fixture(scope="session")
def model_client() -> ModelClient:
    """Create a ModelClient using env vars. Supports any provider (Anthropic, OpenAI, Ollama)."""
    load_dotenv()
    try:
        return create_model_client()
    except ValueError as e:
        pytest.skip(f"Model client initialization failed: {e}")


@pytest.fixture(scope="session")
def haiku_client() -> HaikuClient:
    """Create a HaikuClient using ANTHROPIC_API_KEY from .env or env."""
    load_dotenv()
    api_key = os.environ.get("ANTHROPIC_API_KEY", "")
    if not api_key:
        pytest.skip("ANTHROPIC_API_KEY not set — skipping generation tests")
    return HaikuClient(api_key=api_key)


def _load_refs_for_lang(lang: str) -> list[ReferenceFile]:
    """Load all .polyref files in refs/ matching a given language."""
    assert lang in {"python", "rust"}, f"unsupported lang: {lang}"
    refs: list[ReferenceFile] = []
    if not _REFS_DIR.exists():
        return refs
    for path in sorted(_REFS_DIR.glob("*.polyref")):
        try:
            rf = load_reference_file(path)
        except Exception:
            continue
        if rf.lang == lang:
            refs.append(rf)
    return refs


@pytest.fixture(scope="session")
def rust_refs() -> list[ReferenceFile]:
    """Load all .polyref files with lang==rust."""
    refs = _load_refs_for_lang("rust")
    if not refs:
        pytest.skip("No Rust .polyref files found — skipping Rust generation tests")
    return refs


@pytest.fixture(scope="session")
def python_refs() -> list[ReferenceFile]:
    """Load all .polyref files with lang==python."""
    refs = _load_refs_for_lang("python")
    if not refs:
        pytest.skip("No Python .polyref files found — skipping Python generation tests")
    return refs
