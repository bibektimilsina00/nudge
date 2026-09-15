"""A server with its own database and its own release folder, per test.

Both matter. A shared SQLite file makes tests order-dependent, and a shared
release folder means a test that publishes leaves a file the next one finds.
"""

import pytest
from fastapi.testclient import TestClient
from sqlmodel import Session, SQLModel, create_engine

from app import db as db_module
from app.main import app
from app.settings import settings


@pytest.fixture
def client(tmp_path, monkeypatch):
    engine = create_engine(
        f"sqlite:///{tmp_path / 'test.db'}", connect_args={"check_same_thread": False}
    )
    SQLModel.metadata.create_all(engine)
    monkeypatch.setattr(db_module, "engine", engine)

    releases = tmp_path / "releases"
    releases.mkdir()
    # `settings()` is cached, so the object the app already holds is the one to
    # change -- patching the environment here would be read by nobody.
    monkeypatch.setattr(settings(), "releases_dir", releases)

    def session():
        with Session(engine) as s:
            yield s

    app.dependency_overrides[db_module.session] = session
    with TestClient(app) as c:
        c.releases_dir = releases  # type: ignore[attr-defined]
        c.engine = engine  # type: ignore[attr-defined]
        yield c
    app.dependency_overrides.clear()
