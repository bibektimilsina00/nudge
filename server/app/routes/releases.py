"""Serving builds, and counting them."""

from pathlib import Path

from fastapi import APIRouter, Depends, HTTPException, Request, Response
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


# What the updater calls a platform, and what we do.
#
# Tauri fills `{{target}}` and `{{arch}}` from the running build -- "darwin" and
# "aarch64" on an Apple silicon Mac -- and our rows are keyed by the names the
# download page uses. Mapped explicitly rather than string-built, so an
# unrecognised pair returns "no update" instead of inventing a platform that
# happens to have a row.
_TARGETS = {
    ("darwin", "aarch64"): "macos-arm64",
    ("darwin", "x86_64"): "macos-x64",
    ("windows", "x86_64"): "windows-x64",
    ("linux", "x86_64"): "linux-x64",
}


# `response_model=None`: the two answers have different shapes -- a bare 204 and
# a manifest -- and FastAPI cannot build one response model from both.
@router.get("/api/update/{target}/{arch}/{current}", response_model=None)
def update(
    target: str,
    arch: str,
    current: str,
    request: Request,
    db: Session = Depends(session),
) -> Response | dict:
    """What a running copy asks, every time it starts.

    **204 means "you are up to date"**, and that is the updater's contract rather
    than our choice -- a 404 or an error body here shows up as a failed update
    check in the app, and an app that reports a problem every launch because
    nothing is wrong is worse than one that never checks.

    So every answer that is not a genuine newer build is a 204: unknown
    platform, nothing published, a published build with no update artifact, and
    a build that is the same age or older than the one asking.
    """
    platform = _TARGETS.get((target, arch))
    if not platform:
        return Response(status_code=204)

    release = _current(db, platform)
    # A release published before update artifacts existed has no signature, and
    # offering it would hand the updater a download it cannot verify.
    if not release or not release.update_file or not release.signature:
        return Response(status_code=204)

    if not _newer(current, release.version):
        return Response(status_code=204)

    # Absolute, unlike everywhere else in this file. The updater is handed this
    # URL and fetches it itself, so there is no page for a relative path to be
    # relative to.
    base = settings().public_url.rstrip("/") or str(request.base_url).rstrip("/")
    return {
        "version": release.version,
        "notes": release.notes,
        "pub_date": release.published_at.isoformat(),
        "url": f"{base}/api/update/download/{platform}",
        "signature": release.signature,
    }


def _newer(have: str, offered: str) -> bool:
    """Mirrors `core::newer::newer` in the app, and must keep mirroring it.

    Both sides check: the server so it does not offer a downgrade to everybody
    at once, the app so a wrong answer from a server is still refused by the
    thing installing it. Two cheap checks beat one trusted one.
    """

    def parts(v: str) -> list[int]:
        out = []
        for piece in v.strip().lstrip("v").replace("-", ".").replace("+", ".").split("."):
            out.append(int(piece) if piece.isdigit() else -1)
        return out

    a, b = parts(have), parts(offered)
    for i in range(max(len(a), len(b))):
        x = a[i] if i < len(a) else 0
        y = b[i] if i < len(b) else 0
        if x != y:
            return y > x
    return False


@router.get("/api/update/download/{platform}")
def update_download(platform: str, db: Session = Depends(session)) -> FileResponse:
    """The update artifact itself -- the .app.tar.gz, not the .dmg.

    Separate from `/api/download` because they are different files for different
    moments: one is a person deciding to try this, the other is a copy already
    running replacing itself. Counting them together would make the download
    numbers meaningless the day updates start flowing.
    """
    release = _current(db, platform)
    if not release or not release.update_file:
        raise HTTPException(404, f"no update published for {platform}")

    path: Path = settings().releases_dir / release.update_file
    if not path.is_file():
        raise HTTPException(500, f"{release.update_file} is published but not on disk")

    return FileResponse(
        path,
        media_type="application/octet-stream",
        filename=release.update_file,
    )
