# Speed

The complaint that started this: a comparable product feels instant, and Nudge
does not. This is what is actually slow, what we measured rather than assumed,
and the order in which to fix it.

---

## The finding that frames everything

**They run a bigger, slower, more expensive model than we do and still feel
faster.** Their brain is `claude-sonnet-4-6`. Ours is a Gemini flash.

So the gap cannot be closed by picking a quicker model. We already tried that --
`flash-lite` is nearly three times faster than `3.6-flash` and scores 29% against
50%, which is not a trade worth making. The speed is not in the model.

| Stage | Them | Us |
|---|---|---|
| Speech in | streamed over a socket while you talk | record, upload after you release |
| Transcribe | starts before you stop speaking | one round trip, after |
| Brain | streamed, acts on the first tokens | wait for the whole JSON object |
| Screen | `SCStream`, always running | a fresh `CGWindowListCreateImage` per shot |
| Speech out | streamed | `say`, after the full sentence |

**Nothing in their pipeline waits for the previous stage. Ours is strictly
serial.** That is the whole difference. They overlap; we queue.

---

## The critical path, as it stands

Release the key, and this happens in order, each stage paying in full before the
next begins:

1. `rec.finish()` -- drain the audio callbacks, encode a WAV
2. base64 the WAV
3. **open a new TLS connection** (`core/voice/transcribe.rs:35`)
4. upload and wait for the transcript
5. speak "Sure, one sec." (`app/input/hotkey.rs:202`)
6. `capture::wait_until_still` (`core/run/session.rs:243`)
7. **wait for our own voice to stop** (`core/run/session.rs:259`)
8. gather facts, capture, resize, JPEG, base64
9. the model round trip, waiting for the entire object
10. act

---

## The whole picture, stage by stage

Read this as the working document. Everything else here is an argument for one of
these rows.

**M** = measured. **E** = estimated, and the reason Phase 0 exists. Anything
marked E is a guess until the instrumentation lands, including the ones that look
obviously right.

| Stage | Us, now | Them | Us, planned | What is different afterwards | Then, against them |
|---|---|---|---|---|---|
| **Speech in** | record a WAV, upload after you release | streamed over a socket **while you talk** | on-device, streaming while you talk | ~0.1s (E) → 0. Nothing is uploaded at all, so nothing can be slow about uploading it | **Ahead.** They still cross the network to hear you. We stop crossing it |
| **Transcribe** | Gemini, one round trip, begins only after you release | `gpt-4o-transcribe`, essentially finished as you release | `SFSpeechRecognizer` | 1.0-1.5s (E) → ~0. The stage leaves the timeline rather than shrinking: the words exist at the moment the key comes up | **Ahead.** Theirs is a fast round trip; ours is no round trip, and it works with the wifi off |
| **Connection** | a fresh TLS handshake per utterance | pooled | one shared client | 0.15-0.4s (E) → 0. The first byte leaves immediately instead of after a handshake to a host we were already talking to | **Level.** Table stakes. We are behind on a thing nobody should be behind on |
| **Settle** | 2 screen composites + 120ms, every turn, unconditionally | no equivalent | skip it when nothing was performed | 0.2-0.4s (E) → 0 on turn 1, unchanged later. The first turn stops paying for a settle when nothing has been done to settle | **Not comparable.** A cost we invented, then removed |
| **Hush** | wait for our own acknowledgement to finish before the screenshot | no equivalent | capture first, then speak | ~1.0s (E) → 0. The same sentence plays over the model call instead of in front of the screenshot. Nothing about it changes except that it stops being on the clock | **Not comparable.** Also invented, also removed |
| **Capture** | `CGWindowListCreateImage`, resize, JPEG, base64, per shot | `SCStream` always running -- a frame is already in hand | overlap it with transcription on turn 1 | 0.3-0.5s (E) → hidden on turn 1, unchanged after. The picture is ready before the words are | **Still behind, deliberately.** We hide the cost once; they never pay it. Parity needs `SCStream`, deferred until Phase 0 says it matters |
| **Brain** | `gemini-3.6-flash`, wait for the entire JSON object | `claude-sonnet-4-6`, streamed, acting on early tokens | stream, act on the first complete field | **4.5s median, 9.0s p95 (M)** → time-to-first-motion falls to the first field, while the sentence is still arriving. The total barely moves; the wait stops being empty | **Level, possibly ahead.** Same trick, smaller model. The p95 of 9s is the real problem and streaming hides it better than it fixes it |
| **Grounding** | a vision model finds the pixel | a vision model finds the pixel | AX tree where exposed, vision as fallback | seconds → microseconds, for every app that exposes AX. No screenshot, no model call, no token cost, no chance of a wrong pixel | **Ahead, structurally.** The only row where we would be doing something different in kind rather than degree. Also the only one that can reach zero |
| **Prompt size** | newest step whole, older ones trimmed | not observable from outside | **done** | 70k chars by turn 3 → 21k. Turn 10 now costs what turn 1 costs | **Unknown.** They have the same problem or they solved it; either way it is invisible to us |
| **Speech out** | `say`, only once the full sentence is known | ElevenLabs, streamed | streamed | ~0 on the clock. The reply begins as soon as its first words exist | **Level** |
| **Feedback gap** | "Sure, one sec.", then silence | tokens arrive immediately, so there is no gap to fill | keep the line, but off the critical path | the holding phrase stops covering silence and starts overlapping real work | **Level.** The gap is filled either way; theirs is filled with the answer |

**The net, if all five phases land:** ahead on input, because on-device beats a
fast round trip. Level on the brain, where we copy their trick with a cheaper
model. Ahead on grounding, which is the only row that changes what the product
*is* rather than how quickly it does the same thing. Behind on capture, on
purpose, until a measurement says otherwise.

Four of the eleven rows are costs with no counterpart on their side at all. Those
are not places where they beat us with better engineering -- they are places we
invented work and then paid for it. Phase 1 is the whole of that, and it is
deletion.

### What each phase buys

| Phase | What it buys | Effort | Risk | How we know |
|---|---|---|---|---|
| **0** Measure + shared client | the real split; 0.15-0.4s | hours | none | the log line exists |
| **1** Delete self-inflicted waits | ~1.2s off every turn | hours | none -- it is removal | settle and hush drop out of the line |
| **2** Overlap capture with transcribe | 0.3-0.5s, turn 1 only | hours | **`Settle`'s assumption** -- see the warning in Phase 2 | capture no longer appears in the serial sum |
| **3** On-device STT | 1.0-1.5s, and no network | days | a new `objc2` binding; accuracy of the local recogniser is unmeasured | transcribe drops to ~0 and the accuracy number holds |
| **4** Stream the brain | the largest remaining win against a 4.5s median | days | partial-object parsing has to be exactly right or we act on half a decision | time-to-first-motion, not total |
| **5** AX grounding | turns the dominant row into ~0 for apps that expose it | weeks | a different project; AX coverage varies per app | a click lands with no model call at all |

## Two waits we inflict on ourselves

Both are free to fix. Neither needs streaming, a new dependency, or a rewrite.

### The hush loop waits for our own acknowledgement

`hotkey.rs:202` speaks a short line to mask the model's latency. The comment
above it says this costs nothing, and when it was written that was true.

The hush loop at `session.rs:259` was added later, for a different reason: our
own voice comes out of the same output device the `audio` fact reads, so the fact
reports unknown while we are speaking. Waiting for silence made the fact useful
again on the turn that most needs it -- the one straight after clicking play.

Correct in isolation, and together they fight. The acknowledgement is still
playing when the hush loop starts polling, so we block the screenshot for about a
second waiting for the sentence we added to make the wait feel shorter.

*The fix:* capture before speaking. On the first turn the screen is still by
definition -- nothing has been done yet -- so there is nothing for the hush loop
to protect. It keeps its value on later turns, where the sentence being waited
out belongs to an action that actually changed the screen.

### `wait_until_still` can never return on its first look

```rust
let mut last: Option<Vec<u8>> = None;
while began.elapsed() < max {
    let Ok(shot) = grab(PEEK) else { return };
    let now = shot.fingerprint();
    if last.as_ref().is_some_and(|before| unchanged(before, &now)) {
        return;               // unreachable on iteration one: last is None
    }
    last = Some(now);
    sleep(GAP).await;         // 120ms
}
```

There is nothing to compare the first sample against, so the loop always takes a
second one. Minimum cost is two screen composites plus 120ms -- paid on every
turn including the first, when nothing has happened and the screen is provably
still. The function's own comment notes that compositing, not resizing, is the
expensive part. We pay it twice to learn nothing.

*The fix:* do not settle when nothing was performed.

Together these are roughly **1.2 seconds per turn, for free**, before the
screenshot is taken.

---

## A correction to our own priority list

FINDINGS ranks streaming speech-to-text as "the single biggest win (~1-1.5s)".
That was written when the brain was `flash-lite` at **1.75s median**. We have
since moved to `3.6-flash` for accuracy: **4.5s median, 9.0s p95**.

The brain is now the dominant stage by a wide margin and the old ordering is
stale. This is the argument for measuring before building: the ranking was right
when it was written, nobody revisited it when the model changed, and it would
have sent the next three days at the wrong stage.

---

## The plan

### Phase 0 -- Measure, and take the free wins

One line per turn, every stage named:

```
heard 1.3s · settle 0.4s · hush 1.1s · capture 0.5s · brain 4.4s · total 7.7s
```

There is not a single `Instant::now()` on the request path today. We know the
total and we know the shape; we do not know our own split.

Also here, because it is three lines: `transcribe.rs:35` and `speech.rs:113`
build a fresh `reqwest::Client` per call, so every utterance pays a new TCP and
TLS handshake to a host the provider already holds a warm connection to. The
providers do this correctly -- build once, keep it on the struct.

**Everything below is confirmed or killed by these numbers.**

### Phase 1 -- Delete the waiting we do to ourselves

The two above. Removal, not construction.

### Phase 2 -- Overlap what has no dependency

Start the capture the instant the key is released, concurrently with upload and
transcription. The picture is ready before the words are.

*Careful:* only safe on the first turn. `Settle` assumes a capture taken after
the previous action has landed, and breaking that assumption is how every agent
screenshot came to be taken before its action -- the largest single bug this
project has had.

### Phase 3 -- Delete a round trip: on-device speech-to-text

`transcribe.rs:5` already names `SFSpeechRecognizer` as the intended upgrade. It
is free, offline, and streams while you talk. It does not shrink transcription;
it removes it from the critical path.

They pay a websocket round trip to AssemblyAI. We would pay nothing. Costs an
`objc2` binding instead of an HTTP call.

### Phase 4 -- Stream the brain, act on the first complete field

We wait for the entire JSON object today. Order the shape so the action comes
first and we can move while the spoken sentence is still arriving.

Against a 4.5s median this is the largest single remaining win.

### Phase 5 -- Skip the brain where macOS already knows the answer

This is the phase that makes us faster than them rather than equal to them.

FINDINGS already commits to the principle, in the section on the audio fact:

> Before teaching a model to recognise a state, check whether macOS will simply
> tell us.

Grounding is the same move, not yet taken. The accessibility tree gives a
button's exact frame locally, in microseconds, with no screenshot and no model
call. "Click Send" in an app that exposes AX should never reach a vision model at
all.

Vision stays as the fallback for apps that expose nothing -- which is the case it
is genuinely needed for, rather than the case it currently handles alone.

---

## Deliberately not doing

- **A realtime omni model.** We checked: they are not using one either. It is an
  expensive answer to a problem that is structural.
- **Trading accuracy back for speed.** Measured, once, properly: 29% against 50%.
  A fast wrong click costs more than a slow right one.
- **Rewriting capture onto ScreenCaptureKit before Phase 0 reports.** It is the
  right end state and it is a real piece of work. If capture turns out to be
  400ms of a seven-second turn, it is not where the next week goes.

---

## How we will know it worked

The same instrumentation that opens the plan closes it. Phase 0 prints a stage
breakdown per turn; every phase after it is judged by moving one of those numbers
and leaving the accuracy number alone.

The target is the felt one: **something moves within a second of the key coming
up.** Not the total -- the total can stay where it is if the wait stops being
silent and empty. That is most of what their speed actually is.
