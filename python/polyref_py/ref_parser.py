"""Parser for .polyref reference files with @lang python."""

from __future__ import annotations

import re
from pathlib import Path

from polyref_py.errors import (
    ReferenceFileNotFound,
    ReferenceParseError,
)
from polyref_py.models import EntryKind, ReferenceEntry, ReferenceFile

_ARG_HINT_RE = re.compile(
    r"\[(?:min_args\s*=\s*(\d+))?"
    r"(?:\s*,\s*max_args\s*=\s*(\d+))?\]"
)
_FN_RE = re.compile(
    r"@fn\s+(\w+)\s*\(([^)]*)\)"
    r"(?:\s*->\s*\S+)?"
    r"(?:\s*(\[.*?\]))?"
)
_EXCEPTION_RE = re.compile(r"@exception\s+(\w+)")
_MODULE_RE = re.compile(r"@module\s+(\S+)")
_METHOD_RE = re.compile(
    r"@(method|class_method|static_method)\s+(\w+)\s*\(([^)]*)\)"
    r"(?:\s*->\s*\S+)?"
    r"(?:\s*(\[.*?\]))?"
)
_FIELD_RE = re.compile(r"@field\s+(\w+)\s*:\s*(\S+)")
_CLASS_BLOCK_RE = re.compile(r"@class\s+(\w+)\s*\{([^}]*)\}", re.DOTALL)
_HEADER_LIB_RE = re.compile(r"#\s*Library:\s*(.+)")
_HEADER_VER_RE = re.compile(r"#\s*Version:\s*(.+)")


def parse_arg_hint(hint_str: str) -> tuple[int | None, int | None]:
    """Parse '[min_args=N, max_args=M]' into (min, max)."""
    if not hint_str:
        return (None, None)
    m = _ARG_HINT_RE.search(hint_str)
    if not m:
        return (None, None)
    min_v = int(m.group(1)) if m.group(1) else None
    max_v = int(m.group(2)) if m.group(2) else None
    if min_v is not None and max_v is not None:
        assert min_v <= max_v, f"min_args ({min_v}) > max_args ({max_v})"
    return (min_v, max_v)


def _has_variadic(params: str) -> bool:
    """Check if param string contains **kwargs or *args."""
    return "**" in params or ("*" in params and not params.strip().startswith("*,"))


def parse_function_line(line: str, lib_name: str) -> ReferenceEntry | None:
    """Parse '@fn name(args) -> type [hint]' into a ReferenceEntry."""
    m = _FN_RE.search(line)
    if not m:
        return None
    name = m.group(1)
    params = m.group(2)
    hint_str = m.group(3) or ""
    min_a, max_a = parse_arg_hint(hint_str)
    if _has_variadic(params) and max_a is None:
        max_a = None
    entry = ReferenceEntry(
        name=name,
        kind=EntryKind.FUNCTION,
        type_context=None,
        min_args=min_a,
        max_args=max_a,
        source_lib=lib_name,
    )
    assert entry.kind == EntryKind.FUNCTION
    assert entry.type_context is None
    return entry


def parse_exception_line(line: str, lib_name: str) -> ReferenceEntry | None:
    """Parse '@exception ExceptionName' into a ReferenceEntry."""
    m = _EXCEPTION_RE.search(line)
    if not m:
        return None
    name = m.group(1)
    assert name[0].isupper(), f"exception name must start uppercase: {name}"
    return ReferenceEntry(
        name=name,
        kind=EntryKind.EXCEPTION,
        source_lib=lib_name,
    )


def parse_class_block(
    block_text: str, class_name: str, lib_name: str
) -> list[ReferenceEntry]:
    """Extract @method, @class_method, @static_method, @field from a class block."""
    entries: list[ReferenceEntry] = []
    for line in block_text.splitlines():
        line = line.strip()
        fm = _FIELD_RE.search(line)
        if fm:
            entries.append(
                ReferenceEntry(
                    name=fm.group(1),
                    kind=EntryKind.FIELD,
                    type_context=class_name,
                    source_lib=lib_name,
                )
            )
            continue
        mm = _METHOD_RE.search(line)
        if mm:
            method_kind_str = mm.group(1)
            method_name = mm.group(2)
            params = mm.group(3)
            hint_str = mm.group(4) or ""
            min_a, max_a = parse_arg_hint(hint_str)
            if _has_variadic(params) and max_a is None:
                max_a = None
            kind_map = {
                "method": EntryKind.METHOD,
                "class_method": EntryKind.CLASS_METHOD,
                "static_method": EntryKind.STATIC_METHOD,
            }
            entries.append(
                ReferenceEntry(
                    name=method_name,
                    kind=kind_map[method_kind_str],
                    type_context=class_name,
                    min_args=min_a,
                    max_args=max_a,
                    source_lib=lib_name,
                )
            )
    for e in entries:
        assert e.type_context == class_name
        assert e.name
    return entries


def parse_reference_file(content: str) -> ReferenceFile:
    """Parse a .polyref file with @lang python."""
    if "@lang python" not in content:
        raise ReferenceParseError("missing '@lang python' directive")

    lib_name = ""
    version = ""
    lib_m = _HEADER_LIB_RE.search(content)
    if lib_m:
        lib_name = lib_m.group(1).strip()
    ver_m = _HEADER_VER_RE.search(content)
    if ver_m:
        version = ver_m.group(1).strip()

    entries: list[ReferenceEntry] = []
    seen: set[tuple[str, str, str | None]] = set()

    # Parse class blocks
    for cm in _CLASS_BLOCK_RE.finditer(content):
        cname = cm.group(1)
        block = cm.group(2)
        # Add the class itself
        cls_key = (cname, EntryKind.CLASS, None)
        if cls_key not in seen:
            entries.append(
                ReferenceEntry(
                    name=cname,
                    kind=EntryKind.CLASS,
                    source_lib=lib_name,
                )
            )
            seen.add(cls_key)
        for e in parse_class_block(block, cname, lib_name):
            key = (e.name, e.kind, e.type_context)
            if key not in seen:
                entries.append(e)
                seen.add(key)

    # Parse top-level lines (outside class blocks)
    class_spans = [(cm.start(), cm.end()) for cm in _CLASS_BLOCK_RE.finditer(content)]

    for line in content.splitlines():
        pos = content.find(line)
        in_class = any(s <= pos < e for s, e in class_spans)
        if in_class:
            continue

        fn_entry = parse_function_line(line, lib_name)
        if fn_entry:
            key = (fn_entry.name, fn_entry.kind, fn_entry.type_context)
            if key not in seen:
                entries.append(fn_entry)
                seen.add(key)
            continue

        ex_entry = parse_exception_line(line, lib_name)
        if ex_entry:
            key = (ex_entry.name, ex_entry.kind, ex_entry.type_context)
            if key not in seen:
                entries.append(ex_entry)
                seen.add(key)
            continue

        mod_m = _MODULE_RE.search(line)
        if mod_m:
            mod_name = mod_m.group(1)
            key = (mod_name, EntryKind.MODULE, None)
            if key not in seen:
                entries.append(
                    ReferenceEntry(
                        name=mod_name,
                        kind=EntryKind.MODULE,
                        source_lib=lib_name,
                    )
                )
                seen.add(key)

    if not entries:
        raise ReferenceParseError("no entries found in reference file")

    rf = ReferenceFile(
        lang="python",
        library_name=lib_name,
        version=version,
        entries=entries,
    )
    assert rf.lang == "python"
    return rf


def load_reference_file(path: Path) -> ReferenceFile:
    """Load and parse a .polyref file from disk."""
    assert path.suffix == ".polyref", f"expected .polyref extension, got {path.suffix}"
    if not path.exists():
        raise ReferenceFileNotFound(f"file not found: {path}", path=str(path))
    try:
        content = path.read_text(encoding="utf-8")
    except OSError as e:
        raise ReferenceParseError(str(e), path=str(path)) from e
    try:
        rf = parse_reference_file(content)
    except ReferenceParseError:
        raise
    except Exception as e:
        raise ReferenceParseError(str(e), path=str(path)) from e
    assert rf.library_name, "library_name must be non-empty"
    return rf
