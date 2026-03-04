"""Tests for polyref_py.type_inference."""

from __future__ import annotations

from polyref_py.type_inference import (
    build_type_context,
    infer_annotated_assignment,
    infer_constructor_assignment,
    resolve_type,
)


def test_infer_annotated_assignment() -> None:
    result = infer_annotated_assignment("session: Session = Session()")
    assert result == ("session", "Session")


def test_infer_annotated_assignment_strips_generics() -> None:
    result = infer_annotated_assignment("items: list[str] = []")
    assert result is not None
    assert result[1] == "list"


def test_infer_annotated_assignment_no_match() -> None:
    result = infer_annotated_assignment("x = 42")
    assert result is None


def test_infer_annotated_assignment_union() -> None:
    result = infer_annotated_assignment("val: Session | None = None")
    assert result is not None
    assert result[1] == "Session"


def test_infer_constructor_assignment() -> None:
    result = infer_constructor_assignment("session = Session()")
    assert result == ("session", "Session")


def test_infer_constructor_assignment_with_args() -> None:
    result = infer_constructor_assignment("resp = Response(data=123)")
    assert result == ("resp", "Response")


def test_infer_constructor_assignment_no_match() -> None:
    result = infer_constructor_assignment("x = some_function()")
    assert result is None


def test_build_type_context_multiple_bindings() -> None:
    lines = [
        "session: Session = Session()",
        "response: Response = session.get('url')",
        "adapter = HTTPAdapter()",
    ]
    ctx = build_type_context(lines)
    assert len(ctx) == 3
    assert ctx["session"] == "Session"
    assert ctx["response"] == "Response"
    assert ctx["adapter"] == "HTTPAdapter"


def test_build_type_context_overwrites_reassignment() -> None:
    lines = [
        "s = Session()",
        "s = Response()",
    ]
    ctx = build_type_context(lines)
    assert ctx["s"] == "Response"


def test_resolve_known_type() -> None:
    ctx = {"s": "Session"}
    result = resolve_type("s", ctx)
    assert result == "Session"


def test_resolve_unknown_type() -> None:
    result = resolve_type("s", {})
    assert result is None
