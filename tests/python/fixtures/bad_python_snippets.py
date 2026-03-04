"""Known-bad patterns for integration testing — 13 patterns that should be flagged."""

import requests  # noqa: F401
from requests import Response, Session  # noqa: F401

# BAD-1: Hallucinated method on Session
session = Session()
session.fetch("https://example.com")

# BAD-2: Wrong constructor name
s = requests.Sessionn()  # noqa: F841

# BAD-3: Invented class method
r = Response.from_text("hello")  # noqa: F841

# BAD-4: Too many args to close
session.close(True)

# BAD-5: Too few args to get (needs url)
session.get()

# BAD-6: Wrong attribute name on Response
response = session.get("https://example.com")
print(response.status)

# BAD-7: Invented module path
from requests.network import Proxy  # noqa: F401, E402

# BAD-8: Typo in imported name
from requests import Sessoin  # noqa: F401, E402

# BAD-9: Invented top-level function
requests.fetch_all(["url1", "url2"])

# BAD-10: Wrong method (hallucinated)
session2: Session = Session()
session2.patch_data({"key": "value"})

# BAD-11: raise unknown exception
raise requests.NetworkError("timeout")

# BAD-12: Wrong attribute
headers = response.header  # noqa: F841

# BAD-13: Invented Session class method
Session.create_default()
