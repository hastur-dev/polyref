"""Prompts that elicit pandas API usage."""

from __future__ import annotations

PYTHON_PANDAS_PROMPTS: list[tuple[str, str]] = [
    (
        "pandas_read_csv",
        (
            "Write Python code using pandas to read a CSV file,"
            " filter rows where a column exceeds a threshold,"
            " and save the result."
        ),
    ),
    (
        "pandas_groupby",
        (
            "Write Python code using pandas to group a DataFrame by"
            " a column, aggregate with sum and mean, and sort results."
        ),
    ),
    (
        "pandas_merge",
        (
            "Write Python code using pandas to merge two DataFrames"
            " on a common column using a left join."
        ),
    ),
]
