# Assets

Rive animations. They are binary, so what they contain is written down here --
the names are case-sensitive and there is no way to read them off the file
without `strings`.

## Adding a companion

Drop the `.riv` in here, add a line to `ui/src/companions.ts`, and it appears in
the picker. Nothing in Rust knows the names -- the chosen one is stored as a
string precisely so that adding art never means editing Rust.

What a new one needs: an artboard that reads at 24px (the socket on the home
page) as well as at full size, and a state machine that idles on its own. The
point of carrying a 1.9MB runtime is that the thing is alive when nobody is
asking it for anything; a file that only plays when driven would be better as a
PNG.

## `cat.riv`

The companion. Used by `components/Companion.tsx`.

| | |
|---|---|
| artboard | `Cat` |
| state machine | `State Machine 1` |
| animations | `Idle`, `Blink` |
| inputs | none |

**No inputs at all**, which is why the companion is driven from the outside --
CSS scales it for listening and thinking, and the transform is rewritten every
frame for the trail and the stretch.

It does contain a rig for looking around: `HeadTurn_IK`, and `Pupil1_TARGET_X/Y`
and `Pupil2_TARGET_X/Y` with limits. Nothing drives them. Synthetic `mousemove`
events at the canvas do nothing -- there is no pointer listener, confirmed in the
editor.

To make the eyes follow, the file needs two Number inputs on `State Machine 1`
(say `lookX` and `lookY`, about -1 to 1) bound to those pupil targets. The
component already computes the direction of travel every frame for the stretch,
so wiring them up afterwards is a few lines.

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

A hand that goes to the control and presses it. Used by `components/Pointer.tsx`
on the overlay.

The real pointer still travels -- a click is delivered to whatever is under the
cursor and macOS offers no general way round it, which was measured rather than
assumed. It goes and comes straight back in about 150ms, hidden while it does,
and this is what is on screen instead.

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
