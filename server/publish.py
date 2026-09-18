"""Publish a build: hash it, copy it in, make it the current one.

Run from the repository root after `make build`:

    uv run --directory server publish.py \
      ../src-tauri/target/release/bundle/dmg/Nudge_0.1.0_aarch64.dmg \
      --version 0.1.0 --platform macos-arm64 --notes "First build."

Everything here is deliberately a script rather than an endpoint. Publishing is
rare, done by one person, and the version that needs authentication is the
version that needs an auth system first.
"""

import argparse
import hashlib
import shutil
import sys
from pathlib import Path

from sqlmodel import Session, select

from app.db import create_tables, engine
from app.models import Release
from app.settings import settings


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    # Chunked, because a 20MB file read into memory is fine and a 2GB one is not,
    # and the difference is one argument.
    with path.open("rb") as f:
        for block in iter(lambda: f.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def looks_like(given: str) -> str:
    """The signature, whether a path to it or the thing itself was given."""
    try:
        path = Path(given)
        if path.is_file():
            return path.read_text().strip()
    except OSError:
        # Too long to be a filename, which means it is the signature.
        pass
    return given.strip()


def update_name(version: str, platform: str, update: Path) -> str:
    """What to call the updater's archive on disk.

    The suffix comes off the file rather than a constant. It was ".app.tar.gz"
    for as long as macOS was the only platform; a Linux release ships
    ".AppImage.tar.gz", and a name that lied about it would hand the updater a
    macOS bundle under a Linux row.
    """
    suffix = "".join(update.suffixes[-3:]) or update.suffix
    return f"Nudge-{version}-{platform}{suffix}"


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("file", type=Path)
    ap.add_argument("--version", required=True)
    ap.add_argument("--platform", default="macos-arm64")
    ap.add_argument("--notes", default="")
    # The in-app updater's half of a release. Optional, because a platform can
    # have a download before it has updates -- but published together when they
    # exist, so "the current release" stays one fact rather than two that can
    # drift apart.
    ap.add_argument(
        "--update",
        type=Path,
        help="the archive the updater installs: .app.tar.gz on macOS, .AppImage.tar.gz on Linux",
    )
    ap.add_argument("--signature", help="minisign signature over --update, or a path to it")
    args = ap.parse_args()

    if bool(args.update) != bool(args.signature):
        print("--update and --signature go together or not at all", file=sys.stderr)
        return 1

    if not args.file.is_file():
        print(f"no such file: {args.file}", file=sys.stderr)
        return 1

    create_tables()
    dest_dir = settings().releases_dir
    dest_dir.mkdir(parents=True, exist_ok=True)

    # Named with the version, so somebody's Downloads folder does not fill with
    # three files called Nudge.dmg.
    filename = f"Nudge-{args.version}-{args.platform}{args.file.suffix}"
    dest = dest_dir / filename
    shutil.copy2(args.file, dest)

    update_file = None
    signature = None
    if args.update:
        if not args.update.is_file():
            print(f"no such file: {args.update}", file=sys.stderr)
            return 1
        update_file = update_name(args.version, args.platform, args.update)
        shutil.copy2(args.update, dest_dir / update_file)
        # Either the signature itself or the .sig file holding it. The workflow
        # has a path; a person at a terminal has whichever is nearer.
        signature = looks_like(args.signature)
        if not signature:
            print("the signature is empty", file=sys.stderr)
            return 1

    with Session(engine) as db:
        # Exactly one current release per platform, enforced here rather than in
        # the schema so publishing stays a single command.
        for old in db.exec(
            select(Release).where(Release.platform == args.platform, Release.current)
        ).all():
            old.current = False
            db.add(old)

        db.add(
            Release(
                version=args.version,
                platform=args.platform,
                filename=filename,
                size_bytes=dest.stat().st_size,
                sha256=sha256(dest),
                notes=args.notes,
                current=True,
                update_file=update_file,
                signature=signature,
            )
        )
        db.commit()

    print(f"published {filename} ({dest.stat().st_size / 1_000_000:.1f} MB)")
    print(f"  update: {update_file}" if update_file else "  no update artifact -- downloads only")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
