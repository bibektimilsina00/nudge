"""What the server knows.

Two tables, and the second one is the reason the first exists: a download that
goes to GitHub is a download nobody can count.
"""

from datetime import datetime, timezone
from typing import Optional

from sqlmodel import Field, SQLModel


def now() -> datetime:
    """UTC, always. A naive timestamp is a bug that surfaces months later."""
    return datetime.now(timezone.utc)


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
