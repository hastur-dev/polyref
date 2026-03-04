"""Prompts that elicit tokio API usage — areas where Haiku frequently hallucinates."""

from __future__ import annotations

RUST_TOKIO_PROMPTS: list[tuple[str, str]] = [
    (
        "tokio_spawn_basic",
        (
            "Write a Rust function using tokio that spawns an async task"
            " and awaits its result. Use the tokio runtime."
        ),
    ),
    (
        "tokio_joinset",
        (
            "Write Rust code that creates a JoinSet, spawns 3 tasks into it,"
            " and awaits all results."
        ),
    ),
    (
        "tokio_runtime_block_on",
        (
            "Write Rust code that creates a tokio runtime and uses block_on"
            " to run an async function."
        ),
    ),
    (
        "tokio_channel",
        (
            "Write Rust code using tokio mpsc channel to send 5 messages"
            " from a spawned task to the main task."
        ),
    ),
    (
        "tokio_joinhandle_abort",
        (
            "Write Rust code that spawns a tokio task, stores the JoinHandle,"
            " and aborts it after 100ms."
        ),
    ),
]
