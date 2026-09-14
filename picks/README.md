# Picking cases

Does Nudge pick the control you meant?

    make picks
    cargo run --bin picks -- record "click the search icon" = Search
    cargo run --bin picks -- record "click the thing next to View" = none

A case is a control list, a goal, and the label that should win -- or `none` when
nothing should. They are plain text and meant to be edited by hand: the tricky
ones are easier to write than to arrange in front of a real application.

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

    8/8 right, 0 WRONG, 0 fell through unnecessarily

**Fell through** means the model handles it: four seconds slower, right answer.
**WRONG** means it clicked something nobody asked for, with no model watching to
disagree.

Averaging them into one accuracy figure hides the only number that matters.
`picks` exits non-zero when anything is WRONG, and never for a fall-through.

## What is not covered

Whether the *answer* is true. Nothing here would notice an agent reporting that
macOS 27 ships in 2036 -- it scores which control gets picked, not whether what
came back was worth having.
