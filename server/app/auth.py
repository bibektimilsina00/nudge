"""Who somebody is, and how the app proves it on every later request.

Sign-in happens in the desktop app rather than here. The app runs the provider's
own flow -- Google's loopback redirect with PKCE, GitHub's device flow -- and
arrives holding proof from the provider. Everything in this module exists to
answer two questions about that proof: is it genuine, and who does it describe.

The exchange at the end is the point. The provider's token is not carried
onwards and is not stored: it is scoped to the provider, it expires on the
provider's schedule, and keeping it would make every later request a question
for Google rather than for us. What the app keeps instead is a token of ours,
which is ours to expire and ours to revoke.

Kept apart from `routes/auth.py` so that the deciding can be tested without a
request: everything here takes values and returns values.
"""

import hashlib
import secrets
from dataclasses import dataclass

import httpx

from app.settings import settings


@dataclass(frozen=True)
class Identity:
    """A person, as one provider describes them."""

    provider: str
    # The provider's stable id. This is what a user row is keyed by.
    subject: str
    email: str
    name: str
    avatar_url: str


class NotGenuine(Exception):
    """The proof did not check out. Never carries the provider's words onward.

    A verification failure is reported as a failure and nothing more. Passing
    the provider's message through would hand an attacker a free oracle for
    probing what the server accepts, and reads to an honest user as noise about
    a system they did not know they were talking to.
    """


def mint() -> tuple[str, str]:
    """A new session token, and the hash to file against it.

    256 bits from the system generator. Long enough that guessing is not a
    strategy, and `token_urlsafe` so it survives a header untouched.
    """
    token = secrets.token_urlsafe(32)
    return token, fingerprint(token)


def fingerprint(token: str) -> str:
    """How a token is recognised without being stored.

    Plain SHA-256, deliberately: this is not a password. Slow hashing exists to
    make guessing a human-chosen secret expensive, and there is nothing to guess
    here -- the token is random and full-entropy, so a fast hash gives an
    attacker holding the database nothing to work with.
    """
    return hashlib.sha256(token.encode()).hexdigest()


def identify_google(proof: str) -> Identity:
    """Check a Google ID token and say who it describes.

    Google's tokeninfo endpoint does the verifying. The alternative is to fetch
    Google's signing keys and check the signature here, which is faster and is
    what a high-volume service should do -- but this runs once per device per
    few months, so the round trip costs nothing, and hand-rolled JWT validation
    is a well-known way to accept tokens nobody signed.

    The audience check is not optional and is not Google's to make for us. A
    valid ID token issued to somebody else's client is still a valid ID token;
    without this, anyone with a Google app could mint sign-ins for ours.
    """
    want = settings().google_client_id
    if not want:
        raise NotGenuine("no Google client id configured")

    try:
        reply = httpx.get(
            "https://oauth2.googleapis.com/tokeninfo",
            params={"id_token": proof},
            timeout=10,
        )
    except httpx.HTTPError as exc:
        raise NotGenuine("could not reach Google") from exc

    if reply.status_code != 200:
        raise NotGenuine("Google did not recognise that sign-in")

    claims = reply.json()
    if claims.get("aud") != want:
        raise NotGenuine("that sign-in was issued to a different application")
    if claims.get("iss") not in ("accounts.google.com", "https://accounts.google.com"):
        raise NotGenuine("that sign-in did not come from Google")
    # Google sends the string "true", not a boolean. An unverified address is
    # one anybody can claim, so it is dropped rather than stored -- the account
    # still works, it just shows no email until Google confirms one.
    verified = str(claims.get("email_verified", "")).lower() == "true"

    subject = claims.get("sub", "")
    if not subject:
        raise NotGenuine("that sign-in named nobody")

    return Identity(
        provider="google",
        subject=subject,
        email=claims.get("email", "") if verified else "",
        name=claims.get("name", ""),
        avatar_url=claims.get("picture", ""),
    )


def identify_github(proof: str) -> Identity:
    """Check a GitHub access token by asking GitHub who holds it.

    There is no ID token to verify here -- GitHub's device flow ends in an
    access token and nothing else -- so the proof is that the token works. A
    forged one fails at GitHub rather than at us.
    """
    try:
        reply = httpx.get(
            "https://api.github.com/user",
            headers={
                "Authorization": f"Bearer {proof}",
                "Accept": "application/vnd.github+json",
            },
            timeout=10,
        )
    except httpx.HTTPError as exc:
        raise NotGenuine("could not reach GitHub") from exc

    if reply.status_code != 200:
        raise NotGenuine("GitHub did not recognise that sign-in")

    who = reply.json()
    subject = str(who.get("id", ""))
    if not subject:
        raise NotGenuine("that sign-in named nobody")

    # `email` is null whenever the address is private, which is the default for
    # a lot of accounts. Absent rather than fetched from /user/emails: reading
    # somebody's private addresses to decorate a settings page is not a trade
    # worth making, and nothing here needs one.
    return Identity(
        provider="github",
        subject=subject,
        email=who.get("email") or "",
        name=who.get("name") or who.get("login", ""),
        avatar_url=who.get("avatar_url", ""),
    )


# The only providers there are. A dict rather than an if-chain so that the route
# can reject an unknown one before doing any work, and so adding a third is one
# function and one line.
PROVIDERS = {
    "google": identify_google,
    "github": identify_github,
}
