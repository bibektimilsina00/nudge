# Nudge

**The layer everything else sits under.**

*It looks like it cannot do anything. It can do everything.*

---

## 1. The product

### The sentence

**Nudge is how you operate your computer when you no longer want to operate it
yourself.**

Not an assistant you consult, not a chat window you paste into. The layer between
you and the machine: you say what you want, and the machine does it, using the
same software you would have used.

Every other agent is limited to what has an API. They reach files, shells, HTTP,
repositories -- and they all stop at the same wall: software with no API cannot be
touched. Nudge goes through the wall by using software the way a person does. It
reads the screen, finds the control, presses it. So the set of things it can do is
not *what has been integrated* but **what you can do on this machine**, which is a
much larger set and a permanently larger one.

### The shape it has to keep

A cat in the notch. No window, no sidebar, no roster, no canvas, no greeting. You
hold a key, you say a thing, it happens, and the screen goes back to being yours.
Underneath it drives applications, runs processes, reads files, delegates to other
agents and orchestrates work that takes minutes.

**The gap between those two is the whole idea.** Everything else in the category is
sized like what it can do -- a window because it is important, a sidebar because
there is a lot of it. Nudge is sized like an accessory and works like an operating
system.

This is a constraint, not a mood. It rules things out, which is the useful part:

- **No capability gets a panel.** MCP will add a hundred tools and no interface.
- **The panel is for settings, not for work.** Anything used *while* working
  belongs in the voice loop or the agent card.
- **Growth goes down, not out.** More to do means more per sentence, never more
  per screen.
- **The cat carries what a UI would** -- state, attention, progress.

An operator cannot be a window, because a window competes for the screen with the
very software it is supposed to be operating. That is why everyone who builds one
drifts into being a destination you go *to* and stops being a layer that sits
*over*. Staying in the notch is not modesty; it is the only shape that works.

### Where it ends up

**You run your whole computer through the notch.** Not most of it, not the parts
with integrations -- all of it.

Underneath it uses whatever is installed: a coding agent, a CLI, a shell, an
application driven through its own interface. **The person is not meant to know
which.** They install a coding agent once, authenticate it, never open it again,
say *"clean this up"*, and it happens. Jarvis, with a cat.

The tension in that, named rather than discovered: *"they do not need to know"*
reads badly beside *"something is silently running programs on my machine."* The
two reconcile only on purpose:

> **Never needs to know. Can always find out.**

Invisible by default, inspectable on demand. The agent card is already the right
shape -- a receipt after the fact, not a dialog before it.

### What it makes of everything else

Other agents stop being products you choose between and become **tools it calls**.
You do not open a coding agent; you say what you want and Nudge picks one, watches
it, and reports. Same for whatever replaces it next year, and there will be one.

That is the durable position: **the model layer keeps changing hands, the
interface layer does not.**

---

## 2. Where it is now

### What works

**The loop.** Hold a key, speak, and it acts: clicking, typing, pressing keys,
launching apps, opening URLs, reading and writing files, running commands,
starting long processes, fetching pages, searching the web, planning, delegating.
Twenty-one outcomes the model chooses between. Agents run unattended with a
budget, a card showing their plan and their work, and an instant stop.

**Grounding, which changed shape.** The accessibility tree reports every control
on screen -- name, role, exact rectangle -- in about 200ms, for no tokens. So:

- The model picks a control from a numbered list rather than aiming at a pixel.
- When a request names exactly one control, it never reaches a model at all:
  *"click the View menu"* is about 0.3 seconds end to end.
- Menus are pressed through the accessibility API without touching the pointer.
- Everything else is still a picture and a model, right about two thirds of the
  time.

**Speed.** A turn is about 4.5 seconds and roughly 95% of that is the model.
Everything else totals about 0.22s. What that took is in
[SPEED.md](SPEED.md) and [FINDINGS.md](FINDINGS.md); what matters here is that
latency is no longer a project, and three plausible-sounding ideas were measured
and abandoned rather than built.

**Privacy, as a property rather than a promise.** Transcription happens on the
machine. Screenshots never leave it. The guard refuses to photograph password
managers and windows whose titles name a secret.

**Boundaries that exist because something went wrong without them.** A workspace
nothing reaches past. A shell allow-list of 38 programs with read-only
subcommands. One cursor, one agent. Ask before replacing a file.

### What is built but unproven

Listed because "built" and "works" are different claims, and this project has
confused them before.

| | What is unknown |
|---|---|
| **Multi-monitor** | The arithmetic is tested including negative origins. Whether one window really spans two displays at that level has never run on two displays. |
| **Orchestrating another agent** | `claude`, `codex` and `agy` are verified by execution. `opencode` and `aider` come from documentation. No end-to-end run of the whole flow. |
| **The panel flicker fix** | Two plausible causes addressed. Neither verifiable without watching a Space transition on the machine it happens on. |
| **The Windows port** | The seam is drawn, the portable side is written and runs on macOS behind a feature flag, and the UI Automation tree has never been compiled. See [PORTING.md](PORTING.md). |

### What is broken

- **Not distributable.** Not notarised; `spctl` rejects it. The signing identity
  is one machine's certificate. Nobody else can run this.
- **It says things that are not true.** Asked when macOS 27 ships, on a machine
  running macOS 27, after a successful web search, it answered 2036. Nothing in
  the project can currently detect that.

---

## 3. What stands between here and there

Five groups. Everything in §4 belongs to one of them.

**Trust.** It has to be right, and it has to be checkable. Today one of the two
harnesses measures control selection well, the other measures pixel-grounding on
ten cases which is too few to mean anything, and neither can see whether an answer
is true.

**Reach.** A general local agent does more than this does: any shell command, any
file, real HTTP, email, calendar, chat. Those are not features to copy one by one
-- most arrive through one decision (MCP) and one design (boundaries that a person
can widen).

**Invisibility.** The end state needs Nudge to use what is installed without the
person knowing, which means noticing what is there, asking for what is missing
once, authenticating things nobody opens, and translating failures into something
about the task.

**Learning.** It starts every session knowing nothing about this machine, these
applications, or what went wrong last time.

**Distribution.** Nobody else can install it.

---

## 4. The path

Ordered. Each one says what it is, why it is here rather than later, and how we
will know it is finished.

### Phase 1 — Trust

Everything else is worth less until this is done. An operator that is confidently
wrong will not be given real work, and the more invisible the machinery gets the
more the one visible thing -- what it tells you -- has to be true.

**1.1 A harness that scores answers, not clicks.** — **built, three cases**

`make truth`. A question, strings that must appear, strings that must not; the
real blind loop with search and fetch behind it. Substrings rather than a model
judging free text, because judging with a second model makes the score depend on
a second thing that can also be wrong.

Three outcomes, and unsure is a pass: *"I could not find out"* is the correct
answer to a question it cannot answer, and worth more than a guess that reads the
same as knowledge. Only WRONG exits non-zero. Verified by asserting Berlin is the
capital of France and watching it report `WRONG said "paris"`.

*What is left:* cases. Three is enough to catch a regression and nowhere near
enough to find an unknown failure -- and the 2036 answer has not reproduced,
which means it is intermittent, which means a handful of cases will miss it.
Fifty, spread across: facts after the training cutoff, facts that changed
recently, things with no answer, things where the search returns something
plausible and wrong, and arithmetic on dates.

*Also not covered:* this drives the blind path. The original failure came from an
agent that also had a screenshot.

**1.2 Verify before asserting.** *Built.*

`scrutinise`, in `core/run/subagent.rs`. A subagent that looked something up does
not return the answer directly: a second, independent pass is told the answer
already exists and asked to find what is wrong with it. One extra round trip, and
only on answers that came from looking something up -- a subagent reporting what a
file contains has nothing to refute.

Building it turned up the thing worth writing down. **A checker told to find a
fault will find one**, and the first two versions each made an answer worse:

- Asked to check *"future stock prices do not exist yet"*, it produced a share
  price for next Friday. An honest refusal survived the first pass and was
  destroyed by the pass meant to protect it.
- Asked to check *"macOS 27, September 2026"*, it answered from its training
  cutoff -- macOS 15, 2024 -- without searching at all. The checker committed the
  exact failure it was summoned to catch.

So the rule is not "trust the checker". It is **verification may only lower
confidence, never raise it**, enforced in three places rather than asked for in
the prompt:

1. An answer that already admits uncertainty is not checked. There is nothing to
   refute and everything to lose.
2. A correction from a checker that consulted nothing is discarded. Memory does
   not overrule a source.
3. When both passes looked and disagreed, **neither wins.** The disagreement is
   handed back whole.

Three is the one that matters, and it was not the plan. The plan said "report what
survives", which assumes a winner. Asked when macOS 27 shipped, one pass searched
once and said September 2026; the other searched three times and said it had not
shipped. More searching is not more right, and there was no basis for picking.
Swapping one confident claim for another behind the user's back is the failure
this mechanism exists to prevent, and it does not stop being that failure when we
are the one doing it.

*Status:* both paths seen live -- the checker agreed on one run of the macOS case
and disagreed on the next. That intermittence is 1.1's finding restated, and it is
why the case set came next rather than more mechanism.

**1.1a Grow the truth cases.** *Written, not yet scored clean.*

Three to forty-eight. Weighted toward where the failure lives: 11 stale-cutoff,
8 false-premise, 7 with no answer at all, 10 settled-history as a floor, 7 date
arithmetic (named above as a gap), 5 misremembered specifics.

Most are `never`-only -- say what is definitely false, leave the truth alone.
Naming today's Python release would make the case wrong within a year; naming
3.11 makes it wrong never. It is also the shape that cannot be *authored* wrong,
because it barely claims to know anything.

Two harness bugs came out of the first full run, and the second is the one worth
keeping:

- Six cases at once collected rate limits instead of answers from case 25 on.
  Three, with backoff.
- **A rate limit was scored as a wrong answer.** Twenty-four cases that were
  never asked a question were reported as truthfulness failures -- the harness
  doing to me precisely what it exists to catch. `error` is now its own outcome,
  counted apart and never confused with a wrong answer.

And one bug in 1.2, found only because the cases existed: the disagreement note
ends *"I could not confirm which is right"*, so **every disagreeing answer read as
hedged** to the judge, which scored a confident two-year Bitcoin forecast as an
admission of uncertainty. Hedging is now judged on the agent's own words with our
note cut off first.

*Blocked:* the Gemini project hit its monthly spending cap partway through the
first full run. Cases 001-019 all passed, which is the most that can honestly be
said. A clean full run needs the cap raised.

*Done when:* forty-eight cases run to completion with zero wrong and zero
errored -- and then again, because one green run of an intermittent failure means
very little.

**1.3 Say what is uncertain.** *Built.*

The model had one voice for *"I clicked Send"* and *"macOS 27 ships in 2036"*. It
now has two: a recalled fact is prefixed `From memory:` and an observed action is
not.

Set two ways, and the split is the honest part:

- **The model says so.** `recalled: true` on `done` and `reply`, applied in
  `simple_step` where both outcomes are built, so everything downstream -- spoken,
  shown, written into the history -- carries it without knowing it exists.
- **The subagent works it out.** It has no screen, so if it also consulted
  nothing, there was no source in the room and whatever it said came from memory.
  Marked whether or not the model marked it.

The structural override only exists for the blind path, and that is a real limit
rather than an oversight. The main agent always has a screenshot, so *consulted
nothing* does not mean *not grounded* -- it may be reading the answer off the
screen. Nothing available to us separates a claim about the screen from a claim
about the world, so there the model's own report is all there is.

Same invariant as 1.2: the marker can be added, never removed. `Step::consults()`
is exhaustive rather than a list of the interesting cases, so the next tool added
forces a decision instead of quietly defaulting to *grounded* -- the list-of-four
in `commands::step` went stale the moment a tool was added to it, and this is the
same trap one module over.

The truth harness prints `(from memory)` beside any answer carrying the marker,
including passes. An answer that was right without anything being consulted is
right the way a guess is right.

*Not the same as a hedge*, and tested to keep it that way. *"I did not check
this"* is not *"I do not know"*, and a harness that conflated them would score an
unchecked wrong answer as an honest one.

*Left undone:* the Anthropic provider reads a computer-use tool response, which
has no room for a key we invented, so its main-agent answers can never carry the
model's own report. They still get the subagent's structural one.

*Done when:* a recalled fact and an observed action do not sound the same. **They
do not**, and there is a test by that name.

**1.4 Grow the control cases.**

Fifteen is enough to catch a regression and not enough to find an unknown failure.
Each one costs a sentence. Cover: dense settings panes, file lists, web content,
toolbars with icon-only buttons, dialogs, and at least one application that
exposes nothing so the fallback is exercised.

*Done when:* forty cases, and the wrong-pick count is still zero.

### Phase 2 — Reach

**2.1 MCP.**

Promoted from last place to here. Email, calendar, Slack, GitHub, databases,
Notion -- those servers exist and other people maintain them. Six integrations
written by hand buy six integrations; speaking MCP buys the ones that exist now
and the ones written next year. It is the single decision that closes most of the
parity gap.

The original reason for deferring it still holds and should be respected rather
than ignored: a plugin surface over a tool set that is still moving locks in
shapes that should not be locked. Phase 1 is when the tool set settles.

**It must add no interface.** A hundred new tools, no new panel. Configuration
lives in a file, not a tab.

*Done when:* an MCP server the project has never heard of can be added to a config
file and used by voice on the next turn.

**2.2 Boundaries a person can widen.**

The shell allow-list, the workspace, the GET-only fetch. **Not removed** -- every
one of them is a scar. The shell is read-only because a model reached for `rm` to
get around a refusal. Files are workspace-bound because the first thing that wrote
a file put it in this repository's root. One agent owns the cursor because two of
them sent a voice note to a real person.

Parity means replacing a fixed answer with a decision someone makes:

- **Widening is explicit and visible** -- a setting or a per-task grant, never a
  longer constant in the source.
- **The default stays exactly where it is.** Someone who never opens settings
  keeps today's Nudge, which is right for a thing that listens all day.
- **What was granted is inspectable.** "It has full shell access" must be
  something you can see, not something you have to remember agreeing to.

The general agents are unbounded because nobody has done this work, not because
it is wrong.

*Done when:* a person can grant full shell access on purpose, see that they have,
and take it back.

**2.3 Real HTTP.**

`fetch` is a GET with no authentication. Anything that talks to an API needs more,
and most of the interesting things a person wants automated are behind one.

*Done when:* a request with a method, headers and a body can be made, and refused
as clearly as the shell refuses.

### Phase 3 — Invisibility

Everything here exists because of §1's end state. None of it is needed while the
only user wrote the program.

**3.1 Notice what is installed.**

Detect the coding agents, CLIs and tools present, and work with whatever is there
rather than with a fixed list. Report the absence of something only when it would
have helped.

**3.2 Ask for what is missing, once.**

In plain language, at the moment it matters: *"I could do that if you install X."*
Not a setup wizard, not a checklist. The alternative is a person who never
discovers that their machine is missing the one thing that would have worked.

**3.3 Credentials.**

The unglamorous blocker in the whole vision. A coding agent that is never opened
still has to be authenticated. Whatever the answer is -- inheriting a session,
driving a login once through the screen, an explicit hand-off -- it has to exist
or "install and forget" is not true.

**3.4 Failures that are about the task.**

When the thing underneath breaks, nobody sees it break; they see Nudge fail. A
CLI's stack trace passed up unedited is a bug report about a program the person
does not know is running. Errors have to be rewritten into something about what
they asked for, with the detail still available to anyone who looks.

**3.5 Delegation stops being visible.**

Choosing between agents becomes Nudge's job rather than a choice presented. The
card reports the plan, the commands and the files -- *what was done* -- and not
*who did it*.

*Phase 3 is done when:* someone who has never heard of a coding agent can install
one, forget it, and never be reminded it exists.

### Phase 4 — Learning

**4.1 Memory.**

Per-app notes, written from failure, injected only when that application is in
front: *"CapCut: the timeline view means a project is open."* Earned once the loop
is known to work -- memory that records a broken loop's habits is worse than none.

**4.2 Skills, if memory proves out.**

A remembered sequence that worked, replayable by name. The natural extension, and
the thing every competitor advertises. Deliberately after memory, because a skill
is a memory that has been promoted.

### Phase 5 — Being usable by anyone else

**5.1 Signing and notarisation.** Developer ID, in CI, before any public link
exists. Everything above is theoretical until this is done.

**5.2 Discovery.** The cost of §1's shape: an interface that shows nothing teaches
nothing, and nobody guesses that the thing in the notch can refactor a repository.
The answer has to live in the voice loop -- *"what can you do?"* answered well, and
capability surfacing when it is relevant -- rather than in a menu nobody opens.
The first real blocker on the second user.

**5.3 The resting state.** What is on screen 99% of the time is the pill and the
cat, and they have had the least attention of anything in the project. If the
product is the shape, the shape is what needs the work.

### Phase 6 — Elsewhere

**6.1 Windows.** The seam is drawn and the other side is written. Nothing has been
compiled for the target, because it cannot be from a Mac. Expect a different
latency profile entirely: a capture is 61ms here through ScreenCaptureKit and a
hardware encoder, and about 2100ms through the portable path. See
[PORTING.md](PORTING.md) for which calls to suspect first.

**6.2 Linux.** After Windows. Wayland and X11 handle capture and input injection
completely differently, so it is two ports wearing one name.

### Running alongside all of it

Not phases, but things that must not be allowed to rot:

- **Prove what is already built.** The four unproven items in §2. The one that
  matters most is whether an agent knows *not* to delegate a one-line change --
  the failure nobody notices because it still works.
- **Nothing reads the diff.** An agent reports what it did and Nudge believes it.
  `git diff` is one call away and would turn a claim into a check.
- **Widen the no-model path.** Every phrasing it learns is another turn that costs
  0.3s instead of 4.5. Cheap and compounding, and safe now that a wrong pick is
  caught the moment it appears. Typing into a named field and launching apps by
  name are next.

---

## 5. The safety model, and how it must survive Phase 2

Every rule exists because something went wrong without it:

| Rule | What happened |
|---|---|
| Workspace boundary | The first thing that wrote a file put it in this repository's root |
| Read-only shell, by program | A model reached for `rm` to get around a refusal |
| One cursor, one agent | Two agents shared a WhatsApp chat and sent a voice note to a real person |
| Ask before replacing | Self-evident, once |
| Privacy guard | A password manager is one frontmost window away at all times |
| Everything started is killed | A dev server still holding port 3000 tomorrow would be Nudge's fault |

**"Controls everything" has to mean more enforcement, not less.** Phase 2 widens
what is possible; it must widen what is *recorded* by the same amount. The test
for any new reach: can the person see afterwards what was done with it?

---

## 6. How we will know

Four instruments. Each answers one question and none answers another's.

| | Question | State |
|---|---|---|
| `make picks` | Does it choose the control you meant? | 15 cases, offline, instant. Wrong picks reported separately from fall-throughs and never averaged |
| `make bench` | Can a model find a control in a picture? | 10 cases. Good for latency, too small for accuracy, and measures the surface the tree replaced |
| the timing line | Where did this turn's time go? | Every turn, every stage |
| `cargo run --example captime` | What does looking at the screen cost? | On demand |

**Missing, and Phase 1.1:** does it tell the truth?

Two lessons from building these, worth keeping:

- **Ten cases cannot measure accuracy.** One hit is thirteen points; two identical
  runs scored 50% and 71%. Latency the same harness measures well.
- **Verify the instrument before trusting the result.** A domain check silently
  started answering "taken" for everything after a rate limit. A benchmark that
  cannot fail is not measuring anything.

---

## 7. Deliberately not doing

- **A window.** Ever. See §1.
- **Streaming the model's reply.** Measured four times: the first chunk arrives at
  6.26s of a 6.36s call, because it thinks throughout and then emits in 0.15s.
  Worth 2%.
- **A running capture stream.** Would remove 56ms of a 61ms capture and cost
  continuous screen recording.
- **A workflow scripting engine.** Parallel delegation already exists and is
  bounded by the cursor, not the design. The one useful idea from it -- verify
  before asserting -- is Phase 1.2 and needs no engine.
- **Trading accuracy for speed.** Measured once, properly: 29% against 50%. A fast
  wrong click costs more than a slow right one, and it costs it in trust.
- **Feature parity with any specific competitor.** Parity with what a *general
  agent can do* is §4 Phase 2. Parity with a product is the losing side of every
  fight.

---

## 8. Debt, marked in the code

Searchable with `ponytail:`. The ones that matter:

- **`CGWindowListCreateImageFromArray`** is still the fallback under
  ScreenCaptureKit. Fine, and it should stay: the fast path needs macOS 14 and a
  current screen-recording grant.
- **The JPEG encode** is now the largest part of taking a screenshot -- about 180ms
  of a 296ms capture through the slow path. Only worth touching if capture matters
  again.
- **`done` is filled by tools that never touch the screen**, so the turn after a
  `read` still waits for the screen to settle. Knowing which steps move the screen
  is the fix, and the safe direction is to assume they all do.
- **Audio is the whole output device.** Another application's notification chime
  reads as audio playing. Per-process attribution needs a tap on the stream.
- **The no-model path is English-only.** The verbs and the relational words are
  English strings. A language boundary, not an OS one, and it will follow to every
  platform.

---

## 9. What this document used to say

It began as a plan for a companion that pointed at buttons, and most of it was
about keeping up with a similar product. That product has since moved to a full
window with a sidebar and a roster, which is the shape everything else in the
category has. The framing is gone and so is the comparison.

It was rewritten again when the speed work finished and the accessibility tree
replaced guessing at pixels, because a plan that describes a solved problem as the
next one is worse than no plan.

This version is the first one written from a finished vision backwards, rather
than from the current code forwards.
