"""The model, borrowed.

Nudge needs a Gemini key to do anything at all, and a copy somebody downloaded
has none. Shipping one inside the app was the obvious idea and is not an option:
`strings` pulls it out of the binary in one command, the repository is public,
and it would be one person's quota and one person's bill for everybody who ever
pressed download.

So the key stays here and the app borrows it, with the session it already has
from signing in. That is the same trade every hosted product makes: the
credential lives on a server, is attached to an account, and can be cut off for
one account without a release.

Anyone who would rather not go through here can set their own key in Settings,
and then the app talks to Google directly and never calls this at all.
"""

import httpx
from fastapi import APIRouter, Depends, HTTPException, Request
from fastapi.responses import JSONResponse

from ..models import User
from ..settings import settings
from .auth import current_user

router = APIRouter()

# Upstream, and the only host this will ever talk to. Built from a constant with
# the model interpolated into one path segment -- see `_model` for why that
# segment is checked rather than trusted.
GOOGLE = "https://generativelanguage.googleapis.com/v1beta/models"

# Long enough for a slow model on a big screenshot, short enough that a hung
# upstream does not hold a worker forever.
TIMEOUT = httpx.Timeout(90.0, connect=10.0)

# A screenshot arrives base64-encoded inside the JSON, so this is not a small
# number. It is still a number: without one, this endpoint is an invitation to
# post a gigabyte through somebody else's API key.
MOST = 24 * 1024 * 1024


def _model(name: str) -> str:
    """The model name, or a refusal.

    This goes into a URL path, so it is checked rather than trusted. Without
    this, "../../somewhere" is a request to a URL nobody here chose -- the whole
    shape of an SSRF, delivered through a parameter that looks like a label.
    """
    ok = name.replace("-", "").replace(".", "").replace(":", "").isalnum()
    if not ok or len(name) > 100:
        raise HTTPException(400, "that is not a model name")
    return name


@router.post("/api/think/{model}")
async def think(
    model: str,
    request: Request,
    user: User = Depends(current_user),
) -> JSONResponse:
    """Forward one `generateContent` call, and hand back exactly what came out.

    Deliberately not a wrapper. The app builds its own request body and parses
    its own response -- prompts, shapes and parsing stay in the app where they
    are tested, and this stays a pipe. A server that understood the payload
    would be a second place to change every time the prompt does.
    """
    key = settings().gemini_key
    if not key:
        # Said plainly rather than as a 500. The app shows this to a person, and
        # "the server has no key" is something they can report; "internal server
        # error" is not.
        raise HTTPException(503, "this server has no model key configured")

    body = await request.body()
    if len(body) > MOST:
        raise HTTPException(413, "that request is too big")

    async with httpx.AsyncClient(timeout=TIMEOUT) as http:
        try:
            reply = await http.post(
                f"{GOOGLE}/{_model(model)}:generateContent",
                content=body,
                headers={"content-type": "application/json", "x-goog-api-key": key},
            )
        except httpx.HTTPError as exc:
            # 502, because the failure is upstream and the app should say so
            # rather than tell somebody their own network is down.
            raise HTTPException(502, f"could not reach the model: {exc}") from exc

    # Status and all, including the failures. A 429 has to arrive as a 429 or the
    # app cannot tell "slow down" from "that request was wrong".
    return JSONResponse(
        status_code=reply.status_code,
        content=reply.json() if reply.content else {},
    )
