"""CLI entry point for polyref Python checker."""

from __future__ import annotations

import json
from pathlib import Path

import typer

from polyref_py.checker import check_file
from polyref_py.output import render_issues
from polyref_py.ref_parser import load_reference_file

app = typer.Typer(name="polyref-py", help="Python reference checker")


@app.command()
def check(
    source_file: Path = typer.Argument(..., help="Python source file to check"),
    refs: list[Path] = typer.Option(
        ..., "--refs", "-r", help="Reference files (.polyref)"
    ),
    output_json: bool = typer.Option(False, "--json", help="Output as JSON"),
    lang: str = typer.Option("auto", "--lang", help="Language (auto/python/rust)"),
) -> None:
    """Check a Python source file against reference files."""
    assert source_file.exists(), f"source file not found: {source_file}"
    for ref_path in refs:
        assert ref_path.exists(), f"reference file not found: {ref_path}"
        assert ref_path.suffix == ".polyref", f"expected .polyref: {ref_path}"

    if lang == "auto":
        if source_file.suffix == ".py":
            lang = "python"
        elif source_file.suffix == ".rs":
            lang = "rust"
        else:
            typer.echo(f"Cannot auto-detect language for {source_file.suffix}")
            raise SystemExit(2)

    if lang == "rust":
        typer.echo("Rust checking not supported via Python CLI")
        raise SystemExit(2)

    ref_files = [load_reference_file(p) for p in refs]
    issues = check_file(source_file, ref_files)

    if output_json:
        data = [i.model_dump() for i in issues]
        typer.echo(json.dumps(data, indent=2))
    else:
        render_issues(issues, str(source_file))

    if issues:
        raise SystemExit(1)


@app.command(name="version")
def version_cmd() -> None:
    """Print version."""
    typer.echo("polyref-py 0.1.0")


def main() -> None:
    """Entry point for python -m polyref_py."""
    app()


if __name__ == "__main__":
    main()
