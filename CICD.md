# CI/CD

Three things live in this repository and they deploy on different schedules:

| | On | Where it goes |
|---|---|---|
| `web/` | push to `main`/`alpha` | container on the VPS |
| `server/` | push to `main`/`alpha` | container on the VPS |
| `src-tauri/`, `ui/` | a `v*` tag | signed build → the download API |

## Only what changed

Both `ci.yml` and `deploy.yml` start with a `changed` job that runs
`dorny/paths-filter` and emits one flag per component. Every other job is gated
on its flag, so:

- editing a heading in `web/` runs the web lint, typecheck and build, then
  rebuilds **one container**. The API keeps its process, its database and its
  published builds.
- editing `server/` runs pytest and rebuilds the API. The site is not touched.
- editing `src-tauri/` runs fmt, clippy and the Rust tests, and deploys nothing —
  the app ships on a tag.
- editing `docker-compose.yml` rebuilds both, because it defines both.

`docker compose up -d --build <service>` names the service deliberately. Without
the name it rebuilds everything in the file, which is the behaviour this exists
to avoid.

## Secrets

Already set on the repository:

| Secret | What |
|---|---|
| `DEPLOY_KEY` | private half of a key generated for this, and nothing else |
| `DEPLOY_HOST` | the VPS |
| `DEPLOY_USER` | `ubuntu` |
| `DEPLOY_KNOWN_HOSTS` | the box's fingerprint, so the deploy never trusts blindly |

The key is its own, not a copy of anybody's personal key: it is appended to
`authorized_keys` on the box and can be revoked by deleting that one line
without locking anybody out.

Signing secrets for the app are separate and not set yet — see
[RELEASING.md](RELEASING.md).

## Two things that were wrong before they were right

**`ssh host VAR=x 'script'` does not pass an environment.** It looks like it
does. The remote shell expands `$VAR` in the very command that assigns it, so it
arrives empty — checked on the real box, and it would have published every
release with a blank version. The working form is `ssh host "env VAR=... bash -s"
<<'REMOTE'`, with the values run through `printf %q` so a release note containing
an apostrophe cannot end the string.

**A commit message is untrusted input.** `${{ github.event.head_commit.message }}`
is substituted *before* bash sees the line, so a message carrying a quote and a
semicolon runs as us on the deploy box. It arrives through `env:` instead. This
is the standard Actions injection and it is easy to write by accident.

## A deploy that finishes is not a deploy that works

Each deploy job polls its own container through the proxy until it answers, and
fails with the last forty lines of its log if it does not. Without that, a
container that builds and then crashes on boot reports a green tick.

## Redeploying without a commit

Actions → Deploy → Run workflow, and pick `web`, `server` or `both`. For when a
container died or the box came back up empty.

## Still by hand

Caddy. The site file lives at `/opt/proxy/sites/nudge.caddy` on the box and is
shared with riocut and edumlt — a bad reload takes their sites down too, so it is
a deliberate act with a `caddy validate` in front of it. See
[DEPLOY.md](DEPLOY.md).
