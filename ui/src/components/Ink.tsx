import { useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";

/**
 * The line you draw while you are talking.
 *
 * Circle a thing and say "what is this". The mark is not decoration: the same
 * points are drawn into the screenshot the model is given -- see `core/screen/ink`
 * -- so the gesture is part of the question rather than a flourish over it.
 *
 * It fades from the tail, which is what makes it feel like ink rather than a
 * drawing program: nothing to undo, nothing to clean up, and a circle drawn
 * three seconds ago is gone by the time the answer arrives.
 */
/** How long a piece of the line lives. Long enough to finish a circle and see
 *  it whole, short enough that the screen is clean before the answer lands. */
const LIFE = 1600;
/** At the head, in points. The tail thins towards a third of it, the way a brush
 *  leaves a stroke rather than a cable. */
const NIB = 8;
/** Below this, the pointer is resting rather than drawing, and sampling it adds
 *  a wobble that reads as a shaky hand. */
const MOVED = 2;
/** How much of each new point to believe.
 *
 *  A pointer is reported in whole pixels, so a slow diagonal arrives as a
 *  staircase and a fast one as a scatter. Each point is pulled part of the way
 *  towards the raw reading instead of taken at it, which is a one-line low-pass
 *  filter: the line follows the hand without following the quantisation. */
const FOLLOW = 0.42;
/** Age bands the stroke is drawn in.
 *
 *  The fade needs different opacities along one line, and canvas has no such
 *  brush. Drawing every little curve separately gives you it -- and a seam at
 *  every joint, each one painted twice by the round caps either side, which is
 *  what made the line look beaded rather than drawn. A handful of bands, each a
 *  single continuous path sharing its end point with the next, has the same fade
 *  and a fraction of the seams. */
const BANDS = 7;
/** A gap this long is the pen lifted: two marks, not one line joining them. */
const LIFT = 220;

type Dab = { x: number; y: number; at: number };

export function Ink() {
  const canvas = useRef<HTMLCanvasElement | null>(null);
  const dabs = useRef<Dab[]>([]);
  /** Told by the key watcher, not inferred from the phase: "listening" lasts
   *  until the answer arrives, and the pen has to come up with the key. */
  const down = useRef(false);

  useEffect(() => {
    const subs = [
      listen<boolean>("drawing", (e) => {
        down.current = e.payload;
        // A new gesture starts on clean paper. What is still fading belongs to
        // the last question.
        if (e.payload) dabs.current = [];
      }),
      // The same 60Hz stream the companion follows, already in global points --
      // the overlay never sees a real mouse event, being click-through.
      listen<[number, number]>("cursor", (e) => {
        if (!down.current) return;
        const [x, y] = e.payload;
        const last = dabs.current[dabs.current.length - 1];
        if (last && Math.hypot(x - last.x, y - last.y) < MOVED) return;
        // Pulled part of the way towards the reading rather than placed on it.
        const smoothed = last
          ? {
              x: last.x + (x - last.x) * FOLLOW,
              y: last.y + (y - last.y) * FOLLOW,
              at: performance.now(),
            }
          : { x, y, at: performance.now() };
        dabs.current.push(smoothed);
      }),
    ];

    let frame = 0;
    const paint = () => {
      frame = requestAnimationFrame(paint);
      const el = canvas.current;
      const ctx = el?.getContext("2d");
      if (!el || !ctx) return;

      const now = performance.now();
      // Dropped from the front, so the tail disappears first and the head stays
      // under the pointer.
      dabs.current = dabs.current.filter((d) => now - d.at < LIFE);

      const dpr = window.devicePixelRatio || 1;
      const [w, h] = [window.innerWidth, window.innerHeight];
      if (el.width !== w * dpr || el.height !== h * dpr) {
        el.width = w * dpr;
        el.height = h * dpr;
        el.style.width = `${w}px`;
        el.style.height = `${h}px`;
      }
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      ctx.clearRect(0, 0, w, h);

      const points = dabs.current;
      if (points.length < 3) return;

      ctx.lineCap = "round";
      ctx.lineJoin = "round";
      // A soft halo, so the line reads over a white document and a dark one
      // without changing colour.
      ctx.shadowColor = "rgba(255, 45, 85, 0.45)";
      ctx.shadowBlur = 10;

      // One band at a time, oldest first, so the fresh end is painted over the
      // stale one where they meet.
      const per = Math.ceil(points.length / BANDS);
      for (let start = 0; start < points.length - 1; start += per) {
        // One point of overlap, or the bands would be separate strokes with a
        // gap between them.
        const band = points.slice(start, Math.min(start + per + 1, points.length));
        if (band.length < 2) continue;

        const life = 1 - (now - band[band.length - 1].at) / LIFE;
        if (life <= 0) continue;
        // Eased rather than linear: a line that fades straight down looks like
        // it is being switched off, and this is meant to look like it is drying.
        ctx.strokeStyle = `rgba(255, 45, 85, ${Math.min(1, life * life * 0.95)})`;
        ctx.lineWidth = NIB * (0.35 + 0.65 * life);

        ctx.beginPath();
        ctx.moveTo(band[0].x, band[0].y);
        for (let i = 0; i < band.length - 1; i++) {
          const here = band[i];
          const next = band[i + 1];
          // The pen came up. Finish this path and start another, rather than
          // drawing a line across whatever is between the two marks -- which is
          // usually the thing being pointed at.
          if (next.at - here.at > LIFT) {
            ctx.stroke();
            ctx.beginPath();
            ctx.moveTo(next.x, next.y);
            continue;
          }
          // Through the midpoints, with the sample itself as the control point:
          // the standard way to draw a stroke through samples, and the reason
          // the curve is continuous where two of them meet.
          const mx = (here.x + next.x) / 2;
          const my = (here.y + next.y) / 2;
          ctx.quadraticCurveTo(here.x, here.y, mx, my);
        }
        // The last sample is a control point with nothing after it, so the path
        // stops half a segment short without this.
        const end = band[band.length - 1];
        ctx.lineTo(end.x, end.y);
        ctx.stroke();
      }
    };
    frame = requestAnimationFrame(paint);

    return () => {
      cancelAnimationFrame(frame);
      subs.forEach((p) => void p.then((un) => un()));
    };
  }, []);

  return <canvas ref={canvas} aria-hidden className="pointer-events-none fixed inset-0" />;
}
