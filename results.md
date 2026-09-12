# Run log

Twelve tasks, chosen to break it in different places rather than to be passed.
Each probes something distinct — if all twelve failed the same way that is one
bug, and if they fail twelve different ways that is a message about the approach.

**Before starting:** the log is truncated on every launch, so do not restart
Nudge mid-run. Everything lands in `/tmp/nudge.log` and can be read at the end.

Say each one by holding **Control**. Record what happened in a few words; the log
has the detail.

---

## A. Routing — it must NOT act

| # | Say | Should | Got |
|---|-----|--------|-----|
| A1 | "hey, what can you do?" | Reply out loud. No clicking, no agent. | |
| A2 | "where is the address bar?" *(browser open)* | A ring on the address bar. A **point**, not an agent — you asked where, not for it to be done. | |

## B. One step

| # | Say | Should | Got |
|---|-----|--------|-----|
| B1 | "open my github notifications" | One `Open` with the URL. Done. | |
| B2 | "open photoshop" *(not installed)* | Admit it is not on this machine. Not invent a click, not launch something else. | |

## C. The real thing — several steps, one goal

| # | Say | Should | Got |
|---|-----|--------|-----|
| C1 | "play bohemian rhapsody on youtube" | Opens, finds, plays, stops. | **finished** — 4 turns, clean. Took 3 rounds of fixes: settle waits, own-voice audio, one-click-too-many. |
| C2 | "open capcut and start a new project" | Launch, then grounding inside a native app — not a browser. | **finished** — 2 turns, narrow reading, offered the rest. Took 4 rounds: scope creep → file picker → narrow-reading rule. |
| C3 | "open a private window in safari" | Menu work — turned out menus are the wrong tool; shortcuts are. | **finished** — 3 turns (one wasted repeat, caught by the guard). Was 15 failed menu clicks; found that menu clicks do not land on macOS, and that key codes were hardcoded QWERTY on a Dvorak machine. |
| C4 | "what is the weather in kathmandu" | Browser, read the answer off the screen, say it aloud. Tests whether it can *finish by reading* rather than by clicking. | **finished, heard aloud** — 2 turns. Took two fixes: answer in words not a page, and the agent was never speaking at all. |

## D. Knowing when not to act

| # | Say | Should | Got |
|---|-----|--------|-----|
| D1 | "open youtube" — **with YouTube already open in front** | Notice it is already done and say so. This is what the `screen` field exists for. | |
| D2 | "what's on my screen" — **with a `.env` file open** | Refuse, out loud, before capturing anything. | |

## E. The two flows never tested

| # | Do | Should | Got |
|---|----|--------|-----|
| E1 | "send a whatsapp message" *(no recipient, no text)* | Everything asked up front, in one question, before anything opens. Answer by voice, resume not restart. | **finished** — asks "Who should I message, and what would you like to say?" on turn 0, then sends. |
| E2 | Start C1, then press **Escape** mid-run | Cursor is yours immediately — not at the end of the current model call. | **works** — polled from the hardware, and checked again immediately before acting so no stray click gets through. |

---

## G. Smoke test — run after any prompt change

Six asks, five minutes, one per path through the prompt. Not a thorough test:
enough to catch a rewrite having broken something, before three more features are
built on top of it.

Do them in order, without restarting Nudge, then hand over the log.

| # | Say | Should | Watch for |
|---|-----|--------|-----------|
| G1 | "hey, what can you do?" | Replies aloud. **No agent, no clicking.** | Routing: a request that is not a task |
| G2 | "what's the weather in Tokyo" | `fetch`, answer spoken. **No browser window appears.** | The surface ordering — public fact → fetch |
| G3 | "how many typescript files are in this project" | `run`, answer in 2 turns. **No Terminal window.** | Commands, and not opening an app to run one |
| G4 | "how many unread emails do I have" | Goes to **Mail**, not fetch. Says it cannot tell if Mail is not set up. | The other half of the ordering — their data → the app |
| G5 | "send a whatsapp message" | Asks **who and what in one question, on turn 0**, before anything opens. | Asking up front, agent mode. **Passes** — took 5 rounds; the hardest case in the set. |
| G6 | "make me a landing page for a bakery" | Writes a file. An artifact chip appears on the agent card. | Writing, artifacts, the dock |

**Also worth a glance while G5/G6 run:** the tile in the top-right corner, that it
animates, and that clicking it expands to a card with the commands underneath.

If any of these is wrong, the log says which turn and why -- paste it or just say
which number failed.

## F. Two displays — needs a second screen

Get one without buying one: **Sidecar with an iPad**, or **BetterDisplay** (free
tier) for a virtual one. Place the second display **to the left or above** the
main one for at least one pass — that gives the union a negative origin, which is
where the arithmetic is most likely to be wrong and where a single-screen test
can never catch it.

| # | Do | Should | Got |
|---|----|--------|-----|
| F1 | Move the pointer across the boundary between screens | The companion follows continuously, no jump or teleport at the edge. Proves one window really spans both. | |
| F2 | With the pointer on screen 2, ask *"where is the address bar?"* | The ring lands on the control **on screen 2**. Proves capture picked the right display and the origin was added. | |
| F3 | Same, but pointer on screen 1 | Still correct. Proves the second display did not break the first. | |
| F4 | Run C1 with the browser on screen 2 | Completes as it does on one screen. | |
| F5 | Unplug the second display mid-run | Next turn works. Geometry is read per capture now, so it should recover without a restart — this is the part that used to need one. | |
| F6 | Change the main display's resolution, then ask anything | Correct. Same reason as F5. | |

**F5 and F6 are the ones worth caring about even with one monitor**, because the
old code read geometry once at launch and broke silently on both.

## What to write in "Got"

One of: **finished** · **wrong pixel** (named the right target, landed elsewhere)
· **right pixel, wrong idea** (clicked accurately on the wrong thing) · **never
converged** · **asked when it should have acted** · **acted when it should have
asked** · **crashed or hung**.

The distinction matters — each one has a different fix.
