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

## Linux

The app builds and runs on Linux. `core` is portable already; what differs is
gathered in a handful of places and compiled on both platforms rather than left
to rot:

| macOS | Everywhere else |
|---|---|
| ScreenCaptureKit | `xcap`, in `capture::portable` |
| CGEvent clicks and typing | `enigo` + `device_query`, in `screen/elsewhere/` |
| The accessibility tree | Nothing -- falls back to the vision model |
| Keychain | Secret Service, over D-Bus (`keyring`) |
| `say` | `spd-say`, then `espeak-ng` |
| NSWindow levels and Spaces | `set_visible_on_all_workspaces`, in `ui/elsewhere/` |

**X11, not Wayland.** Injecting input and reading a held modifier are things
Wayland deliberately forbids; doing it there means the RemoteDesktop portal and
libei, which is its own piece of work. On Wayland the window appears and the
hotkey never fires.

The macOS diagnostics in `examples/` are behind `--features appkit`, which is
why `make lint` passes it. Without that they are not built at all, on either
platform.

To check a change compiles there without waiting for CI:

```
docker run --rm -v "$PWD":/w -v nudge-linux-target:/target \
  -e CARGO_TARGET_DIR=/target -w /w/src-tauri nudge-linux \
  cargo clippy --all-targets -- -D warnings
```

Never run `pnpm` in that container against the mounted tree: it swaps the
platform-specific binaries in `ui/node_modules` for Linux ones and `tsc` on the
Mac then refuses to start.

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
