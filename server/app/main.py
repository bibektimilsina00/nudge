"""Nudge's server.

Today it serves the downloads the marketing page offers. It is shaped for what
comes next -- accounts, a database behind them, and a proxy in front of the model
providers -- which is why there is a database here at all for something that
could have been a static file. See SERVER.md at the root of the repository for
why FastAPI, and for the one workload that will eventually want moving to Rust.
"""

from contextlib import asynccontextmanager

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware

from app.db import create_tables
from app.routes import auth, events, releases, think
from app.settings import settings


@asynccontextmanager
async def lifespan(app: FastAPI):
    # Fine while the schema is two tables nobody has data in. The moment a
    # column has to change under real rows, this becomes Alembic -- SQLModel is
    # SQLAlchemy underneath, so that is a migration to add, not a rewrite.
    create_tables()
    yield


app = FastAPI(
    title="Nudge",
    version="0.1.0",
    lifespan=lifespan,
)

app.add_middleware(
    CORSMiddleware,
    # Named, not "*". This server will hold sessions before long, and a
    # permissive default that quietly becomes a hole is worse than one line of
    # config now.
    allow_origins=[settings().web_origin],
    allow_credentials=True,
    allow_methods=["GET", "POST"],
    allow_headers=["*"],
)

app.include_router(auth.router)
app.include_router(releases.router)
app.include_router(think.router)
app.include_router(events.router)


@app.get("/health")
def health() -> dict[str, str]:
    return {"status": "ok"}
