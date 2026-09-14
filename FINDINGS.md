# What the runs taught

A record of every time Nudge was pointed at something real and failed, and what
that failure turned out to be. Newest sections at the bottom.

It is kept because the same lesson kept arriving in different costumes, and
because most of these bugs were invisible from the code. They were found by
running the thing, reading the log, and noticing that a sentence did not match
what was on the screen.

**For what to do next, see [PLAN.md](PLAN.md).** This file has no plan in it -- it
used to, and having two documents with two sets of phase numbers meant "what is
next" had two answers.

The test passes themselves are in [results.md](results.md).

---

## Findings log

Every run teaches something. Recorded here as it happens, newest last, so the
decision in Phase 4 is made against evidence rather than memory.

### Run 1 — instant "Done", nothing happened

The card appeared with the right title, a full progress bar, and DONE. No work.

- **A spawn/end race.** `perform()` spawned the agent, then `advance()` called
  `Nudge::end()`. The agent opens its own session on a worker thread, so the two
  raced; when the agent won, `end()` wiped the session it had just opened, the
  first look found nothing in flight, and it reported Done. *Fixed: read the goal,
  end the foreground session, then hand it over — and spawn from `advance`, not
  from the `perform` the agent loop also calls.*
- **The agent was delegating to itself.** It ran on guide mode's prompt, which
  offers "answer with agent for a whole job" — so on turn one it answered `agent`.
  *Fixed: the prompt splits on `Ask::agent`; guarded by a test.*
- **A 12-turn ceiling.** `session.rs` capped every session at `MAX_STEPS = 12`, so
  the agent's 40-turn budget was fiction. *Fixed: agent sessions use the runtime's
  budget.*

### Run 2 — opened the page, then claimed Done mid-task

Reached the YouTube results page in one step via a URL carrying the query.

- **The overlay killed the session.** Its read-timer fired `cancel(false)` after
  the sentence had been on screen long enough, ending the session the agent was
  driving. *Fixed: a timed dismissal ends only the foreground; Escape still ends
  everything.*
- **`AFTER_OPEN` is 2.2s** and a search results page takes longer, so the next turn
  photographed a still-loading page, saw no change, and re-issued the same Open.
  *Partly fixed by the repeat guard; the real fix is polling until two frames match
  rather than a fixed wait.*

### Run 3 — 23 turns, 19 of them clicking the same link

It played the song, could not tell it had, and kept trying to play it.

- **The repeat guard compared sentences.** The model rewords every turn —
  "Heading straight to YouTube" and "Let's teleport straight to YouTube" were the
  same Open, back to back, and both fired. *Fixed: compare the action — same URL,
  app, text, or a point within 24px, because a model re-aiming at one target
  wanders a few pixels a turn (1106 → 1111 → 1115 → 1108).*
- **History was prose only.** Told "Let's hit play on that first track" five times,
  it could not know all five aimed at the same pixel. *Fixed: history records what
  and where — `Click at (1106, 386) -- ...`.*
- **No loop breaker.** *Fixed: three identical actions fails the agent in seconds
  instead of burning forty turns.*
- **It photographed Nudge.** Turn 2 stopped to close "that mysterious black void of
  a window". This is PLAN.md §2.1, now confirmed live rather than suspected.
- **The stall detector is structurally blind here.** It compares frames, and a
  playing video is the one thing that changes every frame — so the moment the goal
  was met looked like maximum progress. Needs a different signal than pixels.

### Gap found while removing the typed input — questions had nowhere to go

`Step::Question` and `State::Waiting` existed and were wired to a text field on
the agent card. Then the typed input was deleted and the card became
background-only, so a foreground task could ask a question that nothing spoke and
nothing displayed. It would simply hold forever.

*Fixed by making the voice the whole interface, which is what it should have been
in a voice-first app:* the question is spoken aloud, the notch turns red and says
"Your turn", and the next thing said is routed to the waiting agent instead of
starting a new goal. The work so far is kept; the task resumes where it stopped.

The prompt also had it backwards. It said "ask only when genuinely blocked",
which a model reads as discouragement, so it guessed instead — opening WhatsApp
with no idea who to message. It now says plainly that a request naming an action
but not its content (who to send to, what to say, which of several matches) is
blocked, that guessing is worse than asking because a message sent to the wrong
person cannot be taken back, and that questions come one at a time, when needed,
rather than all at the start.

---

## The prompt consolidation, and what it cost

~2,600 words down to ~1,970, and the read-through found real contradictions --
including one the user had already objected to and which my "fix" had left in
place four lines above its own replacement.

It also **dropped a rule that was load-bearing**: *"when what they want has no
application here and lives on the web, open the URL instead."* It read as
redundant next to the tool descriptions. It was the only thing sending WhatsApp
to web.whatsapp.com, and without it the model substituted whatever messaging app
happened to be installed -- which sends a message to a different person entirely.

Three rounds of re-wording did not fix it. What did was **moving it**, not
rewriting it: the rule lived in `launch`'s fine print, and the model chooses a
tool before reading that. It now sits in "choosing where to act", before the tool
list. The installed-apps list -- last in the prompt, freshest in context -- was
also reframed, because it read as a menu of alternatives: *"not a list of ways to
do something, and not a set of alternatives to what was asked for."*

**When a rule is ignored three times, the words are not the problem.** Check
where it sits relative to the decision it is meant to influence.

### And a repair that broke more than it fixed

Seeing the foreground turn launch the wrong app, I stopped foreground steps being
performed at all -- the agent re-decides anyway, so why do it twice. The next run
showed the cost: the foreground chose `Open web.whatsapp.com`, **correctly**, and
that was discarded. The agent restarted from a blank desktop and spent six turns
oscillating between Messages, Safari and Arc before arriving at the same answer.

A first step already taken is the best thing an agent can inherit. Reverted.

**The pattern, three times in one session:** a change to stop X happening, where X
was also doing something useful. The settle wait, `Reply`, and this. Before
removing a behaviour, find out what depends on it -- the bug report only tells you
what it did wrong.

## A third: state the ordering, not the prohibition

The prompt said *"NEVER launch an application to look something up"* after a run
opened the Weather app to read one number and then failed trying to drive it.

That rule names no application, so it is not the YouTube-example mistake. It is a
different one: **absolute where reality has exceptions.** "How many unread emails
do I have", "what is my next meeting", "how much battery is left" are facts too,
and no fetch can reach any of them -- they live inside an app, and "never" would
have blocked every one.

Rewritten as an ordering plus the thing that actually decides it:

> Use the cheapest thing that can ACTUALLY answer: what the system already
> reports, then a command, then a fetch, then the screen.
>
> The question to ask is **whose information it is.** Anything public is on the
> web, so fetch it. Anything that is theirs -- their mail, their calendar, their
> files, their machine's state -- is not on the web at all, and an application or
> the screen is the only honest route.

The test for a rule, next to "could you write it without knowing which app":
**can you think of a case where following it would be wrong?** If yes, it is a
preference and should be written as one. A rule stated as never has to be true
never.

## Two principles these produced

**Structure over instruction.** The first fix for "it did not notice the goal was
already met" was a sentence in the prompt naming a video that is already playing.
That is one website's knowledge in a system prompt, and it does not scale: the
next app needs its own paragraph, forever. Replaced with a required field — every
reply must state what is on screen before it chooses an action. The model cannot
skip a required field, and the rule is the same on Blender, Figma, or a site
neither of us has seen.

The test for anything new: **could you write this rule without knowing which app it
is about?** If not, it does not belong in the prompt — it belongs in the learned
per-app memory described at the end of PLAN.md.

macOS semantics are not app knowledge. "A single click in Finder selects, a double
opens" and "hover opens a submenu" name applications but describe the platform, and
the platform is a fixed, finite surface. One website's play button is not.

**Prefer ground truth to pixels.** *(now implemented — `core/facts.rs`)* Every time we are tempted to teach the model to
recognise a state, check whether the OS already reports it — frontmost app, window
title, URL, now-playing info. Asking a vision model to infer from a screenshot what
macOS will tell us for free is the expensive and unreliable route.

### Run 4 — reached the video in 8 turns, then thrashed for 30

VS Code → Arc → search bar → typed → suggestion → results → result → watch page.
Genuinely good. Then thirty turns of clicking play.

- **Toggle thrash.** It clicked play, the video played, the next turn photographed
  a still frame that looks exactly like a paused one, so it clicked play again and
  stopped it. Repeat. *A still picture cannot show motion* — this is not a
  prompting failure, it is a limit of the input.
- **It believed the wrong application.** One turn reported VS Code while Arc was
  frontmost, and then acted on that belief for several turns.
- **Escape was dead** — a regression. It was a `keydown` handler in the overlay
  webview, and the overlay became permanently non-focusable when the typed input
  was removed. A window that never takes focus never sees a key. *Fixed: polled
  from the hardware in the loop that already watches Control.*
- **And a stop still let one more click through.** `stopping()` was checked at the
  top of the turn, then the model call took two seconds, then it clicked. From the
  user's side: "Escape gave me the cursor back, then it grabbed it again."
  *Fixed: checked again immediately before acting.*

### Run 5 — the agent was never waiting for the screen it had just changed

The largest single find so far, and it reframes every run before it.

`perform()` sets a settle wait after each action — 2.2s after an `Open`, 420ms
after a click — and `advance()` has always honoured it. **The agent loop never
did.** It slept 120ms and photographed. So every agent screenshot showed the
screen *before* the thing it had just done: a blank page, a menu mid-animation, a
video that had not started.

The thirty play-clicks, the "paused at 0:05" that never advanced, the desktop
captures — all consistent with reasoning about a screen that had not caught up.
The model was not as bad at this as the logs made it look; it was being shown
stale pictures. PLAN.md claimed the settle waits were "reused exactly as the
foreground uses them". They were not.

**Result: 37 turns became 4.** Open the results page, click the video, done.

Also this run: **Nudge heard itself.** It speaks each step aloud through the same
output device, so its own voice registered as audio — it opened a search page,
heard itself say "let's sail straight to Queen", and reported Done over a blank
page. `speech::is_playing()` already existed; facts was not asking.

### Run 6 — one click too many

Four turns, right answer, except the last action paused what it had just started.
Turn 1 said out loud *"audio is playing; the goal is not met yet"* and then clicked
play. The fact was there and it hedged, because the prompt told it audio "does not
say the sound is the one you were asked for" — true, but the window title had
already settled it.

*Fixed by making the inference decisive rather than the fact louder:* when the
audio report and the window title agree, the goal is met — say done, and do not
take one more action to be sure, because the control you are reaching for is the
one that undoes it.

### Run 7 — C1 passes

Four turns, clean, no stray click at the end. **The first task Nudge has ever
completed on its own.** Phase 1's premise — that a loop of grounded steps can
finish a real job unattended — is no longer hypothetical.

That is one task, not a hit rate. The accuracy count in Phase 3 is still the
number that decides the project.

### Run 8 — C2 finished, and did more than it was asked

Eleven turns in a native app, and the middle of it is the best thing Nudge has
done: a notification prompt, a Pro upgrade modal and a sign-in popup, none of them
foreseeable, each recognised and cleared before carrying on. That is agentic
behaviour rather than a script.

Then it overshot. The goal was "open CapCut and start a new project". It did that
at turn 5, did not recognise it had, and continued into Import → From device → a
file browser → Import, loading a file off the Desktop that nobody asked for.

Two distinct problems:

- **Did not recognise completion.** No system fact covers "a project exists"; the
  window title said only CapCut. This is the limit of what `facts.rs` can settle
  today, and the case for per-app learned memory later.
- **Scope creep, which is the dangerous one.** An agent that helpfully continues
  past its instructions is a hazard on a real machine with a real cursor. *Fixed:
  do exactly what was asked and stop — starting something is the end of the job,
  and choosing files or filling fields beyond that is deciding on the user's
  behalf. Clearing what is in the way stays in scope; signing in, buying and
  agreeing never are.*

### Runs 9-11 — C2, and what "done" means

C2 took four rounds, and each failure was a different reading of the same goal.

- **Run 9:** created the project, then imported a file off the Desktop nobody
  asked about. Read as scope creep. *Fixed: do exactly what was asked and stop --
  with the carve-out that clearing obstacles stays in scope, or the rule would
  have killed the best part of the run.*
- **Run 10:** still went into Import, and double-clicked a folder inside a file
  picker. Re-reading the log showed it was **not** scope creep: turn 2 said "no
  new project started yet". It could not tell it had finished, so a rule about
  stopping when finished never applied.
- **Run 11:** 2 turns, clean. Click Create project, done, and the part it stopped
  short of became the offer: *"Want to drop in some footage now?"*

The two rules that did it, both generic:

> When a goal could be read narrowly or broadly, TAKE THE NARROW ONE, finish
> there, and offer the rest as `next`. Starting something is finished when the
> thing exists -- not when it has content, not when it is ready to use.

> NEVER open a file picker for a goal that named no file, and never pick a file
> inside one. That dialog is a decision about someone's own documents.

The first also produces the follow-up offer for free, rather than bolting one on:
the broad reading is precisely the thing worth offering.

**Still unresolved underneath:** no system fact tells us "a project exists". The
prompt is routing around that gap rather than closing it. Clearest case yet for
per-app learned memory -- one note, *"CapCut: the timeline view means a project is
open"*, written from this exact failure. After Phase 3, not now.

### Runs 12-13 — C3, menus, and a keyboard that is not QWERTY

**Run 12:** fifteen turns clicking "New Private Window", coordinates jumping
272 → 183 → 107 → 194 → 110. The menu opened correctly every time and the click
never took. That is macOS menu tracking: menus run a nested event loop, and a
synthetic click into an open one often closes it without selecting. Grounding was
not the problem and better grounding would not have helped.

*Fix: a new `Step::Press`.* Shortcuts are the reliable way to run a menu command,
and the menu prints the shortcut beside each item -- so the model can read ⇧⌘N
off the screenshot rather than having to know it. Any modifier combination, plus
the keys that mean something pressed alone (escape to close a dialog, arrows and
return to move inside something already open). Bare letters and digits are
refused: that is typing, and typing has its own guarded path.

**Run 13:** the model chose `cmd+shift+n` immediately, correctly, four times --
and nothing happened.

The test that found it, after three wrong guesses (event source state, secure
input, modifier-as-flags):

    after unicode type_text: "abc"
    after raw key code 'z':  "abc;"

**This machine is not on QWERTY.** Key code 6 is the physical Z key and types `;`
here, which is Dvorak. So `cmd+shift+n` was sending key code 45 -- the B key on
this layout -- and Safari was ignoring shift-command-B, exactly as it should.

*Fix: resolve characters through the layout in use*, via `UCKeyTranslate` over the
current input source, instead of a hardcoded ANSI table. Colemak, AZERTY, QWERTZ
and every non-Latin layout come out right for free. Keys with no character --
escape, return, arrows, function keys -- are hardware-fixed and stay in a table.

Verified against Safari rather than assumed: window count 3 → 4. There is a
`#[ignore]`d test that does exactly that, because parsing being correct proved
nothing -- the first version parsed perfectly and pressed the wrong key.

**The lesson worth keeping:** three of the four hypotheses were plausible and
wrong, and each would have been shipped as a "fix". The one-line experiment that
printed what a key code actually types settled it in seconds. Reach for the
experiment before the third guess.

### Run 14 — C3 passes

Three turns: press the shortcut, a repeated press blocked by the guard before it
could open a second window, then done. C1, C2 and C3 all complete now.

The one wasted turn is the pattern that keeps recurring: **it acts, succeeds, and
cannot tell on the next look.** Turn 1 contradicted itself -- "in private browsing
mode, but a new private window is not yet created". Same shape as CapCut not
knowing a project existed, and YouTube not knowing a video was playing. The audio
fact solved one instance; the general case is still open, and is the strongest
recurring argument for per-app learned memory after Phase 3.

### Runs 15-16 — C4, and the opposite of C2's bug

Two turns, and it reached for `wttr.in/Kathmandu` rather than a search page --
the URL-carries-the-query rule generalising well past the case it was written for.

First attempt got the weather onto the screen and said *"Here is the current
weather report for Kathmandu."* It **showed** the answer instead of **telling** it.
A question goal is not finished when the answer is visible.

*Fixed:* if the goal was a question, the answer is words, and `say` is spoken
aloud, so put it there. Second attempt: *"Today in Kathmandu, it's currently light
rain showers with a temperature of 24 degrees Celsius."*

Worth noticing that this is the **exact opposite** of C2's failure. There the fix
was stop early, do not overreach; here it was do not stop early, you are one step
short. The line between them is not effort -- it is what the goal asked for. An
action goal ends when the action is done; a question goal ends when the answer is
spoken. Two rules that would contradict each other if either were stated as
"be more/less thorough".

**All four C tasks now complete.** A week ago none of them did anything but launch
an app.

### Run 17 — the agent was never speaking

C4's answer was correct in the log and never said out loud. **The agent spoke
nothing but questions** -- only the foreground path spoke, which is why the first
sentence was heard and then silence. It worked out "light rain showers, 24
degrees", wrote it to a file the user cannot see, and stopped.

Found only because the user said so. The log showed the right sentence and I read
that as success -- a trace proves a sentence was *produced*, never that it was
*heard*. **For anything whose output is not a file, confirm with the person.**

*Fixed:* the agent speaks every step, and the final sentence above all -- for a
question goal that sentence is the entire deliverable.

Knock-on fixed at the same time: Nudge talking makes the audio device busy, so
`audio_playing()` would have reported "silent" during every spoken step -- and
"silent" while a video plays is exactly the lie that makes an agent click play on
something already playing. `Facts.audio` is now `Option<bool>`; while Nudge speaks
the answer is **unknown**, and unknown says nothing rather than guessing. A fact
the model is told to trust over its own eyes must never be a guess.

### Run 18 — two agents, one cursor, and a voice note to a real person

The worst failure so far, and the only one with a consequence outside this
machine.

Asked to message someone on WhatsApp. It looked stuck, so the user spoke again.
And again. **Each one started a new agent**, and the log shows three of them
interleaved on the same chat:

    agent#3 turn 7:  Click (751, 882)
    agent#4 turn 0:  Click (1165, 890)
    agent#3 turn 8:  Click (1167, 885)
    agent#4 turn 1:  Click (1171, 880)

That button is **send** when the message box has text and **microphone** when it
is empty. One agent sent the message; the other clicked the same spot a moment
later, by which time it had become the microphone. A voice note went to a real
contact.

PLAN.md had said, under "deliberately not in this pass": *"Concurrent agents. Two
agents fighting over one cursor is incoherent. One at a time; the second queues."*
The reasoning was right and the code was never written. **A deferral with a
correct rationale is still a deferral** -- and this one was reachable by the most
natural user behaviour there is: repeating yourself when nothing seems to happen.

*Fixed in the registry rather than at the call sites*, because the rule is a
property of owning the cursor, not of any one way in. `Agents::start` returns
`None` when one is already running, and the refusal is spoken: *"I'm still on
Messaging. Press escape to stop me."* Silence would leave the user doing exactly
what caused this.

Also worth noting: the agents shared one `Nudge` session, so the second wiped the
first's -- line 68, "agent#3 turn 12: session ended". One-at-a-time makes that
moot, but the shared session is still an architectural sharp edge.

### Runs 19-21 — E1, and asking at the right moment

The question flow works: it asks, the answer arrives by voice, the same agent
resumes with it in context. What took three runs was *when* it asks.

- **First version asked mid-task**, because the prompt said to: *"ask it when you
  need it rather than all at the start."* That was my choice and it was wrong. It
  opened WhatsApp Web, hunted the sidebar, and only then asked who to message --
  wasting the time in between and leaving half-finished work on screen.
  *Fixed: work out everything the task needs before the first action and ask for
  all of it in one natural sentence.*
- **Then it announced a plan it immediately contradicted.** The handover step's
  sentence -- "Opening WhatsApp Web" -- was spoken by the foreground before the
  agent had looked at anything, and the agent's first turn two seconds later was
  the question. *Fixed: the handover is not an action and is no longer spoken.*
- **And one wasted turn:** `Launch { Arc }` followed by `Open { url }`. `Open`
  launches a browser itself, so that was a turn and a stray empty window.

Kept deliberately: a mid-task question is still allowed for what could not have
been known in advance -- a login only the user can complete, or which of several
matches was meant when it could not have known there were several. Collapsing
that would break the logged-out WhatsApp case.

### Phase 1 progress — capture correctness

**§2.1 done.** Nudge no longer photographs itself. Worth recording that the
section overstated the problem: a real capture showed Nudge contributed the notch
pill and the companion, not the "mysterious black void of a window" -- that was
most likely a dark browser window mid-load. The fix is still right, but it was
written up as an emergency and was not one. *Look at the artefact before trusting
the note about it.*

**§2.2 built, unverified.** Geometry is now read per capture rather than once at
startup, which is the actual bug -- it broke on a resolution change or an
unplugged monitor too, not just on a second screen. Single-display behaviour is
unchanged and the arithmetic is tested with displays to the right and above-left.
Whether one window can span two monitors at that window level is untestable
without the hardware, and is written up as such rather than claimed.

### §3.5 step 1 — Nudge can run commands

*"How many TypeScript files are in this project?"* -> two turns, 405, correct.
The first surface that answers a question without looking at anything.

Read-only, enforced in `core/shell.rs` rather than asked for in the prompt --
the privacy guard is the precedent, and the only safety rule here that has never
been talked past. Four layers: an allow-list of programs (not a deny-list, so
anything forgotten fails closed), checked per pipeline stage, with shell syntax
that chains or redirects refused outright, and secret-looking paths refused
whatever the program is running. Plus a fifth for programs that do both:
`git status` yes, `git push` no.

Two bugs found by running it:

- **It launched Terminal first.** `run` needs no terminal window -- the same
  mistake as launching a browser before `open`, and fixed in the same sentence.
- **"A completely black screen with no windows or applications open."** Mine, from
  the capture change: `kCGWindowListExcludeDesktopElements` drops the *wallpaper*,
  and the wallpaper is a window. With nothing else open the composite was solid
  black, and the model honestly described the picture it was given. Hidden most
  of the time because some window usually covers the desktop.

  *The lesson, again:* a change verified on one screen state is verified on one
  screen state. The test now asserts the list still contains something below the
  normal window layer, which is where the desktop lives.

### The fix for both: ask the system, do not look

`core/facts.rs` gathers what macOS already knows, immediately before each capture,
and hands it to the model beside the screenshot:

- **frontmost application and window title** — settles "what am I looking at",
  which the model was getting wrong from the picture
- **whether audio is playing** — `kAudioDevicePropertyDeviceIsRunningSomewhere` on
  the default output device. Public API, no permission, names no application, and
  answers "is it playing" for a browser, a music app, a video in Preview, or
  something nobody has written yet.

Verified against a real sound rather than assumed: `false` in silence, `true`
while `afplay` runs. The prompt tells the model these outrank the picture.

*Known ceiling, marked in the code:* it is the whole output device, so another
app's notification chime reads as audio for the moment it lasts. Per-process
attribution needs a tap on the audio stream — a permission prompt and a
background thread — and is not worth it until a false positive costs something.

**The general move:** before teaching a model to recognise a state, check whether
macOS will simply tell us. Candidates not yet taken: the frontmost window's
bounds, the current URL from the browser, whether a text field has focus.

---

## Latency — measured, not guessed

Measured against a comparable product rather than guessed at. It is **not**
a realtime omni model.

| Stage | Them | Us |
|---|---|---|
| Speech in | AssemblyAI over `wss://`, streamed while you talk | record, upload after release |
| Transcribe | `gpt-4o-transcribe` | Gemini, one round trip |
| Brain | `claude-sonnet-4-6`, streamed (SSE deltas) | `gemini-3.5-flash-lite`, wait for the whole object |
| Screen | ScreenCaptureKit `SCStream`, always running | `CGWindowListCreateImage` per shot |
| Speech out | ElevenLabs over websocket, streamed | macOS `say`, after the full sentence |

They use a **bigger, slower, more expensive model than we do and still feel
faster.** The gap is not the model — it cannot be closed by picking a quicker one,
which we already did. Nothing in their pipeline waits for the previous stage to
finish. Ours is strictly serial: upload, transcribe, capture, vision, act — about
four seconds before anything moves.

In payoff order, once Phase 4 says build: streaming STT (~1–1.5s, the single
biggest win) · capture during transcription, which depends on nothing (careful: only
safe on the first turn, since `Settle` assumes a capture taken after the previous
action landed) · stream the model response and fire on the first complete object ·
streaming TTS, cosmetic for latency and large for feel.

---

---

## The accuracy number, at last

Ten saved screenshots, a recorded answer for each, scored automatically. The
first real measurement this project has had.

| | gemini-3.5-flash-lite | gemini-3.6-flash |
|---|---|---|
| Hits | 2/7 (29%) | **4/8 (50%)** |
| Median | 1.75s | 4.5s |
| p95 | 2.3s | 9.0s |

**The shape of the misses mattered more than the score.** flash-lite was not
aiming badly -- it was landing on the wrong element by exactly one. Same column,
one row up; same strip, one icon over:

    01 wifi icon    said (1119,12)  wanted (1060,15)   59px sideways
    06 Bluetooth    said  (397,312) wanted  (398,342)  x correct, one row up
    07 search icon  said   (24,151) wanted   (19,108)  x correct, one icon down

**Resolution is not the bottleneck.** Re-running at 1920px instead of 1280 made
it slightly *worse*. The detail was already in the image; the model was not
resolving it. A negative result, and worth as much as a positive one -- it ruled
out the obvious fix in one command.

**The model was.** On identical inputs, 3.6-flash roughly doubled the hit rate,
and its remaining misses are 4-6px boundary artifacts plus one recorded box that
was drawn too narrow. Functionally about 6 of 8 against 2.

**The uncomfortable part:** we had been running the weaker model all day. Several
prompt rules written this week were compensating for a model that could not see
well enough -- which is exactly the kind of thing that is invisible without a
number, and exactly why every fix felt like it half-worked.

**On the latency trade.** 2.6x slower per step, and still the right call: a miss
costs a whole retry turn anyway, plus undoing whatever the wrong click did. A
4.5s step that lands beats a 1.75s step that has to be repeated.

**Three of the ten were never clicks.** `cmd+w`, `cmd+n` and `ctrl+f2` -- the
prefer-a-shortcut rule working, consistently, across both models. Those cannot
miss at all, and they are the best argument for pushing more work off the screen
rather than grinding at grounding.

---

## Why a live turn took three times longer than the bench

The bench measured 4.5s per call. Live runs were taking 12-14. Same model, same
kind of screenshot -- so the difference had to be the prompt, and it was:

    guide mode, no history :  12,954 chars
    agent mode, no history :  16,885 chars
    agent mode, 3 turns in :  69,773 chars
       of which history    :  52,888 chars

**Everything a step produced was carried forever.** A `read` returns up to 40,000
characters and all of it went into the history; so did a fetched page, and a
command's output. Each one re-sent on every subsequent turn, for the rest of the
task.

The bench never saw it because a bench case is one call with no history. It was
measuring the best case and calling it the number.

**The fix is about what the model needs when.** The last thing it did, it needs
in full -- that is the result it is reasoning about. What it did five turns ago it
needs to *remember doing*, so it does not do it again; the contents are long
since spent. So the newest steps are kept whole up to a budget, and older ones
are cut to their first line with "output no longer shown" -- said explicitly,
rather than implying there had been none.

70,000 characters back to 21,000, three turns in.

**Worth keeping in mind about benchmarks.** This one has been genuinely useful --
it found the model choice in one afternoon -- and it still measured a condition
that never occurs in practice. A number from a harness answers the question the
harness asks, which is never quite the question you have.

---

## The brain spends most of its time thinking

The table in [SPEED.md](SPEED.md) had one measured row and it dominated
everything else: the model call, 4.5s median. Reading the provider to find out
how to stream it turned up something cheaper first -- nothing anywhere sets
`thinkingConfig`, and flash models in this family reason before they answer.
Those tokens come out before the first character of the JSON, so they are paid in
full on the critical path of every screen action.

It cannot be switched off. `thinkingBudget: 0` is rejected outright: thinking is
part of what this model is, and the only question is how much. Four levels exist
-- `minimal`, `low`, `medium`, `high` -- and `none` is not one of them.

Scored on the ten recorded cases, repeated to pool the noise:

| level | runs | hits | median | p95 | expected time to a hit |
|---|---|---|---|---|---|
| default | 6 | 29/46 = **63%** | 6.05s | 9.3s | 9.6s |
| medium | 3 | 14/24 = 58% | 6.18s | 9.6s | 10.7s |
| low | 6 | 24/47 = **51%** | **2.54s** | 4.0s | **5.0s** |
| minimal | 3 | 10/24 = 42% | 2.22s | 2.7s | 5.3s |

**Latency comes in two tiers, not a slope.** `medium` costs what the default
costs; `low` is two and a half times quicker. The cliff is between them, which
means there is no gentle dial here -- there is a choice.

**Accuracy declines with every step down.** No single pair of those rows is
significant on its own; the gap between default and low is about 1.2 standard
errors, which on its own is nothing. But it is monotone across four levels in the
order you would predict, and four levels landing in the right order by chance is
about a one-in-twenty event. Taken together it is more likely real than not.

**The reframing that matters.** A turn that misses is not a turn that is wrong
forever -- the loop looks again. So the number to compare is not the hit rate and
not the latency but the product: expected time until something is actually
clicked. On that measure `low` wins outright, 5.0s against 9.6s, *because* it is
fast enough to be wrong twice in the time the default is right once.

That is not the whole argument, because a miss is not always free. A near miss --
12px, 22px -- lands on nothing and costs a turn. The 84px miss in the Mail case
lands on the wrong conversation. Time-to-success prices the first kind correctly
and the second kind not at all.

### The finding underneath the finding

**Ten cases cannot measure accuracy.** One hit is thirteen points. Two identical
runs of the default scored 50% and 71%; two runs of `low` scored 57% and 38%. We
have been choosing models on this -- 29% against 50% is in the plan as though it
were settled -- and the honest reading is that it was a coin landing the same way
twice.

Latency, the bench measures well: six runs of the default sat between 4.9s and
7.0s, and six of `low` between 2.3s and 3.1s, with no overlap at all.

So the instrument is good at one of the two things we have been asking it. The
fix is more cases, not more runs: repeating ten cases measures the model'''s
sampling noise, not whether it can find a button it has not been shown before.

## Phase 4 is worth two per cent, and here is why

Streaming the model was the plan's largest remaining win. Before building it,
one request with `alt=sse` and a stopwatch on the first chunk:

| thinking | first chunk | total | streaming could save |
|---|---|---|---|
| medium | 6.26s | 6.36s | 0.10s (2%) |
| low | 4.13s | 4.17s | 0.04s (1%) |

The model thinks for the whole call and then emits the JSON in four chunks over a
tenth of a second. **There is nothing to stream until the thinking stops.**

Obvious in hindsight, and it would have been days of work. The five minutes that
found it were the same five minutes the plan says to spend before every phase.

### And nothing we send matters either

If the wait is not the network and not the tokens coming back, it might have been
the tokens going out -- a 15,000 character prompt and a 140KB screenshot. It is
not. Time to first chunk, three runs each, thinking low:

| what we send | median |
|---|---|
| full image + full 15,424-char prompt | 3.47s |
| full image + 149-char prompt | 3.88s |
| half-size image + full prompt | 5.48s |
| no image at all + full prompt | 2.93s |

Cutting the prompt by ninety-nine per cent does nothing. Halving the image does
nothing. Removing the picture entirely -- which is not an option, it is the whole
job -- saves about half a second.

**So the brain has a floor of roughly three seconds that is not ours to move.**
Not prefill, not upload, not generation. It is the model deciding, and the only
three levers on it are how much it may think, which model it is, and whether we
call it at all.

One of those runs took 19.39 seconds against a median of 5.48 for the same
request. That is worth remembering whenever a p95 looks alarming: some of it is
not us.

### What this leaves

Everything in the plan that was going to shave the model call is now measured and
small. `thinkingLevel` is the only dial with real travel, and it is paid for in
accuracy. That makes the accessibility tree -- the one row that skips the call
instead of shortening it -- no longer the speculative long game at the end of the
list. It is the only lever left.

---

## Phase 5: does macOS already know where the buttons are?

The plan's last phase rested on a doubt: a native app gets an accessibility tree
for free, but an Electron app is a web page in a window and exposes whatever its
authors bothered with. VS Code is in the bench cases, so this was answerable in an
afternoon rather than arguable for a fortnight. `cargo run --bin ax` walks the
frontmost app and reports what it finds.

**It works, and it is fast.**

| app | kind | elements | walk | clickable and named |
|---|---|---|---|---|
| Finder | native | 389 | 131ms | 297 |
| Mail | native | 613 | 412ms | 464 |
| Arc | Chromium | 1576 | 769ms | 689 |
| VS Code | Electron | 1924 | 152ms | 440 |

And the thing that matters, from VS Code -- an Electron app, the hard case:

    AXRadioButton    Search (⇧⌘F)      at (8,116) 36x36
    AXRadioButton    Explorer (⇧⌘E)    at (8,72)  36x36

That first line is bench case 07, `click-the-search-icon-in-the-sidebar`. The
vision model answers it in **4,568ms at best and 9,841ms at worst, and sometimes
wrong**. The tree answers it in **152ms with an exact 36x36 frame**, every time,
for no tokens.

### Four things that took a wrong turn to learn

**Chromium and Electron keep the tree switched off** until an assistive client
announces itself, by setting `AXManualAccessibility` or `AXEnhancedUserInterface`
on the application element. VS Code accepts the first and returns 0; Chromium
rejects both with -25205 and -25208. Building that tree is not free for them
either -- we would be asking a large application to maintain a parallel model of
its whole interface for as long as we are watching. That is the real cost of this
approach and it belongs in the decision, not in a footnote.

**Windows are not always in `AXChildren`.** Some applications answer that with
their menu bar alone and keep the windows behind the specific name `AXWindows`.
The first run of the probe concluded that Chromium exposes nothing but a menu
bar, which was the probe's fault.

**An app with no window exposes no window.** VS Code and Chromium were both
running, both listed as visible by System Events, and both reported
`AXMenuBar(11)` and nothing else -- because neither had a window open. Two
separate rounds of this were read as "Electron exposes nothing" before
`count of windows` said 0 and settled it. *Visible is not the same as having
something to show.*

**A closed menu has no geometry.** Every menu item reads `(0,982) 0x0` until its
menu is open. So menu navigation is not something the tree can shortcut: you
still have to open the menu before you can be told where anything in it is,
which is exactly the click-then-look loop we already have.

### What is actually left to build

Not the lookup. The lookup is a function call and it is done. What is left is the
matching: turning *"the search icon in the sidebar"* into `Search (⇧⌘F)`. That is
string matching where it is easy and a model call where it is not -- but a model
call against forty labels is a different thing from a model call against a
screenshot, and it is the cheap kind.

Two things this probe did not settle, and should before anything is built on it:
the file explorer's tree rows never appeared under any role (the one good run may
simply have had a different sidebar showing), and Arc's web links all reported a
height of one pixel, which is not a thing you can click.

---

## Looking at the screen costs four seconds

The estimate in SPEED.md was 0.3-0.5s for a capture, marked E for estimated. It
was the least examined row in the table and it is the second largest cost in the
product. `cargo run --example captime`:

```
  grab(240)                1021ms
  grab(640)                1226ms
  grab(1280)               1769ms      <- the live setting
  grab(1920)               2457ms

  resize + encode           232ms
  compositing              1529ms      <- everything else

  facts::gather              50ms
  ax::controls              206ms      (248 controls in VS Code)
```

**Compositing the screen costs a second and a half.** The resize and the JPEG,
which were tuned carefully and have a comment explaining the choice of filter,
are 232ms of it. `max_edge` -- described in the config as "the main speed dial"
-- moves the number by about a second across its whole useful range, because it
only touches the small half.

And the part nobody had ever timed: **`wait_until_still` costs 2.1 seconds.** It
composites at 240px twice, because there is nothing to compare a first sample
against, and a 240px composite costs 1021ms -- almost all of it compositing,
since the size barely matters.

So a turn that does anything spends:

| | |
|---|---|
| stillness check | 2.10s |
| the screenshot itself | 1.77s |
| facts and the control tree | 0.26s |
| **screen, total** | **~4.1s** |
| the model | ~6.0s |

**Nearly forty per cent of a turn is compositing the screen**, and it is one
deprecated call doing it: `CGWindowListCreateImageFromArray`, which the code
comment has flagged since it was written as the thing ScreenCaptureKit replaces.

### What this does to the plan

SPEED.md deferred ScreenCaptureKit under "deliberately not doing", with the
reason: *if capture turns out to be 400ms of a seven-second turn, it is not where
the next week goes.* It is 4.1 seconds of a ten-second turn. It is exactly where
the next week goes.

It is also the largest lever left that costs no accuracy. Turning thinking down
saves 3.5s and is paid for in grounding. This saves more and is paid for in a
framework migration.

A cheaper intermediate exists, worth about 1.2s: `fingerprint` resizes to 16x16
before comparing, so samples of different sizes are comparable, and the stillness
check's second composite could be the real screenshot rather than a third one.
Worth doing only if the migration turns out to be slow.

### A smaller note, in our favour

`ax::controls` returned **248 controls from VS Code in 206ms**. Against a 1.8s
screenshot and a 6s model call, asking the system what is on screen is free.

## ScreenCaptureKit: 1.53s of compositing becomes 86ms

Measured before migrating, because "it is the modern API" is not a reason and a
number is.

| | `CGWindowList` | `SCScreenshotManager` |
|---|---|---|
| compositing | 1529ms | **86ms** |
| `grab(240)`, the stillness peek | 1021ms | **90ms** |
| `grab(1280)`, the shot | 1769ms | **296ms** |
| a turn's screen time | **~4.1s** | **~0.7s** |

**Three and a half seconds a turn, and it costs nothing in accuracy.** That is
more than turning thinking down saves, and thinking is paid for in grounding.

It also scales while it composites, so the 232ms of resizing goes as well -- ask
for 1280 wide and 1280 wide is what arrives, done on the GPU on the way past.

Two things worth knowing before relying on the numbers. The **first call in a
process costs 892ms** while the framework wakes up, against 119ms for every one
after; the app is long-lived and pays that once, but a benchmark that measures
one capture measures the wrong thing. And what is left is no longer compositing:
of the 296ms, roughly 86 is the capture, 30 the byte-order conversion and **180
the JPEG encode**, which is now the largest part of taking a screenshot.

### How it was checked

A picture with the right dimensions can still be a black rectangle, and a picture
with the right brightness can still have its colours inverted. So: brightness
spread against a flat image, and **per-channel means against the old path on the
same screen at the same moment** -- they agree to within half a level across all
three, which is what says the BGRA byte order was read correctly rather than
plausibly.

The old path stays underneath. This one needs macOS 14, a current screen
recording grant, and a framework that is entitled to decline; a slow screenshot
beats none.

## Not asking the model at all

The model is now about six seconds of an eight second turn, and its only levers
are how much it may think and whether it is called. This is the second one.

When someone says *"click the View menu"* and the system is already telling us
there is exactly one thing called View, at (195,16), 50 by 33 -- asking a model
to look at a picture and work that out is six seconds spent confirming what we
were told. Measured against a live application:

```
"click the View menu"           obvious: AXMenuBarItem "View" at (195,16)
"click Extensions"              obvious: AXMenuBarItem "Extensions" at (440,16)
"press Help"                    obvious: AXMenuBarItem "Help" at (576,16)
"click the Window menu please"  obvious: AXMenuBarItem "Window" at (518,16)
"click view or edit"            not obvious -- this would go to the model
```

**A turn of about 300ms instead of about eight seconds**, for the cases where
someone names a thing that exists.

### The bar, and why it is set where it is

There is nothing behind this. A model that grounds badly still said what it saw
and can be contradicted by the next turn; a wrong match here clicks something
with nobody disagreeing. So all of these must hold:

- **The goal opens with an instruction to click.** "How do I send this" and "the
  send button is greyed out" both contain the word send and neither is a request
  to press it.
- **A whole label appears in the goal, on word boundaries.** Not a substring:
  "ok" must not match inside "bookmark", and "tab" must not match "table".
- **Exactly one control qualifies** -- and it fires only on the **first turn** of
  a session, which is what makes having no second opinion safe. It can act once
  and then the model has the rest of the task. Something that could fire
  repeatedly could also loop, and nothing here would notice.

Two rules needed more care than expected. *"click send later"* matches both
"Send" and "Send Later", and calling that ambiguous punishes someone for being
precise -- the second contains the first, so it is one control named more fully.
Whereas *"click send or cancel"* matches two unrelated labels and genuinely
cannot be resolved. The rule is therefore: the longest match wins, but only when
every other match sits inside it.

And a test that passed for the wrong reason, caught by reading it rather than
running it: a minimum label length of three characters was silently excluding
"OK" and "No", which are the shortest things anyone actually says, while the
comment above it claimed they survived.

## The screenshot again: 1769ms to 61ms

After ScreenCaptureKit took compositing from 1529ms to 86ms, the largest part of
taking a screenshot was no longer taking it. It was the JPEG.

| | ours | the system's |
|---|---|---|
| composite + convert pixels | 192ms | 56ms |
| encode | **167ms** | **4ms** |

Forty times quicker, and the conversion disappears with it: ScreenCaptureKit
hands over a `CGImage`, ImageIO accepts a `CGImage`, so on this path **the pixels
never pass through Rust at all**. No byte-order loop, no intermediate buffer, no
second copy.

```
  grab(640)    54ms
  grab(1280)   57ms      <- the live setting, from 1769ms
  grab(1920)   58ms
```

Size barely registers any more; it is one hardware operation whatever the
dimensions.

### On running a capture stream instead

The plan had `SCStream` for this row -- keep a session running so a frame is
always in hand, which is what the product we measured against does. It is now
the wrong thing to build. A stream removes the *compositing*, which is 56ms of a
61ms capture; it does nothing about the encode, which was the actual cost. And it
buys that by capturing the screen continuously: CPU and GPU all the time, battery
while nobody is asking for anything, and a privacy story that is hard to tell
about an app whose pitch includes refusing to photograph password managers.

**Asking for one frame when one is wanted is now cheaper than keeping one warm.**

### Checked

The bytes have to survive: `fingerprint` decodes them every turn to decide
whether the screen has stopped changing, so a JPEG that was the right size and
unreadable would break the loop quietly. It decodes, the dimensions match what we
claim to have sent, and the brightness has a real spread rather than being flat.

---

## The first live turn, and what the estimates were worth

Every number in the plan's table was an estimate except three. Here is a real
turn, measured by the instrument built in Phase 0 and read out of a real
conversation:

```
wav 0.03s · heard 0.19s · settle 0.00s · still 0.00s · hush 0.00s · shot 0.00s · brain 4.28s · total 4.50s
```

| stage | the estimate | measured |
|---|---|---|
| record to WAV | ~0.1s | 0.03s |
| transcription | 1.0-1.5s | **0.19s** |
| settle | 0.2-0.4s | 0.00s |
| stillness check | 2.10s (measured) | 0.00s |
| our own voice | ~1.0s | 0.00s |
| screenshot | 1.77s (measured) | 0.00s |
| the model | 6.05s (measured) | 4.28s |
| **everything but the model** | **~6.5s** | **0.22s** |

**Thirty times less overhead**, and a turn that was about eleven seconds is four
and a half. The model is now ninety-five per cent of it.

The zeroes are not the work disappearing -- they are the work moving. The
screenshot still costs 61ms; it happens while the transcription is running, and
the transcription is now 190ms, so it finishes first. On an agent turn, where
there is no transcription to hide behind and something was performed, the same
line reads `still 0.24s · shot 0.33s · brain 7.90s`: real numbers, and still a
third of a second against nearly eight.

**On-device transcription cost nothing in accuracy.** It returned *"When will be
the official release version of macOS 27 will release"* verbatim, including the
phrasing, in 190ms and without a packet leaving the machine.

### What is left

Only the model. Every other row is measured, small, and mostly hidden. The two
levers on it are how much it thinks -- 3.5s, paid for in grounding accuracy -- and
whether it is called at all, which is the direct-match path and currently fires
only for an explicit "click *something*" on the first turn of a session.

### One thing latency does not fix

The same log has the agent answering that macOS 27 *"would likely launch around
September or October 2036"*, on a machine running macOS 27. It searched, got 2744
characters back, and still produced the wrong year. Nothing in this document
would have caught that, and the bench cannot see it either -- it scores where a
click lands, not whether the answer is true.
