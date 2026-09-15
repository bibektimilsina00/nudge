from functools import lru_cache
from pathlib import Path

from pydantic_settings import BaseSettings, SettingsConfigDict


class Settings(BaseSettings):
    """Configuration, from the environment or a .env beside this project."""

    model_config = SettingsConfigDict(env_file=".env", env_prefix="NUDGE_", extra="ignore")

    # SQLite to start, because a landing page that needs a database server before
    # it can run is a landing page nobody runs locally. Point it at Postgres in
    # production and nothing else changes -- SQLModel does not care.
    database_url: str = "sqlite:///./nudge.db"

    # Where the builds actually live. Self-hosted on purpose: a download button
    # that bounces to GitHub hands the relationship, the numbers and the first
    # impression to somebody else.
    releases_dir: Path = Path("releases")

    # The front end, for CORS. Not "*" -- this server will hold sessions before
    # long, and a permissive default that becomes a security hole later is worse
    # than one more line of config now.
    web_origin: str = "http://localhost:3000"


@lru_cache
def settings() -> Settings:
    return Settings()
