# The site, the API, and how they ship

Two pieces, both self-hosted, on the shared box at `161.118.213.166` alongside
riocut and edumlt, behind the Caddy those already use. Nothing here publishes a
port — the proxy reaches both containers by name over the existing `web` network.

    https://nudge.runmycrew.com  →  proxy-caddy  →  nudge-web-1:3000
                                                 →  nudge-api-1:8080   (/api/*)

- **`web/`** — the marketing page. Next, Tailwind, shadcn, TanStack Query, zod,
  zustand.
- **`server/`** — the API that serves the downloads and counts them, and answers
  the in-app updater. FastAPI on uv, SQLModel over SQLite, Postgres by changing
  one URL.

`nudge.riocut.com` is served by the same block and works too, if a record is
pointed at it.

## Working on it

```sh
make site                       # both, with reload
make publish VERSION=0.1.0      # build the app and publish it to the API
cd server && uv run pytest      # the API's tests
```

The site is on :3000, the API on :8080, and its generated docs on
<http://localhost:8080/docs>.

The tests cover the parts with something to get wrong: a missing build is a 404
rather than a crash, a published row whose file is gone is a 500 rather than an
empty download, publishing twice leaves exactly one current release, every
download is counted, platforms do not leak into each other, and the updater
answers 204 for every case that is not a genuine newer build.

## Why FastAPI, not Rust

Chosen for the bulk of the work rather than for the interesting part of it.
Accounts, sessions, migrations, billing and an admin view are most of what a
server like this *is*, and Python's ready-made surface for them — SQLAlchemy 2.0,
Alembic, Stripe's own SDK — is years ahead of the Rust equivalents. The proxy is
I/O-bound, which async Python is fine at.

Rust was the other candidate, and the argument for it was sharing types with
`src-tauri/src/core`. That argument is real but small: a pass-through proxy does
not need to understand what it is forwarding, so almost none of the provider code
would be reused.

**Where it will hurt, so it is not a surprise.** Streaming proxies are the one
workload where Python's per-connection cost shows: each open LLM stream holds a
coroutine and a buffer, and Axum would hold roughly ten times the connections per
box. That bites at *thousands of concurrent streams*, which is a long way from
here and a good problem to have. If it happens, move that one endpoint to Rust
and leave the rest — a proxy endpoint has no database access and no session
handling, so it is the easiest thing in the system to lift out. Do not pre-pay
for it.

The other cost is type drift: Nudge's shapes are Rust structs and the server will
have Pydantic models of the same things. Generate the Rust types from the OpenAPI
schema rather than copying them by hand — two hand-maintained copies of one shape
diverge, and the divergence is found by a user.

## Why the downloads are served from here

A download button that bounces to a GitHub release hands over three things: the
numbers, the relationship, and the first impression — a visitor lands on a page of
build artefacts and has to work out which of them is theirs before they know what
the thing does.

So the API streams the file and writes a row as it goes. `Range` requests work,
which is what lets a 20MB download resume instead of restarting.

The download URL carries no version, so a link posted anywhere survives the next
release. The *file* does, so nobody's Downloads folder fills with three copies of
`Nudge.dmg`.

`server/publish.py` hashes the build, copies it into `releases/`, and makes it the
current one for its platform — flipping whatever was current before, so exactly
one row can be served. It publishes the `.dmg` and, together with it, the
`.app.tar.gz` and minisign signature the in-app updater installs; they go on one
row so "the current release" stays one fact.

It is a script rather than an endpoint on purpose. Publishing is rare, done by one
person, and the version that needs authentication needs an auth system first.

## DNS

One record in Cloudflare, per hostname:

| Type | Name | Content | Proxy |
|---|---|---|---|
| A | `nudge` | `161.118.213.166` | Proxied (orange) |

## Certificates

One Cloudflare Origin certificate per zone, in `/opt/proxy/certs/`, and a Caddy
block per hostname naming its own:

| Host | Certificate |
|---|---|
| `nudge.runmycrew.com` | `runmycrew.pem` / `runmycrew.key` — `*.runmycrew.com` |
| `nudge.riocut.com` | `origin.pem` / `origin.key` — `*.riocut.com`, shared with riocut |

Both are correct for their own name, so **Full (strict)** is safe on either zone.

They briefly shared riocut's certificate, which worked only because
runmycrew.com was on **Full** — that encrypts to the origin without checking what
it is handed. It would have broken the moment anybody switched that zone to
strict, with no clue as to why.

Installing a new one:

```sh
sudo install -o ubuntu -g ubuntu -m 644 cert.pem /opt/proxy/certs/<zone>.pem
sudo install -o ubuntu -g ubuntu -m 600 cert.key /opt/proxy/certs/<zone>.key
```

Check the pair matches before reloading — a mismatched cert and key is another
525 with no explanation:

```sh
openssl x509 -in cert.pem -noout -pubkey | openssl md5
openssl pkey -in cert.key -pubout   | openssl md5
```

**Cloudflare error 525 means the handshake between Cloudflare and this box
failed.** The usual causes, in order: no Caddy block for that hostname (it aborts
rather than presenting anything), a certificate that does not cover the name while
the zone is on Full (strict), or a cert and key that are not a pair.

## What deploys when

Both `ci.yml` and `deploy.yml` start with a `changed` job running
`dorny/paths-filter`, and every other job is gated on its flag:

| Change | What runs |
|---|---|
| `web/` | web lint, typecheck, build → rebuilds **one container** |
| `server/` | pytest → rebuilds the API. The site is not touched |
| `src-tauri/` | fmt, clippy, tests. Deploys nothing — the app ships on a tag |
| `deploy/compose.yml` | rebuilds both, because it defines both |

`docker compose up -d --build <service>` names the service deliberately. Without
the name it rebuilds everything in the file, which is the behaviour this exists to
avoid.

**The filter needs a base.** Given none, the action compares against the
repository's *default* branch — and `main` has no `server/` or `web/` in it at
all, so every path read as added and every push to `alpha` redeployed everything.
`base: ${{ github.ref }}` is what makes it compare against the previous commit on
the same branch.

**A deploy that finishes is not a deploy that works.** Each job polls its own
container through the proxy until it answers, and fails with the last forty lines
of its log if it does not. Without that, a container that builds and then crashes
on boot reports a green tick.

**Redeploying without a commit:** Actions → Deploy → Run workflow, and pick `web`,
`server` or `both`. For when a container died or the box came back up empty.

## Secrets

| Secret | What |
|---|---|
| `DEPLOY_KEY` | private half of a key generated for this, and nothing else |
| `DEPLOY_HOST` | the VPS |
| `DEPLOY_USER` | `ubuntu` |
| `DEPLOY_KNOWN_HOSTS` | the box's fingerprint, so the deploy never trusts blindly |

The key is its own, not a copy of anybody's personal key: it is appended to
`authorized_keys` on the box and can be revoked by deleting that one line without
locking anybody out.

Signing secrets for the app are separate and not set yet — see
[RELEASING.md](RELEASING.md).

## Two things that were wrong before they were right

**`ssh host VAR=x 'script'` does not pass an environment.** It looks like it does.
The remote shell expands `$VAR` in the very command that assigns it, so it arrives
empty — checked on the real box, and it would have published every release with a
blank version. The working form is `ssh host "env VAR=... bash -s" <<'REMOTE'`,
with the values run through `printf %q` so a release note containing an apostrophe
cannot end the string.

**A commit message is untrusted input.** `${{ github.event.head_commit.message }}`
is substituted *before* bash sees the line, so a message carrying a quote and a
semicolon runs as us on the deploy box. It arrives through `env:` instead. This is
the standard Actions injection and it is easy to write by accident.

## By hand, when CI cannot

    rsync -az --delete \
      --exclude node_modules --exclude .next --exclude .venv \
      --exclude __pycache__ --exclude .pytest_cache \
      --exclude nudge.db --exclude releases --exclude .env \
      server web ubuntu@161.118.213.166:/opt/nudge/

    ssh ubuntu@161.118.213.166 'cd /opt/nudge && docker compose up -d --build'

Built on the box rather than pushed from here, which works because the VPS is
`aarch64` and so is the machine building it. If that stops being true, this
becomes a registry push and a pull.

**Never rsync the root `docker-compose.yml`.** That one is for laptops: it
publishes ports and builds its own network. Copying it here once took the
containers off the shared network and the site to 502. The production file is
`deploy/compose.yml` and it goes over as `docker-compose.yml`.

**Taking a build down** — `publish.py` makes a release current and nothing makes
it un-current, so this is by hand:

```sh
ssh <box> "cd /opt/nudge && docker compose exec -T api uv run python -c '
from app.db import engine; from app.models import Release
from sqlmodel import Session, select
with Session(engine) as s:
    r = s.exec(select(Release).where(Release.current)).first()
    r.current = False; s.add(r); s.commit()'"
```

## Things found the hard way on this box

**ghcr.io is denied, even anonymously.** A stale `ghcr.io` credential in
`~/.docker/config.json` makes every pull from there fail, including public images.
It was left alone because other deploys may depend on it; the API's base image was
changed to `python:3.13-slim` instead, which is a better dependency anyway.
**Worth fixing separately** — it will bite the next thing that pulls from ghcr.

**Caddy is shared and serves live sites.** Its file is `/opt/proxy/sites/nudge.caddy`
and a bad reload takes riocut.com and edumlt down with it. Always validate first:

    docker exec proxy-caddy caddy validate --config /etc/caddy/Caddyfile --adapter caddyfile
    docker exec proxy-caddy caddy reload   --config /etc/caddy/Caddyfile --adapter caddyfile

Caddy is the one part still done by hand, deliberately.

**`auto_https off` is set globally**, because edumlt is Cloudflare Flexible and
ACME would fail behind Cloudflare. That is why the nudge block names its
certificate explicitly rather than letting Caddy find one.

**The download needs a longer proxy timeout.** Builds are ~20MB and Caddy's
default response header timeout will cut a slow connection off part way.

**`NEXT_PUBLIC_API` is baked in at build time.** Next substitutes it into the
bundle during `pnpm build`, so setting it in the running container does nothing.
It is a build arg in the compose file for that reason.

**The releases volume is not in the image.** A 20MB binary in a layer is a 20MB
push every time a line of Python changes, and the build outlives whichever deploy
happened to publish it. Builds and the database are on named volumes, so neither
is lost by a rebuild.

## The sentence with an expiry date

The FAQ says *"Nothing routes through a server of ours — there is not one in the
path."* True today, and false the day a model proxy ships.

Nudge is bring-your-own-key: `config.toml` or the Keychain, and `config.rs` opens
by saying that file *is* the bring-your-own-model feature. A proxy inverts it — it
means owning the API bill, being the reason the product is down when the server
is, sitting in the content path of everything anybody asks, and adding a hop to a
loop already at 2.4s. It is also the only way to charge a subscription.

**Both can be true at once:** bring-your-own-key stays and is free; the proxy is
what people pay for. That keeps it *optional*, so it failing degrades Nudge rather
than stopping it — and an optional dependency is a much cheaper thing to operate.

Change that sentence in the same commit as the proxy. It is the kind of line that
survives a launch because nobody remembers writing it, and being wrong about where
somebody's screen goes is not a thing to be wrong about.
