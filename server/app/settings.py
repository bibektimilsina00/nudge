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

    # Where this API is reachable from outside, for URLs that have to be
    # absolute. The updater is the one caller that needs this: it is handed a
    # URL and fetches it from a different process, so a relative path is not
    # something it can resolve.
    #
    # Not derived from the request. Uvicorn runs without `--proxy-headers`, so
    # behind Caddy the request looks like plain HTTP to an internal host, and a
    # download URL built from it would be an http:// address for a host that
    # only answers https. Empty falls back to the request, which is right for
    # running this locally and wrong in exactly one place -- so production sets
    # it, and `/api/update` says so when it is not set.
    public_url: str = ""

    # Which application a Google sign-in must have been issued to. The desktop
    # client id, and the same one the app itself uses -- an ID token names the
    # client it was minted for, and checking it is what stops a token from
    # somebody else's Google app being spent here. Public by design; it ships
    # inside the app either way.
    #
    # Empty means Google sign-in is refused rather than waved through. A missing
    # audience check is not a smaller check, it is no check at all.
    google_client_id: str = ""

    # The model key every signed-in copy borrows -- see routes/think.py.
    #
    # Empty means the proxy refuses rather than failing further in: an app told
    # "no key configured" can say so to a person, and one handed a 500 cannot.
    gemini_key: str = ""

    # How long a device stays signed in. Long, because the thing behind it is a
    # menu-bar app somebody opens for ten seconds at a time, and an app that
    # asks who you are every fortnight is an app people stop opening.
    session_days: int = 90


@lru_cache
def settings() -> Settings:
    return Settings()
