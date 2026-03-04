"""Unit tests for generation harness internals — no real API calls."""

from __future__ import annotations

from collections.abc import Iterator
from types import SimpleNamespace
from typing import Any
from unittest.mock import MagicMock, patch

import pytest
from polyref_py.models import Issue, IssueKind, IssueLevel

from . import haiku_client as haiku_client_mod
from .code_extractor import (
    extract_all_code_blocks,
    extract_code_block,
    extract_code_or_raw,
)
from .generation_runner import GenerationResult, filter_valid_results
from .haiku_client import HAIKU_MODEL, HaikuClient
from .stats import (
    DetectionStats,
    aggregate_stats,
    assert_detection_rate,
    compute_stats,
    format_stats_table,
)

# ---------------------------------------------------------------------------
# HaikuClient tests
# ---------------------------------------------------------------------------


def _make_mock_response(text: str) -> Any:
    """Build a mock Anthropic Messages response."""
    block = SimpleNamespace(text=text)
    return SimpleNamespace(content=[block])


@pytest.fixture()
def mock_anthropic() -> Iterator[MagicMock]:
    """Patch anthropic.Anthropic and return the mock client."""
    mock_cls = MagicMock()
    mock_client = MagicMock()
    mock_cls.return_value = mock_client
    mock_client.messages.create.return_value = _make_mock_response(
        "```python\nx = 1\n```"
    )
    with patch.object(haiku_client_mod.anthropic, "Anthropic", mock_cls):
        yield mock_client


def test_generate_code_returns_nonempty(mock_anthropic: MagicMock) -> None:
    """generate_code returns the raw text from Haiku."""
    client = HaikuClient(api_key="test-key")
    result = client.generate_code("Write x = 1")
    assert result
    assert "x = 1" in result


def test_generate_code_n_times_correct_count(mock_anthropic: MagicMock) -> None:
    """generate_code_n_times returns exactly n results."""
    client = HaikuClient(api_key="test-key")
    results = client.generate_code_n_times("prompt", n=5)
    assert len(results) == 5
    assert all(r for r in results)


def test_generate_code_n_times_handles_failure(mock_anthropic: MagicMock) -> None:
    """Failed calls produce empty strings but don't abort the batch."""
    call_count = 0

    def side_effect(**_kwargs: Any) -> Any:
        nonlocal call_count
        call_count += 1
        if call_count == 3:
            raise RuntimeError("API error")
        return _make_mock_response("```python\ny = 2\n```")

    mock_anthropic.messages.create.side_effect = side_effect
    client = HaikuClient(api_key="test-key")
    results = client.generate_code_n_times("prompt", n=5)
    assert len(results) == 5
    assert "" in results


def test_get_model_name_is_haiku() -> None:
    """get_model_name returns the hardcoded Haiku model string."""
    with patch.object(haiku_client_mod.anthropic, "Anthropic"):
        client = HaikuClient(api_key="test-key")
    name = client.get_model_name()
    assert name == "claude-haiku-4-5-20251001"
    assert name == HAIKU_MODEL


# ---------------------------------------------------------------------------
# Code extractor tests
# ---------------------------------------------------------------------------


def test_extract_code_block_fenced() -> None:
    """Fenced python block extracts inner code."""
    text = "```python\nx = 1\n```"
    result = extract_code_block(text, lang_hint="python")
    assert result == "x = 1"
    assert "```" not in result


def test_extract_code_block_no_fence() -> None:
    """Plain text without fences returns None."""
    result = extract_code_block("just some text with no fences")
    assert result is None


def test_extract_code_block_prefers_lang_hint() -> None:
    """When multiple blocks exist, prefer the one matching lang_hint."""
    text = "```javascript\nconsole.log(1)\n```\n```python\nx = 1\n```"
    result = extract_code_block(text, lang_hint="python")
    assert result == "x = 1"


def test_extract_all_code_blocks_multiple() -> None:
    """All fenced blocks are extracted in order."""
    text = "```python\na = 1\n```\ntext\n```rust\nlet b = 2;\n```"
    blocks = extract_all_code_blocks(text)
    assert len(blocks) == 2
    assert blocks[0] == "a = 1"
    assert blocks[1] == "let b = 2;"


def test_extract_all_code_blocks_empty() -> None:
    """No fences returns empty list."""
    assert extract_all_code_blocks("no code here") == []


def test_extract_code_or_raw_with_fence() -> None:
    """Prefers fenced block when available."""
    text = "```python\nx = 1\n```"
    assert extract_code_or_raw(text, lang_hint="python") == "x = 1"


def test_extract_code_or_raw_fallback() -> None:
    """Falls back to raw text when no fence found."""
    raw = "x = 1"
    result = extract_code_or_raw(raw)
    assert result == raw


def test_extract_code_or_raw_empty_input() -> None:
    """Empty input returns empty string."""
    assert extract_code_or_raw("") == ""
    assert extract_code_or_raw("   ") == ""


# ---------------------------------------------------------------------------
# Stats tests
# ---------------------------------------------------------------------------


def _make_result(
    succeeded: bool = True,
    issue_kinds: list[IssueKind] | None = None,
) -> GenerationResult:
    """Build a GenerationResult for testing."""
    issues: list[Issue] = []
    if issue_kinds:
        for kind in issue_kinds:
            issues.append(
                Issue(
                    kind=kind,
                    level=IssueLevel.ERROR,
                    message=f"test {kind}",
                    line_number=1,
                )
            )
    return GenerationResult(
        prompt_id="test",
        run_index=0,
        raw_output="raw",
        extracted_code="code" if succeeded else "",
        issues_found=issues,
        issue_count=len(issues),
        generation_succeeded=succeeded,
    )


def test_compute_stats_detection_rate() -> None:
    """8 of 10 runs with issues → detection_rate == 0.8."""
    results = [
        _make_result(issue_kinds=[IssueKind.UNKNOWN_METHOD]) for _ in range(8)
    ] + [_make_result() for _ in range(2)]
    stats = compute_stats(results, "test_prompt")
    assert abs(stats.detection_rate - 0.8) < 1e-9
    assert stats.total_runs == 10


def test_compute_stats_issue_breakdown() -> None:
    """Breakdown dict contains expected keys."""
    results = [
        _make_result(issue_kinds=[IssueKind.UNKNOWN_METHOD, IssueKind.TOO_MANY_ARGS]),
        _make_result(issue_kinds=[IssueKind.UNKNOWN_METHOD]),
    ]
    stats = compute_stats(results, "breakdown_test")
    assert "unknown-method" in stats.issue_kind_breakdown
    assert stats.issue_kind_breakdown["unknown-method"] == 2
    assert stats.issue_kind_breakdown["too-many-args"] == 1


def test_aggregate_stats_sums_runs() -> None:
    """Two stats with 10 runs each → aggregate has 20 total_runs."""
    s1 = DetectionStats(
        prompt_id="a",
        total_runs=10,
        valid_runs=10,
        runs_with_issues=8,
        total_issues_found=12,
        detection_rate=0.8,
        avg_issues_per_run=1.2,
        issue_kind_breakdown={"unknown-method": 8},
    )
    s2 = DetectionStats(
        prompt_id="b",
        total_runs=10,
        valid_runs=10,
        runs_with_issues=6,
        total_issues_found=9,
        detection_rate=0.6,
        avg_issues_per_run=0.9,
        issue_kind_breakdown={"unknown-method": 4, "too-many-args": 2},
    )
    agg = aggregate_stats([s1, s2])
    assert agg.total_runs == 20
    assert agg.valid_runs == 20
    assert agg.runs_with_issues == 14
    assert agg.prompt_id == "aggregate"
    assert agg.issue_kind_breakdown["unknown-method"] == 12


def test_assert_detection_rate_passes() -> None:
    """No error raised when rate meets minimum."""
    stats = DetectionStats(
        prompt_id="ok",
        total_runs=10,
        valid_runs=10,
        runs_with_issues=9,
        total_issues_found=15,
        detection_rate=0.9,
        avg_issues_per_run=1.5,
        issue_kind_breakdown={},
    )
    assert_detection_rate(stats, 0.8, "test_label")  # should not raise


def test_assert_detection_rate_fails_with_message() -> None:
    """AssertionError raised with descriptive message when rate is below min."""
    stats = DetectionStats(
        prompt_id="low",
        total_runs=10,
        valid_runs=10,
        runs_with_issues=5,
        total_issues_found=7,
        detection_rate=0.5,
        avg_issues_per_run=0.7,
        issue_kind_breakdown={"unknown-method": 5},
    )
    with pytest.raises(AssertionError, match=r"50\.0%"):
        assert_detection_rate(stats, 0.8, "fail_label")


def test_format_stats_table_contains_prompt_id() -> None:
    """Formatted table includes the prompt_id."""
    stats = DetectionStats(
        prompt_id="my_prompt",
        total_runs=10,
        valid_runs=10,
        runs_with_issues=7,
        total_issues_found=12,
        detection_rate=0.7,
        avg_issues_per_run=1.2,
        issue_kind_breakdown={"unknown-method": 7},
    )
    table = format_stats_table(stats)
    assert table
    assert "my_prompt" in table


# ---------------------------------------------------------------------------
# filter_valid_results tests
# ---------------------------------------------------------------------------


def test_filter_valid_results() -> None:
    """Only succeeded results are returned."""
    results = [
        _make_result(succeeded=True),
        _make_result(succeeded=False),
        _make_result(succeeded=True),
    ]
    valid = filter_valid_results(results)
    assert len(valid) == 2
    assert all(r.generation_succeeded for r in valid)
