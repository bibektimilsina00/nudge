# The server

Nothing is built yet. This is the decision and the escape hatch, written down
before either is forgotten.

## What it is for

Four things, and only the first exists as a need today:

- a page people can download builds from
- accounts, and a database behind them
- a proxy in front of the model providers, so somebody can use Nudge without
  bringing an API key
- whatever an eval or metering harness turns out to want

## FastAPI, not Rust

Chosen for the bulk of the work rather than for the interesting part of it.
Accounts, sessions, migrations, billing and an admin view are most of what a
server like this *is*, and Python's ready-made surface for them — SQLAlchemy 2.0,
Alembic, Stripe's own SDK — is years ahead of the Rust equivalents. The proxy is
I/O-bound, which async Python is fine at.

    FastAPI · Postgres · SQLAlchemy 2.0 · Alembic · httpx · Redis when rate
    limits need it · Next.js in front, deployed separately

Rust was the other candidate, and the argument for it was sharing types with
`src-tauri/src/core`. That argument is real but small: a pass-through proxy does
not need to understand what it is forwarding, so almost none of the provider code
in `core/provider/` would be reused.

## Where it will hurt, so it is not a surprise

**Streaming proxies are the one workload where Python's per-connection cost
shows.** Each open LLM stream holds a coroutine and a buffer; several uvicorn
workers will be doing what one Axum process would, and Axum would hold roughly
ten times the connections per box.

That bites at *thousands of concurrent streams*, which is a long way from here
and a good problem to have. **If it happens, move that one endpoint to Rust and
leave the rest in FastAPI.** A proxy endpoint is the easiest thing in the system
to lift out: it has no database access, no session handling, and a request shape
that is whatever the provider's already is.

Do not pre-pay for this. Building the whole server in Rust to avoid a problem
that may never arrive costs the accounts work, which is certain.

The other cost is type drift. Nudge's `Report`, `Offer` and agent shapes are Rust
structs, and the server will have Pydantic models of the same things. Keep the
contract small and versioned, and generate the Rust types from the OpenAPI schema
rather than copying them by hand — two hand-maintained copies of one shape
diverge, and the divergence is found by a user.

## The product decision hiding inside the proxy

Nudge today is bring-your-own-key: `api_key` in `config.toml`, or the Keychain.
That is not incidental — `config.rs` opens with *"this file **is** the bring your
own model feature"*.

A proxy inverts it. It means owning the API bill for every user, being the reason
the product is down when the server is, sitting in the content path of everything
anybody asks, and adding a hop to a loop already at 2.4s.

It is also the only way to charge a subscription, which is why most products in
this shape do it. **Both can be true at once:** bring-your-own-key stays, and is
free; the proxy is what people pay for. That keeps the proxy *optional*, which
means it failing degrades Nudge rather than stopping it — and an optional
dependency is a much cheaper thing to operate than a required one.

## What has to change when the proxy ships

The marketing page currently answers "does my screen go anywhere?" with
*"Nothing routes through a server of ours — there is not one in the path."*

That is true now and false the day a proxy exists. It is the kind of sentence
that survives a launch because nobody remembers writing it, and being wrong about
where somebody's screen goes is not a sentence to be wrong about. Change it in
the same commit as the proxy, and say plainly which route is which.
