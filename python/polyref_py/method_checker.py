"""Method call extraction and validation using libcst."""

from __future__ import annotations

from collections.abc import Sequence

import libcst as cst
from pydantic import BaseModel, ConfigDict

from polyref_py.fuzzy import find_best_match
from polyref_py.models import (
    EntryKind,
    Issue,
    IssueKind,
    IssueLevel,
    ReferenceFile,
)


class MethodCall(BaseModel):
    """A method call extracted from source code."""

    receiver: str
    method_name: str
    is_class_call: bool
    line_number: int
    col_number: int

    model_config = ConfigDict(frozen=True)


class ConstructorCall(BaseModel):
    """A constructor call like ClassName() extracted from source."""

    class_name: str
    line_number: int
    col_number: int

    model_config = ConfigDict(frozen=True)


class _CallVisitor(cst.CSTVisitor):
    """CST visitor that collects method calls and constructor calls."""

    METADATA_DEPENDENCIES = (cst.metadata.PositionProvider,)

    def __init__(self) -> None:
        self.method_calls: list[MethodCall] = []
        self.constructor_calls: list[ConstructorCall] = []

    def visit_Call(self, node: cst.Call) -> None:  # noqa: N802
        """Extract method calls and constructor calls."""
        pos = self.get_metadata(cst.metadata.PositionProvider, node)
        line_no: int = pos.start.line  # type: ignore[union-attr]
        col_no: int = pos.start.column  # type: ignore[union-attr]

        if isinstance(node.func, cst.Attribute):
            receiver_node = node.func.value
            method_name = node.func.attr.value
            receiver = _expr_to_str(receiver_node)
            if not receiver or not method_name:
                return
            is_class = receiver[0].isupper()
            self.method_calls.append(
                MethodCall(
                    receiver=receiver,
                    method_name=method_name,
                    is_class_call=is_class,
                    line_number=line_no,
                    col_number=col_no,
                )
            )
        elif isinstance(node.func, cst.Name):
            name = node.func.value
            if name and name[0].isupper():
                self.constructor_calls.append(
                    ConstructorCall(
                        class_name=name,
                        line_number=line_no,
                        col_number=col_no,
                    )
                )


def _expr_to_str(node: cst.BaseExpression) -> str:
    """Convert a CST expression to a simple string."""
    if isinstance(node, cst.Name):
        return node.value
    if isinstance(node, cst.Attribute):
        prefix = _expr_to_str(node.value)
        return f"{prefix}.{node.attr.value}" if prefix else node.attr.value
    return ""


def extract_method_calls(
    source: str,
) -> tuple[list[MethodCall], list[ConstructorCall]]:
    """Extract method calls and constructor calls from Python source."""
    try:
        tree = cst.parse_module(source)
    except cst.ParserSyntaxError:
        return [], []

    wrapper = cst.metadata.MetadataWrapper(tree)
    visitor = _CallVisitor()
    wrapper.visit(visitor)

    for mc in visitor.method_calls:
        assert mc.method_name, "method_name must be non-empty"
        assert mc.line_number >= 1
    return visitor.method_calls, visitor.constructor_calls


def check_method_call(
    call: MethodCall,
    refs: list[ReferenceFile],
    type_ctx: dict[str, str],
) -> list[Issue]:
    """Check a single method call against reference entries."""
    assert call.method_name, "method_name must be non-empty"

    if call.is_class_call:
        return _check_class_method_call(call, refs)
    return _check_instance_method_call(call, refs, type_ctx)


def _check_class_method_call(
    call: MethodCall,
    refs: list[ReferenceFile],
) -> list[Issue]:
    """Check ClassName.method() calls."""
    class_name = call.receiver
    candidates: list[str] = []
    for rf in refs:
        for e in rf.entries:
            if e.type_context == class_name and e.kind in {
                EntryKind.METHOD,
                EntryKind.CLASS_METHOD,
                EntryKind.STATIC_METHOD,
            }:
                if e.name == call.method_name:
                    return []
                candidates.append(e.name)

    suggestion = find_best_match(call.method_name, candidates) if candidates else None
    return [
        Issue(
            kind=IssueKind.UNKNOWN_CLASS_METHOD,
            level=IssueLevel.WARNING,
            message=(f"class method '{call.method_name}' not found on '{class_name}'"),
            line_number=call.line_number,
            col_number=call.col_number,
            suggestion=suggestion,
        )
    ]


def _check_instance_method_call(
    call: MethodCall,
    refs: list[ReferenceFile],
    type_ctx: dict[str, str],
) -> list[Issue]:
    """Check instance.method() calls."""
    inferred_type = type_ctx.get(call.receiver)

    if inferred_type:
        return _check_typed_method(call, refs, inferred_type)
    return _check_untyped_method(call, refs)


def _check_typed_method(
    call: MethodCall,
    refs: list[ReferenceFile],
    type_name: str,
) -> list[Issue]:
    """Check method call when receiver type is known."""
    candidates: list[str] = []
    for rf in refs:
        for e in rf.entries:
            if e.type_context == type_name and e.kind in {
                EntryKind.METHOD,
                EntryKind.CLASS_METHOD,
                EntryKind.STATIC_METHOD,
                EntryKind.FIELD,
            }:
                if e.name == call.method_name:
                    return []
                candidates.append(e.name)

    suggestion = find_best_match(call.method_name, candidates) if candidates else None
    return [
        Issue(
            kind=IssueKind.UNKNOWN_METHOD,
            level=IssueLevel.WARNING,
            message=(f"method '{call.method_name}' not found on '{type_name}'"),
            line_number=call.line_number,
            col_number=call.col_number,
            suggestion=suggestion,
        )
    ]


def _check_untyped_method(
    call: MethodCall,
    refs: list[ReferenceFile],
) -> list[Issue]:
    """Check method call when receiver type is unknown — search all methods."""
    all_methods = collect_all_method_names(refs)
    if call.method_name in all_methods:
        return []

    # Also check if it's a top-level function on a module receiver
    for rf in refs:
        if call.receiver == rf.library_name:
            for e in rf.entries:
                if e.name == call.method_name and e.kind == EntryKind.FUNCTION:
                    return []

    suggestion = find_best_match(call.method_name, all_methods) if all_methods else None
    return [
        Issue(
            kind=IssueKind.UNKNOWN_METHOD,
            level=IssueLevel.WARNING,
            message=f"method '{call.method_name}' not found",
            line_number=call.line_number,
            col_number=call.col_number,
            suggestion=suggestion,
        )
    ]


def check_constructor_call(
    class_name: str,
    line: int,
    refs: list[ReferenceFile],
) -> list[Issue]:
    """Validate ClassName() — the class must exist in refs."""
    assert class_name[0].isupper(), "class_name must start uppercase"
    all_classes: list[str] = []
    for rf in refs:
        for e in rf.entries:
            if e.kind == EntryKind.CLASS:
                if e.name == class_name:
                    return []
                all_classes.append(e.name)

    suggestion = find_best_match(class_name, all_classes) if all_classes else None
    return [
        Issue(
            kind=IssueKind.UNKNOWN_CLASS_METHOD,
            level=IssueLevel.WARNING,
            message=f"unknown class '{class_name}'",
            line_number=line,
            suggestion=suggestion,
        )
    ]


def collect_all_method_names(refs: Sequence[ReferenceFile]) -> list[str]:
    """Flatten all method/class_method/static_method names, deduplicated."""
    seen: set[str] = set()
    result: list[str] = []
    for rf in refs:
        for e in rf.entries:
            if e.kind in {
                EntryKind.METHOD,
                EntryKind.CLASS_METHOD,
                EntryKind.STATIC_METHOD,
            }:
                if e.name not in seen:
                    assert e.name, "method name must be non-empty"
                    seen.add(e.name)
                    result.append(e.name)
    return result
