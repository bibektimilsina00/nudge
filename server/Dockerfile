# Nudge's API.
FROM ghcr.io/astral-sh/uv:python3.13-bookworm-slim

WORKDIR /srv

# Dependencies before source, so editing a route does not reinstall the world.
COPY pyproject.toml uv.lock ./
RUN uv sync --frozen --no-dev --no-install-project

COPY app ./app
COPY publish.py ./
RUN uv sync --frozen --no-dev

# Builds live on a volume, not in the image. A 20MB binary baked into a layer is
# a 20MB push every time a line of Python changes, and the release outlives the
# deploy that happened to be current when it was published.
VOLUME ["/srv/releases"]
ENV NUDGE_RELEASES_DIR=/srv/releases

EXPOSE 8080
CMD ["uv", "run", "uvicorn", "app.main:app", "--host", "0.0.0.0", "--port", "8080"]
