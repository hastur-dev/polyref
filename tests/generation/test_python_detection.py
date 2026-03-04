"""Python detection rate tests — calls Haiku API (requires ANTHROPIC_API_KEY)."""

from __future__ import annotations

import os

import pytest
from polyref_py.models import ReferenceFile

from .generation_runner import filter_valid_results, run_generation_batch
from .haiku_client import HaikuClient
from .prompts.python_requests_prompts import PYTHON_REQUESTS_PROMPTS
from .stats import (
    DetectionStats,
    aggregate_stats,
    assert_detection_rate,
    compute_stats,
    format_stats_table,
)

GENERATIONS_PER_PROMPT = 20
MIN_DETECTION_RATE = 0.60

pytestmark = pytest.mark.generation


def _gen_count() -> int:
    return int(os.getenv("POLYREF_GEN_COUNT", str(GENERATIONS_PER_PROMPT)))


def test_python_requests_session_detection_rate(
    haiku_client: HaikuClient, python_refs: list[ReferenceFile]
) -> None:
    """Haiku generates requests Session code Nx. polyref must flag >=60%."""
    prompt_id, prompt = PYTHON_REQUESTS_PROMPTS[0]
    n = _gen_count()
    results = run_generation_batch(
        haiku_client, prompt, prompt_id, python_refs, "python", n
    )
    valid = filter_valid_results(results)
    stats = compute_stats(valid, prompt_id)
    print(format_stats_table(stats))
    assert len(valid) >= n * 0.7, f"Too many failed generations: {len(valid)}/{n} valid"
    assert_detection_rate(stats, MIN_DETECTION_RATE, prompt_id)


def test_python_requests_error_handling_detection_rate(
    haiku_client: HaikuClient, python_refs: list[ReferenceFile]
) -> None:
    """Haiku generates error handling code Nx. polyref must flag >=60%."""
    prompt_id, prompt = PYTHON_REQUESTS_PROMPTS[1]
    n = _gen_count()
    results = run_generation_batch(
        haiku_client, prompt, prompt_id, python_refs, "python", n
    )
    valid = filter_valid_results(results)
    stats = compute_stats(valid, prompt_id)
    print(format_stats_table(stats))
    assert len(valid) >= n * 0.7
    assert_detection_rate(stats, MIN_DETECTION_RATE, prompt_id)


def test_python_requests_auth_detection_rate(
    haiku_client: HaikuClient, python_refs: list[ReferenceFile]
) -> None:
    """Haiku generates auth code Nx. polyref must flag >=60%."""
    prompt_id, prompt = PYTHON_REQUESTS_PROMPTS[2]
    n = _gen_count()
    results = run_generation_batch(
        haiku_client, prompt, prompt_id, python_refs, "python", n
    )
    valid = filter_valid_results(results)
    stats = compute_stats(valid, prompt_id)
    print(format_stats_table(stats))
    assert len(valid) >= n * 0.7
    assert_detection_rate(stats, MIN_DETECTION_RATE, prompt_id)


def test_python_requests_response_detection_rate(
    haiku_client: HaikuClient, python_refs: list[ReferenceFile]
) -> None:
    """Haiku generates response check code Nx. polyref must flag >=60%."""
    prompt_id, prompt = PYTHON_REQUESTS_PROMPTS[3]
    n = _gen_count()
    results = run_generation_batch(
        haiku_client, prompt, prompt_id, python_refs, "python", n
    )
    valid = filter_valid_results(results)
    stats = compute_stats(valid, prompt_id)
    print(format_stats_table(stats))
    assert len(valid) >= n * 0.7
    assert_detection_rate(stats, MIN_DETECTION_RATE, prompt_id)


def test_python_requests_retry_detection_rate(
    haiku_client: HaikuClient, python_refs: list[ReferenceFile]
) -> None:
    """Haiku generates retry logic code Nx. polyref must flag >=60%."""
    prompt_id, prompt = PYTHON_REQUESTS_PROMPTS[4]
    n = _gen_count()
    results = run_generation_batch(
        haiku_client, prompt, prompt_id, python_refs, "python", n
    )
    valid = filter_valid_results(results)
    stats = compute_stats(valid, prompt_id)
    print(format_stats_table(stats))
    assert len(valid) >= n * 0.7
    assert_detection_rate(stats, MIN_DETECTION_RATE, prompt_id)


def test_python_aggregate_detection_rate(
    haiku_client: HaikuClient, python_refs: list[ReferenceFile]
) -> None:
    """Run all Python prompts and check the aggregate detection rate."""
    n = _gen_count()
    all_stats: list[DetectionStats] = []

    for prompt_id, prompt in PYTHON_REQUESTS_PROMPTS:
        results = run_generation_batch(
            haiku_client, prompt, prompt_id, python_refs, "python", n
        )
        valid = filter_valid_results(results)
        all_stats.append(compute_stats(valid, prompt_id))

    agg = aggregate_stats(all_stats)
    print(format_stats_table(agg))

    assert agg.total_runs >= len(PYTHON_REQUESTS_PROMPTS) * n * 0.7
    assert_detection_rate(agg, MIN_DETECTION_RATE, "python_aggregate")
