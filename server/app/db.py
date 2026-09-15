from collections.abc import Generator

from sqlmodel import Session, SQLModel, create_engine

from app.settings import settings

_url = settings().database_url
# `check_same_thread` is a SQLite-only quirk: FastAPI serves requests from a
# thread pool, and SQLite objects to being touched from a thread other than the
# one that made them unless told otherwise.
_args = {"check_same_thread": False} if _url.startswith("sqlite") else {}

engine = create_engine(_url, connect_args=_args)


def create_tables() -> None:
    SQLModel.metadata.create_all(engine)


def session() -> Generator[Session, None, None]:
    with Session(engine) as s:
        yield s
