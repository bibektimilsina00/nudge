# Nudge — what is left to build

The last plan carried six phases and most of them are built. What replaced it is
this: the work that is actually ahead, in the order it should happen, with the
reason for each.

Two things decide that order. The first is a bug this project has had on record
for a while and has never fixed. The second is that nobody outside this machine
can run the app at all.

---

## Beyond this plan

[FEATURES.md](FEATURES.md) is the full inventory between here and *"gets your
everyday tasks done"* — every feature, what exists, and what each would cost.
This plan is the near work; that is where the next one comes from.

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

### 1.1 A decision instead of a bool — *done*

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

*Built.* `permits()` replaces `may()` at all eight call sites and returns a
`Decision` carrying `Answer::{Allow, Ask, Deny}`, a reason, and which grant
decided. Nothing produces `Ask` yet — that arrives with 1.2, which is the thing
able to act on one.

The reason is not decoration: `Grant::denied()` is the single sentence for a
missing grant, and the shell's refusal now composes it rather than writing its
own. There had been two copies of that sentence and they had already drifted to
naming different places to change the setting.

### 1.2 One approval path, not one per thing — *done*

There is already a working approval flow for a single case: replacing a file
asks, remembers the answer per path, and resumes the turn. It is hard-wired to
`(PathBuf, String)`.

Generalise it to *any* pending action, so that a `needs_user` decision from any
gate routes to the same place: the same card, the same spoken question, the same
resume.

*Built.* `Pending` is a value with a question attached, the slot holds one of
those rather than a `(PathBuf, String)`, and `put_to_the_person` is the single
path. What "yes" *means* is decided in one place per kind, and the kinds do not
know about each other.

Also brought forward from 1.3, because an ask nobody remembers answering is an
ask repeated on every page: `Reach::allow_host` remembers a host for the run,
matched exactly — a suffix rule would let `evil-example.com` through on the
strength of `example.com`, so a subdomain is its own decision.

### 1.3 Grants that expire — *done*

An approval now offers **Just now / This session / Always**, shortest first, and
the first of those is dropped when the task ends — however it ends, including
stopped by Escape. A bare "yes" means the narrowest of the three, for the same
reason `is_yes` fails closed: being read as too narrow costs one more question,
and the other mistake costs a standing grant nobody chose.

The buttons send the same words the field accepts, so a decision made by tapping,
typing or saying it out loud goes down one path and means one thing. Only
"Always" is written to disk, in `~/.config/nudge/allowed.toml`.

### 1.4 Egress asks instead of guessing — *done*

With 1.1–1.3 in place, the real fix for `Grant::Http` becomes possible, and the
current shape of it is wrong on two counts:

- it gates the **method**, and a GET is the exfiltration channel
- there is no allow-list, so every host is equal

Replace it with a host allow-list that **asks** for anything new. Refusing
outright would break the ordinary case; asking preserves it. Remember the answer
at session or run scope per 1.3.

The machinery is in place — `Pending::Reach`, the host memory, the single ask
path. What is not decided is **when to ask**, and it is a product question rather
than a coding one:

- Asking on every new host breaks the thing fetch exists for. It is a background
  optimisation — *"prefer the data to the picture of the data"* — and a question
  in front of every one of those is worse than the screenshot it replaced.
- Foreground fetches are the person's own question, seconds after they asked it.
  There is nothing to protect them from and often nobody to ask, since
  `put_to_the_person` needs a card to put it on.
- **Agent runs are the opposite on both counts.** Nobody is watching, the URL may
  have come from something on screen rather than from the person, and there is
  always a card.

So the rule is: **ask when an agent is running and the host is new; leave the
foreground alone.** That is where the danger is and where the question can
actually be answered.

*Built.* Asked per host and remembered for the run, so a page of results does not
ask once per page. The host comes from the same parser the refusal uses — there
were nearly two, and two URL parsers is how `https://github.com@evil.example/`
comes to be refused in one place and read as GitHub in the other.

Not covered, and worth knowing: **search is not gated this way.** It goes to one
fixed provider rather than a host the model chose, so there is no host to ask
about — but the query is still data leaving the machine, and a query is a place
something could be hidden. That is a smaller hole than the one just closed and it
is still a hole.

---

## 2. What it did, recorded

The old plan set the test and never met it: *"can the person see afterwards what
was done with it?"* Today the answer is no. An agent reports what it did and
Nudge believes it.

### 2.1 An audit trail that cannot itself become the leak — *done*

Every tool call, every grant used, every refusal — appended to a local SQLite
log. OpenWorker's `audit.py` and OpenExecutive's `audit/` are each about a
hundred lines and do exactly this.

The part to get right is not the table, it is what goes in it. Both projects
learned the same thing and OpenExecutive says it plainly: tool inputs and outputs
carry tokens, cookies, mail bodies and file contents, and *"persisting them
verbatim into the audit table would turn the audit log itself into a leak
vector"*. **Log the call, its shape and its outcome; never its contents.** Their
clips are 140 characters of input and 300 of result — enough to recognise a call,
not enough to carry a document.

Two details worth taking with it:

- **`summary` and `full` are different columns.** The list view stays small; the
  untruncated payload is fetched only when somebody opens one row.
- **WAL and a busy timeout**, because audit writes swallow their exceptions and
  *"silent loss under contention would be undetectable"*. A log that quietly
  drops rows is worse than no log, because it is trusted.

This is also what makes §1 legible: a permission system whose decisions vanish is
one nobody can check.

**Done.** The card shows a run's refusals and questions from the log rather than
from the model's account of itself, open by default because a refusal is the one
thing here somebody actually needs to see. Verified live: asked to clear a
folder, a model reached for `find -delete`, then `rm -rf *` twice; all three were
refused and all three are on the card. A fetch records the host and how much came
back — never the page — and a tool call records the path and byte count, never a
word of the file.

### 2.2 Read the diff — *done*

A write now reports `git diff --numstat` rather than its own intention: *"Edited
main.rs (+12 -3)"*. Silent outside a repository, where there is nothing to check
against. The write and edit paths were near-identical copies and neither wrote to
the audit at all — they go through one place now, so the log that answers *what
did it do to my files* has file changes in it.

A tool server is a second way to write a file, and live testing found it was the
one with no record at all: a `files/write_file` call was ungated, undiffed and
unlogged, and because its argument was `notes.txt` rather than an absolute path,
no copy was kept either — the backup only looked at absolute paths. Relative
arguments now resolve against the workspace (and are refused if they climb out of
it), so an MCP write gets the same copy, the same diff and the same log entry as
Nudge's own.

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

### 3.2 Auto-update — *done, bar one click*

Shipped before the certificate deliberately: there is no way to update somebody
into having an updater, so whoever downloads the first build without one is
pinned to it for good.

The check is quiet, automatic and twenty seconds after launch; the install is a
button that only appears when there is something to install. Nothing replaces
its own binary while somebody is mid-sentence. Both ends compare versions — the
server so it cannot offer a downgrade to everybody at once, the app so a wrong
answer is still refused by the thing installing it. Every answer that is not a
genuine newer build is a 204, which is the updater's contract rather than a
choice.

Two artifacts per release: the `.dmg` somebody downloads once, and the
`.app.tar.gz` plus minisign signature the updater installs — the one signature
Apple's notarisation does not cover. A test holds the shipped public key against
a signature from the real signing key, because a rotation that updates one and
not the other breaks installs silently and strands everybody on that version.

**Verified**: a 0.1.0 build pointed at a locally served 0.2.0 manifest noticed
it. **Not verified**: the install itself, which needs a click on a screen.

### 3.3 Discovery — *started*

An interface that shows nothing teaches nothing, and nobody guesses that the
thing in the notch can refactor a repository. The answer belongs in the voice
loop — *"what can you do?"* answered well — rather than in a menu nobody opens.

The first half is done: approaching the notch now says **Hold ⌃ to ask**, which
is the one thing somebody has to know before anything else is reachable. See
§3.4. The voice answer is still open.

### 3.4 The resting state — *in progress*

The pill and the cat are what is on screen 99% of the time and have had the least
attention of anything here. If the product is the shape, the shape is the work.

The strip now says what it is for while somebody is on their way to it, on a
wider ring than the dock's — a hint cannot be a delay before opening, because a
dock that hesitates feels broken. It sits *inside* the strip: growing the strip
to make room pushed the companion out past the hardware onto the menu bar and
left a slab of nothing where the width had gone.

Still open: the companion is dark on black at 28px, so at rest what reads is two
eyes and a rim. It lives in the notch deliberately, but ignorable and invisible
are not the same thing.

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

- **One unanswerable question, not four.** The truth suite scored 41/48 with four
  wrong; reading the answers showed two of those four were correct refusals the
  harness could not recognise, and a third was borderline. The hedge list is
  wider now. What is left is 020-bitcoin, which answered *"between $80,000 and
  $98,000"* with no admission anywhere — one real failure, and worth fixing.
  See [FINDINGS.md](FINDINGS.md).
- **The bench is at 57%** (4/7 pointed at, 3 misses, one of them 8px). First
  measurement since thinking moved to `low`, and it is owed a controlled re-run
  against the previous setting before anybody concludes anything from it.
- **Widen the no-model path.** Every phrasing it learns is a turn that costs 0.3s
  instead of 4.5. Typing into a named field and launching apps by name are next.
- **Ablation: does a feature earn its keep?** Run a suite twice with one thing
  switched off and print the per-case delta. Both existing suites are already one
  arm of it, and four features have never been measured this way: per-application
  memory, the early look, skills, and the thinking level. OpenExecutive's
  `ablation.py` is the pattern, and its two rules matter more than its code — the
  "off" arm must be the production path with one knob at zero rather than a
  test-only branch, and the docstring must say what a run costs, because a
  harness that quietly doubles a bill gets run once by accident.
  See [OPENEXECUTIVE.md](OPENEXECUTIVE.md).

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
