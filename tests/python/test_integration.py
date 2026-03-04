"""Integration tests for polyref-py checker."""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

from polyref_py.checker import check_source_string
from polyref_py.models import IssueKind
from polyref_py.ref_parser import load_reference_file

FIXTURES = Path(__file__).resolve().parent / "fixtures"
REFS_DIR = Path(__file__).resolve().parent.parent.parent / "refs"
REQUESTS_POLYREF = REFS_DIR / "requests.polyref"


def _load_requests_ref() -> list:
    return [load_reference_file(REQUESTS_POLYREF)]


def test_detection_rate_benchmark() -> None:
    """Run checker on bad_python_snippets.py and verify detection rate >= 77%."""
    bad_source = (FIXTURES / "bad_python_snippets.py").read_text(encoding="utf-8")
    refs = _load_requests_ref()
    issues = check_source_string(bad_source, refs)

    detected_patterns: list[str] = []

    # BAD-1: Hallucinated method (fetch)
    if any(i.kind == IssueKind.UNKNOWN_METHOD and "fetch" in i.message for i in issues):
        detected_patterns.append("BAD-1: Hallucinated method")
    # BAD-2: Wrong constructor (Sessionn)
    if any("Sessionn" in i.message for i in issues):
        detected_patterns.append("BAD-2: Wrong constructor")
    # BAD-3: Invented class method (from_text)
    if any("from_text" in i.message for i in issues):
        detected_patterns.append("BAD-3: Invented class method")
    # BAD-4: Too many args to close
    if any(i.kind == IssueKind.TOO_MANY_ARGS for i in issues):
        detected_patterns.append("BAD-4: Too many args")
    # BAD-5: Too few args to get
    if any(i.kind == IssueKind.TOO_FEW_ARGS for i in issues):
        detected_patterns.append("BAD-5: Too few args")
    # BAD-6: Wrong attribute (status vs status_code)
    if any(
        "status" in i.message and i.kind == IssueKind.UNKNOWN_METHOD for i in issues
    ):
        detected_patterns.append("BAD-6: Wrong attribute")
    # BAD-7: Invented module (requests.network)
    if any(i.kind == IssueKind.UNKNOWN_IMPORT and "Proxy" in i.message for i in issues):
        detected_patterns.append("BAD-7: Invented module")
    # BAD-8: Typo in imported name (Sessoin)
    if any(
        i.kind == IssueKind.UNKNOWN_IMPORT and "Sessoin" in i.message for i in issues
    ):
        detected_patterns.append("BAD-8: Import typo")
    # BAD-9: Invented function (fetch_all)
    if any("fetch_all" in i.message for i in issues):
        detected_patterns.append("BAD-9: Invented function")
    # BAD-10: Hallucinated method (patch_data)
    if any("patch_data" in i.message for i in issues):
        detected_patterns.append("BAD-10: Hallucinated method")
    # BAD-11: Unknown exception (NetworkError)
    if any("NetworkError" in i.message for i in issues):
        detected_patterns.append("BAD-11: Unknown exception")
    # BAD-12: Wrong attribute (header vs headers)
    if any("header" in i.message for i in issues):
        detected_patterns.append("BAD-12: Wrong attribute")
    # BAD-13: Invented class method (create_default)
    if any("create_default" in i.message for i in issues):
        detected_patterns.append("BAD-13: Invented class method")

    detected = len(detected_patterns)
    total = 13
    rate = detected / total

    assert detected >= 10, (
        f"Detection rate too low: {detected}/{total} ({rate:.0%}). "
        f"Detected: {detected_patterns}"
    )


def test_no_false_positives_on_good_code() -> None:
    """Good code should produce zero issues."""
    good_source = (FIXTURES / "good_python_snippets.py").read_text(encoding="utf-8")
    refs = _load_requests_ref()
    issues = check_source_string(good_source, refs)
    assert issues == [], f"False positives: {[i.message for i in issues]}"


def test_full_pipeline_import_and_method() -> None:
    """Source with wrong import AND wrong method should emit both."""
    source = """from requests import Sessoin
import requests
session = Session()
session.fetch("url")
"""
    refs = _load_requests_ref()
    issues = check_source_string(source, refs)
    kinds = {i.kind for i in issues}
    assert IssueKind.UNKNOWN_IMPORT in kinds
    assert IssueKind.UNKNOWN_METHOD in kinds or IssueKind.UNKNOWN_CLASS_METHOD in kinds


def test_full_pipeline_type_context_scopes_correctly() -> None:
    """Type context should scope method checks to the right class."""
    source = """import requests
from requests import Session

session: Session = Session()
session.fetch("url")
"""
    refs = _load_requests_ref()
    issues = check_source_string(source, refs)
    method_issues = [i for i in issues if i.kind == IssueKind.UNKNOWN_METHOD]
    assert len(method_issues) >= 1
    assert "Session" in method_issues[0].message


def test_full_pipeline_arg_count_integrated() -> None:
    """Arg count check should fire for known methods."""
    source = """import requests
from requests import Session

session = Session()
session.close(True)
"""
    refs = _load_requests_ref()
    issues = check_source_string(source, refs)
    assert any(i.kind == IssueKind.TOO_MANY_ARGS for i in issues)


def test_cli_exit_code_issues(tmp_path: Path) -> None:
    """CLI should exit with 1 when issues are found."""
    bad_file = tmp_path / "bad.py"
    bad_file.write_text(
        "import requests\nfrom requests import Sessoin\n", encoding="utf-8"
    )
    result = subprocess.run(
        [
            sys.executable,
            "-m",
            "polyref_py",
            "check",
            str(bad_file),
            "--refs",
            str(REQUESTS_POLYREF),
        ],
        capture_output=True,
        text=True,
    )
    assert result.returncode == 1


def test_cli_exit_code_clean(tmp_path: Path) -> None:
    """CLI should exit with 0 when no issues."""
    good_file = tmp_path / "good.py"
    good_file.write_text(
        "import requests\nfrom requests import Session\nsession = Session()\n"
        "response = session.get('url')\nresponse.raise_for_status()\n"
        "session.close()\n",
        encoding="utf-8",
    )
    result = subprocess.run(
        [
            sys.executable,
            "-m",
            "polyref_py",
            "check",
            str(good_file),
            "--refs",
            str(REQUESTS_POLYREF),
        ],
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0


def test_cli_json_output(tmp_path: Path) -> None:
    """--json flag should output valid JSON."""
    bad_file = tmp_path / "bad.py"
    bad_file.write_text(
        "import requests\nfrom requests import Sessoin\n", encoding="utf-8"
    )
    result = subprocess.run(
        [
            sys.executable,
            "-m",
            "polyref_py",
            "check",
            str(bad_file),
            "--refs",
            str(REQUESTS_POLYREF),
            "--json",
        ],
        capture_output=True,
        text=True,
    )
    data = json.loads(result.stdout)
    assert isinstance(data, list)
    assert len(data) >= 1
