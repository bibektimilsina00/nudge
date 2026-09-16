# Nudge — what is left to build

The last plan carried six phases and most of them are built. What replaced it is
this: the work that is actually ahead, in the order it should happen, with the
reason for each.

Two things decide that order. The first is a bug this project has had on record
for a while and has never fixed. The second is that nobody outside this machine
can run the app at all.

---

## Where this actually is

Built and working: the screen loop, agents that finish a task, MCP servers,
skills, memory, the notch interface, settings, bug reports, a marketing site
serving its own downloads, and CI/CD that deploys only what changed.

Not working: **the app cannot be opened by anybody else.** `spctl` says
`rejected`. Everything below is theoretical until that changes, which is why
§3 is where the deadline is even though §1 is where the danger is.

Measurements live in [FINDINGS.md](FINDINGS.md) and [SPEED.md](SPEED.md); neither
is a plan and both are still true. The reading of Andrew Ng's OpenWorker that
most of §1 comes from is in [OPENWORKER.md](OPENWORKER.md).

---

## The through-line: the screen is untrusted input

Nudge looks at a screenshot and acts on what it sees. Everything in that
screenshot was written by somebody else — a web page, an email, a terminal
someone else's program is printing to. There is no boundary in the product
between *what the user asked for* and *what the screen says*, and that is the
whole of the risk.

It has already happened three times: Nudge read a suggestion off a Claude Code
transcript on screen and acted on it as though it were an instruction.

OpenWorker's answer is one sentence worth keeping in view for all of §1:

> The attacker can address the agent, never the judge.

Two floors went in already — the shell no longer lets an argument turn a reader
into a writer, and a URL carrying this machine's API key never leaves. Both are
floors, not answers. The answer is §1.

---

## 1. The gate says three things

Nudge's permission model is `may(app, Grant) -> bool`, read at each gate. Three
grants, each on or off, decided once in advance by somebody who cannot know what
will be asked of it. There is no way to say *ask me*.

That single missing answer is why the rest of this section cannot be built.

### 1.1 A decision instead of a bool

```rust
pub struct Decision {
    pub allowed: bool,
    pub reason: String,      // always; a refusal with no reason is a dead end
    pub needs_user: bool,    // pause and ask
    pub rule: Option<String> // which grant allowed it, for the record
}
```

Replaces `may()`. Every gate returns one. The interface can then show *why*
uniformly instead of each call site inventing a sentence.

**Done when** every existing gate returns a `Decision`, the behaviour is
unchanged, and the tests say so.

### 1.2 One approval path, not one per thing

There is already a working approval flow for a single case: replacing a file
asks, remembers the answer per path, and resumes the turn. It is hard-wired to
`(PathBuf, String)`.

Generalise it to *any* pending action, so that a `needs_user` decision from any
gate routes to the same place: the same card, the same spoken question, the same
resume.

**Done when** a file replacement and an egress request use the same code path,
and neither knows about the other.

### 1.3 Grants that expire

Today a granted path is remembered forever and a grant is remembered until quit.
OpenWorker has three scopes and the middle one is the useful one:

| Scope | Lasts |
|---|---|
| config | until the config changes |
| session | until the app closes |
| **run** | until this task ends |

The run scope exists because one approval should cover a paginated loop without
becoming a permanent grant. Nudge has exactly that problem: an agent fetching
six pages asks six times, and the answer people give to that is "always".

**Done when** an approval offers "just now / this session / always", and the
first of those is gone when the task is.

### 1.4 Egress asks instead of guessing

With 1.1–1.3 in place, the real fix for `Grant::Http` becomes possible, and the
current shape of it is wrong on two counts:

- it gates the **method**, and a GET is the exfiltration channel
- there is no allow-list, so every host is equal

Replace it with a host allow-list that **asks** for anything new. Refusing
outright would break the ordinary case; asking preserves it. Remember the answer
at session or run scope per 1.3.

**Done when** fetching a page the user named still works first time, and a host
the model invented produces a question rather than a request.

---

## 2. What it did, recorded

The old plan set the test and never met it: *"can the person see afterwards what
was done with it?"* Today the answer is no. An agent reports what it did and
Nudge believes it.

### 2.1 An audit trail

Every tool call, every grant used, every refusal — appended to a local SQLite
log, with secrets stripped on the way in. OpenWorker's `audit.py` is ~100 lines
and does exactly this.

This is also what makes §1 legible: a permission system whose decisions vanish is
one nobody can check.

**Done when** the agent card can show what a finished run actually did, from the
log rather than from the model's account of itself.

### 2.2 Read the diff

An agent says it edited a file. `git diff` is one call away and would turn that
claim into a check. Nothing does it.

**Done when** a run that touched tracked files reports what changed, not what it
intended.

---

## 3. Somebody else can run it

This is the one with a deadline, because nothing above matters to anybody who
cannot open the app.

### 3.1 The certificate — blocked on a person

The pipeline is built and tested: `scripts/release.sh` signs, notarises, staples
and then asks Gatekeeper the question another Mac will ask. It needs a **Developer
ID Application** certificate, which only the Account Holder of the Nuddg Inc team
can create. The CSR is generated and waiting in `.signing/`. See
[RELEASING.md](RELEASING.md).

Until then `make share` packs a build with instructions for clearing quarantine,
which is fine between people who know each other and is not shipping.

### 3.2 Auto-update

Every person who downloads today is pinned to that build for ever. OpenWorker
ships the Tauri updater plugin with a minisign key and two endpoints — their own
CDN first, the GitHub release second — and a script that writes `latest.json`.

The download API already exists and already knows the current version, so it is
most of the endpoint already.

**Done when** a running app notices a newer build and can take it.

### 3.3 Discovery

An interface that shows nothing teaches nothing, and nobody guesses that the
thing in the notch can refactor a repository. The answer belongs in the voice
loop — *"what can you do?"* answered well — rather than in a menu nobody opens.

The first real blocker on the second user, once there is a second user.

### 3.4 The resting state

The pill and the cat are what is on screen 99% of the time and have had the least
attention of anything here. If the product is the shape, the shape is the work.

---

## 4. Elsewhere

**Windows.** The seam is drawn and the other side is written; nothing has been
compiled for the target, because it cannot be from a Mac. Expect a different
latency profile entirely — capture is 61ms here through ScreenCaptureKit and
about 2100ms through the portable path. [PORTING.md](PORTING.md) says which calls
to suspect first.

**Linux.** After Windows. Wayland and X11 handle capture and input injection
completely differently, so it is two ports wearing one name.

---

## Running alongside

Not phases, and not allowed to rot:

- **The four unanswerable questions.** `make truth` scored 41/48 with four wrong,
  and all four are one bug: asked something that cannot be known — a future
  price, next year's weather, what somebody had for breakfast — it answers
  anyway. Three other cases prove it *can* say it does not know. See
  [FINDINGS.md](FINDINGS.md); this is the highest-value fix in the list.
- **The bench is at 57%** (4/7 pointed at, 3 misses, one of them 8px). First
  measurement since thinking moved to `low`, and it is owed a controlled re-run
  against the previous setting before anybody concludes anything from it.
- **Widen the no-model path.** Every phrasing it learns is a turn that costs 0.3s
  instead of 4.5. Typing into a named field and launching apps by name are next.

---

## The safety model

Every rule here exists because something went wrong without it:

| Rule | What happened |
|---|---|
| Workspace boundary | The first thing that wrote a file put it in this repository's root |
| Read-only shell, by program | A model reached for `rm` to get around a refusal |
| ...and by argument | `python3 -c` and `find -exec` walked straight past that |
| No key in an outgoing URL | A GET is the exfiltration channel |
| One cursor, one agent | Two agents shared a WhatsApp chat and sent a voice note to a real person |
| Ask before replacing | Self-evident, once |
| Privacy guard | A password manager is one frontmost window away at all times |
| Everything started is killed | A dev server still holding port 3000 tomorrow would be Nudge's fault |

**Widening what is possible must widen what is recorded by the same amount.**
The test for any new reach: can the person see afterwards what was done with it?
That is §2, and it is why §2 is not optional.
