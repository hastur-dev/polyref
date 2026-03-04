"""Tests for polyref_py.checker."""

from __future__ import annotations

from pathlib import Path

import pytest
from polyref_py.checker import check_file, check_source_string
from polyref_py.errors import InvalidInputError
from polyref_py.models import IssueKind
from polyref_py.ref_parser import load_reference_file

REQUESTS_POLYREF = (
    Path(__file__).resolve().parent.parent.parent / "refs" / "requests.polyref"
)


def _load_requests_ref() -> list:
    return [load_reference_file(REQUESTS_POLYREF)]


def test_check_source_string_no_issues_on_good_code() -> None:
    source = """import requests
from requests import Session

session = Session()
response = session.get("https://example.com")
response.raise_for_status()
data = response.json()
print(response.status_code)
session.close()
"""
    refs = _load_requests_ref()
    issues = check_source_string(source, refs)
    assert issues == []


def test_check_source_string_flags_unknown_method() -> None:
    source = """import requests
from requests import Session

session = Session()
session.fetch("url")
"""
    refs = _load_requests_ref()
    issues = check_source_string(source, refs)
    assert any(i.kind == IssueKind.UNKNOWN_METHOD for i in issues)


def test_check_source_string_flags_unknown_import() -> None:
    source = """from requests import Sessoin
"""
    refs = _load_requests_ref()
    issues = check_source_string(source, refs)
    assert any(i.kind == IssueKind.UNKNOWN_IMPORT for i in issues)


def test_check_source_string_flags_too_many_args() -> None:
    source = """import requests
from requests import Session

session = Session()
session.close(True)
"""
    refs = _load_requests_ref()
    issues = check_source_string(source, refs)
    assert any(i.kind == IssueKind.TOO_MANY_ARGS for i in issues)


def test_check_source_string_issues_sorted_by_line() -> None:
    source = """from requests import Sessoin
import requests
session = Session()
session.fetch("url")
"""
    refs = _load_requests_ref()
    issues = check_source_string(source, refs)
    if len(issues) > 1:
        assert all(
            issues[i].line_number <= issues[i + 1].line_number
            for i in range(len(issues) - 1)
        )


def test_check_source_string_empty_source_returns_empty() -> None:
    refs = _load_requests_ref()
    issues = check_source_string("", refs)
    assert issues == []


def test_check_file_wrong_extension_raises(tmp_path: Path) -> None:
    bad_file = tmp_path / "file.txt"
    bad_file.write_text("hello")
    with pytest.raises(InvalidInputError):
        check_file(bad_file, _load_requests_ref())
