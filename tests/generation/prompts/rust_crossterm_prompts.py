"""Prompts that elicit crossterm API usage."""

from __future__ import annotations

RUST_CROSSTERM_PROMPTS: list[tuple[str, str]] = [
    (
        "crossterm_enable_raw",
        (
            "Write Rust code using crossterm that enables raw mode,"
            " reads a key event, then disables raw mode."
        ),
    ),
    (
        "crossterm_cursor_move",
        (
            "Write Rust code using crossterm to move the cursor to row 5"
            " column 10, then print colored text."
        ),
    ),
    (
        "crossterm_alternate_screen",
        (
            "Write Rust code using crossterm to enter the alternate screen,"
            " clear it, write text, wait for Enter, and leave."
        ),
    ),
]
