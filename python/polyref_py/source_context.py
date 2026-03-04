"""Source context builder: imports, types, and reference scoping."""

from __future__ import annotations

from pydantic import BaseModel, ConfigDict

from polyref_py.import_checker import ImportStatement, extract_imports
from polyref_py.models import ReferenceFile
from polyref_py.type_inference import build_type_context


class SourceContext(BaseModel):
    """Context extracted from a Python source file."""

    imported_packages: list[str]
    imported_items: dict[str, str]
    type_bindings: dict[str, str]
    import_statements: list[ImportStatement]

    model_config = ConfigDict(frozen=True)


def build_source_context(source: str) -> SourceContext:
    """Build a SourceContext from Python source code."""
    stmts = extract_imports(source)
    lines = source.splitlines()
    type_ctx = build_type_context(lines)

    packages: list[str] = []
    seen_pkgs: set[str] = set()
    imported_items: dict[str, str] = {}

    for stmt in stmts:
        top_pkg = stmt.module_path.split(".")[0]
        if top_pkg not in seen_pkgs:
            packages.append(top_pkg)
            seen_pkgs.add(top_pkg)
        if stmt.is_from_import:
            key = stmt.imported_name
            val = f"{stmt.module_path}.{stmt.imported_name}"
            if key:
                imported_items[key] = val

    assert len(packages) == len(set(packages)), "no duplicate packages"
    assert all(k for k in imported_items), "no empty keys"
    assert all(v for v in imported_items.values()), "no empty values"

    return SourceContext(
        imported_packages=packages,
        imported_items=imported_items,
        type_bindings=type_ctx,
        import_statements=stmts,
    )


def select_relevant_refs(
    ctx: SourceContext,
    all_refs: list[ReferenceFile],
) -> list[ReferenceFile]:
    """Return only refs whose library_name is imported."""
    if not ctx.imported_packages:
        return list(all_refs)
    result = [rf for rf in all_refs if rf.library_name in ctx.imported_packages]
    assert len(result) <= len(all_refs)
    return result
