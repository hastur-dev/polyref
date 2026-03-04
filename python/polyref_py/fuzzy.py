"""Fuzzy string matching helpers using rapidfuzz."""

from __future__ import annotations

from rapidfuzz.fuzz import token_sort_ratio


def similarity(a: str, b: str) -> float:
    """Compute normalized similarity between two strings, 0.0 to 1.0."""
    assert isinstance(a, str), "a must be str"
    assert isinstance(b, str), "b must be str"
    result = token_sort_ratio(a, b) / 100.0
    assert 0.0 <= result <= 1.0, f"similarity out of range: {result}"
    return result


def find_best_match(
    target: str,
    candidates: list[str],
    threshold: float = 0.35,
) -> str | None:
    """Find the candidate most similar to target, or None if below threshold."""
    assert target, "target must be non-empty"
    if not candidates:
        return None
    best_score = 0.0
    best_match: str | None = None
    for c in candidates:
        score = similarity(target, c)
        if score > best_score:
            best_score = score
            best_match = c
    if best_score < threshold:
        return None
    assert best_match is None or best_match in candidates
    return best_match


def find_all_matches_above(
    target: str,
    candidates: list[str],
    threshold: float,
) -> list[tuple[str, float]]:
    """Return all (candidate, score) pairs above threshold, sorted descending."""
    assert target, "target must be non-empty"
    results: list[tuple[str, float]] = []
    for c in candidates:
        score = similarity(target, c)
        if score >= threshold:
            results.append((c, score))
    results.sort(key=lambda x: x[1], reverse=True)
    assert all(s >= threshold for _, s in results)
    return results
