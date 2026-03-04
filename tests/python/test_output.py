"""Tests for polyref_py.output."""

from __future__ import annotations

from polyref_py.models import Issue, IssueKind, IssueLevel
from polyref_py.output import format_issue_pyright_style, format_summary


def _make_issue(
    kind: IssueKind = IssueKind.UNKNOWN_METHOD,
    level: IssueLevel = IssueLevel.WARNING,
    line: int = 10,
    suggestion: str | None = None,
    similarity: float | None = None,
) -> Issue:
    return Issue(
        kind=kind,
        level=level,
        message=f"test issue: {kind.value}",
        line_number=line,
        col_number=5,
        suggestion=suggestion,
        similarity=similarity,
    )


def test_format_issue_contains_path() -> None:
    issue = _make_issue()
    result = format_issue_pyright_style(issue, "/tmp/test.py")
    assert "/tmp/test.py" in result


def test_format_issue_contains_line() -> None:
    issue = _make_issue(line=42)
    result = format_issue_pyright_style(issue, "/tmp/test.py")
    assert "42" in result


def test_format_issue_with_suggestion() -> None:
    issue = _make_issue(suggestion="get", similarity=0.85)
    result = format_issue_pyright_style(issue, "/tmp/test.py")
    assert "get" in result
    assert "0.85" in result


def test_format_issue_non_empty() -> None:
    issue = _make_issue()
    result = format_issue_pyright_style(issue, "/tmp/test.py")
    assert result


def test_format_summary_counts() -> None:
    issues = [
        _make_issue(level=IssueLevel.WARNING),
        _make_issue(level=IssueLevel.ERROR),
        _make_issue(level=IssueLevel.WARNING),
    ]
    result = format_summary(issues, "/tmp/test.py")
    assert "2 warning(s)" in result
    assert "1 error(s)" in result
    assert result


def test_format_summary_no_issues() -> None:
    result = format_summary([], "/tmp/test.py")
    assert "0 warning(s)" in result
    assert "0 error(s)" in result
