# Picking cases

Does Nudge pick the control you meant?

    make picks
    cargo run --bin picks -- record "click the search icon" = Search
    cargo run --bin picks -- record "click the thing next to View" = none
    cargo run --bin picks -- record --pid 631 "open Finder" = none

A case is a control list, a goal, and the label that should win -- or `none` when
nothing should. They are plain text and meant to be edited by hand: the tricky
ones are easier to write than to arrange in front of a real application.

`--pid` records a named application instead of whatever is in front. Without it,
recording means bringing an app forward, so every case comes from the app that was
already there -- which is why the first fifteen were all the same editor.

## What the fifty cases cover

| | | |
|---|---|---|
| Code | 37 | An Electron tree of 195 controls: file rows, icon toolbars, shortcut labels, duplicates |
| Finder | 3 | Seven radio buttons sharing one position, with labels in smart quotes |
| Arc, Music, Preview | 7 | A menu bar and nothing else |
| ghostty | 3 | Seven controls in total -- the fallback has to fire |

Twenty-four expect a control and twenty-six expect a fall-through, which is the
right lean: picking wrongly is the expensive mistake.

The shapes being probed, rather than the applications:

- **The application is also a menu.** Arc, Music, Preview, Ghostty and Finder each
  have a menu named after themselves, so *"open Music"* -- which means launch it --
  lands on a menu bar item if nothing stops it. `click Music` and `open Music` must
  disagree, and they do.
- **Labels carrying shortcuts.** `Toggle Primary Side Bar (⌘B)` is matched by
  someone saying the name without the shortcut, because brackets are stripped.
- **Labels carrying state.** `Source Control (⌃⇧G) - 2 pending changes` is matched
  by nobody, because the count is part of the name. A fall-through, correctly.
- **One word, two controls.** *"click Terminal"* names both a menu and a panel tab.
- **Word boundaries.** *"click the previewer"* must not find `Preview`.
- **Every verb.** click, press, tap, hit, choose, select, open, go to, switch to,
  show me -- each one is now exercised by something.

## What is still missing

Named in the plan and not here, because nothing on this machine had one open when
the trees were taken: **a dense settings pane, real web content, and a dialog**.
Arc contributed a menu bar rather than a page. These want recording the next time
such a window exists -- `--pid` makes that a one-liner.

## Why this and not the screenshot bench

The screenshot bench scores where a click *lands*, which was the right question
when finding a button meant guessing at pixels. It mostly is not any more: the
system says where the controls are, and the job is choosing the right one from a
list.

It is also cheap in a way the old one never was. A screenshot case costs a screen
you have to arrange and a target you have to click; one of these costs a
sentence. That matters because ten cases cannot measure anything -- one hit is
thirteen percentage points -- and the reason there are only ten is that each one
was expensive.

## Read the two failures differently

    50/50 right, 0 WRONG, 0 fell through unnecessarily

**Fell through** means the model handles it: four seconds slower, right answer.
**WRONG** means it clicked something nobody asked for, with no model watching to
disagree.

Averaging them into one accuracy figure hides the only number that matters.
`picks` exits non-zero when anything is WRONG, and never for a fall-through.

## What is not covered

**Whether the cases are hard enough.** Thirty-five were added at once and none of
them failed. That is either a matcher which holds, or a set written by the person
who knew what it does -- and those look identical from here. Three of the first
run's failures were wrong *expectations*, not wrong picks: a tab labelled
`PLAN.md, preview` does not tie with a row labelled `PLAN.md`, and seven Finder
radio buttons sharing a position do not share a name. Check the tree before
believing a failure.

Whether the *answer* is true. Nothing here would notice an agent reporting that
macOS 27 ships in 2036 -- it scores which control gets picked, not whether what
came back was worth having.
