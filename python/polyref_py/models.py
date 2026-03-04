"""Pydantic models for polyref Python checker — mirrors Rust ReferenceEntry/Issue."""

from __future__ import annotations

from enum import StrEnum

from pydantic import BaseModel, ConfigDict, model_validator


class EntryKind(StrEnum):
    """Kind of reference entry."""

    FUNCTION = "function"
    METHOD = "method"
    CLASS_METHOD = "class_method"
    STATIC_METHOD = "static_method"
    CLASS = "class"
    FIELD = "field"
    EXCEPTION = "exception"
    CONSTANT = "constant"
    MODULE = "module"
    REEXPORT = "reexport"


class ReferenceEntry(BaseModel):
    """A single entry in a reference file."""

    name: str
    kind: EntryKind
    type_context: str | None = None
    min_args: int | None = None
    max_args: int | None = None
    description: str = ""
    source_lib: str = ""

    model_config = ConfigDict(frozen=True)

    @model_validator(mode="after")
    def _validate_invariants(self) -> ReferenceEntry:
        assert self.name, "name must be non-empty"
        if self.min_args is not None and self.max_args is not None:
            assert self.min_args <= self.max_args, (
                f"min_args ({self.min_args}) must be <= max_args ({self.max_args})"
            )
        return self


class IssueLevel(StrEnum):
    """Severity level for a validation issue."""

    ERROR = "error"
    WARNING = "warning"
    INFO = "info"


class IssueKind(StrEnum):
    """Kind of validation issue."""

    UNKNOWN_IMPORT = "unknown-import"
    UNKNOWN_METHOD = "unknown-method"
    UNKNOWN_CLASS_METHOD = "unknown-class-method"
    UNKNOWN_ATTRIBUTE = "unknown-attribute"
    TOO_FEW_ARGS = "too-few-args"
    TOO_MANY_ARGS = "too-many-args"
    UNKNOWN_EXCEPTION = "unknown-exception"


class Issue(BaseModel):
    """A single validation issue found during checking."""

    kind: IssueKind
    level: IssueLevel
    message: str
    line_number: int
    col_number: int = 0
    suggestion: str | None = None
    similarity: float | None = None

    model_config = ConfigDict(frozen=True)

    @model_validator(mode="after")
    def _validate_invariants(self) -> Issue:
        assert self.message, "message must be non-empty"
        assert self.line_number >= 1, (
            f"line_number must be >= 1, got {self.line_number}"
        )
        return self


class ReferenceFile(BaseModel):
    """A parsed reference file for one library."""

    lang: str
    library_name: str
    version: str
    entries: list[ReferenceEntry]

    model_config = ConfigDict(frozen=True)

    @model_validator(mode="after")
    def _validate_invariants(self) -> ReferenceFile:
        assert self.lang in {"python", "rust"}, (
            f"lang must be 'python' or 'rust', got '{self.lang}'"
        )
        assert self.entries, "entries must be non-empty"
        return self
