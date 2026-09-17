"""Signing in, without asking Google or GitHub anything.

The provider half is stubbed in most of these: what is worth testing here is
what the server does with an answer, not that httpx can make a request. The
exceptions are the checks that exist to refuse things -- those are tested
against the real verification code with a faked reply, because a security check
that is never exercised is a comment.
"""

from datetime import timedelta

import pytest
from sqlmodel import Session, select

from app import auth
from app.models import Login, User, now
from app.settings import settings


@pytest.fixture
def known(monkeypatch):
    """Make both providers answer with whoever the test asks for."""

    def as_person(provider, subject, email="sam@example.com", name="Sam"):
        monkeypatch.setitem(
            auth.PROVIDERS,
            provider,
            lambda proof: auth.Identity(provider, subject, email, name, ""),
        )

    return as_person


def test_signing_in_makes_an_account_and_a_token(client, known):
    known("google", "sub-1")
    reply = client.post("/api/auth/signin/google", json={"proof": "x"})

    assert reply.status_code == 200
    body = reply.json()
    assert body["token"]
    assert body["user"] == {
        "provider": "google",
        "email": "sam@example.com",
        "name": "Sam",
        "avatar_url": "",
    }


def test_the_token_is_not_stored(client, known):
    """A leaked database should be a list of hashes, not a set of live sessions."""
    known("google", "sub-1")
    token = client.post("/api/auth/signin/google", json={"proof": "x"}).json()["token"]

    with Session(client.engine) as db:
        stored = db.exec(select(Login)).one()
    assert stored.token_hash != token
    assert stored.token_hash == auth.fingerprint(token)


def test_signing_in_again_is_the_same_person_on_a_second_device(client, known):
    known("google", "sub-1")
    client.post("/api/auth/signin/google", json={"proof": "x"})
    client.post("/api/auth/signin/google", json={"proof": "y"})

    with Session(client.engine) as db:
        assert len(db.exec(select(User)).all()) == 1
        assert len(db.exec(select(Login)).all()) == 2


def test_the_same_email_at_two_providers_is_two_accounts(client, known):
    """Not a wart to fix casually -- see the note on `User`."""
    known("google", "sub-1", email="sam@example.com")
    client.post("/api/auth/signin/google", json={"proof": "x"})
    known("github", "99", email="sam@example.com")
    client.post("/api/auth/signin/github", json={"proof": "x"})

    with Session(client.engine) as db:
        assert len(db.exec(select(User)).all()) == 2


def test_a_changed_name_at_the_provider_shows_up_here(client, known):
    known("google", "sub-1", name="Sam")
    client.post("/api/auth/signin/google", json={"proof": "x"})
    known("google", "sub-1", name="Samantha")
    reply = client.post("/api/auth/signin/google", json={"proof": "x"})

    assert reply.json()["user"]["name"] == "Samantha"


def test_me_needs_the_token(client, known):
    known("google", "sub-1")
    token = client.post("/api/auth/signin/google", json={"proof": "x"}).json()["token"]

    assert client.get("/api/auth/me").status_code == 401
    assert client.get("/api/auth/me", headers={"Authorization": "Bearer nope"}).status_code == 401
    assert client.get("/api/auth/me", headers={"Authorization": token}).status_code == 401

    ok = client.get("/api/auth/me", headers={"Authorization": f"Bearer {token}"})
    assert ok.status_code == 200
    assert ok.json()["name"] == "Sam"


def test_an_expired_session_stops_working_and_is_cleared_out(client, known):
    known("google", "sub-1")
    token = client.post("/api/auth/signin/google", json={"proof": "x"}).json()["token"]

    with Session(client.engine) as db:
        login = db.exec(select(Login)).one()
        login.expires_at = now() - timedelta(seconds=1)
        db.add(login)
        db.commit()

    assert client.get("/api/auth/me", headers={"Authorization": f"Bearer {token}"}).status_code == 401
    with Session(client.engine) as db:
        assert db.exec(select(Login)).all() == []


def test_signing_out_ends_that_session_and_leaves_the_other(client, known):
    known("google", "sub-1")
    one = client.post("/api/auth/signin/google", json={"proof": "x"}).json()["token"]
    two = client.post("/api/auth/signin/google", json={"proof": "x"}).json()["token"]

    assert client.post("/api/auth/signout", headers={"Authorization": f"Bearer {one}"}).status_code == 204
    assert client.get("/api/auth/me", headers={"Authorization": f"Bearer {one}"}).status_code == 401
    assert client.get("/api/auth/me", headers={"Authorization": f"Bearer {two}"}).status_code == 200


def test_signing_out_twice_is_not_an_error(client, known):
    known("google", "sub-1")
    token = client.post("/api/auth/signin/google", json={"proof": "x"}).json()["token"]
    client.post("/api/auth/signout", headers={"Authorization": f"Bearer {token}"})

    assert client.post("/api/auth/signout", headers={"Authorization": f"Bearer {token}"}).status_code == 204


def test_an_unknown_provider_is_refused(client):
    assert client.post("/api/auth/signin/facebook", json={"proof": "x"}).status_code == 404


# -- the refusals, against the real verification code ------------------------


class FakeReply:
    def __init__(self, status_code, payload):
        self.status_code = status_code
        self._payload = payload

    def json(self):
        return self._payload


def google_says(monkeypatch, payload, status_code=200):
    monkeypatch.setattr(
        auth.httpx, "get", lambda *a, **k: FakeReply(status_code, payload)
    )


GOOD = {
    "aud": "ours.apps.googleusercontent.com",
    "iss": "https://accounts.google.com",
    "sub": "sub-1",
    "email": "sam@example.com",
    "email_verified": "true",
    "name": "Sam",
    "picture": "https://example.com/sam.png",
}


@pytest.fixture
def ours(monkeypatch):
    monkeypatch.setattr(settings(), "google_client_id", "ours.apps.googleusercontent.com")


def test_a_google_token_for_another_application_is_refused(monkeypatch, ours):
    """Valid, signed by Google, and minted for somebody else's client."""
    google_says(monkeypatch, {**GOOD, "aud": "theirs.apps.googleusercontent.com"})

    with pytest.raises(auth.NotGenuine):
        auth.identify_google("x")


def test_google_sign_in_is_refused_when_no_client_id_is_configured(monkeypatch):
    monkeypatch.setattr(settings(), "google_client_id", "")
    google_says(monkeypatch, GOOD)

    with pytest.raises(auth.NotGenuine):
        auth.identify_google("x")


def test_a_token_google_rejects_is_refused(monkeypatch, ours):
    google_says(monkeypatch, {"error": "invalid_token"}, status_code=400)

    with pytest.raises(auth.NotGenuine):
        auth.identify_google("x")


def test_an_unverified_google_address_is_not_kept(monkeypatch, ours):
    """The account still works. It just does not claim an address anybody could have typed."""
    google_says(monkeypatch, {**GOOD, "email_verified": "false"})

    assert auth.identify_google("x").email == ""


def test_a_good_google_token_describes_the_person(monkeypatch, ours):
    google_says(monkeypatch, GOOD)

    who = auth.identify_google("x")
    assert (who.provider, who.subject, who.email) == ("google", "sub-1", "sam@example.com")


def test_a_github_token_github_rejects_is_refused(monkeypatch):
    monkeypatch.setattr(auth.httpx, "get", lambda *a, **k: FakeReply(401, {}))

    with pytest.raises(auth.NotGenuine):
        auth.identify_github("x")


def test_a_github_account_with_no_public_address_still_signs_in(monkeypatch):
    monkeypatch.setattr(
        auth.httpx,
        "get",
        lambda *a, **k: FakeReply(200, {"id": 99, "email": None, "login": "sam", "name": None}),
    )

    who = auth.identify_github("x")
    assert (who.subject, who.email, who.name) == ("99", "", "sam")
