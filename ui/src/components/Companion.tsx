import { useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { Alignment, Fit, Layout, RuntimeLoader, useRive } from "@rive-app/react-canvas";
// Bundled, not fetched. Rive pulls its WASM from a CDN by default, which the app's
// CSP blocks and which would make the companion vanish offline.
import riveWasm from "@rive-app/canvas/rive.wasm?url";
import cat from "../assets/cat.riv?url";

RuntimeLoader.setWasmUrl(riveWasm);

export type CompanionMode = "idle" | "listening" | "thinking";

/** How far behind the cursor the companion trails, per frame. Lower = looser. */
const FOLLOW = 0.2;
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
        // Offset from the pointer, not on it: close enough to belong to the
        // cursor, clear enough never to cover what you are about to click.
        shell.current.style.transform = `translate3d(${shown.x + 12}px, ${shown.y + 11}px, 0)`;
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
