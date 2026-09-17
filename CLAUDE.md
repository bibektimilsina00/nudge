# Nudge

A macOS menu-bar companion. Hold a key, say what you want, and it works on your
screen: reads it, clicks it, types into it, and talks to the services you have
connected. Rust and Tauri behind a React panel that lives in the notch.

## Where things are

| Path | What it is |
|---|---|
| `src-tauri/src/core/` | Everything that has no Tauri in it. The thinking. |
| `src-tauri/src/app/` | The Tauri layer: windows, commands, input, tray. |
| `src-tauri/src/bin/` | Tools: `errand` runs a real task end to end, `bench`/`picks`/`truth` score accuracy. |
| `ui/` | The panel and overlay. React, Tailwind v4, Vite. |
| `server/` | FastAPI: releases, downloads, accounts. SQLModel over SQLite. |
| `web/` | The marketing site. Next.js. |
| `deploy/` | compose.yml for the box behind `nudge.runmycrew.com`. |

`core` must not import `app`. `tests/layering.rs` enforces it, so a stray
`use tauri::` in `core` fails the build rather than being noticed later.

## Commands

```
make run            # build, sign, restart the app  (the one you usually want)
make dev            # hot reload, borrows the terminal's permissions
make lint           # clippy -D warnings, then cargo fmt --check
make test           # cargo test
cd server && uv run pytest -q
cd ui && pnpm exec tsc --noEmit
```

`make lint` before pushing. CI runs `cargo fmt --check` and a whole-file
reformat is a miserable thing to land in a review.

## Conventions

**Comments say why, not what.** The code already says what. A comment earns its
place by recording a decision, a constraint, or a bug that is invisible from
here. Several in this repo name the exact failure that produced the line.

**Commit messages are prose.** Subject in the imperative, then paragraphs
explaining what was wrong and why this is the fix. Not bullet lists of files.

**Test the logic, not the glue.** Pure functions in `core` get tests. Tauri
command wrappers do not: they are a translation layer and a test of one tests
the framework. If something is hard to test, that usually means logic has
leaked into `app` and belongs in `core`.

**Secrets live in the Keychain.** `connections.toml` only ever *names* one, as
`keychain:<item>`. Nothing is pasted into a file, a commit, or a chat.

## Things that have bitten

**The shell has `noclobber` set** and `cp` is aliased to `cp -i`. `>` on an
existing file fails, and `cp` over one hangs waiting for a prompt nobody can
answer. Use `>|` and `cat >| dest`.

**Signing is not optional, even in development.** macOS ties both TCC grants
and Keychain ACLs to the code signature, and an ad-hoc signature is a fresh
identity on every build. `.signing-identity` pins the certificate; `make build`
signs the bundle and `scripts/signed-run.sh` signs whatever `cargo run` and
`cargo test` produce. Without it, Screen Recording quietly stops applying and
the Keychain forgets "Always Allow" after every rebuild.

**Pushing to `alpha` deploys.** `alpha` and `main` both build and restart the
server and site containers, filtered by which directories changed. A commit
touching `server/` is a production deploy.

**`/tmp/nudge.log`** has the running app's output. It is usually faster than
reasoning about what the agent did.

**`cargo run --bin errand -- "some sentence"`** puts a real request through a
real session and prints every step. Most bugs in this repo were found by
running it rather than by reading.
