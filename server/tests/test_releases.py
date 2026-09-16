from sqlmodel import Session, select

from app.models import Download, Release


def publish(
    client,
    *,
    version="1.0.0",
    platform="macos-arm64",
    body=b"not really a dmg",
    updatable=False,
):
    name = f"Nudge-{version}-{platform}.dmg"
    (client.releases_dir / name).write_bytes(body)
    update_file = None
    if updatable:
        update_file = f"Nudge-{version}-{platform}.app.tar.gz"
        (client.releases_dir / update_file).write_bytes(b"not really a tarball")
    with Session(client.engine) as db:
        for old in db.exec(
            select(Release).where(Release.platform == platform, Release.current)
        ).all():
            old.current = False
            db.add(old)
        db.add(
            Release(
                version=version,
                platform=platform,
                filename=name,
                size_bytes=len(body),
                sha256="a" * 64,
                current=True,
                update_file=update_file,
                signature="minisign" if updatable else None,
            )
        )
        db.commit()


def test_nothing_published_is_a_404_not_a_crash(client):
    assert client.get("/api/releases/macos-arm64").status_code == 404
    assert client.get("/api/download/macos-arm64").status_code == 404


def test_the_manifest_describes_the_build(client):
    publish(client, version="0.2.0")
    body = client.get("/api/releases/macos-arm64").json()
    assert body["version"] == "0.2.0"
    assert body["size_bytes"] == len(b"not really a dmg")
    # Relative, so the same manifest works behind any hostname.
    assert body["download_url"].startswith("/api/")


def test_downloading_serves_the_bytes_and_names_the_file(client):
    publish(client, version="0.3.0", body=b"payload")
    r = client.get("/api/download/macos-arm64")
    assert r.status_code == 200
    assert r.content == b"payload"
    # The URL has no version in it; the file people end up with does.
    assert "Nudge-0.3.0-macos-arm64.dmg" in r.headers["content-disposition"]


def test_every_download_is_counted(client):
    publish(client)
    for _ in range(3):
        client.get("/api/download/macos-arm64", headers={"referer": "https://nudge.app/"})
    with Session(client.engine) as db:
        rows = db.exec(select(Download)).all()
    assert len(rows) == 3
    assert all(d.referrer == "https://nudge.app/" for d in rows)


def test_publishing_again_replaces_what_is_current(client):
    publish(client, version="1.0.0")
    publish(client, version="1.1.0")
    assert client.get("/api/releases/macos-arm64").json()["version"] == "1.1.0"
    with Session(client.engine) as db:
        current = db.exec(select(Release).where(Release.current)).all()
    # Exactly one, or the download route picks whichever the database felt like.
    assert len(current) == 1


def test_a_row_without_its_file_says_so_rather_than_serving_nothing(client):
    publish(client)
    next(client.releases_dir.iterdir()).unlink()
    r = client.get("/api/download/macos-arm64")
    # 500, not 404: the difference between "no build yet" and "the build is
    # missing" is the difference between a wait and a page somebody must fix.
    assert r.status_code == 500


def test_platforms_do_not_leak_into_each_other(client):
    publish(client, platform="macos-arm64", version="1.0.0")
    assert client.get("/api/releases/windows-x64").status_code == 404


# The updater's contract. Every one of these is a 204 rather than an error,
# because a failed update check surfaces in the app as a problem, and an app
# that reports a problem on every launch because nothing is wrong is worse than
# one that never looks.


def test_nothing_to_update_to_is_a_204(client):
    assert client.get("/api/update/darwin/aarch64/1.0.0").status_code == 204


def test_a_download_only_release_offers_no_update(client):
    """The artifact the updater installs is not the one people download.

    A release published before update artifacts existed has a .dmg and no
    signature, and offering it would hand the updater something it cannot
    verify.
    """
    publish(client, version="2.0.0")
    assert client.get("/api/update/darwin/aarch64/1.0.0").status_code == 204


def test_an_older_copy_is_offered_the_new_one(client):
    publish(client, version="2.0.0", updatable=True)
    r = client.get("/api/update/darwin/aarch64/1.0.0")
    assert r.status_code == 200
    body = r.json()
    assert body["version"] == "2.0.0"
    assert body["signature"] == "minisign"
    # Absolute. The updater fetches this from a different process, so there is
    # no page for a relative path to be relative to.
    assert body["url"].startswith("http"), body["url"]


def test_the_same_version_is_not_offered_to_itself(client):
    publish(client, version="2.0.0", updatable=True)
    assert client.get("/api/update/darwin/aarch64/2.0.0").status_code == 204


def test_a_newer_copy_is_never_walked_backwards(client):
    publish(client, version="2.0.0", updatable=True)
    assert client.get("/api/update/darwin/aarch64/3.0.0").status_code == 204


def test_an_unknown_platform_is_quiet_rather_than_wrong(client):
    publish(client, version="2.0.0", updatable=True)
    assert client.get("/api/update/plan9/sparc/1.0.0").status_code == 204


def test_the_update_artifact_is_served_and_is_not_the_dmg(client):
    publish(client, version="2.0.0", updatable=True)
    r = client.get("/api/update/download/macos-arm64")
    assert r.status_code == 200
    assert r.content == b"not really a tarball"


def test_update_downloads_are_not_counted_as_downloads(client):
    """Or the download numbers stop meaning anything the day updates flow."""
    publish(client, version="2.0.0", updatable=True)
    client.get("/api/update/download/macos-arm64")
    with Session(client.engine) as db:
        assert db.exec(select(Download)).all() == []
