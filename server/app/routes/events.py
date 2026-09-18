"""Counting what the app does, without recording what anybody did with it.

Nudge reads people's screens. That makes the usual analytics answer -- drop in a
third-party key and forget about it -- the wrong one: whatever goes in that
payload is a thing somebody can find in the binary and ask about, and "it only
sends counts, honestly" is not an answer anybody has to believe. Sending to this
server instead keeps the answer short and checkable, and it costs one table.

## What is refused

The event name is checked against a list. An open string is a table anybody with
the URL can fill with whatever they like, and the first thing that happens to an
open endpoint is somebody discovering it.

`detail` is clipped to a few words. It exists for "here" or "cloud", "done" or
"failed" -- and a field with no limit is a field that will eventually carry a
sentence somebody pasted, which is exactly what this is meant not to hold.

Nothing is required. A copy that has never signed in still counts, because "how
many people use this" is a question about people, not about accounts.
"""

from fastapi import APIRouter, Depends, Header, HTTPException, Request
from sqlmodel import Session, select

from ..db import session
from ..models import Event, Login, aware, now
from .. import auth

router = APIRouter()

# Every event this server accepts. Adding one is a line here and a line in the
# app; anything else is dropped without comment.
KNOWN = {
    # A turn happened: hotkey to answer.
    "turn",
    # A tour was given.
    "teaching",
    # An agent run ended.
    "agent",
    # First run of a version on a machine.
    "install",
    # Somebody drew on the screen while asking.
    "drawing",
}

# A batch is a handful of turns, not a day of them.
MOST = 50
DETAIL = 40


def _seconds(value: object) -> float:
    """However long it took, or nothing.

    Anything can be posted here, and `float("soon")` raises -- which would turn
    one malformed field into a 500 and, because the batch is written at the end,
    would throw away the well-formed events sitting beside it.
    """
    try:
        seconds = float(value or 0.0)
    except (TypeError, ValueError):
        return 0.0
    # A turn that took a negative amount of time, or a year, is a bug or a
    # prank. Either way it is not a measurement.
    return seconds if 0.0 <= seconds <= 86_400.0 else 0.0


def _whoever(authorization: str, db: Session) -> int | None:
    """The account behind the token, if there is one and it is good.

    Unauthenticated events are kept rather than refused: a copy that has not
    signed in is still somebody using the app, and refusing to count them would
    make every number an answer about signed-in people only.
    """
    scheme, _, token = authorization.partition(" ")
    if scheme.lower() != "bearer" or not token:
        return None
    login = db.exec(select(Login).where(Login.token_hash == auth.fingerprint(token))).first()
    if not login or aware(login.expires_at) <= now():
        return None
    return login.user_id


@router.post("/api/events", status_code=204)
async def record(
    request: Request,
    authorization: str = Header(default=""),
    db: Session = Depends(session),
) -> None:
    """Take a batch of counts. Says nothing back."""
    body = await request.json()
    events = body.get("events") or []
    install = str(body.get("install") or "").strip()[:64]
    version = str(body.get("version") or "").strip()[:20]
    platform = str(body.get("platform") or "").strip()[:20]
    if not install or not isinstance(events, list):
        raise HTTPException(400, "an install id and a list of events")

    user_id = _whoever(authorization, db)
    kept = 0
    for one in events[:MOST]:
        if not isinstance(one, dict):
            continue
        name = str(one.get("name") or "")
        if name not in KNOWN:
            continue
        db.add(
            Event(
                install=install,
                user_id=user_id,
                version=version,
                platform=platform,
                name=name,
                seconds=_seconds(one.get("seconds")),
                detail=str(one.get("detail") or "")[:DETAIL],
            )
        )
        kept += 1
    if kept:
        db.commit()
