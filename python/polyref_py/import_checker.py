"""Import statement extraction and validation using libcst."""

from __future__ import annotations

from collections.abc import Sequence

import libcst as cst
from pydantic import BaseModel, ConfigDict

from polyref_py.fuzzy import find_best_match
from polyref_py.models import (
    EntryKind,
    Issue,
    IssueKind,
    IssueLevel,
    ReferenceFile,
)


class ImportStatement(BaseModel):
    """A single import statement extracted from source."""

    module_path: str
    imported_name: str
    alias: str | None = None
    line_number: int
    is_from_import: bool

    model_config = ConfigDict(frozen=True)


def _module_to_str(
    module: cst.Attribute | cst.Name | None,
) -> str:
    """Convert a libcst module node to a dotted string."""
    if module is None:
        return ""
    if isinstance(module, cst.Name):
        return module.value
    if isinstance(module, cst.Attribute):
        prefix = _module_to_str(module.value)  # type: ignore[arg-type]
        return f"{prefix}.{module.attr.value}"
    return ""


def _name_to_str(name: cst.BaseExpression) -> str:
    """Convert a libcst name node to a string."""
    if isinstance(name, cst.Name):
        return name.value
    if isinstance(name, cst.Attribute):
        prefix = _name_to_str(name.value)
        return f"{prefix}.{name.attr.value}"
    return ""


def _extract_alias_str(alias: cst.ImportAlias) -> str | None:
    """Extract the alias string from an ImportAlias node."""
    if (
        alias.asname
        and isinstance(alias.asname, cst.AsName)
        and isinstance(alias.asname.name, cst.Name)
    ):
        return alias.asname.name.value
    return None


class _ImportVisitor(cst.CSTVisitor):
    """CST visitor that collects import statements with position info."""

    METADATA_DEPENDENCIES = (cst.metadata.PositionProvider,)

    def __init__(self) -> None:
        self.imports: list[ImportStatement] = []

    def visit_ImportFrom(self, node: cst.ImportFrom) -> None:  # noqa: N802
        """Handle 'from X import Y' statements."""
        pos = self.get_metadata(cst.metadata.PositionProvider, node)
        line_no: int = pos.start.line  # type: ignore[union-attr]
        module_path = _module_to_str(node.module)
        if isinstance(node.names, cst.ImportStar):
            return
        assert isinstance(node.names, (list, tuple))
        for alias in node.names:
            assert isinstance(alias, cst.ImportAlias)
            self.imports.append(
                ImportStatement(
                    module_path=module_path,
                    imported_name=_name_to_str(alias.name),
                    alias=_extract_alias_str(alias),
                    line_number=line_no,
                    is_from_import=True,
                )
            )

    def visit_Import(self, node: cst.Import) -> None:  # noqa: N802
        """Handle 'import X' statements."""
        pos = self.get_metadata(cst.metadata.PositionProvider, node)
        line_no: int = pos.start.line  # type: ignore[union-attr]
        if isinstance(node.names, cst.ImportStar):
            return
        assert isinstance(node.names, (list, tuple))
        for alias in node.names:
            assert isinstance(alias, cst.ImportAlias)
            name = _name_to_str(alias.name)
            self.imports.append(
                ImportStatement(
                    module_path=name,
                    imported_name=name,
                    alias=_extract_alias_str(alias),
                    line_number=line_no,
                    is_from_import=False,
                )
            )


def extract_imports(source: str) -> list[ImportStatement]:
    """Extract all import statements from Python source using libcst."""
    try:
        tree = cst.parse_module(source)
    except cst.ParserSyntaxError:
        return []

    wrapper = cst.metadata.MetadataWrapper(tree)
    visitor = _ImportVisitor()
    wrapper.visit(visitor)

    for imp in visitor.imports:
        assert imp.module_path, "module_path must be non-empty"
        assert imp.line_number >= 1
    return visitor.imports


def _get_all_entry_names(refs: Sequence[ReferenceFile]) -> list[str]:
    """Collect all entry names from reference files."""
    names: set[str] = set()
    for rf in refs:
        for e in rf.entries:
            names.add(e.name)
    return list(names)


def _get_module_names(refs: Sequence[ReferenceFile]) -> list[str]:
    """Collect all module entry names from reference files."""
    names: set[str] = set()
    for rf in refs:
        for e in rf.entries:
            if e.kind == EntryKind.MODULE:
                names.add(e.name)
    return list(names)


def _get_top_packages(refs: Sequence[ReferenceFile]) -> list[str]:
    """Get top-level package names from reference files."""
    return [rf.library_name for rf in refs if rf.library_name]


def check_import(
    stmt: ImportStatement,
    refs: list[ReferenceFile],
) -> list[Issue]:
    """Check a single import statement against reference files."""
    assert stmt.module_path, "module_path must be non-empty"

    top_packages = _get_top_packages(refs)
    top_level = stmt.module_path.split(".")[0]

    if top_level not in top_packages:
        return []

    if stmt.is_from_import:
        all_names = _get_all_entry_names(refs)
        name = stmt.imported_name
        if name in all_names:
            return []
        suggestion = find_best_match(name, all_names)
        msg = f"unknown import '{name}' from '{stmt.module_path}'"
        return [
            Issue(
                kind=IssueKind.UNKNOWN_IMPORT,
                level=IssueLevel.ERROR,
                message=msg,
                line_number=stmt.line_number,
                suggestion=suggestion,
            )
        ]

    module_names = _get_module_names(refs)
    all_mods = module_names + top_packages
    if stmt.module_path in all_mods:
        return []
    suggestion = find_best_match(stmt.module_path, all_mods)
    msg = f"unknown module '{stmt.module_path}'"
    return [
        Issue(
            kind=IssueKind.UNKNOWN_IMPORT,
            level=IssueLevel.ERROR,
            message=msg,
            line_number=stmt.line_number,
            suggestion=suggestion,
        )
    ]


def check_all_imports(
    stmts: list[ImportStatement],
    refs: list[ReferenceFile],
) -> list[Issue]:
    """Check all import statements against references."""
    if not stmts:
        return []
    issues: list[Issue] = []
    for stmt in stmts:
        issues.extend(check_import(stmt, refs))
    return issues
