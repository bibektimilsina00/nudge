from sqlmodel import Session, select

from app.models import Download, Release


def publish(client, *, version="1.0.0", platform="macos-arm64", body=b"not really a dmg"):
    name = f"Nudge-{version}-{platform}.dmg"
    (client.releases_dir / name).write_bytes(body)
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
