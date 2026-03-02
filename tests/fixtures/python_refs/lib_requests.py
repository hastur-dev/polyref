# requests Reference - HTTP Library for Python
# pip install requests==2.31.0
# Usage: import requests / from requests import Session, Response

import requests
from requests import Session, Response, HTTPError

# ============================================================================
# TOP-LEVEL FUNCTIONS
# ============================================================================

# HTTP methods
def get(url: str, params: dict = None, **kwargs) -> Response: ...
def post(url: str, data: dict = None, json: dict = None, **kwargs) -> Response: ...
def put(url: str, data: dict = None, **kwargs) -> Response: ...
def delete(url: str, **kwargs) -> Response: ...
def patch(url: str, data: dict = None, **kwargs) -> Response: ...
def head(url: str, **kwargs) -> Response: ...
def options(url: str, **kwargs) -> Response: ...
def request(method: str, url: str, **kwargs) -> Response: ...

# ============================================================================
# RESPONSE CLASS
# ============================================================================

class Response:
    status_code: int
    text: str
    content: bytes
    json_data: dict
    headers: dict
    cookies: dict
    url: str
    encoding: str
    elapsed: timedelta

    def json(self) -> dict: ...
    def raise_for_status(self) -> None: ...
    def iter_content(self, chunk_size: int = 1) -> Iterator[bytes]: ...
    def iter_lines(self, chunk_size: int = 512) -> Iterator[str]: ...
    @property
    def ok(self) -> bool: ...
    @property
    def is_redirect(self) -> bool: ...

# ============================================================================
# SESSION CLASS
# ============================================================================

class Session:
    headers: dict
    cookies: dict
    auth: tuple
    proxies: dict
    verify: bool
    cert: str

    def __init__(self) -> None: ...
    def get(self, url: str, **kwargs) -> Response: ...
    def post(self, url: str, data: dict = None, json: dict = None, **kwargs) -> Response: ...
    def put(self, url: str, data: dict = None, **kwargs) -> Response: ...
    def delete(self, url: str, **kwargs) -> Response: ...
    def close(self) -> None: ...
    def __enter__(self) -> 'Session': ...
    def __exit__(self, *args) -> None: ...

# ============================================================================
# EXCEPTIONS
# ============================================================================

class HTTPError(Exception): ...
class ConnectionError(Exception): ...
class Timeout(Exception): ...
class RequestException(Exception): ...

# ============================================================================
# COMMON PATTERNS
# ============================================================================

# Basic GET request
def example_get():
    response = requests.get("https://api.example.com/data")
    response.raise_for_status()
    data = response.json()

# POST with JSON
def example_post():
    payload = {"key": "value"}
    response = requests.post("https://api.example.com/data", json=payload)

# Session with headers
def example_session():
    with requests.Session() as session:
        session.headers.update({"Authorization": "Bearer token"})
        response = session.get("https://api.example.com/protected")
