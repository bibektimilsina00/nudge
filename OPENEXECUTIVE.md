# What to take from OpenExecutive

[`SenteLabsAI/OpenExecutive`](https://github.com/SenteLabsAI/OpenExecutive) —
Apache 2.0, ~150k lines of Python, 152 commits. An AI executive team: eight
specialist agents behind one voice, advising on a company you describe to it.

**Most of it does not transfer.** It is a different product in a different shape —
no desktop, no screen, no cursor, no permission surface to speak of, because
nothing it does touches your machine. Where OpenWorker was the same problem
solved further along, this is a different problem with two habits worth stealing.

Read at commit `HEAD` of the local clone.

---

## 1. Ablation: the eval that asks whether a feature earns its keep

This is the genuinely valuable thing here and it is about fifty lines.

```
"""RAG ablation harness — quantify how much builtin-knowledge retrieval helps.

Runs the existing eval suite twice — once normally, once with builtin retrieval
disabled — and reports the per-domain change in the judge's `overall` score. A
positive delta means builtin RAG raised the score; a delta near zero (or
negative) means the builtin knowledge base did not earn its token/latency cost.

This answers a question the suite otherwise cannot: *is the builtin knowledge
base actually improving answers, or just adding tokens and latency?*
"""
```

Two details that make it a practice rather than a script:

**It reuses the production path.** The "off" arm is the same retriever with
`n_builtin = 0`, driven from settings — *"so no eval-only code path diverges from
production retrieval"*. An ablation that measures a special test mode measures
the test mode.

**It says what it costs, in the docstring.** *"Running it makes real, paid LLM
calls — about TWICE the normal suite."* A harness that quietly doubles a bill is
a harness somebody runs once by accident.

### Why Nudge needs this specifically

Nudge has features that are *assumed* to help and have never been measured:

| Feature | The question ablation would answer |
|---|---|
| Per-app memory | Does recalling notes change the answer, or just the token count? |
| The early look (`stash`) | Does a picture taken before the turn beat one taken during it? |
| Skills | Does a loaded skill change what the model does? |
| `think = low` | The bench is owed a re-run since this changed, and nobody has run it |

`make bench` already scores control-picking against saved screenshots, and
`make truth` scores answers. Both are one arm of an ablation already. The
missing half is running them twice with one thing switched off and printing the
delta.

That is the cheapest available answer to *"is this feature worth its latency"*,
and Nudge currently answers that by opinion.

## 2. An audit log that cannot itself become the leak

Nudge's plan §2.1 wants an audit trail. OpenExecutive has one, and the part worth
copying is not the table — it is `redaction.py`:

> Tool inputs and outputs can contain secrets (OAuth tokens, auth codes, session
> cookies) or PII (raw email bodies, calendar attendees, drive file contents).
> Persisting them verbatim into the audit table would turn the audit log itself
> into a leak vector. These helpers keep the audit trail useful — "the Executive
> called gmail.send" — **without copying payloads**.

That is the whole design decision, and it is the one a first implementation gets
wrong: log the call, its shape, and its outcome; never its contents. Inputs are
clipped to 140 characters and results to 300, which is enough to recognise a call
and not enough to carry a document.

The event shape is worth copying too:

```python
id, ts, event_type, session_id, turn_id, actor, summary, details, full
```

`summary` versus `full` is the useful split: the list view stays small over the
wire and the untruncated payload is fetched only when somebody opens one row.

And a real operational note in the connection helper — WAL plus a 5s busy
timeout, because *"audit writes swallow exceptions, so silent loss under
contention would be undetectable"*. An audit log that drops rows quietly is worse
than none, because it is trusted.

## 3. What does not transfer, and why

Being specific about this is most of the value of having read it.

**The eight-specialist architecture.** A Chief Strategy Officer agent and a CFO
agent behind one voice is a good answer to "advise me on my company" and no
answer at all to "click the thing I am pointing at". Nudge's one agent with tools
is the right shape for a product whose output is an action rather than an
opinion.

**`memory/episodic.py`, 1,922 lines.** Nudge's memory is a note per application,
recalled by name. Theirs is an episodic store for a system whose whole value is
remembering a company over months. The size difference is the product
difference, not a gap.

**The LLM judge.** Standard LLM-as-judge, and Nudge already learned the harder
version of this lesson: `scrutinise()` was a second model pass that cost three
times the tokens, never caught a real error, and damaged two answers before it
was removed. Their judge works because it scores *advice quality*, which has no
ground truth. Nudge's cases have ground truth — a right control to click, a right
answer to give — so scoring against the recorded answer beats asking a model, and
is free.

**Their surface area generally.** Discord and Slack bots, a scheduler, monitoring
pipelines, departments, talent, onboarding. That is a company's worth of product.

---

## What to do about it

One thing, and it is small:

**Add an ablation arm to `make bench` and `make truth`.** Run the suite twice
with one thing disabled, print the per-case delta. Start with memory, because it
is the feature most likely to be earning nothing — a note per application that is
recalled on every turn whether or not it is relevant.

The rest of this repository is a good product that is not this product.
