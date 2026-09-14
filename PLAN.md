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

> This document was rewritten once the shape of the project changed. It began as
> a companion that pointed at buttons. Most of what it used to say was about
> keeping up with a similar product, and that framing is gone -- see §7.

---

## 1. What it is

Three surfaces, and a rule about which to reach for.

| Surface | What it is for | Cost |
|---|---|---|
| **Facts** | What macOS already knows: frontmost app, window title, whether audio is playing | free |
| **Tools** | Commands, files, the web. No screen involved | one call |
| **Screen** | Clicking, typing, keys. For things with no other handle | ~2.4s and a model guessing at pixels |

**The rule, and the one idea this project keeps rediscovering:** use the cheapest
surface that can actually answer. Prefer the shortcut to the menu, the command to
the click, the data to the picture of the data. Every tool added here should make
Nudge reach for the screen *less*, and the measure of a good one is how much it
shrinks that set.

The screen is not going away -- some things genuinely have no other handle, and
nothing else can drive an app that offers no API. But it is the fallback, not the
default.

### What exists

Eighteen outcomes the model can choose between:

- **Screen** -- `point` (click, double, hover), `press`, `type`, `launch`, `open`
- **Files** -- `read`, `write`, `edit`, `show`
- **World** -- `run`, `fetch`, `search`
- **Coordination** -- `plan`, `task`, `agent`, `question`, `workspace`,
  `done`/`unsure`/`reply`

Plus: voice in and out, a notch dock, an agent card with its plan, its commands
and the files it made, a privacy guard that refuses to photograph password
managers, and a workspace boundary that nothing reaches past.

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

### 2.2 What makes it defensible

Not features. Three things a terminal-bound agent structurally cannot do:

- **It can see your screen.** Half of what people want is in an app with no API.
- **You talk to it.** No window to find, no prompt to type, no context to paste.
- **It is local and open.** Your screenshots never have to leave the machine, and
  anyone can check that claim. For anyone under an NDA, in healthcare or in
  finance, "where do the screenshots go?" is a hard blocker a closed binary
  cannot answer.

The last one is worth more than it sounds. It is also the only one a funded
competitor cannot copy without cannibalising themselves.

### 2.3 What would make it fail

Written down so it can be checked rather than discovered.

- **Grounding is not good enough.** If the screen surface misses often, the
  orchestration story leans entirely on CLIs, and then Nudge is a voice frontend
  to a terminal -- a thinner product. §5 is how we find out.
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
6. **Memory.** Per-app notes, written from failure, injected only when that app is
   in front: *"CapCut: the timeline view means a project is open."* Earned once
   the loop is known to work -- memory that records a broken loop's habits is
   worse than none.
7. **MCP.** The extensibility story, and the right answer for *other people's*
   tools rather than ours. Last, because a plugin surface over a tool set that
   moved this much would lock in shapes still in motion.
8. **Signing and notarisation**, before anyone else can run it.
9. **Windows, when there are Windows users.** The platform seam is drawn and the
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
