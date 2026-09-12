# Nudge — plan to a real product

Written after comparing against **Clicky**, a voice-first desktop AI tutor with a
similar shape. The comparison is useful, but copying its feature list would be a
mistake, and most of this document is about why.

> **This document is the long plan. The active one is [VALIDATE.md](VALIDATE.md)**,
> which tests the two claims everything here rests on. Nothing below Phase 1
> matters until that is answered. Findings from each run are logged there.

Current state: ~3,600 lines, 33 tests, working end to end — voice in, screenshot,
grounding, ring, click/double-click/hover, typing, app launch, URL open, TTS.
Round trip ~2.5s.

---

## 1. The thing that actually matters

**Correction from the first draft of this document.** It claimed Clicky refuses to
touch your mouse, leaving the operator half open for Nudge. That was true of the
original open-source `farzaa/clicky`. It is *not* true of the shipping product:
`clicky.so` now redirects to **heyclicky.com**, a commercial macOS app by
Humansongs, Inc. with an **agent mode that executes tasks autonomously**, at
$20/month Pro and $100/month Max.

So autopilot is not the differentiator. Building the plan on that was building on a
competitor's old README.

### What is actually left

Three things, and they are structural rather than features -- which makes them
harder to copy than any capability:

| | HeyClicky | Nudge |
|---|---|---|
| Cost | $20-100 / month | free |
| Keys | theirs, proxied | yours |
| Data | screenshots to their service; text summaries retained | straight to your provider, or nowhere |
| Source | releases only | open |
| Offline | no | possible |

**The pitch is not "it clicks for you". It is:**

> *Everything that product does, for the price of your own API key -- and with a
> local model, for nothing at all, with nothing leaving your machine.*

That reframes the roadmap. The single most valuable unanswered question in this
document is no longer an animation or an annotation primitive, it is **§5's first
bullet: does the local model actually work?** If `qwen3-vl` grounds well enough,
Nudge is the only one of the two that can be free, offline and private at once, and
that is a category difference rather than a feature difference.

If it does not work, Nudge is a cheaper BYO-key version of a funded product, which
is a real but much smaller claim -- and worth knowing before another week goes into
it.

### The corollary: stop scoring against their feature list

Lesson recording, spaced repetition, a skills plugin system, workflow capture — all
of it exists because Clicky is a *tutor*, and a tutor needs a curriculum. Nudge is
an operator. Building an SM-2 flashcard scheduler into a tool whose job is to click
a button is how a focused product turns into a worse version of two things.

**Deliberately not building** (revisit only if users ask, in this order):
lesson recording · spaced repetition · skills plugin system · workflow capture ·
document ingestion · wake word.

---

## 2. What is genuinely broken

These are correctness problems in the code today, not missing features. Nothing
below is optional for calling this a product.

### 2.1 Nudge photographs its own overlay — **done**

Was: every screenshot contained Nudge. The companion follows the cursor and the
notch pill sits at the top, so a model asked what to click was being shown Nudge
and invited to click it -- one run stopped mid-task to close "that mysterious
black void of a window".

`capture::grab` now composites only windows that are not ours, by owner pid.

*Smaller than this section claimed.* Looking at a real capture rather than
trusting the note: Nudge contributed the notch pill and the companion, not a
black void. That void was most likely a dark browser window mid-load. The fix is
still right -- a cat in every frame is noise a model has to explain away -- but it
was not the emergency it was written up as.

**ponytail: `CGWindowListCreateImageFromArray`**, which Apple deprecated in macOS
14 in favour of ScreenCaptureKit. Chosen because it is one call against a crate
already in the tree, where SCK is a new dependency and an async completion
handler for a synchronous need. Verified working on macOS 26 by taking a real
capture and checking the image is neither null, black, nor sheared -- there is an
`#[ignore]`d test that does exactly that, and it takes `NUDGE_EXCLUDE_PID` so it
can prove the exclusion against the live app rather than only the mechanism.
When it stops working, `objc2-screen-capture-kit` and `SCContentFilter`'s
`excludingWindows` do the same job.

Falls back to `screencapture` if the window list is unreadable: a shot with
Nudge in it beats no shot at all.

### 2.2 Multi-monitor — **built, not yet verified on two screens**

Was: `app/overlay.rs` read `primary_monitor()` once at startup and stored one
`Screen`. On a second display the overlay covered the wrong area and every
coordinate was wrong -- and because the geometry was read at launch, changing
resolution or unplugging a monitor broke it silently until Nudge was restarted.

Now:

- `capture::grab` captures **the display the pointer is on**, chosen per call.
  Whichever screen someone is looking at is the one they mean.
- `Shot` records that display's `origin`, and `to_global` adds it. A point in an
  image means nothing until you know which display it came from.
- The overlay is **one window spanning the union of every display**, rather than
  one per screen: the companion crosses a monitor boundary without teleporting
  and a ring can land anywhere, both free with one window.
- Global points are converted to the overlay's own space at exactly one place --
  where a step leaves Rust for the UI -- so everything behind that line lives in
  a single coordinate space.

**Honest status:** this machine has one display, so only the single-screen path
is verified (unchanged, 60 tests green). The arithmetic is covered by a test with
a second display to the right and one above-left, including negative origins. The
behaviour that cannot be tested without hardware: whether the union window
actually spans both screens, and whether macOS lets one window sit across them at
that level. **Do not mark this done until it has run on two monitors.**

### 2.3 No privacy guard — **done**

Was: Nudge screenshotted the screen and uploaded it on every step with nothing
stopping it when a password manager or a `.env` was open. This project had already
sent a screenshot of an open `.env` tab to a third party during development.

Now `core/privacy.rs` checks the frontmost application and window title *before*
the capture — there is no un-sending a screenshot — and refuses out loud. Still
worth extending as new cases appear; the list is the kind of thing that only ever
grows from real use.

### 2.4 Not distributable

No git repository. Not notarized (`spctl` says `rejected`). The signing identity is
pinned to one machine's certificate. Nobody but you can run this.

*Fix:* `git init` today. Then Developer ID signing + notarization in CI before any
public link exists.

---

## 3. What is worth taking from Clicky

Three things, ranked. Each one strengthens the operator pitch rather than diluting
it.

### 3.1 "Next" and "Repeat" without a round trip — **biggest UX win**

Clicky plans a multi-step lesson once, then advances locally. Nudge makes a full
screenshot + model call for **every single step**, so a five-step task costs five
round trips and ~12s of waiting.

We cannot plan blindly — step 2's target lives in a menu step 1 opens, and that is
why the loop exists. But we can:

- **Cache the last step** so "repeat" replays instantly with no model call.
- **Speculatively fetch the next step** while the user is still acting on the
  current one, and discard it if the screen changed unexpectedly.

The second one is where the felt latency goes to near zero in guide mode, because
the user spends seconds reading and aiming while we sit idle.

### 3.2 Annotation primitives beyond the ring

`[ARROW:x1,y1->x2,y2]`, `[CIRCLE:x,y,r:label]`, `[UNDERLINE:x,y,w]`. A ring can only
say *here*; an arrow can say *from here to there*, which is most of what menu
navigation is. This is a small, contained addition to `Step` and the overlay, and
it visibly raises the quality of the teaching half.

### 3.3 OCR fallback for dense UI

Blender, DAWs and code editors are exactly the apps in the pitch, and exactly where
small labels defeat vision models. A local Tesseract pass when the model returns
`Unsure` gives a second chance at no API cost.

Note this is the *only* place a 1920px screenshot might beat 1280px — worth
re-measuring rather than assuming.

---

## 3.5 The second surface: a terminal

Everything so far drives the screen. That is the hard, unreliable path, and it is
the only one Nudge has: to rename fifty files it would have to click fifty times,
and it cannot write a line of code at all.

The same principle that fixed the Safari menu applies one level up. **Clicking is
the fallback, not the default.** A menu command has a shortcut; a file operation
has a command; a project has a build. Where a command exists, it is faster, more
reliable, inspectable, and undoable in ways a click never is. Screen-driving
should be what Nudge reaches for when there is no better handle -- not its only
tool.

This is a second product surface, not a feature. It is worth being clear-eyed
about that before starting.

### 3.5.1 What it buys

- **Work that is not on screen.** Rename a batch of files, convert some video,
  check what is listening on a port, find where a string appears in a repo.
- **Writing code.** "Make me a landing page for this" produces files, a preview,
  and something to iterate on. This is what Claude Code and the Gemini CLI do,
  and there is no reason a companion that already understands the request cannot
  do it.
- **Reliability.** A command either ran or it did not, and it says which. No
  grounding, no settle wait, no stale screenshot, no toggle clicked twice.

### 3.5.2 The shape

A new outcome beside `point`, `press` and `type`:

```rust
/// Run a shell command and read what it prints.
///
/// Bounded by a working directory the user named, and by the rules below --
/// this is the sharpest thing in the product by a wide margin.
Run { command: String, say: String },
```

The model sees the command's output on the next turn, the same way it sees a
screenshot today. `core/shell.rs` owns running it; `core/` stays Tauri-free, so
it stays testable.

### 3.5.3 Sub-agents, and why they are allowed here

One agent at a time is a rule we learned by breaking it: two agents drove one
WhatsApp chat and sent a voice note to a real person. That rule is about **the
cursor**, which there is only one of.

A terminal agent touches no cursor. Several can run at once without interfering,
which makes "write the HTML while another checks the build" coherent in a way
that two clicking agents never were.

So the distinction belongs in the type rather than in a convention:

```rust
enum Surface {
    /// Drives the real mouse and keyboard. Exclusive -- there is one cursor.
    Screen,
    /// Runs commands. Several may run at once; none of them can fight.
    Headless,
}
```

`Agents::start` already refuses a second agent. It should refuse a second
**Screen** agent, and allow headless ones up to a small cap.

A sub-agent reports back the way the card already reports: title, status,
progress, and a result its parent can read. The registry holds a parent id, and
the Agents tab nests them.

### 3.5.4 Artifacts

An agent that writes files should show them. The Agents tab becomes the place
where work lives rather than a list of what is running:

- Files an agent created or changed, grouped under it, surviving the run.
- A preview where one is obvious -- a rendered page, an image, a diff.
- "Open in Finder" and "Open in the editor", which is what people will actually
  want ninety percent of the time.
- Kept per agent, so "the site you made me on Tuesday" is findable.

This is also the first thing in the product that is worth **persisting**, which
the current registry deliberately does not do.

### 3.5.5 The part that needs care

Shell access driven by a model is a different risk class from clicking, and
pretending otherwise would be dishonest. A wrong click is usually recoverable.
`rm -rf` is not, and neither is a command that posts a file somewhere.

Non-negotiables, in the order they should be built:

1. **A working directory, chosen by the user, and nothing above it.** Commands run
   there. No writing outside it without a fresh, explicit grant. The default is
   not the home folder.
2. **A refusal list that is not a prompt rule.** Enforced in `core/shell.rs`, where
   a model cannot talk its way past it: no `sudo`, no writes to `~/.ssh`,
   `~/.aws`, `.env` or a keychain, no `curl | sh`, no package installs, no
   `git push`, no commands whose output is piped to a network tool. This is the
   same shape as the privacy guard, which already refuses to photograph a
   password manager and is checked in code rather than asked for politely.
3. **Confirmation before anything irreversible or outward-facing** -- deleting,
   overwriting outside what it created, pushing, sending, paying. The machinery
   exists: it is `Step::Question`, which already speaks and waits for a voice
   answer.
4. **Everything it ran, visible.** The Agents tab shows the commands and their
   output. An agent that runs things you cannot inspect afterwards is not one
   people should install.
5. **A hard stop that kills the child process**, not just the loop. Escape already
   stops an agent; it must also send a signal.

The privacy guard is the precedent worth following: the rule that mattered was
the one written in Rust, not the one written in the prompt.

### 3.5.6 Honest sequencing

This is bigger than everything above it combined, and it is tempting because it
is more fun than counting hit rates. Two things to settle first, both cheap:

- **The accuracy number.** The harness exists now (`make bench`). Twenty cases is
  an afternoon, and it tells you whether the screen surface is worth keeping at
  all -- which changes how much of this plan matters.
- **ScreenCaptureKit (§2.1).** Confirmed live: an agent stopped to close "that
  mysterious black void of a window", which was Nudge photographing itself.

Then build the terminal surface in this order, each step usable on its own:

1. ~~`Step::Run` with the working directory and the refusal list.~~ **Done** --
   two turns, correct answer, read-only enforced in `core/shell.rs`.
2. Confirmation for irreversible commands, and the command log in the Agents tab.
3. Writing files. Test: *"make me a landing page"* -- files on disk, opened in the
   browser.
4. Artifacts and persistence in the Agents tab.
5. Headless sub-agents, with the `Surface` split above.

See §3.6 for the full tool list and the order to add the rest in. The short
version: `fetch` before anything else, because it takes work off the screen
surface entirely.

Steps 1--3 are the product. 4 and 5 are what make it feel like a platform, and
neither is worth building before something is producing artifacts worth keeping.

---

## 3.6 The tool list

What Claude Code, Codex and the Gemini CLI converged on, what Nudge has, and what
it should add. Worth settling in one go rather than bolting on a tool each time
something is awkward -- the shape of the set is a product decision.

### What those three all have

Their lists differ in naming and barely at all in substance:

| Tool | What it is for |
|---|---|
| **read** | A file's contents, by line range. Not `cat`: ranges, line numbers, and images and PDFs too |
| **write** | Whole file, created or replaced |
| **edit** | Replace an exact string in a file. The everyday one -- most changes are a line, not a file |
| **list / glob** | What is in a directory; files matching a pattern |
| **grep** | Content search across a tree |
| **shell** | Run a command. The escape hatch that covers everything not named above |
| **background shell** | Start something long-running, read its output later, kill it |
| **web fetch** | Fetch a URL and read it as text |
| **web search** | Search, and get results back as text |
| **subagent** | Hand a scoped job to another agent and read its result |
| **plan / todo** | Write down the steps, tick them off. For work too long to hold in one turn |
| **memory** | Remember something across sessions |
| **MCP** | Everything the author never thought of |

### What Nudge has

Nudge's list is not a subset. It has a whole surface none of them do -- the
screen -- and it is missing most of the file and network surface they all share.

| Nudge has | | Nudge lacks |
|---|---|---|
| `point` click / double / hover | ← nobody else has these | `read` |
| `press` any keyboard shortcut | | `edit` |
| `type` text | | `list` / `glob` / `grep` (partly via `run`) |
| `launch` an application | | background shell |
| `open` a URL | | web fetch |
| `run` a read-only command | | web search |
| `write` a whole file | | subagent |
| `agent` -- hand over a whole task | | plan / todo |
| `question` -- ask, out loud, and wait | | memory |
| `done` / `unsure` / `reply` | | MCP |
| system facts: frontmost app, audio | ← nobody else has this either | |

### What to add, in order of what it buys

**1. ~~`fetch` -- read a URL as text.~~ Done.** The biggest single win, and not a small one.

Right now *"what's the weather in Kathmandu"* costs: open a browser, wait 2.2s for
it to settle, screenshot the whole display, JPEG it, upload ~130KB, and have a
vision model read a number off a picture. A fetch is one HTTP request and a few
KB of text, with no browser window appearing on someone's screen.

It is the same principle as the two that have already paid off here -- prefer the
keyboard shortcut to the menu, prefer the command to the click -- one level up
again: **prefer the data to the picture of the data.** Anything answerable from a
page's text should never touch the screen surface at all.

**2. ~~`edit` -- replace an exact string in a file.~~ Done.** Today the only way to change a
line is to rewrite the whole file, which means the model regenerates hundreds of
lines it cannot see, and every one of them is a chance to lose something. It is
also why the backup in `core/files.rs` exists. `edit` makes small changes small.

**3. ~~`read` -- a file by line range.~~ Done.** `run` can `cat`, but the output is capped
at 4000 characters and a large file simply truncates. Ranges make a big file
usable and make `edit` possible to target.

**4. ~~`search` -- the web, as text.~~ Done.** Pairs with `fetch`: find the page, then read
it. Without it, finding anything means driving a search engine through the screen.

**5. ~~`todo` -- a visible list of steps.~~ Done.** The agent already has a step budget and
a progress bar that counts turns rather than work. A written plan would make the
Agents tab show what it is doing and what is left, instead of a bar that creeps.

**6. Subagents.** Already designed in §3.5.3. The reason they are this far down:
they multiply what the earlier tools can do, and multiplying a small set is not
worth much.

**7. Memory.** The per-app notes described at the end of this document. Earned
once there is something worth remembering.

**8. MCP.** Extensibility is the right long-term answer and the wrong early one:
a plugin surface over an unsettled tool set locks in shapes that are still moving.

### What Nudge should not have

- **`glob` and `grep` as their own tools.** `run` already covers them with `find`
  and `grep`, both on the allow-list, and a separate tool for each would be three
  ways to do one thing. Revisit only if `run` proves awkward in practice.
- **`apply_patch` / diff-based editing.** That exists because coding agents work
  across a repo in a git workflow. Nudge is not that, and `edit` covers the need
  without a patch format to get wrong.
- **Deleting files.** Not now, possibly not ever. The value is small and it is the
  one operation with no undo; `.nudge-backups` cannot save a file that is gone.
  If it ever arrives it moves things to the Trash, which is reversible, rather
  than unlinking them.
- **Arbitrary writes outside the workspace.** The workspace boundary is the thing
  making any of this acceptable. A tool whose purpose is to cross it is a tool
  that removes the reason to trust the rest.

### Build it or borrow it

All of these exist as crates. That does not make using them the right call, and
the answer differs by tool.

| Tool | Build or borrow | Why |
|---|---|---|
| `read` `write` `edit` `list` | **Build** | 30-40 lines each on `std::fs`. The *rules* are the product -- workspace boundary, secrets refusal, ask-before-replacing, automatic backups. Those are the reason any of this is acceptable, and handing them to someone else's package moves the safety boundary into code we do not control |
| `fetch` | **Borrow the middle** | `reqwest` is already a dependency; the hard part is turning a modern page into readable text past the nav, banners and scripts. `dom_smoothie` or `readability` (a port of Firefox Reader Mode) do that. Twenty lines of ours around a crate that has solved the fiddly bit |
| `search` | **Borrowed the provider's** | Confirmed: Gemini grounds on Google Search with the key already configured. `search` is a method on the `Provider` trait, not a service Nudge runs -- no second account, no new dependency. Providers without it say so rather than pretending |
| `glob` `grep` | **Neither** | `run` covers both with `find` and `grep`. If it ever proves awkward, `globset` and `grep-searcher` are ripgrep's own libraries |
| MCP | **Borrow, later** | `rmcp` is the official Rust SDK, so it is viable when the time comes |

**On MCP specifically**, since it is the obvious "get everything for free" answer:

- Most servers are Node or Python processes. Spawning a language runtime from
  inside a signed, notarised `.app` is a distribution problem, not a detail.
- Using `server-filesystem` to get `read_file` -- when `std::fs::read_to_string`
  is one line -- is precisely the over-engineering worth avoiding.
- It moves safety into another process. Its allow-list knows nothing about this
  workspace, this secrets list, or these backups.

MCP's value is **other people's tools**, not ours. That is why it is last on the
list rather than first, and it is a real feature when it arrives -- an
open-source app whose users bring their own model should let them bring their own
tools too.

### The principle underneath the list

Every tool added here should make Nudge reach for the screen **less**. The screen
is the slowest, least reliable, most expensive surface it has -- a 2.4s round trip
and a model guessing at pixels -- and the only one that can fail by clicking the
wrong thing. It exists because some things have no other handle, and the measure
of a good tool here is how much it shrinks that set.

---

## 4. Ordered plan

Each phase ends with something testable. Do not start a phase before the one above
it is done.

### Phase 0 — stop the bleeding (a day)
1. `git init`, first commit, push somewhere private.
2. Privacy guard (§2.3). Blocklist + frontmost-window check.
3. Fix the double-voice bug and confirm the audio-truncation fix holds.

### Phase 1 — correctness (a week)
4. ScreenCaptureKit capture excluding our own windows (§2.1). **Confirmed live:**
   an agent run stopped to close "that mysterious black void of a window" — it was
   photographing Nudge.
5. Multi-monitor: capture and map through the display under the cursor (§2.2).
6. Run the accuracy harness properly: **20 real goals in Blender, count hits.**
   This has still never been done. Every architectural decision since has been
   made without that number.

### Phase 2 — the product claim (a week)
7. Speculative next-step fetch + instant "repeat" (§3.1).
8. Arrow / underline / labelled-circle annotations (§3.2).
9. Autopilot safety: a visible, cancellable countdown before each automated click,
   and a hard stop on any window whose title changes to something unexpected.
   *Operating someone's machine needs a brake that is faster than the action.*

### Phase 3 — shippable (a week)
10. On-device speech recognition (`SFSpeechRecognizer`) — removes the network from
    the input path, makes voice free and offline, and cuts a round trip.
11. Developer ID signing + notarization in CI.
12. First-run flow: permissions, provider choice, a 30-second demo.

### Phase 4 — the terminal surface (§3.5)
Run commands, then write code, then artifacts, then headless sub-agents. Bigger
than every phase above it combined, and the point at which Nudge stops being a
thing that clicks for you and becomes a thing that does work for you. Do not
start it before the accuracy number exists — that number decides how much of the
screen surface is worth carrying alongside it.

### Phase 5 — only if asked
OCR fallback · code-editor mode · multilingual · document context.

### Later, and earned rather than scheduled

- **Learned per-app memory.** The scalable answer to app-specific knowledge is not
  a longer prompt but a note per bundle ID or domain, written when a run fails and
  injected only when that app is in front. The prompt stays constant while the
  knowledge grows, and for an open-source project the notes are shareable — which
  is the one kind of moat a closed competitor cannot copy quickly. This is what
  HeyClicky's Skills tab is, structurally. **Not before the loop works:** memory
  that records a broken loop's habits is worse than none.
- **Latency architecture.** See the findings log in VALIDATE.md — the gap is not
  the model, it is that every stage waits for the previous one to finish.

---

## 5. Open questions worth settling before Phase 2

- **Does the free local model work?** -- *now the decisive question, see §1.*
  Everything is currently Gemini-shaped. If `qwen3-vl` grounds well enough, "free,
  offline, private" becomes the whole pitch rather than a nice extra. Untested, and
  a day's work to answer with the existing probe.
- **Is auto mode actually wanted, or is it a demo?** Watch yourself use it for a
  week. If you always leave it off, the pitch is wrong and the ring is the product.
- **What is the failure rate on a real task?** Not "does it hit the gear icon" —
  can it complete a five-step Blender task end to end, twice in a row?

---

## 6. Debt already marked in the code

Every deliberate shortcut carries a `ponytail:` comment naming its ceiling and the
upgrade path. `grep -rn "ponytail:" src-tauri/src` is the current ledger. The ones
that matter here:

| Where | Ceiling | Upgrade |
|---|---|---|
| `core/capture.rs` | deprecated `CGWindowListCreateImageFromArray` | ScreenCaptureKit, when Apple removes it |
| `core/transcribe.rs` | speech-to-text over the network | `SFSpeechRecognizer`, on-device |
| `app/state.rs` | fixed settle waits, tuned by eye | poll until two frames match — `AFTER_OPEN` at 2.2s is already too short for a search results page |
| `app/cursor.rs` | 60Hz cursor + button poll | drop to 30Hz if battery complains |
| `core/launch.rs` | flat scan of app directories | `mdfind` if apps are missed |
