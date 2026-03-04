"""Aggregate detection rate statistics for generation tests."""

from __future__ import annotations

from pydantic import BaseModel, ConfigDict
from rich.console import Console
from rich.table import Table

from .generation_runner import GenerationResult
from .haiku_client import HAIKU_MODEL


class DetectionStats(BaseModel):
    """Aggregated detection statistics for one prompt or set of prompts."""

    prompt_id: str
    total_runs: int
    valid_runs: int
    runs_with_issues: int
    total_issues_found: int
    detection_rate: float
    avg_issues_per_run: float
    issue_kind_breakdown: dict[str, int]

    model_config = ConfigDict(frozen=True)


def compute_stats(results: list[GenerationResult], prompt_id: str) -> DetectionStats:
    """Compute detection stats from a batch of generation results."""
    total = len(results)
    assert total == len(results)
    valid = [r for r in results if r.generation_succeeded]
    valid_count = len(valid)
    with_issues = sum(1 for r in valid if r.issue_count > 0)
    total_issues = sum(r.issue_count for r in valid)
    rate = with_issues / valid_count if valid_count > 0 else 0.0
    avg = total_issues / valid_count if valid_count > 0 else 0.0

    breakdown: dict[str, int] = {}
    for r in valid:
        for issue in r.issues_found:
            kind = str(issue.kind)
            breakdown[kind] = breakdown.get(kind, 0) + 1

    stats = DetectionStats(
        prompt_id=prompt_id,
        total_runs=total,
        valid_runs=valid_count,
        runs_with_issues=with_issues,
        total_issues_found=total_issues,
        detection_rate=rate,
        avg_issues_per_run=avg,
        issue_kind_breakdown=breakdown,
    )
    assert 0.0 <= stats.detection_rate <= 1.0
    return stats


def aggregate_stats(all_stats: list[DetectionStats]) -> DetectionStats:
    """Merge multiple DetectionStats into a weighted aggregate."""
    assert len(all_stats) >= 1, "need at least one stats object"
    total_runs = sum(s.total_runs for s in all_stats)
    valid_runs = sum(s.valid_runs for s in all_stats)
    runs_with = sum(s.runs_with_issues for s in all_stats)
    total_issues = sum(s.total_issues_found for s in all_stats)
    rate = runs_with / valid_runs if valid_runs > 0 else 0.0
    avg = total_issues / valid_runs if valid_runs > 0 else 0.0

    breakdown: dict[str, int] = {}
    for s in all_stats:
        for kind, count in s.issue_kind_breakdown.items():
            breakdown[kind] = breakdown.get(kind, 0) + count

    result = DetectionStats(
        prompt_id="aggregate",
        total_runs=total_runs,
        valid_runs=valid_runs,
        runs_with_issues=runs_with,
        total_issues_found=total_issues,
        detection_rate=rate,
        avg_issues_per_run=avg,
        issue_kind_breakdown=breakdown,
    )
    assert result.total_runs == sum(s.total_runs for s in all_stats)
    assert 0.0 <= result.detection_rate <= 1.0
    return result


def format_stats_table(stats: DetectionStats) -> str:
    """Return a rich-formatted table string showing all stats fields."""
    table = Table(
        title=f"polyref detection stats \u2014 {stats.prompt_id}",
        show_header=False,
        min_width=56,
    )
    table.add_column("Key", style="bold")
    table.add_column("Value")

    table.add_row("Model", HAIKU_MODEL)
    table.add_row("Total runs", str(stats.total_runs))
    failed = stats.total_runs - stats.valid_runs
    valid_note = f"  ({failed} failed to produce code)" if failed else ""
    table.add_row("Valid runs", f"{stats.valid_runs}{valid_note}")
    table.add_row("Runs with issues", str(stats.runs_with_issues))

    rate_pct = f"{stats.detection_rate:.1%}"
    table.add_row("Detection rate", rate_pct)
    table.add_row("Avg issues/run", f"{stats.avg_issues_per_run:.1f}")

    if stats.issue_kind_breakdown:
        table.add_row("", "")
        table.add_row("Issue breakdown:", "")
        for kind, count in sorted(
            stats.issue_kind_breakdown.items(),
            key=lambda kv: kv[1],
            reverse=True,
        ):
            table.add_row(f"  {kind}", str(count))

    console = Console(width=80, force_terminal=False, no_color=True)
    with console.capture() as capture:
        console.print(table)
    result = capture.get()

    assert result, "formatted table must be non-empty"
    assert stats.prompt_id in result
    return result


def assert_detection_rate(stats: DetectionStats, minimum: float, label: str) -> None:
    """Raise AssertionError if detection rate is below the minimum."""
    assert 0.0 <= minimum <= 1.0, f"minimum must be in [0, 1], got {minimum}"
    assert label, "label must be non-empty"
    if stats.detection_rate < minimum:
        msg = (
            f"polyref detection rate for '{label}':"
            f" {stats.detection_rate:.1%}"
            f" \u2014 below minimum {minimum:.1%}\n"
            f"Breakdown: {stats.issue_kind_breakdown}\n\n"
            f"This means polyref is NOT catching the mistakes"
            f" Haiku makes on this prompt.\n"
            f"Check: are the ref files complete?"
            f" Is the fuzzy threshold too high?"
        )
        raise AssertionError(msg)
