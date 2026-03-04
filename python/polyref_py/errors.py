"""Exception types for polyref Python checker."""

from __future__ import annotations


class PolyrefError(Exception):
    """Base exception for all polyref errors."""

    def __init__(self, message: str, path: str | None = None) -> None:
        assert message, "error message must be non-empty"
        self.message = message
        self.path = path
        super().__init__(message)


class ReferenceFileNotFound(PolyrefError):
    """Reference file does not exist on disk."""


class ReferenceParseError(PolyrefError):
    """Failed to parse a reference file."""


class SourceParseError(PolyrefError):
    """Failed to parse a Python source file."""


class InvalidInputError(PolyrefError):
    """Invalid input (wrong extension, empty file, etc.)."""


class CheckerError(PolyrefError):
    """Internal checker error."""


def format_error(e: PolyrefError) -> str:
    """Format a PolyrefError for terminal display."""
    assert isinstance(e, PolyrefError), "expected a PolyrefError instance"
    type_name = type(e).__name__
    path_part = f" ({e.path})" if e.path else ""
    result = f"polyref error [{type_name}]: {e.message}{path_part}"
    assert result, "formatted error must be non-empty"
    assert type_name in result, "formatted error must contain type name"
    return result
