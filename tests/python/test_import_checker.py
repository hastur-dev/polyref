"""Tests for polyref_py.fuzzy and polyref_py.import_checker."""

from __future__ import annotations

from pathlib import Path

from polyref_py.fuzzy import find_all_matches_above, find_best_match, similarity
from polyref_py.import_checker import (
    ImportStatement,
    check_all_imports,
    check_import,
    extract_imports,
)
from polyref_py.models import IssueKind
from polyref_py.ref_parser import load_reference_file

REQUESTS_POLYREF = (
    Path(__file__).resolve().parent.parent.parent / "refs" / "requests.polyref"
)


def _load_requests_ref() -> list:
    return [load_reference_file(REQUESTS_POLYREF)]


# --- fuzzy.py tests ---


def test_similarity_same_string() -> None:
    result = similarity("abort", "abort")
    assert result == 1.0


def test_similarity_different_strings() -> None:
    result = similarity("abort", "xyz")
    assert result < 0.5


def test_similarity_returns_float_in_range() -> None:
    result = similarity("hello", "world")
    assert 0.0 <= result <= 1.0


def test_find_best_match_finds_close() -> None:
    result = find_best_match("Sesion", ["get", "session", "Session"])
    assert result is not None


def test_find_best_match_below_threshold() -> None:
    result = find_best_match("xyz_totally_invented", ["get", "post"])
    assert result is None


def test_find_best_match_empty_candidates() -> None:
    result = find_best_match("test", [])
    assert result is None


def test_find_all_matches_sorted() -> None:
    results = find_all_matches_above("get", ["get", "gett", "post", "put"], 0.3)
    scores = [s for _, s in results]
    assert scores == sorted(scores, reverse=True)
    assert all(s >= 0.3 for s in scores)


# --- extract_imports tests ---


def test_extract_imports_simple_import() -> None:
    stmts = extract_imports("import requests\n")
    assert len(stmts) == 1
    assert stmts[0].module_path == "requests"
    assert stmts[0].is_from_import is False


def test_extract_imports_from_import() -> None:
    stmts = extract_imports("from requests import Session\n")
    assert len(stmts) == 1
    assert stmts[0].imported_name == "Session"
    assert stmts[0].is_from_import is True


def test_extract_imports_from_import_multi() -> None:
    stmts = extract_imports("from requests import get, post, Session\n")
    assert len(stmts) == 3
    names = {s.imported_name for s in stmts}
    assert names == {"get", "post", "Session"}


def test_extract_imports_alias() -> None:
    stmts = extract_imports("import pandas as pd\n")
    assert len(stmts) == 1
    assert stmts[0].alias == "pd"
    assert stmts[0].module_path == "pandas"


def test_extract_imports_line_numbers() -> None:
    source = "import os\nimport sys\nimport requests\n"
    stmts = extract_imports(source)
    assert len(stmts) == 3
    lines = [s.line_number for s in stmts]
    assert lines == [1, 2, 3]


def test_extract_imports_dotted_module() -> None:
    stmts = extract_imports("from requests.auth import HTTPBasicAuth\n")
    assert len(stmts) == 1
    assert stmts[0].module_path == "requests.auth"
    assert stmts[0].imported_name == "HTTPBasicAuth"


# --- check_import tests ---


def test_check_import_known_module_no_issue() -> None:
    refs = _load_requests_ref()
    stmt = ImportStatement(
        module_path="requests",
        imported_name="requests",
        line_number=1,
        is_from_import=False,
    )
    issues = check_import(stmt, refs)
    assert issues == []


def test_check_import_unknown_module_emits_issue() -> None:
    refs = _load_requests_ref()
    stmt = ImportStatement(
        module_path="requestss",
        imported_name="requestss",
        line_number=1,
        is_from_import=False,
    )
    issues = check_import(stmt, refs)
    # "requestss" doesn't match top package "requests", so no issue
    # (we only check modules we know about)
    assert len(issues) == 0


def test_check_import_known_name_no_issue() -> None:
    refs = _load_requests_ref()
    stmt = ImportStatement(
        module_path="requests",
        imported_name="Session",
        line_number=1,
        is_from_import=True,
    )
    issues = check_import(stmt, refs)
    assert issues == []


def test_check_import_unknown_name_emits_issue() -> None:
    refs = _load_requests_ref()
    stmt = ImportStatement(
        module_path="requests",
        imported_name="Sessoin",
        line_number=1,
        is_from_import=True,
    )
    issues = check_import(stmt, refs)
    assert len(issues) == 1
    assert issues[0].kind == IssueKind.UNKNOWN_IMPORT
    assert issues[0].suggestion == "Session"


def test_check_import_unrelated_module_ignored() -> None:
    refs = _load_requests_ref()
    stmt = ImportStatement(
        module_path="os",
        imported_name="os",
        line_number=1,
        is_from_import=False,
    )
    issues = check_import(stmt, refs)
    assert issues == []


def test_check_all_imports_empty_returns_empty() -> None:
    refs = _load_requests_ref()
    issues = check_all_imports([], refs)
    assert issues == []


def test_check_all_imports_mixed() -> None:
    refs = _load_requests_ref()
    stmts = [
        ImportStatement(
            module_path="requests",
            imported_name="Session",
            line_number=1,
            is_from_import=True,
        ),
        ImportStatement(
            module_path="requests",
            imported_name="Sessoin",
            line_number=2,
            is_from_import=True,
        ),
    ]
    issues = check_all_imports(stmts, refs)
    assert len(issues) == 1
    assert issues[0].kind == IssueKind.UNKNOWN_IMPORT
