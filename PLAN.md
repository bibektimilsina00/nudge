# Plan — doing the job the way somebody would

The last plan is finished. This one comes from watching Nudge attempt a real
multi-step research task and reading what actually happened, rather than from
reading somebody else's repository.

Two things are wrong, and they are the same thing twice: **Nudge reports rather
than checks, and it hands work away rather than supervising it.**

## What the test showed

One task — *research three Rust TOML crates, compare them, write the comparison,
then check your own file* — produced a good document in five turns and forty-five
seconds, and four defects on the way:

| | |
|---|---|
| A finished document was thrown away | The model sent its `write` alongside its plan, as a JSON array, and the parser took the span from the first `{` to the last `}` — which for a list is not JSON. **Fixed.** |
| It said it had verified the file | It had not read it. Fifth instance today of a claim nobody checked. |
| It handed the whole job to `claude -p` | One shot, `acceptEdits`, no supervision, no check of what came back. |
| It answered from memory when the shell refused | It reached for `python3 -c` to call an API, was correctly refused, and fell back to recollection rather than to `fetch`. |

---

## The rule that governs all of it

**A terminal's output is untrusted input, exactly as a screenshot is.**

Everything in §2 involves reading what another program printed and deciding what
to do about it. That program may be printing an attacker's text — a file it was
asked to summarise, a web page it fetched, a commit message. The discipline that
already holds for the screen holds here without exception:

> The attacker can address the agent, never the judge.

So: what a terminal prints is never handed to a judge as prose. The supervisor
extracts **structured facts** from it — is this a question, what kind, which file
or command does it name — and only those facts are judged. A prompt that argues,
claims prior approval, or instructs the reader is not information; it is the
thing being defended against.

---

## 1. A claim is not a check

Five times today an agent reported something it had not done. Twice it wrote a
wrong value it had read off a screen; three times it announced a verification
that never happened. Every one of those is the same shape: the model's account
of itself, believed.

The machinery already exists and is pointed at the wrong thing. `Step::consults`
answers *did this turn learn anything from outside?*, and `subagent.rs` uses it —
an answer from a run that consulted nothing is relabelled as recollection. The
main agent has no such check.

### 1.1 A finished run says whether it looked

**Done when** a run that finishes without having consulted anything says so, in
the same words a subagent's answer already does — and the card shows it.

### 1.2 A run that says it checked, checked — *done*

Stronger than 1.1 and narrower. When a run's final message claims a check —
*verified*, *confirmed*, *made sure*, *double-checked* — and no step in that run
read the artifact it is talking about, the claim is removed and the run says what
it actually did instead.

Not a language model judging language: a mechanical match on the claim, and a
mechanical check of whether the file was read. A miss leaves the message as it
is, so partial coverage only ever moves towards honesty.

**Done.** A claim is a phrase from a fixed list; a check is a step that actually
read something. Both decided in code, so a miss leaves the sentence as it was.

Verified live: a run told not to read its file back said *"I have double-checked
the moons.md file and confirmed it lists the four largest moons"*, and what
reached the card was that, followed by *"(I did not read it back, so this is what
I intended rather than what it says.)"* A run that did read first was left alone.

### 1.3 Writing is not checking

A file it just wrote is not evidence that the file is right; `git` already tells
us what changed, and §2.2 of the last plan put that in the record. What is
missing is the same discipline for a *new* file: reading it back is one step and
it is the difference between "I wrote it" and "it says what I meant".

**Done when** a run that produces a document is expected to read it, and says
plainly when it did not.

---

## 2. Working the terminal, like a person

Today a delegation is `claude -p --permission-mode acceptEdits '<task>'`: one
shot, fire and forget, with a permission mode chosen so it cannot stop to ask.
The reasoning is written in the code and was right when it was written —

> An agent that stops to ask something is an agent that hangs, because there is
> nobody at that terminal.

— and the answer is not to keep choosing flags that avoid questions. It is to
**put somebody at that terminal**.

### 2.1 A real terminal

These tools behave differently when their output is a pipe: no prompts, no
progress, sometimes no colour and a different code path entirely. Supervising one
means giving it a pseudo-terminal, not a pipe.

`running.rs` uses piped stdio. This is the foundation the rest of §2 sits on, and
it is the part with no way around it.

**Done when** `claude` run under Nudge behaves as it does in a terminal, and what
it prints arrives as it is printed rather than in blocks when its buffer fills.

### 2.2 Noticing that it asked something

A question is a shape, not a sentence: output that has stopped, ending in a
prompt, offering choices. Recognised mechanically — trailing `?`, a `[y/N]`, a
numbered list that stopped, a known phrasing per tool — and not by asking a model
to read the prose.

**Done when** a delegated agent that stops to ask is noticed within a second, and
a delegated agent that is merely quiet for a moment is not mistaken for one.

### 2.3 Deciding it — three answers, in order

1. **Mechanically.** Some answers need no judgement: a prompt to continue, to
   trust a workspace Nudge itself chose, to use a model already configured.
2. **The judge.** Anything else goes to §2 of the last plan, given *structured
   facts* — what is being asked for, which file or command it names, and what the
   user originally wanted. Never the prompt's prose.
3. **The person.** Anything the judge is unsure about is a card, exactly as an
   egress question is today.

The order matters and so does the direction: this can only ever *add* a question,
never remove one. A prompt nobody understood is a prompt for a human.

**Done when** an ordinary permission prompt is answered without anybody being
disturbed, and an unusual one reaches a person with what it is asking.

### 2.4 Answering

Writing to the terminal, which needs 2.1. The answer is recorded — the prompt, in
fixed vocabulary, what was answered, and who decided.

**Done when** the audit of a delegated run reads as a conversation somebody could
check afterwards.

### 2.5 Noticing when it is stuck

A person watching a terminal notices three things a program does not: nothing has
happened for a long time, the same thing keeps happening, and it is asking the
same question again. Each is mechanical.

**Done when** a delegated agent that has stalled, looped, or asked twice is
stopped and reported rather than waited on until the fifteen-minute cap.

### 2.6 Checking what came back

The half a person never skips. A delegation ends with output, and today that
output is believed. What it actually did is checkable: which files changed,
whether the build still runs, whether the thing it was asked for exists.

**Done when** a delegation reports what changed on disk rather than what the
agent said about itself — the same bar §2.2 of the last plan set for Nudge's own
writes, applied to work it handed away.

### 2.7 The flags people actually use

`--continue` and `--resume` to carry on a session rather than starting a new one;
choosing a model; pointing at a directory. Today one invocation per tool is
hard-coded, checked by running it, and that is the right instinct — every form
written from memory here has been wrong. Extending it means extending the same
table, and checking each addition the same way.

**Done when** a second delegation to the same tool continues the first rather
than starting again, and the table still only contains forms that have been run.

---

## 3. Tasks big enough to need a plan

### 3.1 Decomposing, and being held to it

`Step::Plan` exists and the card renders it. Nothing asks for one, and nothing
notices when a run wanders off it. A task with four named parts that ends after
one is a task that failed, and it currently reports success.

**Done when** a run that set itself a plan is not finished while items remain
unstarted, or says plainly that it stopped early.

### 3.2 Research without a shell

Asked for a crate's dependency count, the model reached for `python3 -c` to call
an API. The shell refused, correctly — that is arbitrary code execution — and the
model fell back to memory rather than to `fetch`, which was available the whole
time.

A refusal that leaves the model with no route is a refusal that produces a
confident guess. The prompt should route an API call to `fetch` and `request` by
name, in the same place the refusal is explained.

**Done when** a refused shell command hands back what to use instead, where one
exists.

### 3.3 Room to be long

`MAX_STEPS = 40` was a lid over a context ceiling that compaction has since
lifted. The number can rise once §1 makes a long run honest — a budget is only
safe when finishing early is visible.

**Done when** the cap reflects what a task needs rather than what the context
window used to allow.

---

## Order

1. **§1.2**, because it is small, it is five-for-five, and everything in §2.6
   depends on the same idea.
2. **§3.2**, which is a prompt and a sentence.
3. **§2.1**, the pty, because nothing else in §2 can start without it.
4. **§2.2 → 2.3 → 2.4**, which are one feature in three parts.
5. **§2.5** and **§2.6**.
6. **§3.1**, then **§3.3** and **§1.1**.

§2.7 can land any time after 2.1.
