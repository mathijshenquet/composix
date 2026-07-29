import httpx


def proof() -> str:
    return f"uv:{httpx.URL('https://example.invalid/d38').path}"
