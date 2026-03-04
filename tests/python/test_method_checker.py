"""Tests for polyref_py.method_checker."""

from __future__ import annotations

from pathlib import Path

from polyref_py.method_checker import (
    check_constructor_call,
    check_method_call,
    collect_all_method_names,
    extract_method_calls,
)
from polyref_py.models import IssueKind
from polyref_py.ref_parser import load_reference_file

REQUESTS_POLYREF = (
    Path(__file__).resolve().parent.parent.parent / "refs" / "requests.polyref"
)


def _load_requests_ref() -> list:
    return [load_reference_file(REQUESTS_POLYREF)]


# --- extract_method_calls ---


def test_extract_instance_method_call() -> None:
    source = "session.get('http://example.com')\n"
    methods, _constructors = extract_method_calls(source)
    assert len(methods) >= 1
    mc = methods[0]
    assert mc.is_class_call is False
    assert mc.receiver == "session"
    assert mc.method_name == "get"


def test_extract_class_method_call() -> None:
    source = "Session.get('url')\n"
    methods, _constructors = extract_method_calls(source)
    assert len(methods) >= 1
    mc = methods[0]
    assert mc.is_class_call is True
    assert mc.receiver == "Session"


def test_extract_constructor_call() -> None:
    source = "session = Session()\n"
    _methods, constructors = extract_method_calls(source)
    assert len(constructors) >= 1
    assert constructors[0].class_name == "Session"


def test_extract_skips_comment_lines() -> None:
    source = "# session.get() is the right way\nx = 1\n"
    methods, _constructors = extract_method_calls(source)
    # CST doesn't parse comments as code, so no method call extracted
    assert len(methods) == 0


def test_extract_method_line_numbers() -> None:
    source = "x = 1\nsession.get('url')\ny = 2\n"
    methods, _constructors = extract_method_calls(source)
    assert len(methods) >= 1
    assert methods[0].line_number == 2


# --- check_method_call ---


def test_check_known_method_no_issue() -> None:
    refs = _load_requests_ref()
    from polyref_py.method_checker import MethodCall

    call = MethodCall(
        receiver="session",
        method_name="get",
        is_class_call=False,
        line_number=1,
        col_number=0,
    )
    issues = check_method_call(call, refs, {"session": "Session"})
    assert issues == []


def test_check_unknown_method_emits_issue() -> None:
    refs = _load_requests_ref()
    from polyref_py.method_checker import MethodCall

    call = MethodCall(
        receiver="session",
        method_name="fetch",
        is_class_call=False,
        line_number=1,
        col_number=0,
    )
    issues = check_method_call(call, refs, {"session": "Session"})
    assert len(issues) == 1
    assert issues[0].kind == IssueKind.UNKNOWN_METHOD


def test_check_unknown_method_with_suggestion() -> None:
    refs = _load_requests_ref()
    from polyref_py.method_checker import MethodCall

    call = MethodCall(
        receiver="session",
        method_name="gett",
        is_class_call=False,
        line_number=1,
        col_number=0,
    )
    issues = check_method_call(call, refs, {"session": "Session"})
    assert len(issues) == 1
    assert issues[0].suggestion == "get"


def test_check_known_class_method_no_issue() -> None:
    refs = _load_requests_ref()
    from polyref_py.method_checker import MethodCall

    call = MethodCall(
        receiver="Session",
        method_name="get",
        is_class_call=True,
        line_number=1,
        col_number=0,
    )
    issues = check_method_call(call, refs, {})
    assert issues == []


def test_check_unknown_class_method_emits_issue() -> None:
    refs = _load_requests_ref()
    from polyref_py.method_checker import MethodCall

    call = MethodCall(
        receiver="Session",
        method_name="fetch",
        is_class_call=True,
        line_number=1,
        col_number=0,
    )
    issues = check_method_call(call, refs, {})
    assert len(issues) == 1
    assert issues[0].kind == IssueKind.UNKNOWN_CLASS_METHOD


def test_check_method_with_type_context() -> None:
    refs = _load_requests_ref()
    from polyref_py.method_checker import MethodCall

    call = MethodCall(
        receiver="s",
        method_name="get",
        is_class_call=False,
        line_number=1,
        col_number=0,
    )
    issues = check_method_call(call, refs, {"s": "Session"})
    assert issues == []


def test_check_method_with_type_context_wrong_method() -> None:
    refs = _load_requests_ref()
    from polyref_py.method_checker import MethodCall

    call = MethodCall(
        receiver="s",
        method_name="fetch",
        is_class_call=False,
        line_number=1,
        col_number=0,
    )
    issues = check_method_call(call, refs, {"s": "Session"})
    assert len(issues) == 1
    assert "Session" in issues[0].message


# --- check_constructor_call ---


def test_check_constructor_known_class_no_issue() -> None:
    refs = _load_requests_ref()
    issues = check_constructor_call("Session", 1, refs)
    assert issues == []


def test_check_constructor_unknown_class_emits_issue() -> None:
    refs = _load_requests_ref()
    issues = check_constructor_call("Sessoin", 1, refs)
    assert len(issues) == 1
    assert issues[0].suggestion is not None


# --- collect_all_method_names ---


def test_collect_all_method_names_no_duplicates() -> None:
    refs = _load_requests_ref()
    names = collect_all_method_names(refs)
    assert len(names) == len(set(names))
    assert "get" in names
