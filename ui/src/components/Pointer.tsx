import { useEffect } from "react";
import { Alignment, Fit, Layout, useRive, useStateMachineInput } from "@rive-app/react-canvas";
import pointer from "../assets/pointer.riv?url";
import type { Act, Point } from "../lib/nudge";

/**
 * A hand that does the clicking, so the real pointer does not have to be watched
 * doing it.
 *
 * The pointer still moves -- a click goes to whatever is under the cursor and
 * macOS offers no general way around that, which was measured rather than
 * assumed. What it does now is go and come straight back, in about 150ms. This
 * is what that 150ms looks like: something deliberate arriving at the control
 * and pressing it, instead of a cursor twitching across the screen.
 */
const MACHINE = "State Machine 1";

/** Where the hand's fingertip sits inside its own box, as a fraction. */
const TIP = { x: 0.3, y: 0.22 };
const SIZE = 56;

export function Pointer({
  at,
  act = "click",
  following = false,
}: {
  at: Point;
  act?: Act;
  /** Riding the real cursor rather than sitting on a target. Debug only. */
  following?: boolean;
}) {
  const { rive, RiveComponent } = useRive({
    src: pointer,
    stateMachines: MACHINE,
    autoplay: true,
    layout: new Layout({ fit: Fit.Contain, alignment: Alignment.Center }),
  });

  // Names read out of the binary -- see `assets/README.md`. Rive returns null
  // for one it cannot find and says nothing, so none of these is required.
  const click = useStateMachineInput(rive, MACHINE, "Click");
  const hovering = useStateMachineInput(rive, MACHINE, "IsHovering");
  const idle = useStateMachineInput(rive, MACHINE, "isIdle");
  const dark = useStateMachineInput(rive, MACHINE, "isDark");

  useEffect(() => {
    // A hover target is not going to be pressed, so the hand waits over it
    // rather than pressing. It is the one case where the pointer genuinely has
    // to stay put, and it should not look like it clicked.
    if (hovering) hovering.value = act === "hover";
    if (idle) idle.value = false;
    if (dark) dark.value = matchMedia("(prefers-color-scheme: dark)").matches;
  }, [act, hovering, idle, dark]);

  // Once per target. A new point means a new thing being pressed; the same point
  // arriving twice is the loop looking again, not a second click.
  //
  // Never while following. The cursor moves sixty times a second and each move
  // is a new point, so this fired the press animation sixty times a second and
  // the hand never finished one.
  useEffect(() => {
    if (!following && act !== "hover") click?.fire();
  }, [at.x, at.y, act, click, following]);

  return (
    <div
      aria-hidden
      className={[
        "pointer-events-none fixed top-0 left-0",
        // Moving to a target is a journey worth watching, so it eases. Riding
        // the cursor is not: an ease is a lag, and at sixty updates a second it
        // reads as the hand being dragged along behind on a piece of elastic.
        following
          ? ""
          : "motion-safe:transition-transform motion-safe:duration-300 motion-safe:ease-[cubic-bezier(0.23,1,0.32,1)]",
      ].join(" ")}
      style={{
        // `transform`, not `left`/`top`. The old pair moved this by relayout on
        // every frame; a translate is composited and never touches layout.
        transform: `translate3d(${at.x - SIZE * TIP.x}px, ${at.y - SIZE * TIP.y}px, 0)`,
        width: SIZE,
        height: SIZE,
      }}
    >
      <RiveComponent />
    </div>
  );
}
