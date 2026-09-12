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

### 3.3 No background processes

`run` blocks for twenty seconds and returns. Nothing can start a dev server, tail
a log, or drive a long-running agent. Niche until the moment you ask for
something that needs `npm run dev`, and then it is the whole story -- see §2.1.

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

### 5.1 The accuracy number — **blocking**

The harness exists: `make record GOAL="..."` captures the screen and takes the
right answer from where you click; `make bench` scores every case and reports hit
rate and latency.

Ten cases is enough for a signal. Spread them: a menu-bar icon, a small toolbar
button, a row in a dense settings pane, browser chrome, a dialog button, one dark
app and one light one.

**What it decides:**

| Hit rate | Then |
|---|---|
| above 85% | the screen surface is sound; build §2.1 on top of it |
| 70--85% | the failures will cluster; fix the top cluster and re-run. Three rounds, then treat it as the row below |
| below 70% | the screen is a fallback, not a surface. Lean on tools and CLIs, and keep clicking for the cases with no other handle |

It also re-runs forever, which is the real point: change the model or the prompt,
run it again, and the two numbers are comparable because the screenshots did not
move.

### 5.2 Two-display verification

Section F of `results.md`. Needs a second screen -- Sidecar or a virtual display
will do, and one of the passes should put it **left of** or **above** the main
one, because that is where the negative origins are.

### 5.3 Then, in order

1. **Background processes.** The blocker for §2.1, and the last real gap in the
   tool set.
2. **Orchestrating an external agent.** Hand a job to whatever is installed, with
   its own boundary and its whole output visible.
3. **Memory.** Per-app notes, written from failure, injected only when that app is
   in front: *"CapCut: the timeline view means a project is open."* Earned once
   the loop is known to work -- memory that records a broken loop's habits is
   worse than none.
4. **MCP.** The extensibility story, and the right answer for *other people's*
   tools rather than ours. Last, because a plugin surface over a tool set that
   moved this much would lock in shapes still in motion.
5. **Signing and notarisation**, before anyone else can run it.

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

The running log of what each test run taught is in [VALIDATE.md](VALIDATE.md),
and the test passes themselves are in [results.md](results.md).
