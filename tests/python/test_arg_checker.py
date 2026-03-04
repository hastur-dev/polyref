"""Tests for polyref_py.arg_checker."""

from __future__ import annotations

from polyref_py.arg_checker import check_arg_count, count_call_args
from polyref_py.models import EntryKind, IssueKind, ReferenceEntry


def test_count_no_args() -> None:
    source = "session.close()\n"
    result = count_call_args(source, 1)
    assert result == 0


def test_count_one_arg() -> None:
    source = "session.get('url')\n"
    result = count_call_args(source, 1)
    assert result == 1


def test_count_two_args() -> None:
    source = "session.mount('/https', adapter)\n"
    result = count_call_args(source, 1)
    assert result == 2


def test_count_variadic_returns_none() -> None:
    source = "func(*args)\n"
    result = count_call_args(source, 1)
    assert result is None


def test_count_kwargs_spread_returns_none() -> None:
    source = "func(**kwargs)\n"
    result = count_call_args(source, 1)
    assert result is None


def test_count_keyword_args_not_counted() -> None:
    source = "session.get('url', timeout=10)\n"
    result = count_call_args(source, 1)
    assert result == 1


def test_check_too_many_args() -> None:
    entry = ReferenceEntry(name="close", kind=EntryKind.METHOD, min_args=0, max_args=0)
    issue = check_arg_count("close", 1, entry, 1)
    assert issue is not None
    assert issue.kind == IssueKind.TOO_MANY_ARGS


def test_check_too_few_args() -> None:
    entry = ReferenceEntry(name="get", kind=EntryKind.METHOD, min_args=1, max_args=1)
    issue = check_arg_count("get", 0, entry, 1)
    assert issue is not None
    assert issue.kind == IssueKind.TOO_FEW_ARGS


def test_check_correct_args_no_issue() -> None:
    entry = ReferenceEntry(name="get", kind=EntryKind.METHOD, min_args=1, max_args=1)
    issue = check_arg_count("get", 1, entry, 1)
    assert issue is None


def test_check_unknown_bounds_no_issue() -> None:
    entry = ReferenceEntry(name="get", kind=EntryKind.METHOD)
    issue = check_arg_count("get", 5, entry, 1)
    assert issue is None


def test_check_variadic_max_no_issue() -> None:
    entry = ReferenceEntry(name="get", kind=EntryKind.METHOD, min_args=1, max_args=None)
    issue = check_arg_count("get", 10, entry, 1)
    assert issue is None
