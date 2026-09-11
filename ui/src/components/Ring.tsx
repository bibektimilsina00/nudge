import type { Act, Point } from "../lib/nudge";

/**
 * The nudge itself: a pulsing ring around the control.
 *
 * The ring says which *kind* of action, because the bubble's words are not where
 * the user is looking. A double click gets a second ring half a cycle behind the
 * first; a hover target is dashed and still, since nothing is going to be pressed
 * and a pulse reads as "click me".
 */
export function Ring({ at, act = "click" }: { at: Point; act?: Act }) {
  if (act === "hover") {
    return (
      <Pulse
        at={at}
        className="animate-none border-dashed opacity-90 motion-safe:animate-[spin_9s_linear_infinite]"
      />
    );
  }
  return (
    <>
      <Pulse at={at} />
      {act === "doubleClick" && <Pulse at={at} className="[animation-delay:800ms]" />}
    </>
  );
}

function Pulse({ at, className = "" }: { at: Point; className?: string }) {
  return (
    <div
      aria-hidden
      className={[
        "pointer-events-none fixed size-19 -translate-x-1/2 -translate-y-1/2",
        "animate-ring rounded-full border-[3px] border-accent",
        "shadow-[0_0_0_1px_#ffffff66,0_0_28px_var(--color-accent)]",
        className,
      ].join(" ")}
      style={{ left: at.x, top: at.y }}
    />
  );
}
