"""Argument count validation for Python function/method calls."""

from __future__ import annotations

import libcst as cst

from polyref_py.models import Issue, IssueKind, IssueLevel, ReferenceEntry


class _ArgCountVisitor(cst.CSTVisitor):
    """Visitor that finds Call nodes and counts their arguments."""

    METADATA_DEPENDENCIES = (cst.metadata.PositionProvider,)

    def __init__(self, target_line: int) -> None:
        self.target_line = target_line
        self.arg_count: int | None = None
        self._found = False

    def visit_Call(self, node: cst.Call) -> None:  # noqa: N802
        """Count args for the call at the target line."""
        if self._found:
            return
        pos = self.get_metadata(cst.metadata.PositionProvider, node)
        line_no: int = pos.start.line  # type: ignore[union-attr]
        if line_no != self.target_line:
            return

        for arg in node.args:
            if isinstance(arg.keyword, type(None)) and isinstance(arg.value, cst.Name):
                if arg.star == "**" or arg.star == "*":
                    self.arg_count = None
                    self._found = True
                    return
            if arg.star:
                self.arg_count = None
                self._found = True
                return

        count = 0
        for arg in node.args:
            if arg.keyword is None:
                count += 1
        self.arg_count = count
        self._found = True


def count_call_args(source: str, line: int) -> int | None:
    """Count positional arguments at a call site on the given line."""
    try:
        tree = cst.parse_module(source)
    except cst.ParserSyntaxError:
        return None

    wrapper = cst.metadata.MetadataWrapper(tree)
    visitor = _ArgCountVisitor(line)
    wrapper.visit(visitor)

    result = visitor.arg_count
    if result is not None:
        assert result >= 0, f"arg_count must be >= 0, got {result}"
    return result


def check_arg_count(
    call_name: str,
    arg_count: int,
    entry: ReferenceEntry,
    line: int,
) -> Issue | None:
    """Compare arg_count against entry.min_args / entry.max_args."""
    if entry.min_args is None:
        return None

    if arg_count < entry.min_args:
        return Issue(
            kind=IssueKind.TOO_FEW_ARGS,
            level=IssueLevel.ERROR,
            message=(
                f"'{call_name}' expects at least {entry.min_args} "
                f"arg(s), got {arg_count}"
            ),
            line_number=line,
        )

    if entry.max_args is not None and arg_count > entry.max_args:
        return Issue(
            kind=IssueKind.TOO_MANY_ARGS,
            level=IssueLevel.ERROR,
            message=(
                f"'{call_name}' expects at most {entry.max_args} "
                f"arg(s), got {arg_count}"
            ),
            line_number=line,
        )

    return None
