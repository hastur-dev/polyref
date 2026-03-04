"""Tests for polyref_py.ref_parser."""

from __future__ import annotations

from pathlib import Path

import pytest
from polyref_py.errors import (
    ReferenceFileNotFound,
    ReferenceParseError,
)
from polyref_py.models import EntryKind
from polyref_py.ref_parser import (
    load_reference_file,
    parse_arg_hint,
    parse_class_block,
    parse_exception_line,
    parse_function_line,
    parse_reference_file,
)

REQUESTS_POLYREF = (
    Path(__file__).resolve().parent.parent.parent / "refs" / "requests.polyref"
)


# --- parse_arg_hint ---


def test_parse_arg_hint_full() -> None:
    result = parse_arg_hint("[min_args=1, max_args=3]")
    assert result == (1, 3)


def test_parse_arg_hint_min_only() -> None:
    result = parse_arg_hint("[min_args=1]")
    assert result == (1, None)


def test_parse_arg_hint_empty() -> None:
    result = parse_arg_hint("")
    assert result == (None, None)


def test_parse_arg_hint_no_brackets() -> None:
    result = parse_arg_hint("no brackets here")
    assert result == (None, None)


# --- parse_function_line ---


def test_parse_function_line_valid() -> None:
    entry = parse_function_line(
        "@fn get(url: str, **kwargs) -> Response  [min_args=1]", "requests"
    )
    assert entry is not None
    assert entry.name == "get"
    assert entry.kind == EntryKind.FUNCTION
    assert entry.min_args == 1
    assert entry.type_context is None


def test_parse_function_line_no_match() -> None:
    result = parse_function_line("# some comment", "requests")
    assert result is None


def test_parse_function_line_no_hint() -> None:
    entry = parse_function_line("@fn request(method: str, url: str)", "requests")
    assert entry is not None
    assert entry.name == "request"
    assert entry.min_args is None


# --- parse_exception_line ---


def test_parse_exception_line_valid() -> None:
    entry = parse_exception_line("@exception ConnectionError", "requests")
    assert entry is not None
    assert entry.kind == EntryKind.EXCEPTION
    assert entry.name == "ConnectionError"


def test_parse_exception_line_no_match() -> None:
    result = parse_exception_line("@fn something()", "requests")
    assert result is None


# --- parse_class_block ---


def test_parse_class_block_extracts_methods() -> None:
    block = """
    @method get(url: str) -> Response  [min_args=1]
    @method post(url: str) -> Response  [min_args=1]
    @method close() -> None  [min_args=0, max_args=0]
    """
    entries = parse_class_block(block, "Session", "requests")
    assert len(entries) == 3
    assert all(e.kind == EntryKind.METHOD for e in entries)


def test_parse_class_block_extracts_fields() -> None:
    block = """
    @field status_code: int
    @field text: str
    """
    entries = parse_class_block(block, "Response", "requests")
    assert len(entries) == 2
    assert all(e.kind == EntryKind.FIELD for e in entries)
    assert all(e.type_context == "Response" for e in entries)


def test_parse_class_block_sets_type_context() -> None:
    block = """
    @method get(url: str) -> Response  [min_args=1]
    @field text: str
    """
    entries = parse_class_block(block, "Session", "requests")
    assert all(e.type_context == "Session" for e in entries)


def test_parse_class_block_variadic_kwargs() -> None:
    block = """
    @method get(url: str, **kwargs) -> Response  [min_args=1]
    """
    entries = parse_class_block(block, "Session", "requests")
    assert len(entries) == 1
    assert entries[0].max_args is None


def test_parse_class_block_class_method() -> None:
    block = """
    @class_method from_data(data: bytes) -> Foo  [min_args=1]
    """
    entries = parse_class_block(block, "Foo", "mylib")
    assert len(entries) == 1
    assert entries[0].kind == EntryKind.CLASS_METHOD


# --- parse_reference_file ---


def test_parse_reference_file_full() -> None:
    content = REQUESTS_POLYREF.read_text(encoding="utf-8")
    rf = parse_reference_file(content)
    assert rf.lang == "python"
    assert rf.library_name == "requests"
    assert len(rf.entries) > 0
    # Should have Session, Response classes + methods + functions + exceptions
    kinds = {e.kind for e in rf.entries}
    assert EntryKind.FUNCTION in kinds
    assert EntryKind.METHOD in kinds
    assert EntryKind.FIELD in kinds
    assert EntryKind.EXCEPTION in kinds
    assert EntryKind.CLASS in kinds
    assert EntryKind.MODULE in kinds


def test_parse_reference_file_wrong_lang_raises() -> None:
    content = "# Library: foo\n@lang rust\n@fn bar() -> Baz\n"
    with pytest.raises(ReferenceParseError, match="@lang python"):
        parse_reference_file(content)


def test_parse_reference_file_deduplicates() -> None:
    content = """
@lang python
# Library: test
# Version: 1.0
@fn get(url: str) -> Response  [min_args=1]
@fn get(url: str) -> Response  [min_args=1]
@exception Timeout
@exception Timeout
"""
    rf = parse_reference_file(content)
    names = [e.name for e in rf.entries]
    assert names.count("get") == 1
    assert names.count("Timeout") == 1


def test_parse_reference_file_empty_raises() -> None:
    content = "@lang python\n# Library: test\n# Version: 1.0\n"
    with pytest.raises(ReferenceParseError, match="no entries"):
        parse_reference_file(content)


# --- load_reference_file ---


def test_load_reference_file_success() -> None:
    rf = load_reference_file(REQUESTS_POLYREF)
    assert rf.library_name == "requests"
    assert rf.lang == "python"


def test_load_reference_file_missing_path_raises() -> None:
    with pytest.raises(ReferenceFileNotFound):
        load_reference_file(Path("/nonexistent/missing.polyref"))


def test_load_reference_file_wrong_extension_raises() -> None:
    with pytest.raises(AssertionError, match="expected .polyref"):
        load_reference_file(Path("/some/file.txt"))
