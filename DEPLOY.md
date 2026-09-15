# Deploying

Nudge's site runs on the shared box at `161.118.213.166`, alongside riocut and
edumlt, behind the Caddy those already use. Nothing here publishes a port — the
proxy reaches both containers by name over the existing `web` network.

    https://nudge.runmycrew.com  →  proxy-caddy  →  nudge-web-1:3000
                                                 →  nudge-api-1:8080   (/api/*)

`nudge.riocut.com` is served by the same block and works too, if a record is
pointed at it.

## DNS

One record in Cloudflare, per hostname:

| Type | Name | Content | Proxy |
|---|---|---|---|
| A | `nudge` | `161.118.213.166` | Proxied (orange) |

## The certificate, and why this one works

Caddy on this box serves `*.riocut.com`'s Cloudflare Origin certificate for
**both** hostnames. For `nudge.riocut.com` that is correct. For
`nudge.runmycrew.com` it is the wrong name, and works anyway because that zone
is on Cloudflare's **Full** mode, which encrypts to the origin without checking
what it is handed.

That is worth knowing rather than forgetting. **Switching runmycrew.com to Full
(strict) breaks this host immediately** — strict wants a certificate valid for
the name. The fix is a minute's work: create an Origin Certificate for
`*.runmycrew.com` in Cloudflare, put the pair in `/opt/proxy/certs/`, and give
that hostname its own block with its own `tls` line.

Until then: Cloudflare to the browser is properly encrypted, and Cloudflare to
this box is encrypted but unauthenticated.

**The symptom, if this is ever wrong: Cloudflare error 525.** That is the
handshake between Cloudflare and here failing, and the usual cause is a hostname
Caddy has no block for — it aborts rather than presenting anything, and 525 is
what the visitor sees.

## Deploying a change

    rsync -az --delete \
      --exclude node_modules --exclude .next --exclude .venv \
      --exclude __pycache__ --exclude .pytest_cache \
      --exclude nudge.db --exclude releases --exclude .env \
      server web ubuntu@161.118.213.166:/opt/nudge/

    ssh ubuntu@161.118.213.166 'cd /opt/nudge && docker compose up -d --build'

Built on the box rather than pushed from here, which works because the VPS is
`aarch64` and so is the machine building it. If that ever stops being true, this
becomes a registry push and a pull.

## Publishing a build

    make build
    scp src-tauri/target/release/bundle/dmg/Nudge_0.1.0_aarch64.dmg \
        ubuntu@161.118.213.166:/tmp/Nudge.dmg

    ssh ubuntu@161.118.213.166 'cd /opt/nudge && \
      docker cp /tmp/Nudge.dmg nudge-api-1:/tmp/Nudge.dmg && \
      docker compose exec -T api uv run publish.py /tmp/Nudge.dmg \
        --version 0.1.0 --platform macos-arm64 --notes "..." && \
      rm -f /tmp/Nudge.dmg'

The builds and the database are on named volumes, so neither is lost by a
rebuild.

## Things found the hard way on this box

**ghcr.io is denied, even anonymously.** There is a stale `ghcr.io` credential in
`~/.docker/config.json` which makes every pull from there fail, including public
images. It was left alone because other deploys may depend on it; the API's base
image was changed to `python:3.13-slim` instead, which is a better dependency
anyway. **Worth fixing separately** — it will bite the next thing that pulls from
ghcr.

**Caddy is shared and serves live sites.** Always validate before reloading:

    docker exec proxy-caddy caddy validate --config /etc/caddy/Caddyfile --adapter caddyfile
    docker exec proxy-caddy caddy reload   --config /etc/caddy/Caddyfile --adapter caddyfile

A reload with a bad file takes riocut.com down with it.

**`auto_https off` is set globally**, because edumlt is Cloudflare Flexible and
ACME would fail behind Cloudflare. That is why the nudge block names its
certificate explicitly rather than letting Caddy find one.

**The download needs a longer proxy timeout.** Builds are ~20MB and Caddy's
default response header timeout will cut a slow connection off part way.
