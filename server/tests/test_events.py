"""What the counting endpoint keeps, and what it refuses.

The rule being tested is the one the product's honesty rests on: a row here
cannot say what anybody was doing.
"""

import pytest
from sqlmodel import Session, select

from app import auth
from app import db as db_module
from app.models import Event


def send(client, events, install="machine-1", headers=None):
    return client.post(
        "/api/events",
        json={"install": install, "version": "0.1.9", "platform": "macos", "events": events},
        headers=headers or {},
    )


def kept():
    # Through the module, not a name bound at import: the fixture swaps the
    # engine for a database of its own and a captured reference misses it.
    with Session(db_module.engine) as db:
        return list(db.exec(select(Event)))


def test_a_copy_that_never_signed_in_still_counts(client):
    # Otherwise every number is an answer about signed-in people only.
    assert send(client, [{"name": "turn", "seconds": 3.2, "detail": "here"}]).status_code == 204
    rows = kept()
    assert len(rows) == 1
    assert rows[0].user_id is None
    assert rows[0].name == "turn"
    assert rows[0].seconds == pytest.approx(3.2)


def test_a_name_nobody_agreed_to_is_dropped(client):
    """An open string is a table anybody with the URL can fill."""
    send(client, [{"name": "turn"}, {"name": "whatever_i_like"}, {"name": "'; drop table"}])
    assert [r.name for r in kept()] == ["turn"]


def test_a_sentence_cannot_be_smuggled_through_detail(client):
    """`detail` is for "here" or "done". A field with no limit eventually
    carries something somebody pasted, which is the thing this must not hold."""
    send(client, [{"name": "turn", "detail": "x" * 500}])
    assert len(kept()[0].detail) <= 40


def test_a_signed_in_copy_is_counted_as_a_person(client, monkeypatch):
    monkeypatch.setitem(
        auth.PROVIDERS,
        "google",
        lambda proof: auth.Identity("google", "sub-1", "sam@example.com", "Sam", ""),
    )
    token = client.post("/api/auth/signin/google", json={"proof": "x"}).json()["token"]
    send(client, [{"name": "turn"}], headers={"Authorization": f"Bearer {token}"})
    assert kept()[-1].user_id is not None


def test_a_batch_is_a_handful_not_a_day(client):
    send(client, [{"name": "turn"} for _ in range(500)])
    assert len(kept()) <= 50


def test_nothing_useful_without_an_install(client):
    assert send(client, [{"name": "turn"}], install="").status_code == 400


def test_a_nonsense_duration_does_not_take_the_batch_down_with_it(client):
    """One bad field must not lose the good events beside it.

    `float("soon")` raises, and the batch is committed at the end -- so an
    unguarded parse would turn a single malformed value into a 500 and drop
    every well-formed event in the same request.
    """
    reply = send(
        client,
        [
            {"name": "turn", "seconds": "soon"},
            {"name": "turn", "seconds": -5},
            {"name": "turn", "seconds": 1.5},
        ],
    )
    assert reply.status_code == 204
    turns = [e for e in kept() if e.name == "turn"]
    assert len(turns) == 3, "all three are kept"
    assert sorted(e.seconds for e in turns) == [0.0, 0.0, 1.5]
