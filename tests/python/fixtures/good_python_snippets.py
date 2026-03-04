"""Good patterns — should produce zero issues."""

import requests  # noqa: F401
from requests import Session

session = Session()
response = session.get("https://example.com")
response.raise_for_status()
data = response.json()  # noqa: F841
print(response.status_code)
session.close()
