# Assets

Rive animations. Both are binary, so what they contain is written down here --
the names are case-sensitive and there is no way to read them off the file
without `strings`.

## `cat.riv`

The companion. Used by `components/Companion.tsx`.

## `agent.riv`

The agent, as something visibly working. Used by `components/Face.tsx` on the
agent card.

| | |
|---|---|
| state machine | `State Machine 1` |
| inputs | `float`, `recording`, `zoom`, `press` |
| states | `Floating`, `Listening`, `Talking` |

Read out of the binary rather than guessed, which is why they are written here.
The component treats every input as optional -- Rive returns null for a name it
cannot find and says nothing about it, so a rename costs the expression rather
than the card.

## `pointer.riv`

A hand that can point and click, for showing what Nudge did without taking the
real pointer to do it. Not wired up yet.

| | |
|---|---|
| artboard | `New Artboard` |
| state machine | `State Machine 1` |
| trigger | `Click` |
| booleans | `IsHovering`, `isDark`, `isIdle` |

Note the capitalisation: `IsHovering` but `isDark` and `isIdle`. That is how the
file has them, and Rive will silently ignore an input it cannot find.

There are two click transitions inside -- *Click from idle* and *Click from
hover* -- so firing `Click` looks right whether or not `IsHovering` was set
first.
