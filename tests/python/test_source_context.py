"""Tests for polyref_py.source_context."""

from __future__ import annotations

from pathlib import Path

from polyref_py.models import EntryKind, ReferenceEntry, ReferenceFile
from polyref_py.source_context import build_source_context, select_relevant_refs

REQUESTS_POLYREF = (
    Path(__file__).resolve().parent.parent.parent / "refs" / "requests.polyref"
)


def _make_ref(name: str) -> ReferenceFile:
    return ReferenceFile(
        lang="python",
        library_name=name,
        version="1.0",
        entries=[ReferenceEntry(name="foo", kind=EntryKind.FUNCTION, source_lib=name)],
    )


def test_build_context_extracts_packages() -> None:
    source = "import requests\nimport pandas as pd\n"
    ctx = build_source_context(source)
    assert "requests" in ctx.imported_packages
    assert "pandas" in ctx.imported_packages


def test_build_context_no_duplicates() -> None:
    source = "import requests\nimport requests\n"
    ctx = build_source_context(source)
    assert ctx.imported_packages.count("requests") == 1


def test_build_context_populates_type_bindings() -> None:
    source = "session: Session = Session()\n"
    ctx = build_source_context(source)
    assert ctx.type_bindings["session"] == "Session"


def test_build_context_populates_imported_items() -> None:
    source = "from requests import Session\n"
    ctx = build_source_context(source)
    assert "Session" in ctx.imported_items


def test_select_relevant_refs_filters() -> None:
    ctx = build_source_context("import requests\n")
    requests_ref = _make_ref("requests")
    pandas_ref = _make_ref("pandas")
    result = select_relevant_refs(ctx, [requests_ref, pandas_ref])
    assert len(result) == 1
    assert result[0].library_name == "requests"


def test_select_relevant_refs_empty_imports_returns_all() -> None:
    ctx = build_source_context("x = 1\n")
    refs = [_make_ref("requests"), _make_ref("pandas")]
    result = select_relevant_refs(ctx, refs)
    assert len(result) == 2
