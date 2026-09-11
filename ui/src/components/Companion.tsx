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
const WAVE = { x0: 2, x1: 30, mid: 12, points: 13, amplitude: 8 };
const SNAP = "cubic-bezier(0.23, 1, 0.32, 1)";

/**
 * The companion: a cat that trails the cursor, and a live waveform while you talk.
 *
 * The cat is a running Rive state machine, not a rendered frame -- it idles and
 * blinks on its own, which is the entire reason to carry a 1.9MB runtime. Freezing
 * it into an image would cost the same and buy a sticker.
 *
 * Listening is shown *around* the cat rather than by replacing it: ripples behind,
 * and a waveform driven by the real microphone level from Rust. Swapping the cat
 * out for a microphone glyph would throw away the one thing that makes the
 * companion recognisable between states.
 */
export function Companion({ mode }: { mode: CompanionMode }) {
  const listening = mode === "listening";
  const shell = useRef<HTMLDivElement>(null);
  const body = useRef<HTMLDivElement>(null);
  const wave = useRef<SVGPathElement>(null);

  const { RiveComponent } = useRive({
    src: cat,
    // Run the machine, do not just play a timeline: this is what gives the idle
    // its own life instead of a loop we drive.
    stateMachines: "State Machine 1",
    autoplay: true,
    layout: new Layout({ fit: Fit.Contain, alignment: Alignment.Center }),
  });

  useEffect(() => {
    const target = { x: -200, y: -200 };
    const shown = { x: -200, y: -200 };
    const level = { raw: 0, shown: 0 };
    let phase = 0;
    let frame = 0;

    const subs = [
      listen<[number, number]>("cursor", (e) => {
        [target.x, target.y] = e.payload;
      }),
      listen<number>("level", (e) => {
        level.raw = Math.min(1, Math.max(0, e.payload));
      }),
    ];

    const tick = () => {
      frame = requestAnimationFrame(tick);

      const dx = target.x - shown.x;
      const dy = target.y - shown.y;
      shown.x += dx * FOLLOW;
      shown.y += dy * FOLLOW;
      if (shell.current) {
        shell.current.style.transform = `translate3d(${shown.x + 20}px, ${shown.y + 20}px, 0)`;
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

      if (wave.current) {
        // Rises fast, falls slowly -- how a real VU meter behaves, and why it reads
        // as sound rather than as a slider being dragged.
        const up = level.raw > level.shown;
        level.shown += (level.raw - level.shown) * (up ? 0.35 : 0.07);
        phase += 0.19;

        const { x0, x1, mid, points, amplitude } = WAVE;
        const step = (x1 - x0) / (points - 1);
        const amp = level.shown * amplitude;
        const d = Array.from({ length: points }, (_, i) => {
          const t = i / (points - 1);
          // Tapered, so it grows out of a flat line instead of hinging off its ends.
          const y = mid + Math.sin(phase + i * 0.9) * amp * Math.sin(t * Math.PI);
          return `${i ? "L" : "M"}${(x0 + i * step).toFixed(2)} ${y.toFixed(2)}`;
        }).join(" ");
        wave.current.setAttribute("d", d);
      }
    };
    frame = requestAnimationFrame(tick);

    return () => {
      cancelAnimationFrame(frame);
      subs.forEach((p) => void p.then((un) => un()));
    };
  }, []);

  return (
    <div ref={shell} aria-hidden className="pointer-events-none fixed top-0 left-0">
      <div ref={body} className="origin-center will-change-transform">
        <div className="absolute -translate-x-1/2 -translate-y-1/2">
          <div className="relative grid size-14 place-items-center">
            {listening && (
              <>
                <Ripple />
                {/* Half a cycle behind, so a ring is always in flight. */}
                <Ripple className="[animation-delay:950ms]" />
              </>
            )}

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

            {/* Sits under the cat, like something it is saying. */}
            <svg
              viewBox="0 0 32 24"
              className="absolute -bottom-5 left-1/2 w-14 -translate-x-1/2 overflow-visible"
              style={{
                transition: `opacity 220ms ${SNAP} ${listening ? "120ms" : "0ms"}`,
                opacity: listening ? 1 : 0,
              }}
            >
              <defs>
                <linearGradient id="waveInk" x1="0" x2="1">
                  <stop offset="0%" stopColor="var(--color-accent)" stopOpacity="0.1" />
                  <stop offset="38%" stopColor="#fff" />
                  <stop offset="100%" stopColor="var(--color-accent)" stopOpacity="0.1" />
                </linearGradient>
              </defs>
              <path
                ref={wave}
                d={`M${WAVE.x0} ${WAVE.mid} L${WAVE.x1} ${WAVE.mid}`}
                fill="none"
                stroke="url(#waveInk)"
                strokeWidth="2.4"
                strokeLinecap="round"
                className="[filter:drop-shadow(0_0_5px_rgba(255,45,85,0.85))]"
              />
            </svg>
          </div>
        </div>
      </div>
    </div>
  );
}

/** Sound leaving the cat: expands and dissipates, never loops back inward. */
function Ripple({ className = "" }: { className?: string }) {
  return (
    <span
      className={`absolute size-14 rounded-full border border-accent motion-safe:animate-ripple ${className}`}
    />
  );
}
