"""Tests for polyref_py.models and polyref_py.errors."""

from __future__ import annotations

import pytest
from polyref_py.errors import (
    PolyrefError,
    ReferenceFileNotFound,
    ReferenceParseError,
    format_error,
)
from polyref_py.models import (
    EntryKind,
    Issue,
    IssueKind,
    IssueLevel,
    ReferenceEntry,
    ReferenceFile,
)
from pydantic import ValidationError

# --- ReferenceEntry tests ---


def test_reference_entry_valid() -> None:
    entry = ReferenceEntry(name="get", kind=EntryKind.FUNCTION, source_lib="requests")
    assert entry.name == "get"
    assert entry.kind == EntryKind.FUNCTION


def test_reference_entry_with_args() -> None:
    entry = ReferenceEntry(
        name="get",
        kind=EntryKind.METHOD,
        min_args=1,
        max_args=3,
        type_context="Session",
    )
    assert entry.min_args == 1
    assert entry.max_args == 3
    assert entry.type_context == "Session"


def test_reference_entry_empty_name_raises() -> None:
    with pytest.raises(ValidationError, match="name must be non-empty"):
        ReferenceEntry(name="", kind=EntryKind.FUNCTION)


def test_reference_entry_invalid_arg_range_raises() -> None:
    with pytest.raises(ValidationError, match="min_args.*<=.*max_args"):
        ReferenceEntry(name="get", kind=EntryKind.FUNCTION, min_args=5, max_args=2)


def test_reference_entry_frozen() -> None:
    entry = ReferenceEntry(name="get", kind=EntryKind.FUNCTION)
    with pytest.raises(ValidationError):
        entry.name = "post"  # type: ignore[misc]


# --- Issue tests ---


def test_issue_valid() -> None:
    issue = Issue(
        kind=IssueKind.UNKNOWN_METHOD,
        level=IssueLevel.WARNING,
        message="method 'fetch' not found",
        line_number=10,
    )
    assert issue.line_number >= 1
    assert issue.kind == IssueKind.UNKNOWN_METHOD


def test_issue_with_suggestion() -> None:
    issue = Issue(
        kind=IssueKind.UNKNOWN_METHOD,
        level=IssueLevel.WARNING,
        message="not found",
        line_number=5,
        suggestion="get",
        similarity=0.85,
    )
    assert issue.suggestion == "get"
    assert issue.similarity == 0.85


def test_issue_empty_message_raises() -> None:
    with pytest.raises(ValidationError, match="message must be non-empty"):
        Issue(
            kind=IssueKind.UNKNOWN_METHOD,
            level=IssueLevel.ERROR,
            message="",
            line_number=1,
        )


def test_issue_line_zero_raises() -> None:
    with pytest.raises(ValidationError, match="line_number must be >= 1"):
        Issue(
            kind=IssueKind.UNKNOWN_METHOD,
            level=IssueLevel.ERROR,
            message="bad",
            line_number=0,
        )


# --- ReferenceFile tests ---


def test_reference_file_valid() -> None:
    entry = ReferenceEntry(name="get", kind=EntryKind.FUNCTION, source_lib="requests")
    rf = ReferenceFile(
        lang="python", library_name="requests", version="2.32", entries=[entry]
    )
    assert rf.lang == "python"
    assert rf.library_name == "requests"
    assert len(rf.entries) == 1


def test_reference_file_wrong_lang_raises() -> None:
    entry = ReferenceEntry(name="get", kind=EntryKind.FUNCTION)
    with pytest.raises(ValidationError, match="lang must be"):
        ReferenceFile(lang="java", library_name="foo", version="1.0", entries=[entry])


def test_reference_file_empty_entries_raises() -> None:
    with pytest.raises(ValidationError, match="entries must be non-empty"):
        ReferenceFile(
            lang="python", library_name="requests", version="2.32", entries=[]
        )


def test_reference_file_rust_lang_valid() -> None:
    entry = ReferenceEntry(name="spawn", kind=EntryKind.FUNCTION)
    rf = ReferenceFile(
        lang="rust", library_name="tokio", version="1.0", entries=[entry]
    )
    assert rf.lang == "rust"


# --- EntryKind tests ---


def test_entry_kind_values_are_strings() -> None:
    assert all(isinstance(k.value, str) for k in EntryKind)


def test_entry_kind_is_str_subclass() -> None:
    assert all(isinstance(k, str) for k in EntryKind)


# --- Error tests ---


def test_format_error_contains_type_name() -> None:
    result = format_error(ReferenceFileNotFound("file missing"))
    assert "ReferenceFileNotFound" in result
    assert "file missing" in result
    assert result


def test_format_error_with_path() -> None:
    result = format_error(ReferenceParseError("bad format", path="/tmp/bad.polyref"))
    assert "ReferenceParseError" in result
    assert "/tmp/bad.polyref" in result


def test_polyref_error_message_stored() -> None:
    e = PolyrefError("test message", path="/some/path")
    assert e.message == "test message"
    assert e.path == "/some/path"


def test_polyref_error_empty_message_raises() -> None:
    with pytest.raises(AssertionError):
        PolyrefError("")
