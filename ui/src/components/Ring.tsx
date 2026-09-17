import type { Act, Point } from "../lib/nudge";

/**
 * The nudge itself: a pulsing ring around the control, and its name.
 *
 * The ring says which *kind* of action, because the bubble's words are not where
 * the user is looking. A double click gets a second ring half a cycle behind the
 * first; a hover target is dashed and still, since nothing is going to be pressed
 * and a pulse reads as "click me".
 */
export function Ring({
  at,
  act = "click",
  control,
}: {
  at: Point;
  act?: Act;
  /** What the thing is called, when the system knew. */
  control?: string | null;
}) {
  return (
    <>
      {act === "hover" ? (
        <Pulse
          at={at}
          className="animate-none border-dashed opacity-90 motion-safe:animate-[spin_9s_linear_infinite]"
        />
      ) : (
        <>
          <Pulse at={at} />
          {act === "doubleClick" && <Pulse at={at} className="[animation-delay:800ms]" />}
        </>
      )}
      {control && <Name at={at} text={control} />}
    </>
  );
}

/**
 * The control's name, on the ring.
 *
 * A ring says *there*; it does not say *what*. The sentence naming the thing is
 * spoken aloud and is gone by the time anybody has finished looking, and it used
 * to be repeated in a bubble at the foot of the screen -- which is a caption
 * about something a thousand points away from where the eye already is.
 *
 * Above the ring rather than below, because below is where a pointer sits and
 * where most controls keep their own labels and tooltips.
 */
function Name({ at, text }: { at: Point; text: string }) {
  return (
    <div
      aria-hidden
      className="pointer-events-none fixed -translate-x-1/2 -translate-y-full"
      // 46: the ring's own radius plus a little air.
      style={{ left: at.x, top: at.y - 46 }}
    >
      <span
        className={[
          "block max-w-[15rem] truncate rounded-full px-2.5 py-[3px]",
          "bg-accent text-[11.5px] font-semibold text-white",
          // The same halo the ring carries, so the two read as one mark.
          "shadow-[0_0_0_1px_#ffffff59,0_2px_12px_rgba(0,0,0,0.45)]",
        ].join(" ")}
      >
        {text}
      </span>
    </div>
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
