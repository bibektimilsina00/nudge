# Nudge

**The layer everything else sits under.**

*It looks like it cannot do anything. It can do everything.*

---

## 1. The product

### The sentence

**Nudge is how you operate your computer when you no longer want to operate it
yourself.**

Not an assistant you consult, not a chat window you paste into. The layer between
you and the machine: you say what you want, and the machine does it, using the
same software you would have used.

Every other agent is limited to what has an API. They reach files, shells, HTTP,
repositories -- and they all stop at the same wall: software with no API cannot be
touched. Nudge goes through the wall by using software the way a person does. It
reads the screen, finds the control, presses it. So the set of things it can do is
not *what has been integrated* but **what you can do on this machine**, which is a
much larger set and a permanently larger one.

### The shape it has to keep

A cat in the notch. No window, no sidebar, no roster, no canvas, no greeting. You
hold a key, you say a thing, it happens, and the screen goes back to being yours.
Underneath it drives applications, runs processes, reads files, delegates to other
agents and orchestrates work that takes minutes.

**The gap between those two is the whole idea.** Everything else in the category is
sized like what it can do -- a window because it is important, a sidebar because
there is a lot of it. Nudge is sized like an accessory and works like an operating
system.

This is a constraint, not a mood. It rules things out, which is the useful part:

- **No capability gets a panel.** MCP will add a hundred tools and no interface.
- **The panel is for settings, not for work.** Anything used *while* working
  belongs in the voice loop or the agent card.
- **Growth goes down, not out.** More to do means more per sentence, never more
  per screen.
- **The cat carries what a UI would** -- state, attention, progress.

An operator cannot be a window, because a window competes for the screen with the
very software it is supposed to be operating. That is why everyone who builds one
drifts into being a destination you go *to* and stops being a layer that sits
*over*. Staying in the notch is not modesty; it is the only shape that works.

### Where it ends up

**You run your whole computer through the notch.** Not most of it, not the parts
with integrations -- all of it.

Underneath it uses whatever is installed: a coding agent, a CLI, a shell, an
application driven through its own interface. **The person is not meant to know
which.** They install a coding agent once, authenticate it, never open it again,
say *"clean this up"*, and it happens. Jarvis, with a cat.

The tension in that, named rather than discovered: *"they do not need to know"*
reads badly beside *"something is silently running programs on my machine."* The
two reconcile only on purpose:

> **Never needs to know. Can always find out.**

Invisible by default, inspectable on demand. The agent card is already the right
shape -- a receipt after the fact, not a dialog before it.

### What it makes of everything else

Other agents stop being products you choose between and become **tools it calls**.
You do not open a coding agent; you say what you want and Nudge picks one, watches
it, and reports. Same for whatever replaces it next year, and there will be one.

That is the durable position: **the model layer keeps changing hands, the
interface layer does not.**

---

## 2. Where it is now

### What works

**The loop.** Hold a key, speak, and it acts: clicking, typing, pressing keys,
launching apps, opening URLs, reading and writing files, running commands,
starting long processes, fetching pages, searching the web, planning, delegating.
Twenty-one outcomes the model chooses between. Agents run unattended with a
budget, a card showing their plan and their work, and an instant stop.

**Grounding, which changed shape.** The accessibility tree reports every control
on screen -- name, role, exact rectangle -- in about 200ms, for no tokens. So:

- The model picks a control from a numbered list rather than aiming at a pixel.
- When a request names exactly one control, it never reaches a model at all:
  *"click the View menu"* is about 0.3 seconds end to end.
- Menus are pressed through the accessibility API without touching the pointer.
- Everything else is still a picture and a model, right about two thirds of the
  time.

**Speed.** A turn is about 4.5 seconds and roughly 95% of that is the model.
Everything else totals about 0.22s. What that took is in
[SPEED.md](SPEED.md) and [FINDINGS.md](FINDINGS.md); what matters here is that
latency is no longer a project, and three plausible-sounding ideas were measured
and abandoned rather than built.

**Privacy, as a property rather than a promise.** Transcription happens on the
machine. Screenshots never leave it. The guard refuses to photograph password
managers and windows whose titles name a secret.

**Boundaries that exist because something went wrong without them.** A workspace
nothing reaches past. A shell allow-list of 38 programs with read-only
subcommands. One cursor, one agent. Ask before replacing a file.

### What is built but unproven

Listed because "built" and "works" are different claims, and this project has
confused them before.

| | What is unknown |
|---|---|
| **Multi-monitor** | The arithmetic is tested including negative origins. Whether one window really spans two displays at that level has never run on two displays. |
| **Orchestrating another agent** | `claude`, `codex` and `agy` are verified by execution. `opencode` and `aider` come from documentation. No end-to-end run of the whole flow. |
| **The panel flicker fix** | Two plausible causes addressed. Neither verifiable without watching a Space transition on the machine it happens on. |
| **The Windows port** | The seam is drawn, the portable side is written and runs on macOS behind a feature flag, and the UI Automation tree has never been compiled. See [PORTING.md](PORTING.md). |

### What is broken

- **Not distributable.** Not notarised; `spctl` rejects it. The signing identity
  is one machine's certificate. Nobody else can run this.
- **It says things that are not true.** Asked when macOS 27 ships, on a machine
  running macOS 27, after a successful web search, it answered 2036. Nothing in
  the project can currently detect that.

---

## 3. What stands between here and there

Five groups. Everything in §4 belongs to one of them.

**Trust.** It has to be right, and it has to be checkable. Today one of the two
harnesses measures control selection well, the other measures pixel-grounding on
ten cases which is too few to mean anything, and neither can see whether an answer
is true.

**Reach.** A general local agent does more than this does: any shell command, any
file, real HTTP, email, calendar, chat. Those are not features to copy one by one
-- most arrive through one decision (MCP) and one design (boundaries that a person
can widen).

**Invisibility.** The end state needs Nudge to use what is installed without the
person knowing, which means noticing what is there, asking for what is missing
once, authenticating things nobody opens, and translating failures into something
about the task.

**Learning.** It starts every session knowing nothing about this machine, these
applications, or what went wrong last time.

**Distribution.** Nobody else can install it.

---

## 4. The path

Ordered. Each one says what it is, why it is here rather than later, and how we
will know it is finished.

### Phase 1 — Trust

Everything else is worth less until this is done. An operator that is confidently
wrong will not be given real work, and the more invisible the machinery gets the
more the one visible thing -- what it tells you -- has to be true.

**1.1 A harness that scores answers, not clicks.** — **built, three cases**

`make truth`. A question, strings that must appear, strings that must not; the
real blind loop with search and fetch behind it. Substrings rather than a model
judging free text, because judging with a second model makes the score depend on
a second thing that can also be wrong.

Three outcomes, and unsure is a pass: *"I could not find out"* is the correct
answer to a question it cannot answer, and worth more than a guess that reads the
same as knowledge. Only WRONG exits non-zero. Verified by asserting Berlin is the
capital of France and watching it report `WRONG said "paris"`.

*What is left:* cases. Three is enough to catch a regression and nowhere near
enough to find an unknown failure -- and the 2036 answer has not reproduced,
which means it is intermittent, which means a handful of cases will miss it.
Fifty, spread across: facts after the training cutoff, facts that changed
recently, things with no answer, things where the search returns something
plausible and wrong, and arithmetic on dates.

*Also not covered:* this drives the blind path. The original failure came from an
agent that also had a screenshot.

**1.2 Verify before asserting.** *Built, and then parked. It is off.*

### Why it is off

One full trace of case 001: answering took two model calls carrying 428
characters of search results. Checking took **four more carrying 7,059**, because
every checker turn re-sends the whole prompt plus everything found so far. Three
times the calls, worse in tokens. Against which it has not yet caught a real
error, and during development it damaged two answers before the guards went in.

**Nobody is relying on this product yet.** A wrong answer today costs one
person's afternoon and is recoverable in a sentence. The tokens are not
recoverable at all, and they are what is actually scarce -- the Gemini project hit
its monthly spending cap partway through the first full truth run. Paying triple
for accuracy that has no user is the wrong order to spend in. Until then a wrong
answer is the model's to answer for, and the switch is right there.

`verify = false` in config, and `cargo run --bin truth -- --verify` is the only
thing that turns it on -- deliberately not `make truth`, because a harness that
quietly re-enables it every run is still paying the bill, just somewhere easier
to miss.

*Revisit when* there are people using this whose trust is worth more than the
tokens. The mechanism, the guards and the harness are built and tested; what is
deferred is the spending, not the work.

### What it does, when it is on

`scrutinise`, in `core/run/subagent.rs`. A subagent that looked something up does
not return the answer directly: a second, independent pass is told the answer
already exists and asked to find what is wrong with it. Gated three ways -- it
must have looked something up, must not already be an admission, and must state a
specific, because the failure it exists for is a confidently wrong *number* and an
answer with no digit in it is not that failure.

Building it turned up the thing worth writing down. **A checker told to find a
fault will find one**, and the first two versions each made an answer worse:

- Asked to check *"future stock prices do not exist yet"*, it produced a share
  price for next Friday. An honest refusal survived the first pass and was
  destroyed by the pass meant to protect it.
- Asked to check *"macOS 27, September 2026"*, it answered from its training
  cutoff -- macOS 15, 2024 -- without searching at all. The checker committed the
  exact failure it was summoned to catch.

So the rule is not "trust the checker". It is **verification may only lower
confidence, never raise it**, enforced in three places rather than asked for in
the prompt:

1. An answer that already admits uncertainty is not checked. There is nothing to
   refute and everything to lose.
2. A correction from a checker that consulted nothing is discarded. Memory does
   not overrule a source.
3. When both passes looked and disagreed, **neither wins.** The disagreement is
   handed back whole.

Three is the one that matters, and it was not the plan. The plan said "report what
survives", which assumes a winner. Asked when macOS 27 shipped, one pass searched
once and said September 2026; the other searched three times and said it had not
shipped. More searching is not more right, and there was no basis for picking.
Swapping one confident claim for another behind the user's back is the failure
this mechanism exists to prevent, and it does not stop being that failure when we
are the one doing it.

*Status:* both paths seen live -- the checker agreed on one run of the macOS case
and disagreed on the next. That intermittence is 1.1's finding restated, and it is
why the case set came next rather than more mechanism. It is also why parking this
costs little: one green run would not have settled anything anyway.

**1.1a Grow the truth cases.** *Written, not yet scored clean.*

Three to forty-eight. Weighted toward where the failure lives: 11 stale-cutoff,
8 false-premise, 7 with no answer at all, 10 settled-history as a floor, 7 date
arithmetic (named above as a gap), 5 misremembered specifics.

Most are `never`-only -- say what is definitely false, leave the truth alone.
Naming today's Python release would make the case wrong within a year; naming
3.11 makes it wrong never. It is also the shape that cannot be *authored* wrong,
because it barely claims to know anything.

Two harness bugs came out of the first full run, and the second is the one worth
keeping:

- Six cases at once collected rate limits instead of answers from case 25 on.
  Three, with backoff.
- **A rate limit was scored as a wrong answer.** Twenty-four cases that were
  never asked a question were reported as truthfulness failures -- the harness
  doing to me precisely what it exists to catch. `error` is now its own outcome,
  counted apart and never confused with a wrong answer.

And one bug in 1.2, found only because the cases existed: the disagreement note
ends *"I could not confirm which is right"*, so **every disagreeing answer read as
hedged** to the judge, which scored a confident two-year Bitcoin forecast as an
admission of uncertainty. Hedging is now judged on the agent's own words with our
note cut off first.

*Blocked:* the Gemini project hit its monthly spending cap partway through the
first full run. Cases 001-019 all passed, which is the most that can honestly be
said. A clean full run needs the cap raised.

*Done when:* forty-eight cases run to completion with zero wrong and zero
errored -- and then again, because one green run of an intermittent failure means
very little.

**1.3 Say what is uncertain.** *Built.*

The model had one voice for *"I clicked Send"* and *"macOS 27 ships in 2036"*. It
now has two: a recalled fact is prefixed `From memory:` and an observed action is
not.

Set two ways, and the split is the honest part:

- **The model says so.** `recalled: true` on `done` and `reply`, applied in
  `simple_step` where both outcomes are built, so everything downstream -- spoken,
  shown, written into the history -- carries it without knowing it exists.
- **The subagent works it out.** It has no screen, so if it also consulted
  nothing, there was no source in the room and whatever it said came from memory.
  Marked whether or not the model marked it.

The structural override only exists for the blind path, and that is a real limit
rather than an oversight. The main agent always has a screenshot, so *consulted
nothing* does not mean *not grounded* -- it may be reading the answer off the
screen. Nothing available to us separates a claim about the screen from a claim
about the world, so there the model's own report is all there is.

Same invariant as 1.2: the marker can be added, never removed. `Step::consults()`
is exhaustive rather than a list of the interesting cases, so the next tool added
forces a decision instead of quietly defaulting to *grounded* -- the list-of-four
in `commands::step` went stale the moment a tool was added to it, and this is the
same trap one module over.

The truth harness prints `(from memory)` beside any answer carrying the marker,
including passes. An answer that was right without anything being consulted is
right the way a guess is right.

*Not the same as a hedge*, and tested to keep it that way. *"I did not check
this"* is not *"I do not know"*, and a harness that conflated them would score an
unchecked wrong answer as an honest one.

*Left undone:* the Anthropic provider reads a computer-use tool response, which
has no room for a key we invented, so its main-agent answers can never carry the
model's own report. They still get the subagent's structural one.

*Done when:* a recalled fact and an observed action do not sound the same. **They
do not**, and there is a test by that name.

**1.4 Grow the control cases.** *Fifty cases, 0 wrong.*

Fifteen were all the same editor, and the reason turned out to be mechanical:
recording used whatever was in front, so every case came from the app that was
already there. `picks record --pid N` fixed that -- it reaches a named application
without taking over the screen, which is the difference between a harness at
fifteen cases and one at fifty.

Five applications now, and the second number is the one that matters:

| | | |
|---|---|---|
| Code | 37 | 195 controls: file rows, icon toolbars, shortcut labels, duplicates |
| Finder | 3 | Seven radio buttons sharing one position, labels in smart quotes |
| Arc, Music, Preview | 7 | A menu bar and nothing else |
| ghostty | 3 | Seven controls in total -- the fallback has to fire |

The shape worth naming: **five of those applications have a menu named after
themselves.** *"open Music"* means launch it, and lands on a menu bar item if
nothing stops it. That guard existed already; nothing had ever tested it, because
the only app in the case set was called Code and nobody says "open Code" meaning
the menu.

*Not covered, and named in the original wording:* a dense settings pane, real web
content, and a dialog. Nothing on this machine had such a window open -- Arc
contributed a menu bar rather than a page. `--pid` makes each a one-liner the next
time one exists.

*The honest caveat:* thirty-five cases were added at once and none of them failed.
That is either a matcher which holds or a set written by whoever knew what it
does, and the two look identical from here. Three apparent failures in the first
run were wrong **expectations** rather than wrong picks -- a tab labelled
`PLAN.md, preview` does not tie with a row labelled `PLAN.md`, and seven Finder
radio buttons sharing a position do not share a name.

*Done when:* forty cases, and the wrong-pick count is still zero. **Both true.**

### Phase 2 — Reach

**2.1 MCP.** *Built.*

`core/tools/mcp.rs`. JSON-RPC over a child process's stdin and stdout -- the stdio
transport, not the HTTP one, which is for servers somebody else hosts and brings
authentication with it.

Verified against a server this project had never heard of: `npx -y
@modelcontextprotocol/server-filesystem` connected, described **14 tools**, read a
file, and refused a path outside its own directory -- with the refusal arriving as
an error rather than as an answer. That test is `#[ignore]`d, since it wants `npx`
and a network; `cargo test --lib mcp -- --ignored` runs it.

**No interface was added.** A server is a few lines in `config.toml`. There is no
panel, no tab and nothing to click, because these shapes belong to whoever wrote
the server.

Three decisions worth keeping:

- **One mutex per server, held across request and reply.** Calls to one server are
  therefore serial and the usual id-routing table is unnecessary -- there is only
  ever one request outstanding. Different servers still run at once, which is the
  parallelism that matters.
- **Tools are listed to the model as one line each, not as JSON Schema.** A server
  with thirty tools would put several thousand tokens of schema into *every* turn,
  paid on every hotkey press whether or not anything reaches for a tool -- more
  than the screenshot costs. Names and starred-required argument names instead; a
  wrong type comes back as an error the model can read, costing one round trip on
  the rare turn rather than tokens on all of them.
- **Started in the background.** `npx` may spend a minute fetching a server it has
  never run, and the hotkey has to work during that minute. A server that fails to
  start is reported and skipped; its tools are absent and nothing else breaks.

Credentials go in each server's `env`, beside the server that needs them. Nudge
has no business holding somebody else's token.

*Not yet:* nothing bounds what a tool may do. A filesystem server given `/` can
write anywhere, and the shell allow-list and workspace rules do not reach inside
somebody else's process. That is 2.2, and it is now the more urgent half.

*Done when:* an MCP server the project has never heard of can be added to a config
file and used by voice on the next turn. **Done, and proven by voice.**

Spoken into a running Nudge, with nothing but three lines of config added:

    agent#2 started: "What's on my shopping list"
      turn 0: files/search_files  {"path":".","pattern":"*shop*"}
      turn 1: files/read_file     {"path":"shopping.txt"}
      turn 2: Done "Your shopping list has Milk, Coffee beans, and Oat milk."
    reach: tool server "files" switched off

Three turns, two tool calls, and the last line is the Tools menu being clicked.
Nothing in that chain existed a day earlier, and none of it needed a line of UI.

Worth noting what the model did *unprompted*: asked for a shopping list, it
searched for one before reading it, rather than guessing at a filename. The tools
were described to it in one line each -- names and starred arguments, no schemas --
which was the decision taken to keep the token cost down, and it was enough.

**2.2 Boundaries a person can widen.**

The shell allow-list, the workspace, the GET-only fetch. **Not removed** -- every
one of them is a scar. The shell is read-only because a model reached for `rm` to
get around a refusal. Files are workspace-bound because the first thing that wrote
a file put it in this repository's root. One agent owns the cursor because two of
them sent a voice note to a real person.

Parity means replacing a fixed answer with a decision someone makes:

- **Widening is explicit and visible** -- a setting or a per-task grant, never a
  longer constant in the source.
- **The default stays exactly where it is.** Someone who never opens settings
  keeps today's Nudge, which is right for a thing that listens all day.
- **What was granted is inspectable.** "It has full shell access" must be
  something you can see, not something you have to remember agreeing to.

The general agents are unbounded because nobody has done this work, not because
it is wrong.

*Done when:* a person can grant full shell access on purpose, see that they have,
and take it back. **Done**, for the shell and for files.

`core/reach.rs`. Two grants -- `shell` and `files` -- each starting closed, each
readable from `reach = [...]` in the config and flippable from a new **Allowed
to** submenu in the menu bar, with a tick beside anything granted. Clicking the
tick takes it back, in the same place and with the same gesture that gave it.
Nothing is written back to the config, so the widest Nudge has ever been is one
restart from the narrowest.

The grants are also in the **prompt**, so a widened Nudge appears in the
transcript of every turn that had it -- which is the difference between something
you can see and something you have to remember agreeing to. It costs nothing when
nothing is granted: the section is empty rather than saying "you may not".

Two decisions worth keeping:

- **Secrets stay refused however wide the grant.** Anything matching `.ssh`,
  `.env` or a keychain is refused with full shell access. Granting a shell is a
  decision about *capability* -- somebody wants their assistant to move a file or
  run a build -- and it is not the same thing as handing over their keys said
  twice. A permission people would not have given if asked plainly is not one
  they gave.
- **The syntax rules fall with the allow-list, not separately.** `&&` and `;` are
  refused because they smuggle a second command past a check on the first. With
  no allow-list there is no check to get past, and refusing `npm ci && npm test`
  would make the granted shell useless for the work it was granted for.

One source of truth: `Reach` lives on `Nudge`, which both the menu and the prompt
read. Two copies would drift the first time somebody clicked, leaving a tick
saying one thing and a gate enforcing another -- the worst available failure for
a thing whose whole job is being visible.

*Deliberately not built:* there is **no `http` grant**, because the thing it would
permit -- a request with a method, headers and a body -- does not exist until 2.3.
A menu item that ticks and changes nothing teaches people that the ticks mean
nothing.

**2.2a Tool servers, under the same three rules.** *Closed.*

2.1 bought reach without the means to see or limit it, which made this the only
unbounded thing left once 2.2 landed.

Servers get the opposite default to the grants, on purpose: **putting a server in
the config is already the explicit decision**, so a configured server is on. What
was missing was the other two rules. Both are now in a **Tools** submenu, one line
per configured server:

    files — 14 tools
    github — starting…

The count is the *see what it can do* half. A server described only by its name is
something you have to trust; one that says it brought fourteen tools is something
you can weigh. Clicking takes it back.

Switched off means **gone from the prompt**, not merely refused later -- a tool the
model is still told it has is one it will keep reaching for. It is also refused at
the dispatcher, because a history from before the switch still names it and a model
repeating its last step must not get through.

*What made this awkward, and what fixed it:* the menu is built at startup and the
servers connect a minute later, so their names and counts arrive after the menu
exists. The menu is now **a projection of state, rebuilt on every change**, rather
than a set of items ticked by hand -- which also removed the older arrangement
where the menu and the truth were two things that had to be kept in step. `refresh`
is called when the servers land and the menu simply says more than it did a minute
ago.

*Still true, and not fixable from here:* none of this bounds what a tool does once
called. A filesystem server pointed at `/` can write anywhere, because these rules
stop at the edge of our own process. What can be decided is whether to call it at
all, and that is now decidable.

**2.3 Real HTTP.** *Built.*

`fetch::request`, and a `request` step beside the existing `fetch`. The two are
kept apart because they want opposite things from a reply: `fetch` reads a page as
prose and treats anything that is not a success as a failure, while an API
answering **422 with a JSON explanation has answered**, and turning that into an
error throws away the only useful part. So `request` leads with the status and
hands back the body -- a model that can see `401 Unauthorized` knows to look for a
token, where one handed a bare error guesses.

Verified against a real server: a POST carrying a header and a JSON body arrived
with all three intact, and the same call without the grant never left the machine.

**The `http` grant is now real**, which is why it was left out of 2.2. It gates
*methods that act*, not headers -- an authenticated GET against somebody's API is
still reading, and gating that would mean granting permission to do the ordinary
thing.

What does not move whatever is granted:

- **Where a request may go.** The same host rules as `fetch`: nothing on this
  machine or this network. A granted POST to a router on the home network is the
  single request this most needs to refuse.
- **Redirects are checked per hop.** Following is on by default and a redirect can
  point anywhere, including back at this machine -- which would walk straight
  around the check on the first URL. Every hop goes through the same rule, and the
  chain stops at five.
- **Headers that would redirect or split the request.** `Host` would send a
  request aimed at an allowed name somewhere else entirely; a line break in either
  half of a header turns one request into two.

*Refused as clearly as the shell refuses*, which was the bar. The refusal names
the method, says it could change something, says where to grant it, and says what
it can still do -- and there is a test asserting all of that, because a refusal
the model cannot act on is the same as a silent one.

*What the live exercise cost, and it was not the feature:* two hours of the
session went on being unable to see what the app was doing. Running the binary
directly aborts -- macOS will not apply the bundle's `Info.plist` to a Mach-O
started from a shell, so the first call into speech recognition is killed by TCC
with `SIGABRT` and no message -- and `open --stdout` does not capture it either.

The answer was already in the codebase: `keep_a_log` has been redirecting stdout
and stderr to `/tmp/nudge.log` since long before any of this, and its comment
explains the same TCC reasoning that a crash report was read to rediscover. A
second copy was written before that was found, and deleted after. **`tail -f
/tmp/nudge.log` is how you watch Nudge**, and it is written here because it was
not written anywhere a reader would look.

**3.0 Say something without saying it out loud.** *Built, and it paid for itself
immediately.*

`app/input/inject.rs`. Every end-to-end check used to cost a person holding a key
and speaking a sentence, which made checking rare, made it manual, and meant the
part being exercised was never the part under test -- a tool call, a refusal, a
routing decision -- but always the microphone in front of it.

A line of text arrives instead and everything after transcription runs exactly as
it does for speech. Off unless the run was started with `NUDGE_SAY=/path`: this is
a way to make Nudge do things, `/tmp` is writable by everyone, and an environment
variable lasts exactly as long as the process somebody started on purpose, where a
config setting is turned on once and forgotten.

**It found a data-loss bug on its first use.** Asked to add bread to a shopping
list, Nudge called `files/write_file` with the single word `bread`, replacing
three lines with one -- then read the file back, saw what it had just written, and
reported *"I have added bread to your shopping list."*

Two fixes, structural and prompted, in that order:

- **`files::guard`.** Every existing file named anywhere in a tool call's
  arguments is copied aside before the call, exactly as Nudge's own writes are.
  A tool on somebody else's server is opaque -- there is no way to know whether
  `write_file` appends or replaces, and no way to make it ask -- so what cannot be
  prevented is made survivable. Absolute paths only; a relative one belongs to the
  server's root, which is its business.
- **The prompt says what `write_file` actually does**: it replaces. Adding a line
  means reading first and writing the whole thing back. And the sting -- *reading
  it back afterwards will not tell you what you destroyed, it will show you exactly
  what you wrote and look like success.*

Verified by re-running the same sentence: read, then a surgical edit, then a true
report. Three turns instead of five, nothing lost.

### Phase 3 — Invisibility

Everything here exists because of §1's end state. None of it is needed while the
only user wrote the program.

**3.1 Notice what is installed.** *Built.*

`core/tools/present.rs`. Around forty names a model reaches for -- `gh`, `ffmpeg`,
`jq`, `docker`, the package managers, the macOS ones -- checked against the PATH
once and grouped into a line of prompt.

**Usable, not merely installed**, and that distinction is the whole design.
Telling the model `ffmpeg` exists while the shell is still read-only buys a
refusal and a wasted turn, so the list is what is installed *and* currently
allowed. On this machine:

    read-only:   git; docker; node, npm, python3, pip3, cargo, rustc, java,
                 ruby, swift; brew
    full shell:  + gh, make, kubectl, pnpm, deno, uv, jq, sqlite3, ffmpeg,
                 zip, unzip, curl, osascript, shortcuts, pbcopy, mdfind

Granting the shell nearly doubles it. That is the right shape: a grant should not
only permit more, it should visibly *offer* more, and the offer arrives without
anybody being told to look again.

**Absence is never reported.** Only what is here is named. A list of what is
missing would be longer, mostly irrelevant, and would spend tokens on every turn
saying that a machine is a normal machine. Naming something missing belongs at the
moment it would have helped -- which is 3.2.

Also: `agents_installed` used to run `which` six times at startup, on the path
where somebody is waiting. It walks the PATH now, like this does.

*Two things the tests caught, both mine:* the first version tested the finished
sentence and reported `gh` was being offered when it was not, because the word
*right* contains "gh". And cross-checking the result with `command -v` said `rg`
was installed when it is a shell function -- `/bin/sh -c`, which is what Nudge
actually runs commands through, cannot see it. Excluding it was correct; the
instrument was wrong.

*Done when:* the prompt names what is here rather than the model guessing. **Done.**

**3.2 Ask for what is missing, once.** *Built.*

The defect this turned out to be about: **two different failures were wearing one
message.** Reaching for something not on the allow-list said *"X is not one of the
commands I may run -- I can only read, not change anything"*, whether X was
forbidden or simply absent. Those ask opposite things of the person listening --
one is a permission they can grant in the menu bar, the other is software they
have to install -- and being told the wrong one sends them looking in the wrong
place.

Now:

    zzconvert   →  "zzconvert is not on this Mac"
    ffmpeg      →  "ffmpeg is not on this Mac. I can do that once it is --
                    `brew install ffmpeg`"
    cp          →  "cp is here, but running it is not something I have been
                    allowed to do. Someone can change that under "Allowed to"
                    in the menu bar."

*Not installed* is checked first, because it is the more useful answer when both
are true: granting a shell does not conjure `ffmpeg`.

The same sentence covers the case the allow-list cannot see. `docker` is allowed
and plenty of Macs do not have it, so the shell answers 127 and `sh: docker:
command not found` reaches the model. That is now translated at the point it
happens.

**Install advice only where it is not a guess.** About twenty names have one; the
rest are named without it. A wrong instruction is worse than none -- somebody runs
it, it fails, and now they have a broken command and a reason to distrust the next
thing they are told. And `brew install x` is only offered when Homebrew is
actually here to run it. Being told *this machine does not have it* is the useful
half on its own.

The prompt says to pass it on: name the thing, give the command if there was one,
offer what is possible without it -- and **never quietly substitute a different
program**, which is how a person never finds out that one command would have
worked.

*On "once":* read as *at the moment it matters rather than up front*. There is no
suppression table, because nothing here is proactive -- a sentence is only ever
produced because something was actually reached for, and the history carries what
was already said. A checklist at startup is the thing being avoided, and this is
its opposite.

*Done when:* a person is told what is missing, in words they can act on. **Done.**

**3.3 Credentials.** *Built.*

Two things go wrong with tokens, and they are different problems.

**They live in the wrong place.** A tool server needs a GitHub token, so the token
goes in `config.toml` -- a file that gets copied between machines, opened in an
editor and pasted into bug reports. This project has already leaked an API key
once by printing that file. A value of `keychain:some-name` is now looked up in
the Keychain instead:

    security add-generic-password -s nudge-github -a nudge -w ghp_xxx
    env = { GITHUB_PERSONAL_ACCESS_TOKEN = "keychain:nudge-github" }

**Reading only.** Nudge never writes a secret and never offers to -- storing one is
a person deciding to trust this program with a credential, and that belongs at a
shell prompt they typed, not inside a turn they spoke.

Resolved *before* the child is spawned, so a missing item stops the server with a
sentence about the Keychain rather than starting it with a blank token to fail
later, further away, in the server's own words. Proven live:

    mcp: config: no Keychain item called "nudge-github". Store it with:
        security add-generic-password -s nudge-github -a nudge -w <the-token>

**And when one is missing, nothing says so.** A coding agent that has never been
signed into fails with its own words -- *"Invalid API key"*, *"run `claude
login`"* -- inside the output of a subprocess nobody reads, and from the outside
that is indistinguishable from the agent declining to work. Output that says
nobody is signed in is now named as that, with the command that fixes it, keeping
the original words underneath. Somebody told *"the build failed"* goes and looks at
their build.

Guessing at somebody else's wording is what this is, and the cost is small both
ways: a false positive suggests signing in to something already signed into, and a
false negative leaves the output exactly as it was.

**A server that fails to start is now visible.** It used to be logged and then
absent -- missing from the menu and missing from the prompt, which reads as *not
configured*. The Tools menu says `github — did not start`, because the most likely
reason is a credential and that is precisely the thing somebody needs telling
about.

*Not done, and the plan named it:* driving a login through the screen. Nothing
here logs anybody in; it tells them, precisely, what to run. That is the honest
half, and the other half wants a person at the keyboard anyway -- every one of
these logins ends in a browser.

**3.4 Failures that are about the task.** *Built.*

**A credential was leaking through an error message.** `reqwest` puts the whole
URL in its text and the provider's URL carries the API key, so one rate limit put
the key in `/tmp/nudge.log`, in the error bubble on screen, and in whatever got
pasted into a bug report. It reached a terminal once already this session.

Fixed at the root -- `Error::Http` redacts where it is turned into text -- so every
path is covered at once, including the ones written next year. Markers rather than
shapes: `key=`, `Bearer `, `token=`. Guessing which long strings are secret means
deciding how long is long, which redacts a commit hash and misses a short token.
(`Authorization: ` is deliberately *not* a marker: it is followed by the scheme,
so cutting there left the token standing. The scheme word is the reliable one.)

**`error::plainly`** turns what is left into a sentence about the task. Nobody
using Nudge knows `reqwest` exists, or which of the programs underneath just
failed. *"That file is not there while trying to open my shopping list"*, not
`No such file or directory (os error 2)`. Nudge's own refusals pass through
untouched, because they are already sentences with the fix in them. The original
still goes to the log and to the model, which is the audience it was written for.

**And a cancelled task was announcing success.** From the shakedown: Escape
stopped an agent, and it then said *"I have submitted the task to add bread to
your shopping list"*. The cause is a plain ordering bug -- a terminal step speaks
and returns *before* the stop check, which only ever guarded acting:

    model returns Done → speak(…) → set_state(Done) → return
                                 ↑ the stop check was below this

Two fixes. The check moved above the speaking, and `set_state` now refuses to move
an agent out of `Stopped` at all. Two seconds of in-flight model call is enough for
that race, which makes it not a race worth being careful about but one the type has
to refuse.

The prompt carries the rest: say what an error means for what they asked, keep the
original to yourself, and **never report finishing something you did not finish** --
a wrong "done" costs more than a failure, because a failure is something a person
can act on.

*Honestly unverified:* the live stop. A synthetic Escape from System Events is not
seen by `escape_down`, which polls the physical key, so the race could not be
reproduced from here. The guard has a unit test; the ordering change is reasoned,
not observed.

**3.5 Delegation stops being visible.** *Built.*

The prompt used to list every installed coding agent by product name, with its
exact invocation, so the model could match what somebody said against the list and
copy a command line. Two things wrong with that. It is a choice being presented --
and **a list of product names in the prompt guarantees those names come back out
of the assistant's mouth.**

Now there is one outcome, `delegate`, carrying the job written out in full and
nothing about who does it. `running::choose` picks, in the order the agents were
verified in. The invocation moved out of the prompt and into
`running::command_for`, which is a second win: the flags differ between agents and
a rearranged one makes an agent ignore the job entirely *while appearing to run
fine*, so that belongs where it is written down and checked rather than copied
freshly every time somebody asks for something.

**Naming one is still honoured.** Somebody who says "use Codex for this" picked it
for a reason, so `named` carries that through, matched loosely because a spoken
name arrives as whatever the ear made of it. If the one they named is absent they
are told which -- never quietly given a different agent, which is the one thing
worse than saying so.

The test that used to assert every agent was named now asserts the opposite, and
is narrowed to the delegation section: `Claude Code URL Handler` is a real
application and the apps list is right to name it, which is what caught the first
version of this.

*Honestly unverified:* no live delegation. Given a one-line bug to fix and a haiku
to write, the model correctly did both itself rather than handing them over -- so
the path is unit-tested at both ends and never yet run end to end. Forcing it
wants a job big enough to be worth an agent, which costs real time and somebody's
quota.

*Phase 3 is done when:* someone who has never heard of a coding agent can install
one, forget it, and never be reminded it exists. **The prompt no longer contains
their names, and there is a test holding that.**

### Phase 4 — Learning

Memory is four different things, and only one of them is built. Naming them apart
matters because they have different lifetimes, different writers, and different
costs.

|  | Scope | Written by | Loaded |
|---|---|---|---|
| 4.1 | One application | the model, from failure | when that app is in front |
| 4.3 | This person | both | every turn |
| 4.4 | This workspace | both | when working there |
| 4.5 | The last few minutes | the loop | while the thread is warm |

**The shape is taken from Claude Code, deliberately**, because it is the design
that has survived contact with the most users: memory is **plain Markdown a person
can open and edit**, it is **loaded into the request rather than retrieved**, it is
**hierarchical and merged**, and **both the person and the agent write to it**. A
database would be better at searching and worse at everything that matters here --
being readable, being editable, being deletable with one line, and being obviously
yours.

Two places Nudge must differ, and both are about its own shape rather than
preference:

- **It pays per turn, out loud.** Claude Code loads its memory into a session that
  then runs for an hour. Nudge starts fresh on every hotkey press, so every
  kilobyte of always-loaded memory is paid again each time somebody speaks. Sizes
  are capped here in a way they are not there, and the per-application tier exists
  precisely so that most of what is known costs nothing most of the time.
- **It has a screen.** The per-application tier has no analogue in a terminal
  agent, because a terminal agent never has to know that CapCut's timeline view
  means a project is open.

**4.1 Per-application notes.** *Built.*

`core/memory.rs`, plus a `remember` outcome. Notes are kept per application in
`~/.config/nudge/memory.toml` and put back in front of the model **only when that
application is in front**.

The scoping is the whole design. A general pile of advice is paid for on every
turn and is about the wrong program almost always; thirty words about CapCut cost
nothing on the other three hundred and sixty-four days.

**Written from failure, not success.** The prompt says so explicitly: *"it worked"*
teaches nothing, because next time would have done that anyway. Write what was
surprising, as a fact about the application rather than a story about this turn.

Bounded and deduplicated -- eight notes per application, 200 characters each, and
a note that repeats one already held is dropped. Beyond that it stops being a hint
and becomes a second set of instructions competing with the real ones.

**Visible and forgettable**, by the rule everything that changes behaviour follows
here: a **Learned** submenu in the menu bar shows each application with a count,
and one click forgets it. Plain items rather than checkboxes, because a tick would
imply it can be switched back on. Something that silently learns is something you
cannot reason about when it starts behaving oddly.

*Verified:* the file loads at startup (`memory: 2 notes about 2 applications`), and
`prompt()` is unit-tested to return a note for its own application and nothing for
any other.

*Not verified:* a note being written by the model from a real failure. Asked
directly to *"remember that Ghostty has no window open"*, it launched Ghostty --
correctly, since that is what the sentence asks for. Provoking a genuine
learn-from-failure on demand is harder than it sounds, and the honest position is
that the write path is unit-tested and unobserved.

**4.2 waits on that.** The plan's own condition is *"if memory proves out"*, and it
has not yet -- nothing has been learned in anger. Promoting a remembered sequence
into a named skill while the thing it is promoted from is unproven would be
building the second floor first.

**4.2 Skills, if memory proves out.** *Waiting, on purpose.*

A remembered sequence that worked, replayable by name. Deliberately after memory,
because a skill is a memory that has been promoted -- and nothing has been
remembered in anger yet. See 4.1.

**4.3 What it knows about you.**

`~/.config/nudge/NUDGE.md`, in front of the model on every turn. The Claude Code
`CLAUDE.md` idea, at the user level: who you are, what you are working on, which
Sara you mean, where your shopping list lives, that you prefer short answers.

Both write to it. You edit the file; the model appends a bullet when it learns
something durable about **you** rather than about an application. Appended under a
heading it owns, never rewriting what a person put there -- an agent that reformats
your notes is one you stop keeping notes in.

**Capped, and the cap is the interesting part.** This is read on every hotkey
press, so it is the one piece of memory whose size is a latency and a billing
decision rather than a taste one. Something like 2KB, with what happens at the
limit stated rather than discovered: the oldest agent-written line goes, and
anything a person wrote stays.

*Done when:* it knows your name without being told twice, and you can open the file
and see exactly why it thinks so.

**4.4 What it knows about this workspace.**

`NUDGE.md` in the workspace, loaded only while working there. The project tier,
and the same file convention, so moving a folder between machines carries what was
learned about it.

Merged with 4.3 rather than replacing it, nearest scope last, so a workspace can
contradict a general preference. When two scopes disagree the narrower one wins,
and that rule is written down because it is the thing nobody can guess.

*Done when:* the same question in two different workspaces gets two different
right answers.

**4.5 The thread of a conversation.**

**Nudge has no conversational memory at all**, and this is the gap that most
contradicts the vision. Every hotkey press calls `begin`, which starts `done`
empty. *"Open Safari"* then *"now go to Wikipedia"* -- the second turn has no idea
what the first did, except what it can see on the screen.

It survives today because the screen carries the context. It stops surviving the
moment the answer was not visual: what a command printed, what a search returned,
what was decided. Those exist nowhere after the turn ends.

The fix is small: carry the last few turns forward while the thread is warm --
a handful of exchanges, expiring after a few minutes of silence, and cleared by
anything that is plainly a new subject. Not a transcript, not a database; the same
`done` list that already exists, not thrown away quite so eagerly.

The precedent is already in the codebase and was written for exactly this reason:
`begin_agent` carries `done` into a handover, because throwing it away meant a
foreground search found an answer, the handover wiped it, and the agent searched
again for the same thing.

*Done when:* "what did that print?" is answerable one turn later. **Done:**

    turn 1: "run ls in my workspace and tell me what is there"
            → files/list_directory → hello.py, index.html, otters.txt, shopping.txt

    turn 2: "how many of those were text files?"
      saw: "The active directory listing from earlier shows two text files…"
            → Done in 3.3s, one turn, nothing re-run

*Built as:* `Nudge::end` keeps the finished session's goal and tail for five
minutes; `open` picks it up if the thread is still warm. Bounded twice over,
because this is paid on every turn that follows another closely -- six entries and
2,000 characters, newest first, so a command's output survives and a file dump from
four turns ago does not.

**Kept apart from `done`, and that is the part that matters.** `done` is *what I
have done towards this goal*; the carried tail is *what was going on a moment ago*.
Handed the two as one list, a model believes it has already made progress on
something it has not started -- so the prompt gives it its own heading and says
plainly that none of it counts towards the goal, and to ignore it if the subject
has plainly changed.

Not persisted. A conversation does not survive quitting the application, any more
than one survives the other person leaving the room.

*Order:* **4.5, then 4.3, then 4.4, then 4.2.** 4.5 is the smallest and the most
felt -- it is the difference between an assistant and a command line. 4.3 is next
because most of what a person wants remembered is about themselves. 4.4 only earns
its keep once there is more than one workspace. 4.2 stays gated on 4.1 proving
out.

### Phase 5 — Being usable by anyone else

**5.1 Signing and notarisation.** Developer ID, in CI, before any public link
exists. Everything above is theoretical until this is done.

**5.2 Discovery.** The cost of §1's shape: an interface that shows nothing teaches
nothing, and nobody guesses that the thing in the notch can refactor a repository.
The answer has to live in the voice loop -- *"what can you do?"* answered well, and
capability surfacing when it is relevant -- rather than in a menu nobody opens.
The first real blocker on the second user.

**5.3 The resting state.** What is on screen 99% of the time is the pill and the
cat, and they have had the least attention of anything in the project. If the
product is the shape, the shape is what needs the work.

### Phase 6 — Elsewhere

**6.1 Windows.** The seam is drawn and the other side is written. Nothing has been
compiled for the target, because it cannot be from a Mac. Expect a different
latency profile entirely: a capture is 61ms here through ScreenCaptureKit and a
hardware encoder, and about 2100ms through the portable path. See
[PORTING.md](PORTING.md) for which calls to suspect first.

**6.2 Linux.** After Windows. Wayland and X11 handle capture and input injection
completely differently, so it is two ports wearing one name.

### Running alongside all of it

Not phases, but things that must not be allowed to rot:

- **Prove what is already built.** The four unproven items in §2. The one that
  matters most is whether an agent knows *not* to delegate a one-line change --
  the failure nobody notices because it still works.
- **Nothing reads the diff.** An agent reports what it did and Nudge believes it.
  `git diff` is one call away and would turn a claim into a check.
- **Widen the no-model path.** Every phrasing it learns is another turn that costs
  0.3s instead of 4.5. Cheap and compounding, and safe now that a wrong pick is
  caught the moment it appears. Typing into a named field and launching apps by
  name are next.

---

## 5. The safety model, and how it must survive Phase 2

Every rule exists because something went wrong without it:

| Rule | What happened |
|---|---|
| Workspace boundary | The first thing that wrote a file put it in this repository's root |
| Read-only shell, by program | A model reached for `rm` to get around a refusal |
| One cursor, one agent | Two agents shared a WhatsApp chat and sent a voice note to a real person |
| Ask before replacing | Self-evident, once |
| Privacy guard | A password manager is one frontmost window away at all times |
| Everything started is killed | A dev server still holding port 3000 tomorrow would be Nudge's fault |

**"Controls everything" has to mean more enforcement, not less.** Phase 2 widens
what is possible; it must widen what is *recorded* by the same amount. The test
for any new reach: can the person see afterwards what was done with it?

---

## 6. How we will know

Four instruments. Each answers one question and none answers another's.

| | Question | State |
|---|---|---|
| `make picks` | Does it choose the control you meant? | 15 cases, offline, instant. Wrong picks reported separately from fall-throughs and never averaged |
| `make bench` | Can a model find a control in a picture? | 10 cases. Good for latency, too small for accuracy, and measures the surface the tree replaced |
| the timing line | Where did this turn's time go? | Every turn, every stage |
| `cargo run --example captime` | What does looking at the screen cost? | On demand |

**Missing, and Phase 1.1:** does it tell the truth?

Two lessons from building these, worth keeping:

- **Ten cases cannot measure accuracy.** One hit is thirteen points; two identical
  runs scored 50% and 71%. Latency the same harness measures well.
- **Verify the instrument before trusting the result.** A domain check silently
  started answering "taken" for everything after a rate limit. A benchmark that
  cannot fail is not measuring anything.

---

## 7. Deliberately not doing

**Verifying every answer, for now.** Built and switched off -- see 1.2. Three
times the tokens to protect users who do not exist yet. The order matters: get
something worth trusting in front of people, then pay to make it trustworthy.

- **A window.** Ever. See §1.
- **Streaming the model's reply.** Measured four times: the first chunk arrives at
  6.26s of a 6.36s call, because it thinks throughout and then emits in 0.15s.
  Worth 2%.
- **A running capture stream.** Would remove 56ms of a 61ms capture and cost
  continuous screen recording.
- **A workflow scripting engine.** Parallel delegation already exists and is
  bounded by the cursor, not the design. The one useful idea from it -- verify
  before asserting -- is Phase 1.2 and needs no engine.
- **Trading accuracy for speed.** Measured once, properly: 29% against 50%. A fast
  wrong click costs more than a slow right one, and it costs it in trust.
- **Feature parity with any specific competitor.** Parity with what a *general
  agent can do* is §4 Phase 2. Parity with a product is the losing side of every
  fight.

---

## 8. Debt, marked in the code

Searchable with `ponytail:`. The ones that matter:

- **`CGWindowListCreateImageFromArray`** is still the fallback under
  ScreenCaptureKit. Fine, and it should stay: the fast path needs macOS 14 and a
  current screen-recording grant.
- **The JPEG encode** is now the largest part of taking a screenshot -- about 180ms
  of a 296ms capture through the slow path. Only worth touching if capture matters
  again.
- **`done` is filled by tools that never touch the screen**, so the turn after a
  `read` still waits for the screen to settle. Knowing which steps move the screen
  is the fix, and the safe direction is to assume they all do.
- **Audio is the whole output device.** Another application's notification chime
  reads as audio playing. Per-process attribution needs a tap on the stream.
- **The no-model path is English-only.** The verbs and the relational words are
  English strings. A language boundary, not an OS one, and it will follow to every
  platform.

---

## 9. What this document used to say

It began as a plan for a companion that pointed at buttons, and most of it was
about keeping up with a similar product. That product has since moved to a full
window with a sidebar and a roster, which is the shape everything else in the
category has. The framing is gone and so is the comparison.

It was rewritten again when the speed work finished and the accessibility tree
replaced guessing at pixels, because a plan that describes a solved problem as the
next one is worse than no plan.

This version is the first one written from a finished vision backwards, rather
than from the current code forwards.
