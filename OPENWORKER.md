# What to take from OpenWorker

[`andrewyng/openworker`](https://github.com/andrewyng/openworker) — MIT, ~50k lines of
Python, 386 commits, 598 merged PRs, in open beta. Same shape of product as Nudge (a
desktop AI agent that acts on your machine), and — usefully — **the same stack for the
shell**: Tauri + React over a local backend.

It is further along than Nudge in exactly the places Nudge's own plan admits it is
weakest. This is a reading of it, and what is worth copying.

Read at commit `5bc10d9`.

---

## 1. The single biggest idea: a permission *engine*, not a set of switches

Nudge has three grants — shell, files, http — each on or off, read at the gate with
`may(app, Grant::Shell)`. OpenWorker has an engine that returns a **decision** per call.

```python
@dataclass
class Decision:
    allowed: bool
    reason: str = ""
    needs_user: bool = False   # surface should prompt
    human_only: bool = False   # no automated reviewer may clear this
    rule: str = ""             # which standing rule allowed it, for the audit
```

Three things here that Nudge does not have and should:

**A third answer.** `allow / deny / ask` beats `allow / deny`. Nudge's model forces every
capability into a decision made once, in advance, with no idea what will be asked of it.
"Ask" is what lets the default be safe without being useless.

**A reason, always.** Every denial carries the sentence explaining it. Nudge's refusals are
mostly `Err(...)` strings written at the call site; a single shape means the interface can
show *why* uniformly.

**Modes over flags.** Six of them, and the enum is worth reading for the naming alone:

| Mode | What it means |
|---|---|
| `DISCUSS` | read-only, no planning workflow |
| `PLAN` | read-only + explore → propose_plan → execute |
| `INTERACTIVE` | ask on writes and commands (default) |
| `AUTO_APPROVE` | an LLM reviewer clears the routine ones |
| `BYPASS_APPROVALS` | full access, minus hard floors |
| `CUSTOM` | interactive + a configured allow-list |

Note the comment on `BYPASS_APPROVALS`: renamed from `auto` because *"bypass names the
action — switching a safety system off"*, and deliberately **not** called
`bypass-all-approvals`, because the hard floors still hold and "all" would be a false
promise. That is the level of care taken over a single identifier.

### Grants have scopes, and the scopes are the design

```python
auto_allow_tools      # from config, permanent
session_allow_tools   # "always allow", until the app closes
run_allow_tools       # "allow for this request", cleared when the run ends
allowed_domains       # egress, matched by host or subdomain suffix
```

The `run_allow` scope exists specifically for connectors and MCP, where one approval would
otherwise be re-asked on every page of a paginated loop. Nudge has exactly this problem in
`Grants.granted` — a per-path set with no expiry.

## 2. Shell safety: the part Nudge gets wrong

Nudge's `PLAN.md` records the incident: *"A model reached for `rm` to get around a
refusal."* Its answer was a read-only shell with an allowed-program list. OpenWorker went
much further, and the reasoning is in `permissions.py` and `readonly.py`:

```python
_OPAQUE_CONSTRUCTS = ("`", "$(", "$", ">", "<", "(")
_SEPARATORS        = ("&&", "||", ";", "|&", "|", "&", "\n", "\r")
_ARG_EXECUTORS     = {"xargs", "env", "sudo", "ssh", "docker", "npx", "uvx", ...}
_INLINE_CODE_FLAGS = {"-c", "-e", "--eval", "--command", "-EncodedCommand"}
_DANGEROUS_FLAGS   = {"-exec", "-execdir", "-delete", "-ok", "-fprintf"}
```

Four attacks a program allow-list does not stop, all handled here:

1. **`find . -exec rm {} +`** — the allowed program is `find`; the flag turns it into a
   deletion tool.
2. **`npx something`** — the allowed program runs *another* program named in its arguments,
   which the rule never saw.
3. **`python -c "..."`** — an interpreter carrying inline code.
4. **`git status > /etc/thing`** — a redirection writes somewhere the allow-list never
   vetted.

And a detail worth stealing outright: compound commands are **split and each part checked
independently**, because the previous behaviour rejected `git status && git diff` outright
*and* still allowed `find . -exec rm {} +` — over-strict and under-strict at once. The
comment notes over-splitting is safe by construction: it only ever produces more parts to
justify, never fewer.

`readonly.py` is the same idea for the "allow read-only commands this session" grant, and
carries the best single line in the codebase:

> Network clients (curl/wget/ssh/nc) are deliberately excluded even for GET — an
> auto-allowed network command is an exfiltration channel under prompt injection.

Nudge's `Grant::Http` is currently "GET is fine, anything else needs permission". That is
the wrong axis: a GET *is* the exfiltration.

## 3. The reviewer, and why Nudge's version was right to be deleted

Nudge had `scrutinise()` — a second model pass over answers — and it was removed for
costing three times the tokens and never catching a real error, after damaging two answers.
OpenWorker's reviewer looks superficially similar and is a completely different thing:

- It judges **one proposed action**, not an answer. A turn with several calls fires several
  concurrent reviewer requests, *one action each*, so a verdict physically cannot land on
  the wrong action.
- It can only turn **"ask the human" into "go ahead"**. Hard denies never reach it. It
  cannot widen the permission surface, only reduce interruptions.
- It **fails closed**: malformed JSON, unknown verdict, empty response, timeout, provider
  error — all become `unsure`, which means the human decides. *"There is no parse path that
  results in execution."*
- After 5 denials in a row it pauses itself for the rest of the turn and hands approvals
  back to the person.

That is the design Nudge's `verify` flag should have had: not a second opinion on the
answer, but a filter on interruptions, which can only ever make the product quieter and
never less safe.

## 4. The invariant Nudge most needs: the judge never reads untrusted text

> The reviewer never reads untrusted content. Its input is the instructions, the known
> world (folders and remotes only), the user's own messages, and the proposed action. Page
> text, mail bodies, and file contents never appear — **the attacker can address the agent,
> never the judge.**

Nudge has the matching bug written down and unfixed: *the screen is untrusted input* — it
read a suggestion off a Claude Code transcript three times and acted on it. Everything
Nudge sees is a screenshot of content somebody else may have written.

`provenance.py` is the companion idea and is small enough to copy this week. The reviewer
cannot judge `python scripts/setup.py` from its text, because the effect is inside a file
nobody shows it. But the engine knows something neither the reviewer nor the human does:
**whether it wrote or downloaded that file moments ago.** So it keeps that record and
renders one line of fixed-vocabulary fact.

The module's own scope note is the part to imitate:

> Deliberately NOT here: reading file contents, analysing what a script does, or tracing
> values out of untrusted text. A miss leaves behaviour exactly as it is today, so partial
> coverage only ever moves toward caution — unlike a detector, whose false negatives would
> breed false confidence.

## 5. Things Nudge has no equivalent of

**Auto-update.** `tauri.conf.json` has the updater plugin, a minisign public key, and two
endpoints — their own CDN and the GitHub release, in that order. `packaging/make_update_manifest.py`
generates `latest.json`. Nudge ships a `.dmg` and has no update path at all, which means
every user is pinned to whatever build they first downloaded.

**Compaction.** When the outbound history approaches the context limit, the older portion
is replaced with an LLM summary plus mechanically extracted state; recent turns and *all
user messages* survive. The persisted transcript is never modified — only what is sent.
Note the structural choice: `compaction.py` is pure functions plus one dataclass, and the
engine owns *when* and *with what*. That keeps the policy testable without a provider.

**Self-wake.** `sleep_until` and `wake_on` turn an always-on agent into suspend/resume at
roughly zero idle cost. Nudge's agents run to completion or die.

**An audit store.** A SQLite log of every connector/tool action, with secret keys and
message bodies stripped on the way in. Nudge's `PLAN.md` sets the test — *"can the person
see afterwards what was done with it?"* — and currently the answer is no.

**Workspace trust.** A repository may declare allowed command prefixes in
`.coworker/config.toml`, but they take effect only after the user trusts that exact
canonical path — and trust follows the path, not a snapshot, so later changes are accepted
until revoked. Nudge's workspace is a single config value with no notion of trusting one.

## 6. Provider abstraction, compared

Nudge has three providers behind `trait Provider`. OpenWorker has nine, and two pieces
Nudge lacks:

**A capability probe.** `capabilities_for(model)` returns what a model can actually do,
from a curated matrix with heuristic fallback. The Ollama branch is instructive: *"many
fake/mishandle parallel tool calls — assume tools work but stay conservative otherwise"*,
with vision detected from naming conventions. Nudge assumes every provider behaves the
same and finds out otherwise at runtime.

**A registry with descriptors.** `ProviderDescriptor` / `ProviderField` describe what each
provider needs to be configured, so the settings UI is generated rather than hand-written
per provider. Nudge's new Model picker hard-codes three options in Rust.

Worth noting the contract is deliberately *blocking* and deliberately has **no `max_turns`
loop** — the runtime owns the agent loop, the provider returns one turn. Nudge's
`Provider::next_step` is the same shape, which is a good sign.

## 7. What they do that Nudge should *not* copy

Being specific about this matters as much as the rest.

**Their CI is simpler than ours.** Three jobs — pytest, gui-unit, gui-e2e — on
`[push, pull_request]`, no path filters, everything runs every time. Nudge's pipeline
filters by path and deploys per service. For a 50k-line repo with three surfaces, they
chose "run it all" over "work out what changed". That is a real vote for simplicity, and if
Nudge's filtering ever misfires, theirs is the fallback position.

**Server-in-a-subprocess.** OpenWorker runs a Python server the GUI talks to
(`manager.py` is 6,257 lines). Nudge's logic is in the same binary as the UI. Theirs is the
cost of Python; it is not a model to follow.

**Scale of surface.** 4,894 lines of connector tools, a teams store, subscriptions,
cloud sync, an inbox. Nudge is one person's project at v0.1.0 and copying the surface area
would be copying the wrong thing.

## 8. Practices worth adopting regardless of the code

**Design docs referenced from the code.** Modules cite `ocw-context/docs/reviewed-auto-mode.md
Part 8`, `spec §1.2`, `UX-DECISIONS §25`, and ticket ids (`OPE-114`, `OPE-136`). The
reasoning lives somewhere durable and the code points at it. Nudge keeps its reasoning in
comments, which works at this size and will not at ten times it.

**Decisions dated and attributed in comments.** *"2→5 + streak semantics, owner ruling
2026-08-24 — a cumulative 2 silently downgraded long agentic turns to hand-approval after
one over-strict pair."* A tuning constant, why it changed, who decided, and the failure
that prompted it.

**Tests as invariants, not coverage.** `test_approval_integrity.py` exists because
`POST /v1/inbox/{id}/resolve` takes a raw string, *"so without validation any local API
caller could mint a grant the UI deliberately never offers."* The test names the attack.

**Comments that say what is deliberately absent.** `web_fetch` is excluded from the
download-tracking list with a note: *"it returns page text and never writes a file… Listing
it here would claim coverage we do not have."*

---

## What to do first

In order of value per hour, all small:

1. **Fix `Grant::Http`.** A GET is the exfiltration channel. The current split — reads free,
   writes gated — protects the wrong direction.
2. **Add the shell argument checks** from `permissions.py`: arg-executors, inline-code
   flags, dangerous flags, redirection, and per-part checking of compound commands. Nudge
   already has the allow-list; this is the part that makes it mean something.
3. **Give the permission gate a third answer.** `may()` returning `bool` is the thing
   blocking everything else here.
4. **Copy `provenance.py` wholesale.** It is ~100 lines and it is the cheapest available
   answer to "the screen is untrusted input".
5. **Auto-update**, once there is a Developer ID certificate. Every user is currently
   pinned to their first download.
