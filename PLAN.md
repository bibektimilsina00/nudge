# Plan

The last plan is finished. Every section of it is done or blocked on somebody
else: the gate says three things, a yes lasts as long as was agreed, what was
done is written down and readable, and the app can update itself. What is left
there needs a certificate only the Nuddg Inc Account Holder can create, and a
Windows build that cannot be compiled from a Mac.

So this one is about the sentence Nudge is actually aiming at:

> AI that gets your everyday tasks done. It works in the tools you use every
> day, **carries tasks through from start to finish**, and **checks in before
> important actions**, so you stay in charge.

Checking in is largely built. Carrying a task through is not started at all — a
run lives inside one process and dies with it. That is the gap this plan closes.

[FEATURES.md](FEATURES.md) is the full inventory — every feature, what exists,
what done would mean. This is the near work drawn from it, in dependency order.

## Where this actually is

Built and working: the screen loop, agents that finish a task, MCP servers,
skills, memory, the notch interface, settings, bug reports, a marketing site
serving its own downloads, CI/CD that deploys only what changed, a permission
gate with scopes, an audit trail, and an updater.

Not working: **the app cannot be opened by anybody else.** `spctl` says
`rejected`, and that is a certificate, not code.

Measurements live in [FINDINGS.md](FINDINGS.md) and [SPEED.md](SPEED.md); neither
is a plan and both are still true. The reading of OpenWorker that this plan and
FEATURES.md come from is in [OPENWORKER.md](OPENWORKER.md).

---

## The through-line: the screen is untrusted input

Nudge looks at a screenshot and acts on what it sees. Everything in that
screenshot was written by somebody else — a web page, an email, a terminal
someone else's program is printing to. There is no boundary in the product
between *what the user asked for* and *what the screen says*, and that is the
whole of the risk.

It has happened four times now. Three were a suggestion read off a Claude Code
transcript and acted on as an instruction. The fourth was this month: a run read
its own goal text out of a terminal, decided the file had already been written,
and reported success having done nothing.

OpenWorker's answer is one sentence, and it is the spine of §2:

> The attacker can address the agent, never the judge.

Floors already in: the shell cannot be argued into writing, a version check
cannot be handed a program to run, a URL carrying this machine's API key never
leaves, and a write reports what `git` says changed rather than what it meant to
do. Floors, not an answer. The answer is §2.

---

## 1. Risk is a property, not a list of names

Today `Grant` names a *capability* — shell, files, http — and the gate asks which
one a step needs. That works because every tool is one this repository wrote.

It stops working at the first tool Nudge has never heard of. An MCP server
arrives with thirty tools and the gate has no way to know which of them write to
the outside world, so they are all treated alike. OpenWorker hit this and
replaced hardcoded `WRITE_TOOLS` / `SHELL_TOOL` name sets with a declared risk
class that one `classify` reads.

Small on its own, and §2 cannot be built without it: a judge that cannot tell a
read from a write has nothing to judge.

**Done when** a tool this repository has never seen arrives carrying a risk class,
and the gate decides on that rather than on its name.

---

## 2. The reviewer, and the invariant that makes it safe

A second model call judges **one proposed action** against what the user actually
asked for. Routine actions run; only genuinely questionable ones interrupt.

This is not a nicety. OpenWorker built it from measured pain — *~15
hand-approvals per run* in security scans — and Nudge meets the same wall the
moment agents get longer than forty steps. Asking about everything and asking
about nothing are both failures, and today Nudge can only do one or the other.

**The invariant is the whole feature.** The reviewer never reads untrusted
content. Its input is the instructions, the known world (folders and remotes,
never contents), the user's own messages, and the proposed action. Page text,
mail bodies, file contents and **screenshots** never reach it.

That last one is Nudge's version and it is not in OpenWorker, because OpenWorker
has no screen. Nudge's agent sees a screenshot every turn; the judge must not.
If the judge can see the screen, the attacker is addressing the judge.

**Done when** a long run interrupts a handful of times instead of forty, and
there is a test that fails if anything an attacker could have written reaches the
judge's prompt.

### 2.1 Provenance

The engine knows one thing neither the judge nor the person does: whether it
wrote or downloaded that file moments ago. One line of fixed vocabulary, never
file content.

Worth less than it was — running a script now needs an explicit grant — but it is
still the answer to *"you are about to run a file you wrote a moment ago"*.

**Done when** a command naming a file this run created says so, in the audit and
in the question.

---

## 3. Compaction: the ceiling on how long a task can be

`session.done` grows one line per step and every step sends the whole thing.
Nothing trims it. `MAX_STEPS = 40` is not a budget, it is a lid hiding the fact
that a task needing two hundred steps cannot be run at all.

The structure is the part to copy: pure functions plus one dataclass, with the
runtime owning *when* and *with what*, so the policy is testable without a
provider. Older turns become a summary plus mechanically extracted state; recent
turns and **every user message** survive. The stored transcript is never edited —
only what is sent.

**Done when** a run can exceed the context window without being cut off, and the
user's own words are still in the prompt at step two hundred.

---

## 4. Runs that survive the process

Quit Nudge mid-task and the run is gone. Finished runs persist to `history.json`
for display; in-flight ones are dropped on purpose, because claiming a run is
still going when the process running it has died would be a lie.

That honesty is right and the situation it describes is not. A task that cannot
survive a restart is a session, not a task — and "carries tasks through from
start to finish" is the promise this plan exists for.

Needs: the goal, the history, the plan, where it got to, and enough to pick the
thread back up. Plus the question the old behaviour was protecting — an
interrupted run must come back as *interrupted*, offering to continue, never as
though it had been running the whole time.

**Done when** quitting mid-task and reopening offers to carry on.

### 4.1 Self-wake

`sleep_until` for a timer, `wake_on` for a backgrounded job. An agent that can
wait costs nothing while it waits, and *"run the tests and tell me when they are
green"* stops holding a turn open for eight minutes.

**Done when** waiting does not burn a turn a second.

---

## 5. Connections

A tool server is three lines in `config.toml` today. Right for the protocol,
wrong for a person: no notion of an account, credentials in plaintext, no way to
see what is connected or to take it away.

**Not thirty-five hand-written connectors.** `mcp.rs` already makes the argument
in its own header — six integrations written by hand buy six integrations, and
speaking the protocol buys the ones written next year. What is missing is the
account model around it, and credentials in the Keychain rather than in a file
anybody can `cat`.

And the rule from OpenWorker's catalogue, which its test suite enforces: every
connectable thing states what access it gets **before** consent, in plain
statements of behaviour rather than marketing. Overclaiming there is a product
bug.

**Done when** connecting an account means picking it from a list, its token is
not readable with `cat`, and it can be disconnected.

---

## Waiting on somebody else

- **The certificate.** A Developer ID Application certificate, which only the
  Nuddg Inc Account Holder can create. The CSR is in `.signing/`, the pipeline is
  built and refuses to start without all six secrets, and the download page is
  deliberately empty until then. See [RELEASING.md](RELEASING.md).
- **One click.** The updater notices a newer build; nobody has watched it install
  one. Needs a person at the machine for about a minute.
- **Windows, then Linux.** Cannot be compiled from a Mac. See
  [PORTING.md](PORTING.md).

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
| ...and a version check is only a version check | `python3 x.py` ran a file with no grant at all — write a script, run it, never asked |
| No key in an outgoing URL | A GET is the exfiltration channel |
| One cursor, one agent | Two agents shared a WhatsApp chat and sent a voice note to a real person |
| Ask before replacing | Self-evident, once |
| Privacy guard | A password manager is one frontmost window away at all times |
| Everything started is killed | A dev server still holding port 3000 tomorrow would be Nudge's fault |

**Widening what is possible must widen what is recorded by the same amount.**
The test for any new reach: can the person see afterwards what was done with it?
That is §2, and it is why §2 is not optional.
