"""N-shot batch runner: generate code and check it with polyref."""

from __future__ import annotations

import logging

from polyref_py.checker import check_source_string
from polyref_py.models import Issue, ReferenceFile
from pydantic import BaseModel, ConfigDict
from rich.progress import Progress

from .code_extractor import extract_code_or_raw
from .haiku_client import HaikuClient

logger = logging.getLogger(__name__)


class GenerationResult(BaseModel):
    """Result of a single Haiku generation + polyref check."""

    prompt_id: str
    run_index: int
    raw_output: str
    extracted_code: str
    issues_found: list[Issue]
    issue_count: int
    generation_succeeded: bool

    model_config = ConfigDict(frozen=True)


def _lang_hint(lang: str) -> str:
    """Map language name to markdown fence hint."""
    return {"rust": "rust", "python": "python"}.get(lang, "")


def run_generation_batch(
    client: HaikuClient,
    prompt: str,
    prompt_id: str,
    refs: list[ReferenceFile],
    lang: str,
    n: int = 20,
) -> list[GenerationResult]:
    """Generate code n times, run polyref on each, return results."""
    assert lang in {"rust", "python"}, f"lang must be 'rust' or 'python', got '{lang}'"
    assert n >= 1, f"n must be >= 1, got {n}"

    raw_outputs = client.generate_code_n_times(prompt, n)
    results: list[GenerationResult] = []
    hint = _lang_hint(lang)

    with Progress(transient=True) as progress:
        task = progress.add_task(f"Checking {prompt_id}", total=n)
        for i, raw in enumerate(raw_outputs):
            code = extract_code_or_raw(raw, lang_hint=hint) if raw else ""
            succeeded = bool(code.strip())
            issues: list[Issue] = []

            if succeeded and refs:
                try:
                    issues = check_source_string(code, refs)
                except Exception:
                    logger.warning(
                        "polyref check failed for %s run %d",
                        prompt_id,
                        i,
                        exc_info=True,
                    )

            results.append(
                GenerationResult(
                    prompt_id=prompt_id,
                    run_index=i,
                    raw_output=raw,
                    extracted_code=code,
                    issues_found=issues,
                    issue_count=len(issues),
                    generation_succeeded=succeeded,
                )
            )
            progress.advance(task)

    assert len(results) == n, f"expected {n} results, got {len(results)}"
    return results


def filter_valid_results(
    results: list[GenerationResult],
) -> list[GenerationResult]:
    """Return only results where generation succeeded."""
    valid = [r for r in results if r.generation_succeeded]
    assert all(r.generation_succeeded for r in valid)
    assert len(valid) <= len(results)
    return valid
