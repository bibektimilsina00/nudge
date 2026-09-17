"""Signing in, staying signed in, and signing out.

Four endpoints and no pages. The browser half of signing in belongs to the
provider; by the time anything here is called, the app is already holding proof
from Google or GitHub and wants a token of ours in exchange.

The deciding lives in `app.auth`, which takes values and returns values. This
file is only the part that has to know about requests.
"""

from fastapi import APIRouter, Depends, Header, HTTPException
from pydantic import BaseModel
from sqlmodel import Session, select

from app import auth
from app.db import session
from app.models import Login, User, aware, expiry, now
from app.settings import settings

router = APIRouter(tags=["auth"])


class Proof(BaseModel):
    """What the app arrived holding.

    One field whatever the provider, because the app should not have to know
    that a Google sign-in ends in an ID token and a GitHub one ends in an access
    token. It ran the flow; this is what fell out.
    """

    proof: str
    # Free text, shown back in a future "signed in on these devices" list.
    device: str = ""


class Me(BaseModel):
    """A person, as the app draws them."""

    provider: str
    email: str
    name: str
    avatar_url: str


class SignedIn(BaseModel):
    token: str
    # ISO 8601. The app stores it so it can tell the difference between a
    # session that ran out and one that was revoked, which are different
    # sentences to show somebody.
    expires_at: str
    user: Me


def _portrait(user: User) -> Me:
    return Me(
        provider=user.provider,
        email=user.email,
        name=user.name,
        avatar_url=user.avatar_url,
    )


def current_user(
    authorization: str = Header(default=""),
    db: Session = Depends(session),
) -> User:
    """The person behind a bearer token, or 401.

    Expiry is enforced here rather than by a sweep, so a session is dead the
    moment it is out of date even if nothing has tidied the table yet.
    """
    scheme, _, token = authorization.partition(" ")
    if scheme.lower() != "bearer" or not token:
        raise HTTPException(401, "not signed in")

    login = db.exec(
        select(Login).where(Login.token_hash == auth.fingerprint(token))
    ).first()
    if not login:
        raise HTTPException(401, "not signed in")
    if aware(login.expires_at) <= now():
        # Taken out of the table on the way past. The alternative is a session
        # that fails forever and stays listed as a device somebody is signed in
        # on, which is a lie in a settings page.
        db.delete(login)
        db.commit()
        raise HTTPException(401, "that session has expired")

    user = db.get(User, login.user_id)
    if not user:
        raise HTTPException(401, "not signed in")

    login.last_used_at = now()
    db.add(login)
    db.commit()
    return user


@router.post("/api/auth/signin/{provider}", response_model=SignedIn)
def sign_in(provider: str, body: Proof, db: Session = Depends(session)) -> SignedIn:
    """Trade a provider's proof for one of ours.

    Under `/signin/` rather than at `/api/auth/{provider}`, which read better
    and quietly ate `/api/auth/signout`: FastAPI matches in declaration order,
    so "signout" arrived here as the name of a provider and was refused for
    having no body.

    Sign-up and sign-in are the same request on purpose. The app cannot know
    whether somebody has been here before -- only the provider's subject says
    that, and it arrives with the proof -- so asking them to choose a door
    before they know which one is theirs is a question with no answer.
    """
    identify = auth.PROVIDERS.get(provider)
    if not identify:
        raise HTTPException(404, f"no such sign-in: {provider}")

    try:
        who = identify(body.proof)
    except auth.NotGenuine as exc:
        raise HTTPException(401, str(exc)) from exc

    user = db.exec(
        select(User).where(User.provider == who.provider, User.subject == who.subject)
    ).first()
    if user:
        # Refreshed every time, because a name or a picture changing at the
        # provider should show here without anybody doing anything.
        user.email, user.name, user.avatar_url = who.email, who.name, who.avatar_url
        user.last_seen_at = now()
    else:
        user = User(
            provider=who.provider,
            subject=who.subject,
            email=who.email,
            name=who.name,
            avatar_url=who.avatar_url,
        )
    db.add(user)
    db.commit()
    db.refresh(user)

    token, token_hash = auth.mint()
    until = expiry(settings().session_days)
    db.add(
        Login(
            user_id=user.id,
            token_hash=token_hash,
            expires_at=until,
            device=body.device[:200],
        )
    )
    db.commit()

    return SignedIn(token=token, expires_at=until.isoformat(), user=_portrait(user))


@router.get("/api/auth/me", response_model=Me)
def me(user: User = Depends(current_user)) -> Me:
    """Who the app is signed in as.

    Called on launch. It is the only way to find out that a session was revoked
    from somewhere else, which a stored token cannot tell you by looking at it.
    """
    return _portrait(user)


@router.post("/api/auth/signout", status_code=204)
def sign_out(
    authorization: str = Header(default=""),
    db: Session = Depends(session),
) -> None:
    """Forget this device.

    Deliberately not `Depends(current_user)`: signing out of an expired or
    already-revoked session should quietly succeed. Answering 401 to somebody
    asking to be signed out is a refusal to do the thing that has already
    happened.
    """
    _, _, token = authorization.partition(" ")
    if not token:
        return
    login = db.exec(
        select(Login).where(Login.token_hash == auth.fingerprint(token))
    ).first()
    if login:
        db.delete(login)
        db.commit()
