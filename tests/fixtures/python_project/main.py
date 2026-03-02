import requests
from requests import Session, NonExistent  # ERROR: NonExistent not in requests

# Valid usage
response = requests.get("https://example.com")
data = response.json()
response.raise_for_status()

# Invalid: wrong function name
result = requests.fetch("https://example.com")  # ERROR: no 'fetch' function

# Invalid: missing required argument
response = requests.get()  # ERROR: 'url' is required

# Invalid: wrong method on Response
response.nonexistent_method()  # ERROR: Response has no 'nonexistent_method'

# Valid: Session usage
with Session() as s:
    s.headers.update({"Authorization": "Bearer token"})
    resp = s.get("https://api.example.com")
