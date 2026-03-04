"""Type inference for Python variable bindings."""

from __future__ import annotations

import re

_ANNOTATED_RE = re.compile(
    r"^\s*(\w+)\s*:\s*([A-Za-z_]\w*)"
    r"(?:\[.*\])?"
    r"(?:\s*\|.*)?"
    r"(?:\s*=.*)?"
    r"\s*$"
)

_CONSTRUCTOR_RE = re.compile(r"^\s*(\w+)\s*=\s*([A-Z]\w*)\s*\(")

_KNOWN_BUILTINS = frozenset(
    {
        "int",
        "str",
        "float",
        "bool",
        "bytes",
        "list",
        "dict",
        "set",
        "tuple",
        "frozenset",
        "type",
        "object",
        "complex",
        "range",
        "bytearray",
        "memoryview",
    }
)


def infer_annotated_assignment(line: str) -> tuple[str, str] | None:
    """Match 'var: TypeName = ...' or 'var: TypeName' and return (var, type)."""
    m = _ANNOTATED_RE.match(line)
    if not m:
        return None
    var_name = m.group(1)
    type_name = m.group(2)
    if not var_name or " " in var_name:
        return None
    if not (type_name[0].isupper() or type_name in _KNOWN_BUILTINS):
        return None
    assert var_name, "var_name must be non-empty"
    assert type_name, "type_name must be non-empty"
    return (var_name, type_name)


def infer_constructor_assignment(line: str) -> tuple[str, str] | None:
    """Match 'var = ClassName(...)' and return (var, class_name)."""
    m = _CONSTRUCTOR_RE.match(line)
    if not m:
        return None
    var_name = m.group(1)
    class_name = m.group(2)
    if not var_name or " " in var_name:
        return None
    assert class_name[0].isupper(), f"class_name must start uppercase: {class_name}"
    assert " " not in var_name
    return (var_name, class_name)


def build_type_context(source_lines: list[str]) -> dict[str, str]:
    """Build variable → type mapping from source lines."""
    ctx: dict[str, str] = {}
    for line in source_lines:
        result = infer_annotated_assignment(line)
        if result:
            ctx[result[0]] = result[1]
            continue
        result = infer_constructor_assignment(line)
        if result:
            ctx[result[0]] = result[1]

    assert all(k for k in ctx), "all keys must be non-empty"
    assert all(v for v in ctx.values()), "all values must be non-empty"
    return ctx


def resolve_type(var_name: str, ctx: dict[str, str]) -> str | None:
    """Resolve a variable name to its inferred type, or None."""
    assert var_name, "var_name must be non-empty"
    result = ctx.get(var_name)
    if result is not None:
        assert result, "resolved type must be non-empty"
    return result
