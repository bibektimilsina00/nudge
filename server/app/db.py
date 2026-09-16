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
    _add_missing_columns()


def _add_missing_columns() -> None:
    """Add columns that `create_all` will not, because the table already exists.

    `create_all` creates missing *tables* and never touches an existing one, so
    a new field on a live table is silently absent until something selects it
    and SQLite says `no such column`.

    ponytail: a hand-rolled add-column pass, not a migration tool. It handles
    the only schema change this has ever needed -- a nullable column with no
    default -- and nothing else: no renames, no drops, no type changes, no down
    path. The moment one of those is needed, this is not the thing to extend;
    bring in Alembic, which the server already has the dependencies for.
    """
    from sqlalchemy import inspect, text

    inspector = inspect(engine)
    with engine.begin() as conn:
        for table in SQLModel.metadata.sorted_tables:
            if table.name not in inspector.get_table_names():
                continue
            have = {c["name"] for c in inspector.get_columns(table.name)}
            for column in table.columns:
                if column.name in have or not column.nullable:
                    continue
                kind = column.type.compile(engine.dialect)
                conn.execute(
                    text(f'ALTER TABLE "{table.name}" ADD COLUMN "{column.name}" {kind}')
                )
                print(f"db: added {table.name}.{column.name}")


def session() -> Generator[Session, None, None]:
    with Session(engine) as s:
        yield s
