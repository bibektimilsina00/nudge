import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Alignment, Fit, Layout, useRive } from "@rive-app/react-canvas";
import { DEFAULT_LOOK, lookUp, type Look } from "../companions";

export type CompanionMode = "idle" | "listening" | "thinking";

/** How far behind the cursor the companion trails, per frame. Lower = looser. */
const FOLLOW = 0.2;
/** Below this, in points, the cat has arrived and there is nothing to rewrite. */
const SETTLED = 0.05;

/**
 * Where it sits relative to the pointer.
 *
 * Offset from it, never on it: close enough to belong to the cursor, clear
 * enough never to cover the thing you are about to click. Up and to the right,
 * because the pointer's own hotspot is its top-left corner and the arrow hangs
 * down and left of it -- sitting below meant sitting behind the arrow.
 */
const BESIDE = { x: 22, y: 5 };

/*
 * The eyes do not follow anything yet, and this is what it would take.
 *
 * The file has the rig for it -- `HeadTurn_IK`, and `Pupil1_TARGET_X/Y`,
 * `Pupil2_TARGET_X/Y` with limits -- but nothing in the state machine is
 * listening. A first attempt dispatched synthetic `mousemove` events at the
 * canvas, on the theory that the rig was wired to a pointer listener the way
 * these files usually are. It is not: checked in the Rive editor, the eyes never
 * move.
 *
 * Only the editor can fix that, because inputs live inside the binary. What is
 * needed is two Number inputs on `State Machine 1` -- call them `lookX` and
 * `lookY`, roughly -1 to 1 -- bound to those pupil targets. Then this component
 * sets them from `dx`/`dy`, which it already computes every frame for the
 * stretch, and the cat looks where it is going for four more lines.
 */
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
  // Kept in a ref, not state: the loop reads it every frame and a render per
  // change would defeat the point of writing to the nodes directly.
  const stretched = useRef(false);
  const shell = useRef<HTMLDivElement>(null);
  const body = useRef<HTMLDivElement>(null);

  // Which character to wear. Asked for once and then listened for, because the
  // window that changes it is not this one -- settings live in the panel and the
  // companion is usually on the overlay.
  const [key, setKey] = useState(DEFAULT_LOOK);
  useEffect(() => {
    void invoke<string>("look").then(setKey).catch(() => {});
    const sub = listen<string>("look", (e) => setKey(e.payload));
    return () => void sub.then((un) => un());
  }, []);
  const look = lookUp(key);

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

      // The chase does not stop for the overview.
      //
      // It did, and the cure was worse than the complaint: a cat that holds
      // still while the pointer moves does not read as "considerate", it reads
      // as stuck. Clicky keeps following throughout, which is the whole reason
      // it feels alive there.
      //
      // What stops instead is work that changes nothing -- the canvas, which is
      // paused below, and the transform, which is written only when the thing
      // actually moved. Following a moving pointer costs a repaint per frame
      // and always will; that one is the point.

      const dx = target.x - shown.x;
      const dy = target.y - shown.y;
      shown.x += dx * FOLLOW;
      shown.y += dy * FOLLOW;
      // Only when it actually moved. Settled, the loop was rewriting an
      // identical transform every frame for as long as the cursor stayed put,
      // which is a repaint for no change at all.
      const moved = Math.abs(dx) > SETTLED || Math.abs(dy) > SETTLED;
      if (shell.current && moved) {
        shell.current.style.transform = `translate3d(${shown.x + BESIDE.x}px, ${shown.y + BESIDE.y}px, 0)`;
      }

      // Stretch along travel, pinch across it -- that pairing keeps the volume
      // looking constant instead of the thing merely scaling up. Kept gentler than
      // for a vector bead: this is a canvas, and heavy scaling shows as softness.
      const speed = Math.hypot(dx, dy);
      const s = Math.min(MAX_STRETCH, speed * STRETCH);
      if (body.current && (moved || stretched.current)) {
        stretched.current = s >= 0.004;
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
              {/* Keyed, so changing character mounts a fresh machine. `useRive`
                  reads its options once; handed a new `src` in place it keeps
                  playing the old file and says nothing. */}
              <Skin key={look.key} look={look} />
            </div>

          </div>
        </div>
      </div>
    </div>
  );
}

/**
 * One character, running.
 *
 * Its own component only so that `key` can force it to rebuild -- see the call
 * site. Nothing else belongs in here: the trail, the stretch and the fade are all
 * written straight to the nodes outside, every frame, and must not go through
 * React at all.
 */
function Skin({ look }: { look: Look }) {
  const { rive, RiveComponent } = useRive({
    src: look.src,
    // Run the machine, do not just play a timeline: this is what gives the idle
    // its own life instead of a loop we drive.
    stateMachines: look.machine,
    autoplay: true,
    layout: new Layout({ fit: Fit.Contain, alignment: Alignment.Center }),
  });

  // Still while the overview is up.
  //
  // This is a WebGL canvas redrawing sixty times a second so the cat is alive
  // when nobody is asking it for anything, which is the whole reason for
  // carrying the runtime. Mission Control is already compositing an animation
  // of every window on the machine, and two more live surfaces on top of that
  // is the one thing here that Clicky does not do -- its companion is SwiftUI
  // drawing native shapes, not a canvas.
  //
  // Paused, not hidden. It stays exactly where it is and stops costing frames.
  useEffect(() => {
    if (!rive) return;
    const sub = listen<boolean>("overview", (e) => {
      if (e.payload) rive.pause();
      else rive.play();
    });
    return () => {
      void sub.then((off) => off());
    };
  }, [rive]);

  return <RiveComponent className="size-full" />;
}
