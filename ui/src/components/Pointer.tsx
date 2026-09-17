import { useEffect, useRef } from "react";
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

/** The flat grey the artboard is published with, per channel. */
const PLATE = 49;
/** How far from it still counts as plate, summed across the three channels. */
const FRINGE = 90;

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

  // The artboard's own background, keyed out.
  //
  // Rive draws the artboard's background colour beneath everything and offers no
  // way to change it: `Artboard` exposes bounds, size and `node()`, and nothing
  // about colour. Measured off a screenshot, the plate is a flat #313131 filling
  // exactly the artboard's 56 points -- the editor's default, left in the file
  // by whoever published it. Nobody noticed because it normally exists for 150ms.
  //
  // So the frame is copied to a second canvas with that colour turned
  // transparent. Proportionally, not as a threshold: a hard cut leaves a grey
  // fringe wherever the hand is antialiased against the plate, so alpha rises
  // with distance from the key and the edge stays soft.
  const source = useRef<HTMLDivElement>(null);
  const keyed = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    let frame = 0;
    const paint = () => {
      frame = requestAnimationFrame(paint);
      const from = source.current?.querySelector("canvas");
      const to = keyed.current;
      if (!from || !to || !from.width) return;
      if (to.width !== from.width || to.height !== from.height) {
        to.width = from.width;
        to.height = from.height;
      }
      const ctx = to.getContext("2d", { willReadFrequently: true });
      if (!ctx) return;
      ctx.clearRect(0, 0, to.width, to.height);
      ctx.drawImage(from, 0, 0);
      const image = ctx.getImageData(0, 0, to.width, to.height);
      const px = image.data;
      for (let i = 0; i < px.length; i += 4) {
        const off =
          Math.abs(px[i] - PLATE) + Math.abs(px[i + 1] - PLATE) + Math.abs(px[i + 2] - PLATE);
        if (off < FRINGE) px[i + 3] = Math.round(px[i + 3] * (off / FRINGE));
      }
      ctx.putImageData(image, 0, 0);
    };
    frame = requestAnimationFrame(paint);
    return () => cancelAnimationFrame(frame);
  }, []);

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
      {/* Drawn, but never shown: this is the frame the one below is made from.
          `opacity-0` rather than `hidden`, because a canvas that is not laid out
          is a canvas Rive stops advancing. */}
      <div ref={source} className="absolute inset-0 opacity-0">
        <RiveComponent />
      </div>
      <canvas ref={keyed} className="absolute inset-0 size-full" />
    </div>
  );
}
