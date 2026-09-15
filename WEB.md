# The site and its API

Two pieces, both self-hosted:

- **`web/`** — the marketing page. Next, Tailwind, shadcn, TanStack Query, zod,
  zustand.
- **`server/`** — the API that serves the downloads and counts them. FastAPI on
  uv, SQLModel over SQLite, Postgres by changing one URL.

```sh
make site                       # both, with reload
make publish VERSION=0.1.0      # build the app and publish it to the API
```

The site is on :3000, the API on :8080, and its generated docs on
<http://localhost:8080/docs>.

## Why the downloads are served from here

A download button that bounces to a GitHub release hands over three things: the
numbers, the relationship, and the first impression — a visitor lands on a page
of build artefacts and has to work out which of them is theirs before they know
what the thing does.

So the API streams the file and writes a row as it goes. `Range` requests work,
which is what lets a 20MB download resume instead of restarting.

The download URL carries no version, so a link posted anywhere survives the next
release. The *file* does, so nobody's Downloads folder fills with three copies of
`Nudge.dmg`.

## Publishing

`server/publish.py` hashes the build, copies it into `releases/`, and makes it
the current one for its platform — flipping whatever was current before, so
exactly one row can be served.

It is a script rather than an endpoint on purpose. Publishing is rare, done by
one person, and the version that needs authentication needs an auth system first.

## Deploying

```sh
docker compose up --build
```

Two containers behind one hostname. Point a reverse proxy at both, sending
`/api` to the API and everything else to the site — then the browser makes no
cross-origin request at all, which is one fewer thing to get wrong. `PUBLIC_API`
is empty in that setup, which is why it defaults to empty.

Two things that bite:

- **`NEXT_PUBLIC_API` is baked in at build time.** Next substitutes it into the
  bundle during `pnpm build`, so setting it in the running container does
  nothing. It is a build arg in the compose file for that reason.
- **The releases volume is not in the image.** A 20MB binary in a layer is a
  20MB push every time a line of Python changes, and the build outlives whichever
  deploy happened to publish it.

## One line on the page has an expiry date

The FAQ says *"Nothing routes through a server of ours — there is not one in the
path."* That is true today and stops being true the moment the proxy in
[SERVER.md](SERVER.md) ships. Change it in the same commit.

## Tests

```sh
cd server && uv run pytest
```

Seven, covering the parts with something to get wrong: a missing build is a 404
rather than a crash, a published row whose file is gone is a 500 rather than an
empty download, publishing twice leaves exactly one current release, every
download is counted, and platforms do not leak into each other.
