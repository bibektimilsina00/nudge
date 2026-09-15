"""Serving builds, and counting them."""

from pathlib import Path

from fastapi import APIRouter, Depends, HTTPException, Request
from fastapi.responses import FileResponse
from pydantic import BaseModel
from sqlmodel import Session, select

from app.db import session
from app.models import Download, Release
from app.settings import settings

router = APIRouter(tags=["releases"])


class ReleaseOut(BaseModel):
    """What the site needs to render a download button honestly."""

    version: str
    platform: str
    size_bytes: int
    sha256: str
    notes: str
    published_at: str
    # Relative, so the same manifest works behind any hostname.
    download_url: str


def _current(db: Session, platform: str) -> Release | None:
    return db.exec(
        select(Release).where(Release.platform == platform, Release.current)
    ).first()


@router.get("/api/releases/{platform}", response_model=ReleaseOut)
def latest(platform: str, db: Session = Depends(session)) -> ReleaseOut:
    release = _current(db, platform)
    if not release:
        raise HTTPException(404, f"nothing published for {platform} yet")
    return ReleaseOut(
        version=release.version,
        platform=release.platform,
        size_bytes=release.size_bytes,
        sha256=release.sha256,
        notes=release.notes,
        published_at=release.published_at.isoformat(),
        download_url=f"/api/download/{platform}",
    )


@router.get("/api/download/{platform}")
def download(platform: str, request: Request, db: Session = Depends(session)) -> FileResponse:
    """Stream the build, and write down that it happened.

    A stable URL rather than one with a version in it, so a link posted anywhere
    keeps working after the next release. The filename people end up with still
    carries the version, because a Downloads folder with three files called
    `Nudge.dmg` helps nobody.
    """
    release = _current(db, platform)
    if not release:
        raise HTTPException(404, f"nothing published for {platform} yet")

    path: Path = settings().releases_dir / release.filename
    if not path.is_file():
        # The row and the disk disagreeing is an operational problem, not a
        # missing page, and saying so is what makes it findable.
        raise HTTPException(500, f"{release.filename} is published but not on disk")

    # Recorded before the file is sent, not after. A large download that is
    # cancelled half way still tells us somebody tried, which is the number that
    # matters -- and FileResponse hands off to the server, so there is no "after"
    # to hook anyway.
    db.add(
        Download(
            release_id=release.id,  # type: ignore[arg-type]
            user_agent=request.headers.get("user-agent", "")[:400],
            referrer=request.headers.get("referer", "")[:400],
        )
    )
    db.commit()

    # FileResponse handles Range requests, which is what lets a 20MB download
    # resume rather than start again.
    return FileResponse(
        path,
        media_type="application/octet-stream",
        filename=release.filename,
    )
