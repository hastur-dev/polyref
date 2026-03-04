"""Orchestrating checker: wires all sub-checkers for a single file."""

from __future__ import annotations

from pathlib import Path

from polyref_py.arg_checker import check_arg_count, count_call_args
from polyref_py.errors import InvalidInputError
from polyref_py.import_checker import check_all_imports
from polyref_py.method_checker import (
    check_constructor_call,
    check_method_call,
    extract_method_calls,
)
from polyref_py.models import (
    EntryKind,
    Issue,
    ReferenceFile,
)
from polyref_py.source_context import build_source_context, select_relevant_refs


def check_source_string(
    source: str,
    refs: list[ReferenceFile],
    filename: str = "<string>",
) -> list[Issue]:
    """Check a Python source string against reference files."""
    if not source.strip():
        return []

    ctx = build_source_context(source)
    relevant = select_relevant_refs(ctx, refs)
    if not relevant:
        return []

    issues: list[Issue] = []

    # 1. Check imports
    import_issues = check_all_imports(ctx.import_statements, relevant)
    issues.extend(import_issues)

    # 2. Check method calls + constructor calls
    method_calls, constructor_calls = extract_method_calls(source)
    for mc in method_calls:
        mc_issues = check_method_call(mc, relevant, ctx.type_bindings)
        issues.extend(mc_issues)

        # 3. Arg count check for matched methods
        if not mc_issues:
            _check_args_for_call(
                source,
                mc.method_name,
                mc.line_number,
                mc.receiver,
                relevant,
                ctx.type_bindings,
                issues,
            )

    for cc in constructor_calls:
        cc_issues = check_constructor_call(cc.class_name, cc.line_number, relevant)
        issues.extend(cc_issues)

    # Sort by line number
    issues.sort(key=lambda i: i.line_number)
    assert all(
        issues[i].line_number <= issues[i + 1].line_number
        for i in range(len(issues) - 1)
    )
    return issues


def _check_args_for_call(
    source: str,
    method_name: str,
    line: int,
    receiver: str,
    refs: list[ReferenceFile],
    type_ctx: dict[str, str],
    issues: list[Issue],
) -> None:
    """Check arg count for a known method call."""
    arg_count = count_call_args(source, line)
    if arg_count is None:
        return

    type_name = type_ctx.get(receiver)
    for rf in refs:
        for entry in rf.entries:
            if entry.name != method_name:
                continue
            if entry.kind not in {
                EntryKind.METHOD,
                EntryKind.FUNCTION,
                EntryKind.CLASS_METHOD,
                EntryKind.STATIC_METHOD,
            }:
                continue
            if type_name and entry.type_context and entry.type_context != type_name:
                continue
            issue = check_arg_count(method_name, arg_count, entry, line)
            if issue:
                issues.append(issue)
            return


def check_file(
    source_path: Path,
    refs: list[ReferenceFile],
) -> list[Issue]:
    """Check a Python source file against reference files."""
    if source_path.suffix != ".py":
        raise InvalidInputError(
            f"expected .py file, got '{source_path.suffix}'",
            path=str(source_path),
        )
    assert source_path.suffix == ".py"
    source = source_path.read_text(encoding="utf-8")
    issues = check_source_string(source, refs, filename=str(source_path))
    assert all(
        issues[i].line_number <= issues[i + 1].line_number
        for i in range(len(issues) - 1)
    )
    return issues
