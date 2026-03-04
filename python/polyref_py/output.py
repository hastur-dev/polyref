"""Output formatting for polyref Python checker — pyright-style output."""

from __future__ import annotations

from rich.console import Console

from polyref_py.models import Issue, IssueLevel

_CONSOLE = Console(stderr=True)


def format_issue_pyright_style(issue: Issue, source_path: str) -> str:
    """Format an issue in pyright-compatible style."""
    level = issue.level.value
    kind = issue.kind.value
    header = f"{level}[{kind}]: {issue.message}"
    location = f"  {source_path}:{issue.line_number}:{issue.col_number}"
    lines = [header, location]
    if issue.suggestion:
        sim_part = f" (similarity: {issue.similarity:.2f})" if issue.similarity else ""
        lines.append(f"  Did you mean '{issue.suggestion}'?{sim_part}")
    result = "\n".join(lines)
    assert source_path in result
    assert str(issue.line_number) in result
    assert result
    return result


def format_summary(issues: list[Issue], source_path: str) -> str:
    """Format a summary line with issue counts."""
    warnings = sum(1 for i in issues if i.level == IssueLevel.WARNING)
    errors = sum(1 for i in issues if i.level == IssueLevel.ERROR)
    result = f"polyref: {warnings} warning(s), {errors} error(s) in {source_path}"
    assert warnings + errors <= len(issues)
    assert result
    return result


def render_issues(issues: list[Issue], source_path: str) -> None:
    """Print formatted issues using rich."""
    if not issues:
        _CONSOLE.print(f"[green]polyref: no issues in {source_path}[/green]")
        return
    for issue in issues:
        text = format_issue_pyright_style(issue, source_path)
        if issue.level == IssueLevel.ERROR:
            _CONSOLE.print(f"[red]{text}[/red]")
        elif issue.level == IssueLevel.WARNING:
            _CONSOLE.print(f"[yellow]{text}[/yellow]")
        else:
            _CONSOLE.print(text)
    _CONSOLE.print(format_summary(issues, source_path))
