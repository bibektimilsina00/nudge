# Nudge

**The layer everything else sits under.**

You do not open a terminal, find the right agent, and type at it. You say what you
want, out loud, to something that can already see your screen — and it works out
whether that is a click, a command, a page to read, or a job to hand to a
specialist.

Coding agents will keep getting better. That is fine: Nudge is not trying to beat
them at writing code. It is trying to be the thing you talk to, which then talks
to them. One voice, one cursor, one place that knows what is on your screen and
what you asked for ten seconds ago.

> Rewritten twice. It began as a companion that pointed at buttons, and most of
> what it used to say was about keeping up with a similar product -- that framing
> is gone, see §7. Rewritten again once the speed work finished and the
> accessibility tree replaced guessing at pixels, because a plan that describes a
> solved problem as the next one is worse than no plan.

---

## 1. What it is

Four surfaces, and a rule about which to reach for. Every cost below is measured,
not estimated -- see [FINDINGS.md](FINDINGS.md).

| Surface | What it is for | Cost |
|---|---|---|
| **Facts** | What macOS already knows: frontmost app, window title, whether audio is playing | 50ms |
| **Tree** | What macOS says is *on screen*: every control, its name, its exact rectangle | 200ms |
| **Tools** | Commands, files, the web. No screen involved | one call |
| **Screen** | A picture, for anything the tree does not expose | 61ms to take, ~6s for the model to read, right about two thirds of the time |

**The rule, and the one idea this project keeps rediscovering:** use the cheapest
surface that can actually answer. Prefer the shortcut to the menu, the command to
the click, the data to the picture of the data. Every tool added here should make
Nudge reach for the screen *less*, and the measure of a good one is how much it
shrinks that set.

**The tree is new and it moved the floor.** Before it, finding a button meant a
model guessing at pixels: several seconds, and wrong about a third of the time,
with the misses landing 12 to 87 pixels out. The tree answers *where the Send
button is* in 200ms, exactly, for no tokens -- so the model now chooses from a
numbered list rather than aiming, and when a request names one control
unambiguously it never reaches a model at all. *"Click the View menu"* is 0.3
seconds end to end.

The picture is not going away. Canvases, games, video and custom-drawn interfaces
expose nothing, and a closed menu has no geometry until it opens. But it is now
the fallback under a fallback rather than the way things get done.

### What exists

Twenty-one outcomes the model can choose between:

- **Screen** -- `point` (by control name or by pixel; click, double, hover),
  `press`, `type`, `launch`, `open`
- **Files** -- `read`, `write`, `edit`, `show`
- **World** -- `run`, `start`, `output`, `kill`, `fetch`, `search`
- **Coordination** -- `plan`, `task`, `agent`, `question`, `workspace`,
  `done`/`unsure`/`reply`

Plus: voice in and out with transcription on the machine, a notch dock, an agent
card with its plan and its commands and the files it made, a privacy guard that
refuses to photograph password managers, and a workspace boundary that nothing
reaches past.

**And two instruments, because the numbers above had to come from somewhere:**

- `make picks` -- does it choose the control you meant? Offline, instant, fifteen
  cases. Reports wrong picks separately from fall-throughs and never averages
  them, because one costs four seconds and the other clicks the wrong thing.
- `make bench` -- can a model find a control in a picture? Ten saved screenshots
  with recorded answers. Good for latency, too small for accuracy, and it measures
  the surface the tree replaced -- kept for the cases that still need it.

Plus `cargo run --example captime` for what looking at the screen costs, and a
timing line on every turn naming each stage.

---

## 2. Where it is going

### 2.1 Orchestration

Nudge already delegates: `task` hands a scoped job to a headless subagent that
cannot touch the cursor, which is why several can run at once. The next step is
that a subagent need not be *ours*.

Asked to refactor something large, the right move is not for Nudge to write the
code. It is to hand the job to whatever coding agent is installed, watch it work,
and report back -- the same shape as `task`, with a different worker.

What that needs:

- **Running a long process and reading its output.** `run` is read-only and
  blocks for twenty seconds; an agent CLI runs for minutes and streams. This is
  the single largest missing capability, and everything else in this section
  waits on it.
- **A different permission for a different worker.** Our shell allow-list exists
  because a model asked to delete something tried `rm`. An external agent *will*
  write files and make commits -- that is what it is for. It needs its own
  boundary: a workspace it may change, a confirmation before anything leaves the
  machine, and the whole of its output visible afterwards.
- **Knowing what is installed.** The same problem as applications, one layer up.

### 2.2 Where this sits

Worth writing down, because the field moved twice while this was being built and
both moves were *away* from where Nudge stands.

**OpenClaw** is the local agent now -- a quarter of a million stars inside two
months, its author gone to OpenAI to run personal agents, the project handed to a
foundation. It reads and writes files, runs shell commands, browses, sends email,
manages a calendar, and reaches you through WhatsApp or Telegram or Slack. It is
very good and it is free.

**It cannot see your screen, and it cannot drive one.** That is not a feature it
has not got round to; it is a different architecture. Everything it does goes
through an API, a file, or a shell. Half of what people actually want to automate
lives in an application with none of those.

**HeyClicky** was in this space and left it. It was a small thing in the menu bar
that listened and clicked; it is now a full window with a sidebar, a roster of
named assistants and a greeting. That is the same shape as every other agent
product, and it competes with Claude Desktop rather than with this.

So the position is narrow and currently empty: **the agent that works the
graphical interface, by voice, without taking over the screen.**

### 2.3 What makes it defensible

Not features. Four things, and the fourth is new:

- **It can see your screen and drive it.** The thing a terminal-bound agent
  structurally cannot do, and the thing the biggest player in the category has
  chosen not to build.
- **It asks the system rather than guessing.** The accessibility tree gives exact
  rectangles for named controls in 200ms. A competitor bolting screen control onto
  a shell agent would start with screenshots and a vision model, which is where
  this project started and spent a day climbing out of.
- **You talk to it, and it never takes the screen.** No window to find, no prompt
  to type, nothing to paste. It borrows the pointer for 150ms and gives it back.
- **It is local and open.** Transcription happens on the machine and never leaves
  it; screenshots do not either, and anyone can check that claim. For anyone under
  an NDA, in healthcare or in finance, "where do the screenshots go?" is a hard
  blocker a closed binary cannot answer.

The last two are the ones a funded competitor cannot copy without cannibalising
something -- a window app cannot become ambient, and a hosted product cannot
become local.

### 2.4 Parity, then the screen on top

**The goal: everything a general local agent can do, as a subset of what Nudge
can do.** Not because the list is the product -- the screen is -- but because
"bounded subset, plus the screen" is a harder thing to choose than "everything
they do, plus the screen", and the second one is reachable.

Measured against the code rather than guessed, this is the actual gap:

| | Them | Here | What it needs |
|---|---|---|---|
| Shell | anything | 38 programs, read-only subcommands | a way to widen it on purpose |
| Files | anywhere | workspace only | granted paths, not a deleted boundary |
| HTTP | full client | `fetch`, GET only, no auth | a real request tool |
| Email · calendar · chat | native | nothing | MCP, almost certainly |
| Extensibility | plugins | nothing | MCP |
| Learned behaviour | skills | nothing | §5.3, memory |

**MCP does most of this, and that is the whole argument for doing it early.**
Email, calendar, Slack, Notion, GitHub, databases -- those servers exist and are
maintained by other people. Writing six integrations by hand buys six
integrations; speaking MCP buys the ones that exist now and the ones written next
year. It moves from item 7 in §5.3 to somewhere near the front.

**The part that needs design rather than deletion.** Every boundary here is a
scar. The shell is read-only because a model reached for `rm` to get around a
refusal. Files are workspace-bound because the first thing that wrote a file put
it in this repository's root. One agent owns the cursor because two of them sent
a voice note to a real person.

So parity cannot mean removing them. It means replacing a fixed answer with a
decision someone makes:

- **Widening is explicit and visible.** A setting, a per-task grant, a prompt --
  not a longer constant in the source.
- **The default stays where it is.** Someone who never opens settings keeps
  today's boundaries, which is the right default for a thing that listens all day.
- **What was granted is inspectable.** "It has full shell access" must be
  something you can see, not something you have to remember agreeing to.

That is a different and better answer than either extreme, and it is the only
version of parity worth having: the general agents are unbounded because nobody
has done this work, not because it is wrong.

### 2.5 What would make it fail

Written down so it can be checked rather than discovered.

- ~~**Grounding is not good enough.**~~ **Largely answered, and not by improving
  the guessing.** The tree gives exact rectangles for anything an application
  exposes, so the question shrank to *which* control rather than *where* it is.
  What is left is the gap: applications that expose nothing, where it is still a
  model looking at a picture and still right about two thirds of the time.
- **It confidently says things that are not true.** The new one, and the worst
  one, because it survives every test written so far. Asked when macOS 27 ships,
  on a machine running macOS 27, after a successful web search, it answered 2036.
  Neither harness can see that. A fast agent that is wrong is worse than a slow
  one that is right, and it is the only failure that costs trust rather than
  seconds.
- **Too many tools.** A longer menu makes worse choices, and that is measured,
  not theoretical: this project has watched a model open Messages instead of
  WhatsApp Web, launch Weather to read a number, and open Terminal to run a
  command that needs no terminal. Every addition has to earn its line.
- **The boundaries erode.** Every rule -- workspace, allow-list, one cursor, ask
  before replacing -- exists because something went wrong without it. "Controls
  everything" must mean *more* enforcement, not less. A voice note went to a real
  person the one time two agents shared a cursor.

---

## 3. What is genuinely broken

### 3.1 Multi-monitor — built, unverified

`capture::grab` takes the display under the pointer, `Shot` records its origin,
and the overlay spans the union of every screen. The arithmetic is tested,
including a display above-left with a negative origin.

Untested without the hardware: whether one window really spans two monitors at
that level. **Do not mark this done until it has run on two screens.**

### 3.2 Not distributable

Not notarised; `spctl` says rejected. The signing identity is pinned to one
machine's certificate. Nobody else can run this.

*Fix:* Developer ID signing and notarisation in CI, before any public link exists.

### 3.3 Background processes — **done**

`start`, `output` and `kill`. Something that does not finish -- a dev server, a
build, a watcher, another agent -- is launched and left running, and read from on
a later turn.

A different boundary from `run`, not a relaxed one. `run` is read-only because a
model tried `rm` to get around a refusal; this cannot be, because writing is what
a dev server is for. So the rule changes shape: a short list of what may be
*started*, rather than a promise about what it does once running. Same syntax
rules, same workspace, at most four at once.

Everything started is killed when the task ends. A process nobody is watching is
the whole risk of being able to start one, and a dev server still holding port
3000 tomorrow would be Nudge's fault.

---

## 4. The safety model

Not a section about being careful. A list of rules that are **enforced in Rust**,
each written after something went wrong.

| Rule | Where | Written because |
|---|---|---|
| Never photograph a password manager, or a window whose title names a secret | `core/screen/privacy.rs` | a screenshot of an open `.env` went to a third party during development |
| Commands are read-only, allow-listed by program, checked per pipeline stage, with shell syntax that chains or redirects refused outright | `core/tools/shell.rs` | asked to replace a file, a model tried `rm` to get around the refusal |
| Files only inside the workspace; secret-looking paths refused even there | `core/tools/files.rs` | `../../.ssh/authorized_keys` is a path a model can produce |
| Creating a file is free; replacing one asks, and keeps a copy | `core/tools/files.rs` | permission is not safety -- people say yes to things they misunderstood |
| One agent owns the cursor; a second is refused, out loud | `core/run/agent.rs` | two agents drove one WhatsApp chat and sent a voice note to a real person |
| Escape stops everything, checked again immediately before each action | `app/input/cursor.rs` | a stop that waits for the current model call is not a stop |

**The principle:** anything that cannot be undone is guarded in code, not in the
prompt. Prompt rules were ignored often enough -- three times in one afternoon --
to stop trusting them with consequences.

---

## 5. What to do next

Two things are outstanding. Everything else is a choice rather than a blocker.

### 5.1 The accuracy number — **the question changed**

The screenshot bench measured whether a click lands on the control the model
named. That was the right question when finding a button meant guessing at
pixels, and it mostly is not any more: the system reports where the controls are,
and the job is choosing the right one from a list.

Two things were learned the hard way and both are now written down properly in
[FINDINGS.md](FINDINGS.md):

**Ten cases cannot measure accuracy.** One hit is thirteen points. Two identical
runs of the same model scored 50% and 71%. The 29%-against-50% comparison that
picked the current model is a coin that landed the same way twice. Latency it
measures well -- six runs at each setting, cleanly separated, no overlap.

**So there is a second harness now**, and it is cheap in the way the first never
was. `make picks` scores a control list, a goal, and the label that should win --
no screenshot, no network, milliseconds to run, a sentence to record. Fifteen
cases, and the awkward ones ("click the thing next to View") are easier to write
by hand than to stage in front of a real application.

It reports two failures and never averages them, because they are not the same
kind of mistake:

| | |
|---|---|
| **fell through** | the model handles it. Four seconds slower, right answer |
| **WRONG** | clicked something nobody asked for, with nothing watching |

Only WRONG exits non-zero. That distinction found a live bug within an hour of
existing: "click the thing next to View" was clicking View.

**What neither harness can see:** whether the answer is *true*. The same log that
proved the latency work also has the agent reporting that macOS 27 ships in 2036,
on a machine running macOS 27, after a successful web search. Nothing here would
catch that, and it is the failure that costs trust rather than seconds.

### 5.2 Two-display verification

Section F of `results.md`. Needs a second screen -- Sidecar or a virtual display
will do, and one of the passes should put it **left of** or **above** the main
one, because that is where the negative origins are.

### 5.3 Then, in order

Latency came off this list today. A turn is 4.5 seconds and everything that is
not the model accounts for 0.22 of it -- see [SPEED.md](SPEED.md) for what was
tried, what worked, and the three ideas that were measured and abandoned.

1. ~~**Background processes.**~~ **Done** -- see §3.3.
2. ~~**Latency.**~~ **Done.** Overhead 6.5s to 0.22s. The screenshot went from
   1769ms to 61, transcription from 1300ms to 190 and off the network entirely,
   and "click the View menu" now takes 0.3 seconds because it never reaches the
   model at all.
3. **Whether the answers are true.** The one genuinely unmeasured thing, and the
   only kind of wrong that costs trust rather than time. It needs a harness that
   scores *answers* rather than clicks, which neither existing one does.
4. **Widen the no-model path.** Every phrasing it learns is another turn that
   costs 0.3s instead of 4.5. Cheap, compounding, and now safe to do because
   `picks` catches a wrong match the moment it appears. Typing into a named
   field and launching apps by name are the next two.
5. **Orchestrating an external agent — built, not yet proven.**

   Nudge finds which coding agents are on the machine, knows the name people
   call each one by, and knows how to run it unattended. The job goes through
   `start`, so it runs in the same workspace, its output is read a piece at a
   time, its exit code is reported, and it is killed when the task ends.

   Verified by execution rather than memory, which mattered: two of the three
   invocations written from memory were wrong, and both failed in the way that
   looks like nothing happening. `claude -p` alone stops dead the first time it
   wants to edit a file, and `agy -p --mode ...` swallows the next flag as its
   prompt, runs, exits zero and does nothing that was asked.

   **What is left, and why it is parked rather than finished:**

   - **No end-to-end run yet.** Section I of `results.md` has the four tests. The
     one that matters most is I4 -- whether it knows *not* to delegate a one-line
     change, which is the failure nobody notices because it still works.
   - **`opencode` and `aider` are unverified.** Not installed here, so their forms
     come from documentation. Marked as such in the code.
   - **Nothing reads the diff.** The agent reports what it did and Nudge believes
     it. `git diff` is one `run` away and would turn a claim into a check.
6. **MCP, for parity.** Promoted from last place. It was going to be the
   extensibility story once the tool set settled; it is now the cheapest route to
   §2.4, because email, calendar, Slack, GitHub and the rest already exist as
   servers someone else maintains. Six integrations written by hand buy six
   integrations. Speaking MCP buys every one that exists and every one written
   next year.

   The original reason for deferring it still stands and is worth holding to: a
   plugin surface over a tool set that is still moving locks in shapes that should
   not be locked. So the internal tools want to settle first -- but "settle" is
   now weeks away rather than a phase.

7. **The boundaries, made adjustable.** The shell allow-list, the workspace, the
   GET-only fetch. Not removed -- see §2.4 for why every one of them is a scar --
   but turned from a constant in the source into something a person can widen on
   purpose, visibly, with today's behaviour as the default.

8. **Memory.** Per-app notes, written from failure, injected only when that app is
   in front: *"CapCut: the timeline view means a project is open."* Earned once
   the loop is known to work -- memory that records a broken loop's habits is
   worse than none.
9. **Signing and notarisation**, before anyone else can run it.
10. **Windows, when there are Windows users.** The platform seam is drawn and the
   other side of it is written -- `xcap`, `enigo`, `device_query`,
   `active-win-pos-rs`, and a UI Automation tree -- but none of it has ever been
   compiled for the target, because it cannot be from a Mac. See
   [PORTING.md](PORTING.md) for what is done, what is missing, and which calls to
   suspect first. Expect a very different latency profile: a capture is 61ms here
   through ScreenCaptureKit and a hardware encoder, and about 2100ms through the
   portable path.


### Deliberately not doing

- `glob` and `grep` as separate tools -- `run` covers both, and three ways to do
  one thing is worse than one.
- `apply_patch` -- exists for git workflows across a repo; `edit` covers the need.
- Deleting files. Small value, no undo, and `.nudge-backups` cannot save a file
  that is gone. If it ever arrives it moves things to the Trash.
- Writing outside the workspace. That boundary is what makes the rest acceptable.

---

## 6. Debt marked in the code

Every deliberate shortcut carries a `ponytail:` comment naming its ceiling and the
upgrade path. `grep -rn "ponytail:" src-tauri/src` is the live ledger.

| Where | Ceiling | Upgrade |
|---|---|---|
| `core/screen/capture.rs` | deprecated `CGWindowListCreateImageFromArray` | ScreenCaptureKit, when Apple removes it |
| `core/voice/transcribe.rs` | speech-to-text over the network | `SFSpeechRecognizer`, on-device -- also removes a round trip |
| `core/screen/facts.rs` | audio is device-wide, so another app's chime reads as playing | per-process attribution needs an audio tap, a permission prompt and a thread |
| `app/input/cursor.rs` | 60Hz cursor and button poll | drop to 30Hz if battery complains |
| `core/screen/launch.rs` | flat scan of application directories | `mdfind` if apps are missed |
| `core/run/session.rs` | one session, so a subagent carries its own history instead | a real multi-session store, when something needs two at once |

---

## 7. What this document used to say

It was written to compare Nudge against a similar product, feature by feature.
That framing is gone, for two reasons.

**Feature parity is the losing side of every fight.** They ship faster in their
own codebase, they have users telling them what is broken, and their audience is
not clonable. "Same but mine" is not a reason for anyone to switch.

**And the interesting part turned out to be elsewhere.** The things that actually
moved this project were not on anyone's feature list: preferring a keyboard
shortcut to a menu because menus cannot be clicked reliably; resolving key codes
through the layout in use because not every keyboard is QWERTY; waiting for the
screen to stop moving instead of guessing at a delay; asking the system whether
audio is playing instead of asking a model to look at a still frame.

None of those are features. They are the difference between something that works
and something that demos.

The running log of what each test run taught is in [FINDINGS.md](FINDINGS.md),
and the test passes themselves are in [results.md](results.md).
