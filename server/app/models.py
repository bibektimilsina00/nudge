"""What the server knows.

Releases and the downloads that prove they were wanted, and the people who
signed in. A download that goes to GitHub is a download nobody can count; an
account is what turns a count into a person.
"""

from datetime import datetime, timedelta, timezone
from typing import Optional

from sqlmodel import Field, SQLModel, UniqueConstraint


def now() -> datetime:
    """UTC, always. A naive timestamp is a bug that surfaces months later."""
    return datetime.now(timezone.utc)


def aware(when: datetime) -> datetime:
    """The same moment, with a timezone on it.

    SQLite has no timestamp type, so what goes in as UTC-aware comes back naive
    and comparing it to `now()` raises rather than answering. Postgres keeps the
    offset and this changes nothing -- which is the point, since the difference
    should not decide whether a session is expired.
    """
    return when if when.tzinfo else when.replace(tzinfo=timezone.utc)


class Release(SQLModel, table=True):
    """A build people can download.

    Rows rather than a directory listing, because a release has facts a filename
    cannot carry -- notes, a checksum, whether it is the current one.
    """

    id: Optional[int] = Field(default=None, primary_key=True)
    version: str = Field(index=True)
    # "macos-arm64", "macos-x64" later, "windows-x64" after that. A string
    # rather than an enum: adding a platform should not be a migration.
    platform: str = Field(index=True)
    # Relative to `releases_dir`. Stored rather than derived so a file can be
    # renamed or moved without rewriting the URL people already have.
    filename: str
    size_bytes: int
    # Published so somebody can check what they downloaded is what was offered.
    # The whole point of self-hosting is that this is our claim to make.
    sha256: str
    notes: str = ""
    # Exactly one per platform should be current. Enforced in the query rather
    # than the schema, so publishing a new one is a single update.
    current: bool = Field(default=False, index=True)
    published_at: datetime = Field(default_factory=now)

    # What the in-app updater installs, which is not what a person downloads.
    # A .dmg is a disk image somebody mounts and drags out of once; the updater
    # wants the .app.tar.gz and the minisign signature over it. Both are
    # produced by the same build, and keeping them on one row is what makes
    # "the current release" one fact rather than two that can disagree.
    #
    # Optional because a release published before this existed has neither, and
    # a row with no update artifact should read as "no update offered" rather
    # than crash the endpoint.
    update_file: Optional[str] = None
    signature: Optional[str] = None


class Download(SQLModel, table=True):
    """One download, recorded as it is served.

    Deliberately thin. There is no account yet and no cookie, and the honest
    version of "who downloaded this" before there are accounts is "nobody, but
    here is how many and roughly from where".
    """

    id: Optional[int] = Field(default=None, primary_key=True)
    release_id: int = Field(foreign_key="release.id", index=True)
    at: datetime = Field(default_factory=now, index=True)
    # Kept because it distinguishes a person from a link checker, and dropped as
    # soon as there is anything better. Not an identifier.
    user_agent: str = ""
    # Where somebody arrived from, which is the only marketing number that
    # matters early on.
    referrer: str = ""


class User(SQLModel, table=True):
    """Somebody with an account.

    Identified by the provider's subject rather than by the email address.
    Emails change hands -- a company address outlives the person who held it --
    and the subject is the only thing Google or GitHub promises is stable and
    theirs alone.

    Signing in with Google and then with GitHub makes two users, even for one
    person with one address. Merging them means deciding that a matching email
    proves the same human, which is only true when both providers say they
    verified it, and is a well-worn way to let somebody into an account that is
    not theirs. Linking is a feature to design on purpose, not a default to fall
    into; until then the honest behaviour is two accounts.
    """

    __table_args__ = (UniqueConstraint("provider", "subject", name="one_row_per_identity"),)

    id: Optional[int] = Field(default=None, primary_key=True)
    # "google" | "github". A string rather than an enum, for the same reason
    # `Release.platform` is one: adding a provider should not be a migration.
    provider: str = Field(index=True)
    # The provider's own id for this person. Opaque, and never shown.
    subject: str = Field(index=True)
    # Stored to show in the app, not to identify by. May be empty: a GitHub
    # account can keep every address private.
    email: str = Field(default="", index=True)
    name: str = ""
    avatar_url: str = ""
    created_at: datetime = Field(default_factory=now)
    last_seen_at: datetime = Field(default_factory=now)


class Login(SQLModel, table=True):
    """One signed-in copy of the app.

    A row per device rather than a column on the user, so signing out of a Mac
    you no longer have does not sign you out of the one in front of you.

    What is stored is the hash of the token, never the token. The app holds the
    only copy; a leaked database is then a list of useless hashes rather than a
    set of live sessions. It is the same reason passwords are not stored either,
    and it costs one line.
    """

    id: Optional[int] = Field(default=None, primary_key=True)
    user_id: int = Field(foreign_key="user.id", index=True)
    token_hash: str = Field(index=True, unique=True)
    created_at: datetime = Field(default_factory=now)
    last_used_at: datetime = Field(default_factory=now)
    expires_at: datetime
    # Which app asked, for a future "signed in on these devices" list. Free to
    # record now and impossible to backfill later.
    device: str = ""


def expiry(days: int) -> datetime:
    """When a session minted now should stop working."""
    return now() + timedelta(days=days)
