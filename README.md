# Nudge

Points at the thing you need to click, in whatever desktop app you're already in.
Press a hotkey, say what you want, follow the ring.

Status: **skeleton**. It builds, the loop is wired, the coordinate math is tested.
Whether the pointing is *accurate enough to be a product* is still an open question
— see [Is this even possible?](#is-this-even-possible) before building on it.

## Run it

```sh
make tools                                           # one time
cp config.example.toml ~/.config/nudge/config.toml   # set provider + api_key
chmod 600 ~/.config/nudge/config.toml
make pin-identity ID="Apple Development: ..."        # see Code signing below
make run
```

`make` on its own lists every target.

Nudge lives in the **menu bar** — no Dock icon, no window. A small orb trails your
cursor so you can see it's awake.

**Hold** `cmd+shift+space`, say what you want, release. **Tap** the same key to type
instead, or to skip to the next step. A ring appears over the control to click, and
stays there until you actually click it -- then the next step loads by itself.

**Escape** cancels and hands the machine straight back.

### Menu bar

| Item | What |
|---|---|
| **Click for me** | Nudge clicks the control instead of pointing at it. Needs Accessibility. |
| **Natural voice (uses API)** | Gemini TTS instead of the free offline macOS voice. Off by default. |

Both override the config file at runtime, so trying either costs a click rather than
an edit and a restart.

Nudge asks for **Screen Recording** and **Microphone** on first use. Grant both, then
relaunch — the toggles don't reach a running process. Build the `.app` rather than
`cargo run`: a bundled app requests permission as itself and reads your config,
where a bare binary inherits your terminal's permissions and shell env instead.

For a free, offline, keyless setup, `brew install ollama && ollama pull qwen3-vl:4b`
and set `provider = "ollama"`.

## Is this even possible?

The entire product rests on one question: *can a model return the on-screen position
of a named control accurately enough to draw a ring around it?* Everything else here
is plumbing.

The `probe` bin answers it empirically, using the same providers and the same
coordinate math as the app -- it just writes a PNG instead of an overlay:

```sh
export GEMINI_API_KEY=...          # free tier, no card
cd src-tauri && cargo run --bin probe -- "open the UV editor"

# compare models without touching config
NUDGE_PROVIDER=gemini NUDGE_MODEL=gemini-robotics-er-2-preview \
    cargo run --bin probe -- "open the UV editor"
```

It screenshots, asks the model, and writes `hit-<provider>.png` with a ring drawn
on. Run ~20 real goals in the app you care about and count the hits.
That number decides the shape of the project:

| Result | What Nudge becomes |
|---|---|
| Local model hits | Genuinely free, offline, open source |
| Only hosted hits | Free tier ships it; power users bring a key |
| Only frontier hits | Real, but ~$0.003–0.017 per step, BYO key |

Don't pick an architecture before you know which row you're in.

## Develop

```sh
make dev      # hot-reloading
make test
make lint     # clippy -D warnings + fmt check
make probe GOAL="open the UV editor" MODEL=gemini-robotics-er-2-preview
```

Vite hot-reloads the overlay; editing Rust rebuilds and restarts the app.

Dev runs an **unsigned binary, not a bundle**, so it cannot use Nudge's own
permission grants -- it inherits them from whatever launched it. Run it from a
terminal that already has Screen Recording and you are fine. Signed bundle for real
use, inherited grant for dev.

### Code signing is not optional on macOS

TCC identifies an app by its code signature. An ad-hoc signed bundle gets a new hash
on every build, so macOS treats each rebuild as a brand-new app and your Screen
Recording grant silently stops applying -- the toggle still looks on. `tauri.conf.json`
sets `bundle.macOS.signingIdentity` to a stable identity to avoid this.

Which certificate signs the build is pinned in `.signing-identity` (gitignored) via
`make pin-identity`, not auto-detected. `security find-identity` does not order its
output stably, so on a machine with several certificates auto-detection picks a
different one between runs -- the signature changes and the grant is quietly voided
while the toggle still reads "on". Auto-detect only kicks in when there is exactly
one candidate and therefore no choice to get wrong.

Changing the pinned identity voids existing grants: run `make reset-perms` after.

## Layout

```
ui/                     React + Tailwind overlay (Vite)
  src/lib/              nudge.ts (typed commands) · useNudge.ts (events + phase)
  src/components/       Companion · Ring · Bubble · AskInput

src-tauri/src/
  lib.rs                module tree, nothing else
  config.rs             ~/.config/nudge/config.toml
  error.rs              one error enum

  core/                 what Nudge does. No Tauri, no windows, unit-testable.
    capture.rs          screenshot → the image the model sees + the mapping back
    session.rs          the step loop
    voice.rs            microphone → WAV
    transcribe.rs       speech → text
    speech.rs           reads each nudge aloud (Gemini TTS, `say` fallback)
    click.rs            posts clicks, and watches for real ones
    launch.rs           opens an app by name — the one guarded action
    provider/           mod.rs (trait + prompt + dispatch)
                        ollama.rs · gemini.rs · anthropic.rs

  app/                  wiring core to the OS. Thin by design.
    mod.rs              builder + setup
    commands.rs         the frontend's entire API surface
    overlay.rs          the transparent click-through window
    hotkey.rs           push-to-talk: tap advances, hold speaks
    tray.rs             menu bar item
    cursor.rs           60Hz poll driving the companion orb
    state.rs            Screen · Mic

  bin/probe.rs          accuracy harness — same code path, writes a PNG
tests/layering.rs       enforces that core never imports Tauri
```

### The four answers

A nudge is not always "click here". `Step` has four variants because there are
genuinely four things to say, and every one collapsed into the others causes a
visible bug:

| Outcome | Means | Ends the session? |
|---|---|---|
| `Point` | click this control | no |
| `Launch` | the app isn't open — open it | no |
| `Unsure` | not on this screen, and I won't guess | no |
| `Done` | already achieved | yes |

`Unsure` exists because a model that may only point or finish will **invent
coordinates** when shown a screen with no matching control. `Launch` exists because
"open Blender" cannot be satisfied by pointing at anything.

`Unsure` and `Launch` are not recorded as completed steps — logging "I can't see it"
as progress makes the next call believe the user already did it.

### The overlay cannot be both click-through and typable

A full-screen window that accepts the keyboard also swallows **every click on
screen**, and the app underneath goes dead. Pointing and asking therefore need
different window shapes, not a flag on one shape:

- **pointing** — full screen, click-through, input passes to the app below
- **asking** — shrunk to the prompt box, takes the keyboard, covers nothing else

`overlay::set_prompt_mode` switches between them, and losing focus while asking
hands input straight back. Get this wrong and the symptom is not a broken overlay,
it is a machine where clicking stops working.

### Guide mode and auto mode share one path

"Click for me" posts a click; the same watcher that notices *your* clicks notices
that one and advances the step. There is no separate automation loop, so the two
modes cannot drift apart. Auto mode therefore drives itself, which is why sessions
are capped at 12 steps -- guide mode is bounded by the user's patience.

### Launching is the only thing Nudge does *to* your machine

Everything else shows you where to click. `launch.rs` runs `open -a <name>` and
nothing else: it cannot open a file, follow a URL, run a binary, or pass an argument.
The name comes from a language model, so it is validated as untrusted input — paths,
flags, shell metacharacters and URLs are all refused, with tests for each.

### The one architectural rule

`core` must not know Tauri exists. That is what makes the logic testable without
spawning a window, and it is checked by `tests/layering.rs` rather than trusted --
a documented boundary decays the first time someone wants an `AppHandle` inside a
provider to emit a progress event.

Dependencies point one way: `app → core`. `config` and `error` are shared by both.

## Adding a model

One file in `src-tauri/src/provider/`, one arm in `provider::build()`. There is no
registry and no plugin loader — with three providers, a `match` is still smaller
than the machinery to avoid one.

Implement `next_step` and return a `Step`. `point: None` means "nothing left to
click", which is how a session reports that the goal is met.

The only thing that meaningfully differs between providers is the coordinate
convention, and it is a minefield:

| Provider | Returns |
|---|---|
| Anthropic | Absolute pixels, 1:1 with the image sent (under 2576px long edge) |
| Gemini | `[y, x]` normalised to 0–1000 — **y first, and not pixels** |
| Qwen/Ollama | Absolute pixels |

Each conversion is unit-tested. Add a test with your provider; an off-by-2× ring is
invisible in code review and obvious in a test.

## Design notes

**Why a loop, not a plan.** "Unwrap UVs" is five steps, but step 2's target lives
inside a menu step 1 opens — it isn't in any earlier screenshot. So each step needs
a fresh capture. Cost and latency scale with *steps*, not questions.

**Why the hotkey advances.** Detecting "did they click it?" wants click hooks,
accessibility observers, or screenshot diffing. Pressing the hotkey again costs none
of that and keeps the user in control, which is the safety story anyway.

**Why one window.** The overlay is click-through so clicks reach the app underneath,
and only grabs the cursor while the input box is open. Two windows would need to stay
in sync for no gain.

**Why coordinates convert exactly once.** `Shot::to_overlay` folds the Retina backing
factor and our own downscale into a single ratio. Two separate conversions is how you
ship a ring that is off by exactly 2×.

## Not built yet

- **Accessibility API fast path.** Would give exact bounds free in native apps — but
  is useless in Blender, DAWs and CAD, which are one GPU canvas. Vision is the
  universal path and the one that has to be proven first.
- **Windows/Linux.** Tauri makes it reachable; `capture.rs` is the only part that
  shells out to something mac-specific.
