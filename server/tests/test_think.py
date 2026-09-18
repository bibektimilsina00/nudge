"""The model proxy: who may use it, and what it refuses.

Google is never called here. What matters is the boundary -- a session is
required, the model name goes into a URL and so is checked, and the upstream's
status comes back rather than being flattened into a 200.
"""

import httpx
import pytest

from app import auth
from app.routes import think
from app.settings import settings


@pytest.fixture
def signed_in(client, monkeypatch):
    monkeypatch.setitem(
        auth.PROVIDERS,
        "google",
        lambda proof: auth.Identity("google", "sub-1", "sam@example.com", "Sam", ""),
    )
    reply = client.post("/api/auth/signin/google", json={"proof": "x"})
    return {"Authorization": f"Bearer {reply.json()['token']}"}


@pytest.fixture
def upstream(monkeypatch):
    """Stand in for Google, and record what it was asked."""
    seen = {}

    class Reply:
        status_code = 200
        content = b"{}"

        def json(self):
            return {"candidates": []}

    class Client:
        def __init__(self, **kw):
            pass

        async def __aenter__(self):
            return self

        async def __aexit__(self, *a):
            return False

        async def post(self, url, content, headers):
            seen["url"] = url
            seen["key"] = headers.get("x-goog-api-key")
            seen["body"] = content
            return Reply()

    monkeypatch.setattr(think.httpx, "AsyncClient", Client)
    monkeypatch.setattr(settings(), "gemini_key", "server-side-key")
    return seen


def test_a_signed_in_app_borrows_the_servers_key(client, signed_in, upstream):
    reply = client.post("/api/think/gemini-3.5-flash", json={"contents": []}, headers=signed_in)

    assert reply.status_code == 200
    assert upstream["key"] == "server-side-key"
    assert upstream["url"].endswith("/models/gemini-3.5-flash:generateContent")
    # Forwarded whole. The app builds the body and parses the reply; anything
    # this understood would be a second place to change when the prompt does.
    assert b'"contents"' in upstream["body"]


def test_without_a_session_it_is_nobodys_key(client, upstream):
    reply = client.post("/api/think/gemini-3.5-flash", json={"contents": []})
    assert reply.status_code == 401
    assert "url" not in upstream, "it called Google for somebody with no account"


def test_a_model_name_cannot_walk_out_of_the_url(client, signed_in, upstream):
    reply = client.post(
        "/api/think/..%2F..%2Fsomewhere", json={"contents": []}, headers=signed_in
    )
    assert reply.status_code in (400, 404)
    assert "url" not in upstream


def test_a_server_with_no_key_says_so(client, signed_in, upstream, monkeypatch):
    monkeypatch.setattr(settings(), "gemini_key", "")
    reply = client.post("/api/think/gemini-3.5-flash", json={"contents": []}, headers=signed_in)
    assert reply.status_code == 503
    assert "key" in reply.json()["detail"]


def test_the_upstreams_answer_arrives_intact(client, signed_in, monkeypatch):
    """A 429 has to stay a 429, or the app cannot tell slow down from wrong."""

    class Busy:
        status_code = 429
        content = b'{"error":"quota"}'

        def json(self):
            return {"error": "quota"}

    class Client:
        def __init__(self, **kw):
            pass

        async def __aenter__(self):
            return self

        async def __aexit__(self, *a):
            return False

        async def post(self, url, content, headers):
            return Busy()

    monkeypatch.setattr(think.httpx, "AsyncClient", Client)
    monkeypatch.setattr(settings(), "gemini_key", "k")
    reply = client.post("/api/think/gemini-3.5-flash", json={}, headers=signed_in)
    assert reply.status_code == 429
