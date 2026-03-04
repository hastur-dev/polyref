"""Extract code blocks from Haiku markdown output."""

from __future__ import annotations

import re

_FENCED_BLOCK_RE = re.compile(r"```(\w*)\n(.*?)```", re.DOTALL)


def extract_code_block(text: str, lang_hint: str = "") -> str | None:
    """Extract the first fenced code block, preferring lang_hint if given."""
    assert isinstance(text, str)
    matches = list(_FENCED_BLOCK_RE.finditer(text))
    if not matches:
        return None

    # Prefer block whose lang tag matches hint
    if lang_hint:
        for m in matches:
            if m.group(1).lower() == lang_hint.lower():
                code = m.group(2).strip()
                assert "```" not in code, "extracted block contains backticks"
                assert code, "extracted block is empty"
                return code

    # Fall back to the first block
    code = matches[0].group(2).strip()
    if not code:
        return None
    assert "```" not in code, "extracted block contains backticks"
    return code


def extract_all_code_blocks(text: str) -> list[str]:
    """Return content of ALL fenced code blocks in order."""
    assert isinstance(text, str)
    results: list[str] = []
    for m in _FENCED_BLOCK_RE.finditer(text):
        code = m.group(2).strip()
        if code:
            assert "```" not in code
            results.append(code)
    assert isinstance(results, list)
    return results


def extract_code_or_raw(text: str, lang_hint: str = "") -> str:
    """Extract code block or fall back to raw text."""
    assert isinstance(text, str)
    if not text.strip():
        return ""
    block = extract_code_block(text, lang_hint=lang_hint)
    if block is not None:
        return block
    result = text.strip()
    assert result or not text.strip(), "non-empty input produced empty output"
    return result
