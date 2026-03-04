"""Prompts that elicit requests API usage."""

from __future__ import annotations

PYTHON_REQUESTS_PROMPTS: list[tuple[str, str]] = [
    (
        "requests_session_get",
        (
            "Write Python code that creates a requests Session,"
            " makes a GET request, and prints the JSON response."
        ),
    ),
    (
        "requests_error_handling",
        (
            "Write Python code using requests that handles"
            " ConnectionError and Timeout exceptions."
        ),
    ),
    (
        "requests_session_auth",
        (
            "Write Python code using requests Session with HTTPBasicAuth"
            " to make an authenticated POST request."
        ),
    ),
    (
        "requests_response_check",
        (
            "Write Python code that uses requests to fetch a URL,"
            " check if it succeeded, and read the response body."
        ),
    ),
    (
        "requests_retry_logic",
        (
            "Write Python code using requests with a Session that mounts"
            " a retry adapter and makes a GET request."
        ),
    ),
]
