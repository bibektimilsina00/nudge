import { useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { Alignment, Fit, Layout, useRive } from "@rive-app/react-canvas";
import cat from "../assets/cat.riv?url";

export type CompanionMode = "idle" | "listening" | "thinking";

/** How far behind the cursor the companion trails, per frame. Lower = looser. */
const FOLLOW = 0.2;

/**
 * Where it sits relative to the pointer.
 *
 * Offset from it, never on it: close enough to belong to the cursor, clear
 * enough never to cover the thing you are about to click. Up and to the right,
 * because the pointer's own hotspot is its top-left corner and the arrow hangs
 * down and left of it -- sitting below meant sitting behind the arrow.
 */
const BESIDE = { x: 22, y: 5 };

/**
 * How far the eyes are asked to look, as a fraction of the cat's own size.
 *
 * The rig inside the file has `HeadTurn_IK` and a target for each pupil, driven
 * by a pointer listener -- the cat is built to watch your mouse when your mouse
 * is over it. Ours never is: the overlay is click-through, and the canvas is
 * fourteen pixels of a cat that follows the cursor rather than being visited by
 * it. So it is told where to look, with a pointer event it never received.
 *
 * Past the edge of the canvas rather than inside it, because the IK clamps at
 * its own limits: a target out here reads as a glance, one near the middle reads
 * as a stare.
 */
const GAZE = 1.6;
/** Lag converted to stretch. The further behind it is, the more it deforms. */
const STRETCH = 0.02;
const MAX_STRETCH = 0.22;
const SNAP = "cubic-bezier(0.23, 1, 0.32, 1)";

/**
 * The companion: a cat that trails the cursor.
 *
 * The cat is a running Rive state machine, not a rendered frame -- it idles and
 * blinks on its own, which is the entire reason to carry a 1.9MB runtime. Freezing
 * it into an image would cost the same and buy a sticker.
 *
 * It does not report state. Listening, thinking and speaking are the notch's job
 * now, and the ripples and waveform that used to ring the cursor were a second
 * copy of the same information in the corner of your eye.
 */
export function Companion({
  mode,
  anchored = false,
}: {
  mode: CompanionMode;
  /** Sit still and centred, for the panel, instead of chasing the cursor. */
  anchored?: boolean;
}) {
  const listening = mode === "listening";
  const shell = useRef<HTMLDivElement>(null);
  const body = useRef<HTMLDivElement>(null);

  const { RiveComponent } = useRive({
    src: cat,
    // Run the machine, do not just play a timeline: this is what gives the idle
    // its own life instead of a loop we drive.
    stateMachines: "State Machine 1",
    autoplay: true,
    layout: new Layout({ fit: Fit.Contain, alignment: Alignment.Center }),
  });

  useEffect(() => {
    if (anchored) return;
    const target = { x: -200, y: -200 };
    const shown = { x: -200, y: -200 };
    let frame = 0;

    const subs = [
      listen<[number, number]>("cursor", (e) => {
        [target.x, target.y] = e.payload;
      }),
      // Fade out while the pointer is at rest -- which is every case where an app
      // hides it: video, presentations, an editor while you type. Written straight
      // to the node like the transform above, so it never costs a React render.
      listen<boolean>("cursor-visible", (e) => {
        if (body.current) body.current.style.opacity = e.payload ? "1" : "0";
      }),
    ];

    const tick = () => {
      frame = requestAnimationFrame(tick);

      const dx = target.x - shown.x;
      const dy = target.y - shown.y;
      shown.x += dx * FOLLOW;
      shown.y += dy * FOLLOW;
      if (shell.current) {
        shell.current.style.transform = `translate3d(${shown.x + BESIDE.x}px, ${shown.y + BESIDE.y}px, 0)`;
      }

      // Stretch along travel, pinch across it -- that pairing keeps the volume
      // looking constant instead of the thing merely scaling up. Kept gentler than
      // for a vector bead: this is a canvas, and heavy scaling shows as softness.
      const speed = Math.hypot(dx, dy);
      const s = Math.min(MAX_STRETCH, speed * STRETCH);
      if (body.current) {
        const angle = (Math.atan2(dy, dx) * 180) / Math.PI;
        body.current.style.transform =
          s < 0.004
            ? ""
            : `rotate(${angle}deg) scale(${1 + s}, ${1 - s * 0.6}) rotate(${-angle}deg)`;
      }

      // Look where it is going.
      //
      // The cat trails the cursor, so the line from where it is to where the
      // cursor is *is* the direction of travel. No separate notion of facing is
      // needed, and it stays right when the pointer stops: the two converge, the
      // reach falls below the threshold, and the gaze settles wherever it was.
      const canvas = shell.current?.querySelector("canvas");
      if (canvas && speed > 0.4) {
        const box = canvas.getBoundingClientRect();
        const reach = speed || 1;
        canvas.dispatchEvent(
          new MouseEvent("mousemove", {
            bubbles: true,
            clientX: box.left + box.width / 2 + (dx / reach) * box.width * GAZE,
            clientY: box.top + box.height / 2 + (dy / reach) * box.height * GAZE,
          }),
        );
      }
    };
    frame = requestAnimationFrame(tick);

    return () => {
      cancelAnimationFrame(frame);
      subs.forEach((p) => void p.then((un) => un()));
    };
  }, [anchored]);

  return (
    <div
      ref={shell}
      aria-hidden
      className={
        anchored
          ? "relative grid place-items-center"
          : "pointer-events-none fixed top-0 left-0"
      }
    >
      <div
        ref={body}
        // Only opacity transitions: the transform is rewritten every frame and
        // must not be interpolated on top of that.
        className="origin-center transition-opacity duration-300 ease-[cubic-bezier(0.23,1,0.32,1)] will-change-transform"
      >
        <div className="absolute -translate-x-1/2 -translate-y-1/2">
          <div className="relative grid size-14 place-items-center">
            <div
              style={{ transitionTimingFunction: SNAP }}
              className={[
                "size-14 transition-[transform,filter] duration-300",
                // The cat has no background of its own, so it needs a shadow to sit
                // on top of a document rather than float in front of nothing.
                "[filter:drop-shadow(0_2px_4px_rgba(0,0,0,0.45))]",
                listening ? "scale-110" : mode === "thinking" ? "motion-safe:animate-think" : "",
              ].join(" ")}
            >
              <RiveComponent className="size-full" />
            </div>

          </div>
        </div>
      </div>
    </div>
  );
}
