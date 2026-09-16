# Features — what "gets your everyday tasks done" costs

The sentence this is measured against, which is OpenWorker's and is the shape
Nudge is aiming at:

> **AI that gets your everyday tasks done.** Ask for an outcome, not just an
> answer. It works in the tools you use every day — your files, repos, Slack,
> calendar, and more — and carries tasks through from start to finish: your code
> reviewed for vulnerabilities with fixes ready to go, your cloud configuration
> checked, a polished document delivered, a Slack thread answered. **It checks in
> before important actions, so you stay in charge.**

Four promises, and they are not equally far away:

| Promise | Nudge today |
|---|---|
| **Checks in before important actions** | **largely done** — the gate, the scopes, the audit trail |
| **Ask for an outcome, not an answer** | partly — agents run to completion and produce artifacts |
| **Works in the tools you use every day** | the protocol is there, the plumbing is not |
| **Carries tasks through from start to finish** | not at all — a run dies with the process |

This document enumerates every feature between here and that sentence. It is not
a plan; [PLAN.md](PLAN.md) is the plan and it is deliberately short. This is the
inventory the plan draws from — the current plan takes C5, C6, C7, B2, B1, B3 and
A1–A4 from here, in that order.

## The one thing not to lose

OpenWorker reaches your tools through **35 hand-written connectors**, and it
cannot touch anything without an API. Nudge drives the screen and the cursor,
which means it can work in an app that has no API, no MCP server and no
integration — which is most software most people use.

**That is the moat, and every feature below is worth having only if it does not
cost it.** A Nudge that becomes a headless connector runtime has traded a rare
thing for a crowded one.

---

## A. Reach — working in the tools you already use

### A1. Connections as a first-class thing — *not started*

Today a tool server is three lines in `config.toml`. That is right for the
protocol and wrong for a person: there is no notion of *an account*, no
credentials that are not plaintext in a file, no way to see what is connected,
and no way to disconnect one.

A **Connection** is an account plus its credentials plus what it is allowed to
do. It needs: a store, a lifecycle (connect, re-auth, revoke), a health check,
and a place in the panel that is not the inert catalogue that is there now.

**Done when** somebody can connect an account without editing a file, see that
it is connected, and take it away again.

### A2. MCP as the substrate, not 35 hand-written connectors — *have the protocol*

`core/tools/mcp.rs` already speaks it, and its own header makes the argument:
*"Six integrations written by hand buy six integrations. Speaking the protocol
buys the ones that exist now and the ones written next year."*

OpenWorker wrote `gmail`, `slack`, `github`, `notion`, `jira`, `linear`,
`stripe`, `figma` and 27 more by hand, each with its own auth, scopes, tools and
privacy filters. **Nudge should not repeat that.** The same ground is covered by
bundling and managing MCP servers.

**Done when** connecting Slack means picking it from a list, not finding a server
binary and writing TOML.

### A3. Credentials in the Keychain, never in the config — *not started*

`config.toml` already holds an API key and the file's own comment calls that the
bring-your-own-model feature. A connector's OAuth refresh token is a different
class of secret: it is somebody's mailbox.

**Done when** no connector credential is readable with `cat`.

### A4. What it does and what it can see, before you connect — *not started*

OpenWorker's `catalog_copy.py` carries About/Access copy for every connector and
its test suite *fails the build* if a connector has no Access line. The module
header is the rule worth stealing: *"Plain statements of behavior, not marketing
— overclaiming here is a product bug."*

**Done when** every connectable thing says what access it gets before consent,
and a connector with no such statement cannot ship.

### A5. Per-connection privacy filters — *not started*

Gmail can hide chosen senders and labels from agents entirely; HubSpot can hide
chosen properties. This is not the permission gate — it is narrower and it
applies before the model sees anything.

**Done when** a connection can be told "never show the agent anything from this
sender" and it holds.

### A6. Bundled servers and their lifecycle — *not started*

Somebody who connects Slack should not be installing Node. That means shipping or
fetching server binaries, starting and supervising them, and surviving one
crashing.

**Done when** a connection works on a machine with no developer tools on it.

---

## B. Carrying a task through from start to finish

This is the whole of the second promise and Nudge has none of it. A run today
lives inside one process: quit Nudge, or let it crash, and an agent that was
halfway through is simply gone. Finished runs are kept in `history.json` for
display; **in-flight ones are deliberately dropped**, because saying a run is
still going when the process that was running it has died would be a lie.

### B1. Durable runs — *not started*

A run needs to be a record that survives, not a struct in memory: its goal, its
history, its plan, where it got to, and enough to pick the thread back up.

**Done when** quitting Nudge mid-task and reopening it offers to carry on.

### B2. Compaction — *not started*

`session.done` grows one line per step with nothing trimming it, and every step
sends the whole thing. Today `MAX_STEPS = 40` hides the problem by ending the run
first. A task that genuinely takes two hundred steps cannot be run at all.

OpenWorker's `compaction.py` is 561 lines and the structure is the part to copy:
pure functions plus one dataclass, with the engine owning *when* and *with what*,
so the policy is testable without a provider. Older turns become an LLM summary
plus mechanically extracted state; recent turns and **every user message**
survive; the persisted transcript is never edited, only what is sent.

**Done when** a run can exceed the context window without being cut off.

### B3. Self-wake — *half done*

`selfwake.py` turns an always-on agent into suspend/resume at roughly zero idle
cost: `sleep_until` for a timer, `wake_on` for a backgrounded job.

**`wake_on` is done.** `await` waits for a background job to end and comes back
with how it ended — one turn for the whole wait, where polling cost a turn every
twenty seconds. Sliced so a ten-minute wait still notices Escape.

**`sleep_until` is not**, and is worth less here: Nudge is not always-on, so a
timer that outlives the process is really §B4's scheduler wearing a smaller hat.

### B4. Scheduled and recurring work — *not started*

`automation/` — scheduler, store, models, tools. *"Check my calendar every
morning and tell me what needs preparing"* is an everyday task and is currently
impossible to ask for.

**Done when** a task can be asked for once and happen on a schedule.

### B5. Projects — *not started*

`projects.py` binds a session to a project identity — preferring the git repo, so
every worktree of one repo collapses to one project, and falling back to the
resolved folder path. It is the key under boards and workspace memory.

Nudge has a single `workspace` config value and no notion of which project a run
belongs to, so memory and trust have nothing to hang off.

**Done when** two runs in the same repo share context and two runs in different
repos do not.

### B6. Background jobs that outlive a turn — *partly*

`core/tools/running.rs` starts and stops processes and `Background::stop_all`
kills them when a run ends — deliberately, so nothing holds port 3000 tomorrow.
The missing half is a job that is *supposed* to outlive the turn, with the agent
woken when it finishes (B3).

**Done when** "run the test suite and tell me when it's green" does not hold a
turn open for eight minutes.

---

## C. Checking in before important actions

**This is the promise Nudge is closest to**, and most of it is done.

### C1. A gate with three answers — *done*

`Answer::{Allow, Ask, Deny}` with a reason and the rule that allowed it.

### C2. Scopes on a yes — *done*

Just now / this session / always, dropped when the task ends.

### C3. An audit trail that cannot become the leak — *done*

The call, its shape and its outcome; never its contents.

### C4. Shell that cannot be argued into writing — *done*

Eight ways an argument turned a reader into a writer, all closed.

### C5. Risk as a declared property — *not started*

OpenWorker's `risk.py` replaced hardcoded `WRITE_TOOLS` / `SHELL_TOOL` name sets
with a declared risk class that one `classify` reads. Nudge's `Grant` is close but
is a capability, not a risk class: there is no way to say *this MCP tool from
this server writes to the outside world* without naming it somewhere.

**Done when** a tool Nudge has never heard of arrives with a risk class and is
gated on it.

### C6. The reviewer — *not started, and the biggest single win*

A second model call judges one proposed action against what the user actually
asked for, so routine actions run without a card and only genuinely questionable
ones interrupt. It exists because of measured approval fatigue — *"~15
hand-approvals per run"* in security scans.

Nudge will meet the same wall the moment agents get longer. Asking about
everything and asking about nothing are both failures.

**The invariant that makes it safe** is the one OPENWORKER.md calls *the
invariant Nudge most needs*: **the reviewer never reads untrusted content.** Its
input is the instructions, the known world, the user's own messages and the
proposed action — never page text, mail bodies or file contents. *The attacker
can address the agent, never the judge.*

**Done when** a long run interrupts a handful of times instead of forty, and the
judge's inputs are provably free of anything an attacker wrote.

### C7. Provenance — *not started*

`provenance.py`, ~250 lines. The engine knows one thing neither the reviewer nor
the person does: **whether it wrote or downloaded that file moments ago.** It
keeps that record and renders one line of fixed vocabulary — never file content.

Its scope note is the part to imitate: deliberately *not* reading file contents
or analysing what a script does, because a miss only ever moves toward caution,
unlike a detector whose false negatives breed false confidence.

Worth less than it was — executing a script now needs an explicit grant — but it
is still the answer to *"you are about to run a file you wrote a moment ago."*

**Done when** a command naming a file this run created says so, in the audit and
in the question.

### C8. A read-only classifier for session grants — *not started*

`readonly.py` auto-allows a command for the session only when a conservative
classifier accepts it. Nudge's allow-list is close, but it is a static list rather
than something a session grant can defer to.

### C9. Session facts — the known world — *not started*

`session_facts.py` renders what was already familiar when the session began and
what arrived from outside since — deterministically, with no model involved. In
v1 it changes no decision; it orients the reviewer. Folders and remotes only,
never content.

---

## D. Being reachable

Nudge is reachable one way: you hold a key at your Mac. Every OpenWorker promise
about Slack threads and answered mail depends on the agent being reachable when
you are not there.

### D1. The Inbox — *not started*

`inbox.py` is the cross-session queue of what agents need from a human: an
**approval**, a **question**, or a **notification**. It is the store of record,
and messaging connectors are transports of the same items rather than parallel
paths.

Nudge's `Pending` is the same idea for one run in one process. The Inbox is what
it becomes when there are several runs and you are not at the machine.

**Done when** a question from a run survives you closing the lid.

### D2. Transports — *not started*

Telegram and Slack carry Inbox items out and answers back. `interactions.py`
renders discrete choices as **buttons**, with the item id riding in the button
value so a click resolves the exact item — no reply-parsing fragility.

### D3. Mentions and routing — *not started*

`mentions.py`, `inbox_routing.py` — mention the agent on a GitHub issue or in a
channel and it picks the work up.

### D4. Unattended mode — *not started*

A run that continues while you are away, with everything it needs going to the
Inbox rather than to a card nobody is looking at.

---

## E. Outcomes, not answers

### E1. Artifacts — *partly*

Agents record files they made and the card offers them. What is missing is the
shape of a *deliverable*: the tagline promises "a polished document delivered"
and "fixes ready to go", which is a pull request, not a file path.

### E2. Attachments in — *not started*

`attachments.py` + `pdf_support.py` build content parts from images, PDFs and
text files. Nudge takes a screenshot and a sentence. You cannot hand it a PDF.

**Done when** "summarise this contract" works on a file you point at.

### E3. Code review with fixes ready to go — *not started*

The tagline's flagship example. Needs: repo reach (A), durable runs (B1),
compaction (B2), and a deliverable shape (E1).

---

## F. Knowing things

### F1. Workspace memory — *partly*

`core/memory.rs` remembers **what applications are like** — CapCut's timeline,
Chromium's menus. That is screen-driving knowledge and is Nudge's own idea.

What it does not have is memory about *your work*: this repo's conventions, who
Sara is, which of the three Stripe accounts is the live one. OpenWorker's
`memory/` is a SQLite store with an index threshold and its own tools.

### F2. Personas — *have the simpler version*

`personas/` are declarative bundles — YAML frontmatter plus a markdown body that
is the system prompt. Nudge's `core/skills.rs` is the same idea in a folder and
is enough; the part not covered is a persona *selecting a toolset*, which is
`catalog.py`.

### F3. A vetted tool catalog — *not started*

`catalog.py` bundles tools behind a stable capability id, with what context they
need and what risk they can produce. It is what lets a persona say "I need
calendar and mail" without naming twelve tools.

---

## G. Foundations

| | |
|---|---|
| **G1. Audit store** | **done** — JSONL, redacted, clipped |
| **G2. Secrets** | **done** — refused in paths, redacted in logs, leak-checked outbound |
| **G3. Providers** | 3 behind a trait; OpenWorker has 9 |
| **G4. Auto-update** | **done** — checks quietly, installs on a button |
| **G5. Workspace trust** | *not started* — a repo may declare allowed command prefixes, but only after the user trusts that exact canonical path, and trust follows the path rather than a snapshot |
| **G6. Signing** | blocked on a Developer ID certificate |

---

## What not to copy

**The always-on server.** `server/` is 9,318 lines. Nudge is a menu-bar app on
somebody's Mac and most of that exists to be a multi-tenant service.

**Thirty-five hand-written connectors.** See A2. Each is auth, scopes, tools,
privacy filters and a test suite, forever.

**The cloud relay.** A hosted GitHub App and relay is a business, not a feature.

**Nine providers.** Three is enough until somebody asks for a fourth.

---

## Order

Dependencies first, then value per hour.

1. **C5 risk classes** — small, and C6 needs it.
2. **C6 the reviewer** — the largest single improvement to how a long run feels,
   and the one invariant worth getting exactly right.
3. **B2 compaction** — the ceiling on how long a task can be.
4. **B1 durable runs** — turns a session into a task.
5. **A1–A3 connections** — the first promise, and the first one a person sees.
6. **B3/B4 self-wake and schedules** — "every morning" and "when CI is green".
7. **D1 the Inbox** — only once there is more than one run to queue.
8. **E2 attachments** — small, and unlocks a whole class of everyday ask.

C7 provenance, C8 the read-only classifier, C9 session facts, F1 workspace
memory, F3 the catalog and G5 workspace trust are each small and can land
whenever they are convenient.
