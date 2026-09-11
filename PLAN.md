# Nudge — plan to a real product

Written after comparing against **Clicky**, a voice-first desktop AI tutor with a
similar shape. The comparison is useful, but copying its feature list would be a
mistake, and most of this document is about why.

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

### 2.1 Nudge photographs its own overlay — **highest priority**

`screencapture` grabs the whole screen, including our ring, our cat and our bubble.
The model is being shown Nudge's own output as if it were part of the app, and is
asked not to be confused by it. It has coped so far. It will not always.

*Fix:* ScreenCaptureKit with a content filter excluding our window. Replaces the
`screencapture` shell-out in `core/capture.rs` and removes the ~150ms process spawn
at the same time. **This is the single highest-value change in the document.**

### 2.2 Multi-monitor is unimplemented

`app/overlay.rs` reads `primary_monitor()` once at startup and stores one `Screen`.
On a second display the overlay covers the wrong area and every coordinate is
wrong. Clicky captures all monitors; we capture one and assume.

*Fix:* capture the display under the cursor, store its geometry per-capture rather
than once at startup, and map coordinates through that display's origin.

### 2.3 No privacy guard

Nudge screenshots the screen and uploads it to Google on every step. There is
currently **nothing** stopping that when a password manager, a `.env`, or a banking
page is on screen. This project has already sent a screenshot containing an open
`.env` tab to a third party during development.

*Fix:* check the frontmost window's application and title before capture; refuse
and say so for a blocklist (1Password, Bitwarden, Keychain Access, anything whose
title matches `.env`/`secret`/`credential`, bank domains). Cheap, and it is the
difference between a tool people install and one they uninstall.

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

## 4. Ordered plan

Each phase ends with something testable. Do not start a phase before the one above
it is done.

### Phase 0 — stop the bleeding (a day)
1. `git init`, first commit, push somewhere private.
2. Privacy guard (§2.3). Blocklist + frontmost-window check.
3. Fix the double-voice bug and confirm the audio-truncation fix holds.

### Phase 1 — correctness (a week)
4. ScreenCaptureKit capture excluding our own window (§2.1).
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

### Phase 4 — only if asked
OCR fallback · code-editor mode · multilingual · document context.

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
| `core/capture.rs` | shells out to `screencapture` | ScreenCaptureKit — also fixes §2.1 |
| `core/transcribe.rs` | speech-to-text over the network | `SFSpeechRecognizer`, on-device |
| `app/state.rs` | fixed settle waits, tuned by eye | poll until two frames match |
| `app/cursor.rs` | 60Hz cursor + button poll | drop to 30Hz if battery complains |
| `core/launch.rs` | flat scan of app directories | `mdfind` if apps are missed |
